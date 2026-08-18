import { describe, expect, it } from "vitest";
import type {
  BoundedTaskResult,
  PostRunReviewEvidence,
  SharedBoundedTaskResult,
} from "../types";
import { deriveReviewReceipt, redactReviewText } from "./review-receipt";

const result: BoundedTaskResult = {
  outcome: "verified",
  selectedTask: "tasks/issues/028-review.md",
  runtime: null,
  project: {
    root: "/tmp/project",
    name: "project",
    branch: "main",
    dirty: true,
    files: [],
    skills: [],
    errors: [],
  },
  taskEvidence: {
    path: "tasks/issues/028-review.md",
    version: "sha256:task",
    content: `# 028

Status: complete

## Acceptance Criteria

- [x] One receipt exists.
- [ ] Founder trial remains.

## Evidence Log

| Date | Evidence | Result |
| --- | --- | --- |
| now | bun run check | pass |

## Risks

- Not notarized.
`,
  },
  resolver: null,
  stopGates: null,
  refreshFailures: [],
  repositoryVerified: true,
  reason: "Repository evidence passed",
  loopState: {
    state: "continueAvailable",
    nextTask: "tasks/issues/028a.md",
    blockingGates: [],
    expectedEffects: [],
    explicitConfirmationRequired: true,
    automaticExecutionStarted: false,
    reason: "Fresh task selected",
  },
};

const gitEvidence: PostRunReviewEvidence = {
  scopeNote: "Current Git worktree; may include pre-existing changes.",
  truncated: false,
  changedFiles: [{
    path: "src/review.ts",
    status: " M",
    diff: "+safe\n+password=opaque",
    diffUnavailableReason: null,
    truncated: false,
  }],
};

describe("review receipt projection", () => {
  it("links outcome, diff, checks, criteria, tracker, risk, and next decision", () => {
    const receipt = deriveReviewReceipt({
      result,
      gitEvidence,
      gitEvidenceFailure: null,
      sharedResult: null,
      recovery: null,
    });
    expect(receipt).toMatchObject({
      tone: "completed",
      headline: "Repository verification passed",
      criteria: [
        { text: "One receipt exists.", passed: true },
        { text: "Founder trial remains.", passed: false },
      ],
      checks: [{ label: "now", result: "pass" }],
      tracker: {
        selectedTaskStatus: "complete",
        loopState: "continueAvailable",
        nextTask: "tasks/issues/028a.md",
      },
      risks: ["Not notarized."],
    });
    expect(receipt.changedFiles[0].diff).toContain("[REDACTED sensitive line]");
    expect(JSON.stringify(receipt)).not.toContain("opaque");
  });

  it("keeps shared repair debt attached to local completion without claiming Codex repair", () => {
    const shared = {
      binding: {
        session: { access: "collaborator" },
        local: {
          taskPath: result.selectedTask,
          taskSha256: "sha256:local",
        },
      },
      claim: { status: "claimed", remoteVersion: 7, recoveredFromReadback: false },
      completion: { status: "collaborationRepairRequired" },
      codexStarted: true,
      sharedIterationBlocked: true,
    } as unknown as SharedBoundedTaskResult;
    const receipt = deriveReviewReceipt({
      result,
      gitEvidence: null,
      gitEvidenceFailure: "review unavailable",
      sharedResult: shared,
      recovery: null,
    });
    expect(receipt.tone).toBe("completed");
    expect(receipt.shared).toMatchObject({
      access: "collaborator",
      claim: "claimed at remote version 7",
      completion: "collaborationRepairRequired",
      repairState: "shared continuation blocked",
      codexStarted: true,
    });
    expect(receipt.changeEvidenceUnavailable).toBe("review unavailable");
  });

  it("redacts ANSI, control text, capability URLs, and common secret lines", () => {
    expect(redactReviewText("\u001b[31mred\u001b[0m\u0000\nhttps://host.test/?token=opaque\nsafe"))
      .toBe("red\n[REDACTED sensitive line]\nsafe");
  });

  it.each([
    ["completed", "verified", "noReadyTaskStop", true],
    ["failed", "verificationFailed", "failureStop", false],
    ["blocked", "waitExternal", "externalStop", false],
    ["cancelled", "stopped", "cancelledStop", false],
    ["partial", "stopped", "noReadyTaskStop", false],
  ] as const)("keeps %s visually and semantically distinct", (expected, outcome, loopState, verified) => {
    const receipt = deriveReviewReceipt({
      result: {
        ...result,
        outcome,
        repositoryVerified: verified,
        loopState: {
          ...result.loopState,
          state: loopState,
          blockingGates: expected === "blocked" ? ["external gate"] : [],
        },
      },
      gitEvidence: null,
      gitEvidenceFailure: "unavailable",
      sharedResult: null,
      recovery: null,
    });
    expect(receipt.tone).toBe(expected);
  });
});
