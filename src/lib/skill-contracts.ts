import engineeringPrinciples from "../../skill-ui/build-right-engineering-principles.json";
import execution from "../../skill-ui/build-right-execution.json";
import featurePlanning from "../../skill-ui/build-right-feature-planning.json";
import preflight from "../../skill-ui/build-right-preflight.json";
import skillsLock from "../../skills-lock.json";
import type { SkillSummary } from "../types";

type ContractValue = Record<string, unknown>;
type SkillLock = { source: string; computedHash: string };
type FirstPartySpec = { phase: SkillSummary["phase"]; helpers: ReadonlySet<string> };

const contracts = [preflight, featurePlanning, execution, engineeringPrinciples] as ContractValue[];
const phases = new Set<SkillSummary["phase"]>(["Discover", "Plan", "Build", "Principles"]);
const renderers = new Set<SkillSummary["renderer"]>(["operating-card", "generic-markdown"]);
const firstPartySpecs: Record<string, FirstPartySpec> = {
  "build-right-preflight": { phase: "Discover", helpers: new Set(["preflight-check"]) },
  "build-right-feature-planning": { phase: "Plan", helpers: new Set(["feature-planning-check"]) },
  "build-right-execution": { phase: "Build", helpers: new Set(["continue-check", "execution-check"]) },
  "build-right-engineering-principles": { phase: "Principles", helpers: new Set() },
};
const contractFields = new Set([
  "version", "id", "name", "lifecyclePhase", "purpose", "reads", "writes", "decisions",
  "helpers", "requiredEvidence", "stopStates", "renderer", "provenance",
]);

function hasOnlyFields(value: ContractValue, allowed: Set<string>) {
  return Object.keys(value).every((field) => allowed.has(field));
}

function stringArray(value: unknown, field: string): string[] {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string" || !item.trim())) {
    throw new Error(`Invalid skill UI contract ${field}`);
  }
  return value;
}

function requiredString(contract: ContractValue, field: string): string {
  const value = contract[field];
  if (typeof value !== "string" || !value.trim()) throw new Error(`Invalid skill UI contract ${field}`);
  return value;
}

export function validateFirstPartyContract(
  contract: ContractValue,
  lockEntries: Record<string, SkillLock> = skillsLock.skills,
): SkillSummary {
  if (contract.version !== 1) throw new Error("Unsupported skill UI contract version");
  if (!hasOnlyFields(contract, contractFields)) throw new Error("Invalid skill UI contract field");

  const id = requiredString(contract, "id");
  const spec = firstPartySpecs[id];
  if (!spec) throw new Error("Unknown first-party skill UI contract identity");
  const phase = requiredString(contract, "lifecyclePhase") as SkillSummary["phase"];
  const renderer = requiredString(contract, "renderer") as SkillSummary["renderer"];
  const provenance = contract.provenance;
  if (!phases.has(phase) || phase !== spec.phase || renderer !== "operating-card" || !renderers.has(renderer)
    || !provenance || typeof provenance !== "object" || Array.isArray(provenance)) {
    throw new Error("Invalid skill UI contract shape");
  }
  const source = provenance as ContractValue;
  if (!hasOnlyFields(source, new Set(["source", "installedPath", "lockHash"]))) {
    throw new Error("Invalid skill UI provenance field");
  }
  const sourceName = requiredString(source, "source");
  const installedPath = requiredString(source, "installedPath");
  const lockHash = requiredString(source, "lockHash");
  const lockEntry = lockEntries[id];
  if (!lockEntry || sourceName !== lockEntry.source
    || installedPath !== `.agents/skills/${id}/SKILL.md`
    || lockHash !== lockEntry.computedHash) {
    throw new Error("Skill UI provenance does not match skills-lock.json");
  }
  const helpers = contract.helpers;
  if (!Array.isArray(helpers) || helpers.some((helper) => !helper || typeof helper !== "object" || Array.isArray(helper)
    || !hasOnlyFields(helper as ContractValue, new Set(["id", "execution"]))
    || (helper as ContractValue).execution !== "explicit-user-action"
    || typeof (helper as ContractValue).id !== "string"
    || !spec.helpers.has((helper as ContractValue).id as string))) {
    throw new Error("Invalid executable helper declaration");
  }

  return {
    id,
    name: requiredString(contract, "name"),
    phase,
    purpose: requiredString(contract, "purpose"),
    reads: stringArray(contract.reads, "reads"),
    writes: stringArray(contract.writes, "writes"),
    decisions: stringArray(contract.decisions, "decisions"),
    helpers: helpers.map((helper) => requiredString(helper as ContractValue, "id")),
    requiredEvidence: stringArray(contract.requiredEvidence, "requiredEvidence"),
    stopStates: stringArray(contract.stopStates, "stopStates"),
    renderer,
    executable: false,
    source: sourceName,
    installedPath,
    lockHash,
  };
}

export const firstPartySkillSummaries = contracts.map((contract) => validateFirstPartyContract(contract));

export function genericSkillFallback(id: string): SkillSummary {
  return {
    id,
    name: id.replaceAll("-", " "),
    phase: "Unknown",
    purpose: "No validated first-party UI contract is available for this installed skill.",
    reads: [],
    writes: [],
    decisions: [],
    helpers: [],
    requiredEvidence: [],
    stopStates: [],
    renderer: "generic-markdown",
    executable: false,
    source: "unverified installed skill",
    installedPath: `.agents/skills/${id}/SKILL.md`,
  };
}
