import type {
  BoundedTaskResult,
  GoalRecovery,
  PostRunReviewEvidence,
  SharedBoundedTaskResult,
} from "../types";

export type ReviewReceiptTone =
  | "completed"
  | "failed"
  | "blocked"
  | "cancelled"
  | "partial";

export interface ReviewCriterion {
  text: string;
  passed: boolean;
}

export interface ReviewCheck {
  label: string;
  result: "pass" | "fail" | "unknown";
}

export interface ReviewReceipt {
  tone: ReviewReceiptTone;
  headline: string;
  reason: string;
  selectedTask: string | null;
  repositoryVerified: boolean;
  changedFiles: PostRunReviewEvidence["changedFiles"];
  changeScopeNote: string;
  changeEvidenceUnavailable: string | null;
  criteria: ReviewCriterion[];
  checks: ReviewCheck[];
  tracker: {
    selectedTaskStatus: string;
    loopState: string;
    nextTask: string | null;
    nextReason: string;
  };
  risks: string[];
  shared: {
    access: string;
    sourceTask: string;
    sourceHash: string;
    claim: string;
    completion: string;
    repairState: string;
    codexStarted: boolean;
  } | null;
  rawEvents: Array<{ sequence: number; kind: string; summary: string }>;
}

const MAX_TEXT = 8_192;
const MAX_ITEMS = 100;

export function redactReviewText(value: string): string {
  const withoutAnsi = value.replace(/\u001B(?:\[[0-?]*[ -/]*[@-~]|[@-_])/gu, "");
  const bounded = withoutAnsi.slice(0, MAX_TEXT);
  return bounded
    .split(/\r?\n/u)
    .map((line) => {
      const lower = line.toLowerCase();
      const sensitive = [
        "authorization:",
        "bearer ",
        "password",
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "client_secret",
        "capability",
      ].some((marker) => lower.includes(marker))
        || ((lower.includes("https://") || lower.includes("http://")) && lower.includes("?"));
      if (sensitive) {
        const diffPrefix = /^[+\- ]/u.test(line) ? line[0] : "";
        return `${diffPrefix}[REDACTED sensitive line]`;
      }
      return line.replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/gu, "");
    })
    .join("\n");
}

