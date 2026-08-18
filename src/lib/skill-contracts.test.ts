import { describe, expect, it } from "vitest";
import preflight from "../../skill-ui/build-right-preflight.json";
import { firstPartySkillSummaries, genericSkillFallback, validateFirstPartyContract } from "./skill-contracts";

describe("first-party skill UI contracts", () => {
  it("exposes validated summaries for every installed Build Right skill", () => {
    expect(firstPartySkillSummaries.map((skill) => skill.id)).toEqual([
      "build-right-preflight",
      "build-right-feature-planning",
      "build-right-execution",
      "build-right-engineering-principles",
    ]);
    expect(firstPartySkillSummaries.every((skill) => skill.renderer === "operating-card" && !skill.executable && !!skill.lockHash)).toBe(true);
  });

  it("uses a viewer-only non-executable fallback for unknown skills", () => {
    const fallback = genericSkillFallback("unknown-skill");
    expect(fallback).toMatchObject({
      phase: "Unknown",
      renderer: "generic-markdown",
      executable: false,
      helpers: [],
      decisions: [],
    });
  });

  it("rejects missing, unknown-version, and executable contract fields", () => {
    const missingRenderer = { ...preflight } as Record<string, unknown>;
    delete missingRenderer.renderer;
    expect(() => validateFirstPartyContract(missingRenderer)).toThrow("renderer");

    expect(() => validateFirstPartyContract({ ...preflight, version: 2 })).toThrow("version");
    expect(() => validateFirstPartyContract({ ...preflight, executable: true })).toThrow("field");
    expect(() => validateFirstPartyContract({
      ...preflight,
      helpers: [{ id: "shell", execution: "automatic" }],
    })).toThrow("helper");
  });

  it("rejects contract provenance that does not match the lockfile", () => {
    expect(() => validateFirstPartyContract({
      ...preflight,
      provenance: { ...preflight.provenance, lockHash: "stale" },
    })).toThrow("skills-lock.json");
  });

  it("rejects unknown first-party identities and cross-skill helpers", () => {
    expect(() => validateFirstPartyContract({
      ...preflight,
      id: "unknown-skill",
      provenance: {
        ...preflight.provenance,
        installedPath: ".agents/skills/unknown-skill/SKILL.md",
      },
    }, {
      "unknown-skill": { source: "pax-k/build-right", computedHash: preflight.provenance.lockHash },
    })).toThrow("identity");

    expect(() => validateFirstPartyContract({
      ...preflight,
      helpers: [{ id: "execution-check", execution: "explicit-user-action" }],
    })).toThrow("helper");

    expect(() => validateFirstPartyContract({
      ...preflight,
      lifecyclePhase: "Build",
    })).toThrow("shape");
  });

  it("rejects blank semantic values", () => {
    expect(() => validateFirstPartyContract({ ...preflight, purpose: "" })).toThrow("purpose");
    expect(() => validateFirstPartyContract({ ...preflight, reads: [""] })).toThrow("reads");
    expect(() => validateFirstPartyContract({ ...preflight, purpose: "   " })).toThrow("purpose");
    expect(() => validateFirstPartyContract({ ...preflight, reads: ["\t"] })).toThrow("reads");
  });
});
