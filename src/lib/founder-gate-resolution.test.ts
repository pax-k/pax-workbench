import { describe, expect, it } from "vitest";
import { buildFounderGateDrafts, validateFounderGateInputs } from "./founder-gate-resolution";

describe("founder gate resolution", () => {
  it("requires substantive context and explicit scope confirmation", () => {
    expect(validateFounderGateInputs({ context: "short", scopeConfirmed: true })).toMatch(/20/);
    expect(validateFounderGateInputs({ context: "A sufficiently detailed founder context.", scopeConfirmed: false })).toMatch(/Confirm/);
  });

  it("builds one create and two version-bound updates without upgrading customer evidence", () => {
    const drafts = buildFounderGateDrafts(
      { context: "The founder confirms one local checklist workflow.", scopeConfirmed: true },
      { path: "docs/mvp-scope.md", content: "## Validation Required Before Product Truth\n", version: "sha256:mvp" },
      { path: "docs/blueprint-status.md", content: "MVP | needs-validation\n", version: "sha256:blueprint" },
      "2026-07-23",
    );
    expect(drafts.map((draft) => draft.path)).toEqual([
      "docs/raw/founder-dump.md",
      "docs/mvp-scope.md",
      "docs/blueprint-status.md",
    ]);
    expect(drafts[0].content).toContain("not customer validation");
    expect(drafts[1]).toMatchObject({ expectedVersion: "sha256:mvp" });
    expect(drafts[1].content).not.toContain("Validation Required Before Product Truth");
    expect(drafts[2].content).not.toContain("needs-validation");
  });
});