function section(markdown: string, heading: string): string {
  const lines = markdown.split(/\r?\n/u);
  const start = lines.findIndex(
    (line) => line.trim().toLowerCase() === `## ${heading.toLowerCase()}`,
  );
  if (start < 0) return "";
  const end = lines.findIndex((line, index) => index > start && /^##\s+/u.test(line.trim()));
  return lines.slice(start + 1, end < 0 ? undefined : end).join("\n");
}

function taskStatus(markdown: string): string {
  return markdown.match(/^Status:\s*(.+)$/imu)?.[1]?.trim() ?? "unavailable";
}

function criteria(markdown: string): ReviewCriterion[] {
  return section(markdown, "Acceptance Criteria")
    .split(/\r?\n/u)
    .map((line) => line.match(/^\s*-\s+\[([ xX])\]\s+(.+)$/u))
    .filter((match): match is RegExpMatchArray => Boolean(match))
    .slice(0, MAX_ITEMS)
    .map((match) => ({
      passed: match[1].toLowerCase() === "x",
      text: redactReviewText(match[2].trim()),
    }));
}

function checks(markdown: string): ReviewCheck[] {
  const evidence = section(markdown, "Evidence Log");
  const rows = evidence
    .split(/\r?\n/u)
    .filter((line) => line.trim().startsWith("|") && !/^\|\s*[-:]+\s*\|/u.test(line.trim()))
    .map((line) => line.split("|").map((cell) => cell.trim()).filter(Boolean))
    .filter((cells) => cells.length >= 2 && cells[0].toLowerCase() !== "date");
  return rows.slice(0, MAX_ITEMS).map((cells) => {
    const joined = cells.slice(1).join(" ").toLowerCase();
    return {
      label: redactReviewText(cells[0]),
      result: /\bpass(?:ed)?\b/u.test(joined)
        ? "pass"
        : /\bfail(?:ed|ure)?\b/u.test(joined)
          ? "fail"
          : "unknown",
    };
  });
}

function risks(markdown: string): string[] {
  return ["Risks", "Follow-Ups", "Blockers"]
    .flatMap((heading) =>
      section(markdown, heading)
        .split(/\r?\n/u)
        .filter((line) => /^\s*-\s+/u.test(line))
        .map((line) => redactReviewText(line.replace(/^\s*-\s+/u, "").trim())),
    )
    .filter((line) => line && line.toLowerCase() !== "none.")
    .slice(0, MAX_ITEMS);
}

function tone(result: BoundedTaskResult): ReviewReceiptTone {
  if (result.loopState.state === "cancelledStop") return "cancelled";
  if (result.outcome === "verified" && result.repositoryVerified) return "completed";
  if (result.outcome === "verificationFailed" || result.loopState.state === "failureStop") {
    return "failed";
  }
  if (result.outcome === "waitExternal" || result.loopState.blockingGates.length > 0) {
    return "blocked";
  }
  return "partial";
}

function sharedProjection(
  shared: SharedBoundedTaskResult | null,
  recovery: GoalRecovery | null,
): ReviewReceipt["shared"] {
  if (!shared && !recovery?.collaboration) return null;
  const cursor = recovery?.collaboration;
  const binding = shared?.binding;
  const claim = shared
    ? shared.claim.status === "claimed" || shared.claim.status === "claimedRepairRequired"
      ? `${shared.claim.status} at remote version ${shared.claim.remoteVersion}`
      : shared.claim.status
    : cursor
      ? `recovered claim at remote version ${cursor.intent.claimedTaskVersion}`
      : "unavailable";
  return {
    access: binding?.session.access ?? cursor?.intent.access ?? "unavailable",
    sourceTask: binding?.local.taskPath ?? cursor?.intent.localTaskPath ?? "unavailable",
    sourceHash: binding?.local.taskSha256 ?? cursor?.intent.localTaskSha256 ?? "unavailable",
    claim,
    completion: cursor?.state ?? shared?.completion.status ?? "unavailable",
    repairState: cursor?.missingEffects.length
      ? `${cursor.missingEffects.length} shared effect(s) missing`
      : shared?.sharedIterationBlocked
        ? "shared continuation blocked"
        : "reconciled or not required",
    codexStarted: shared?.codexStarted ?? false,
  };
}

export function deriveReviewReceipt(input: {
  result: BoundedTaskResult;
  gitEvidence: PostRunReviewEvidence | null;
  gitEvidenceFailure: string | null;
  sharedResult: SharedBoundedTaskResult | null;
  recovery: GoalRecovery | null;
}): ReviewReceipt {
  const markdown = input.result.taskEvidence?.content ?? "";
  const resultTone = tone(input.result);
  const changedFiles = input.gitEvidence?.changedFiles.slice(0, MAX_ITEMS).map((file) => ({
    ...file,
    path: redactReviewText(file.path),
    status: redactReviewText(file.status),
    diff: file.diff ? redactReviewText(file.diff) : null,
    diffUnavailableReason: file.diffUnavailableReason
      ? redactReviewText(file.diffUnavailableReason)
      : null,
  })) ?? [];
  return {
    tone: resultTone,
    headline:
      resultTone === "completed"
        ? "Repository verification passed"
        : resultTone === "cancelled"
          ? "Run cancelled"
          : resultTone === "blocked"
            ? "Run stopped on a blocker"
            : resultTone === "failed"
              ? "Repository verification failed"
              : "Run ended with partial evidence",
    reason: redactReviewText(input.result.reason),
    selectedTask: input.result.selectedTask,
    repositoryVerified: input.result.repositoryVerified,
    changedFiles,
    changeScopeNote: input.gitEvidence?.scopeNote
      ?? "Current Git change evidence was not obtained.",
    changeEvidenceUnavailable: input.gitEvidenceFailure
      ? redactReviewText(input.gitEvidenceFailure)
      : null,
    criteria: criteria(markdown),
    checks: checks(markdown),
    tracker: {
      selectedTaskStatus: taskStatus(markdown),
      loopState: input.result.loopState.state,
      nextTask: input.result.loopState.nextTask,
      nextReason: redactReviewText(input.result.loopState.reason),
    },
    risks: risks(markdown),
    shared: sharedProjection(input.sharedResult, input.recovery),
    rawEvents: (input.result.runtime?.events ?? []).slice(0, MAX_ITEMS).map((event) => ({
      sequence: event.sequence,
      kind: event.kind,
      summary: redactReviewText(event.summary),
    })),
  };
}
