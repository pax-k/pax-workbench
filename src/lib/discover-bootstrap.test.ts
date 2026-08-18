import { describe, expect, it } from "vitest";
import type { ProjectSnapshot } from "../types";
import {
  bootstrapArtifactPaths,
  buildBootstrapDrafts,
  deriveBootstrapInventory,
  validateFounderInputs,
} from "./discover-bootstrap";

const blank: ProjectSnapshot = {
  root: "/tmp/blank",
  name: "blank",
  branch: "main",
  dirty: false,
  files: [],
  skills: [],
  errors: [],
};

const answers = {
  productName: "Signal Forge",
  primaryUser: "Independent product founder",
  primaryWorkflow: "Turn a product idea into one verified engineering task",
  valueMoment: "A ready task with explicit evidence",
  hardConstraint: "Repository Markdown remains authoritative",
};

describe("guided Discover bootstrap model", () => {
  it("reports exact missing authority and resumes from repository inventory", () => {
    expect(deriveBootstrapInventory(blank).missingPaths).toEqual(bootstrapArtifactPaths);
    const partial = {
      ...blank,
      files: [
        { path: "AGENTS.md", name: "AGENTS", kind: "instruction" as const },
        { path: "docs/mvp-scope.md", name: "MVP Scope", kind: "document" as const },
      ],
    };
    const inventory = deriveBootstrapInventory(partial);
    expect(inventory.existingPaths).toEqual(["AGENTS.md", "docs/mvp-scope.md"]);
    expect(inventory.missingPaths).not.toContain("docs/mvp-scope.md");
  });

  it("does not fabricate product truth when founder input is absent", () => {
    const empty = Object.fromEntries(
      Object.keys(answers).map((key) => [key, ""]),
    ) as typeof answers;
    expect(validateFounderInputs(empty)).toEqual(Object.keys(answers));
    expect(buildBootstrapDrafts(blank, empty)).toEqual([]);
  });

  it("builds only missing create targets and labels founder claims", () => {
    const drafts = buildBootstrapDrafts(blank, answers, "2026-07-23");
    expect(drafts.map((draft) => draft.path)).toEqual(bootstrapArtifactPaths);
    const interview = drafts.find((draft) => draft.path === "docs/raw/founder-interview.md");
    expect(interview?.content).toContain("founder-claimed");
    expect(interview?.content).toContain("not customer validation");
    expect(JSON.stringify(drafts)).not.toContain("MDSync");
    expect(JSON.stringify(drafts)).not.toContain("HA2HA");
  });
});
