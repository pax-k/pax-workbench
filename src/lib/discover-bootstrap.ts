import type { ArtifactDraft, ProjectSnapshot } from "../types";

export const bootstrapArtifactPaths = [
  "AGENTS.md",
  "docs/raw/founder-interview.md",
  "docs/source-index.md",
  "docs/mvp-scope.md",
  "docs/blueprint-status.md",
  "docs/decision-log.md",
  "docs/conflicts.md",
  "docs/execution-rules.md",
  "docs/release-gates.md",
  "docs/evidence/preflight.md",
  "tasks/sprint-0.md",
  "tasks/issues/001-establish-execution-baseline.md",
] as const;

export interface FounderBootstrapInputs {
  productName: string;
  primaryUser: string;
  primaryWorkflow: string;
  valueMoment: string;
  hardConstraint: string;
}

export type FounderInputKey = keyof FounderBootstrapInputs;

export const founderInputContract: Array<{
  key: FounderInputKey;
  label: string;
  evidence: "repository fact" | "founder decision" | "founder claim";
  prompt: string;
}> = [
  {
    key: "productName",
    label: "Product name",
    evidence: "founder decision",
    prompt: "The working name used in the new authority files",
  },
  {
    key: "primaryUser",
    label: "Primary user",
    evidence: "founder claim",
    prompt: "One specific person who feels the problem most sharply",
  },
  {
    key: "primaryWorkflow",
    label: "Current workflow",
    evidence: "founder claim",
    prompt: "What that person needs to accomplish, in one sentence",
  },
  {
    key: "valueMoment",
    label: "Value moment",
    evidence: "founder decision",
    prompt: "The smallest honest moment when the product becomes useful",
  },
  {
    key: "hardConstraint",
    label: "Hard constraint",
    evidence: "founder decision",
    prompt: "One boundary the first implementation must not violate",
  },
];

export interface BootstrapInventory {
  existingPaths: string[];
  missingPaths: string[];
  complete: boolean;
  sourceMode: "founder-fed";
}

export function deriveBootstrapInventory(project: ProjectSnapshot): BootstrapInventory {
  const existing = new Set(project.files.map((file) => file.path));
  const existingPaths = bootstrapArtifactPaths.filter((path) => existing.has(path));
  const missingPaths = bootstrapArtifactPaths.filter((path) => !existing.has(path));
  return {
    existingPaths,
    missingPaths,
    complete: missingPaths.length === 0,
    sourceMode: "founder-fed",
  };
}

function clean(value: string) {
  return value.trim().replace(/\s+/g, " ");
}

export function validateFounderInputs(inputs: FounderBootstrapInputs) {
  return founderInputContract
    .filter(({ key }) => clean(inputs[key]).length < 2)
    .map(({ key }) => key);
}

function contentByPath(
  path: string,
  input: FounderBootstrapInputs,
  date: string,
): string {
  const product = clean(input.productName);
  const user = clean(input.primaryUser);
  const workflow = clean(input.primaryWorkflow);
  const value = clean(input.valueMoment);
  const constraint = clean(input.hardConstraint);
  const firstTask = "tasks/issues/001-establish-execution-baseline.md";
  const documents = bootstrapArtifactPaths
    .filter((candidate) => candidate.startsWith("docs/"))
    .map((candidate) => `| ${candidate} | Bootstrap authority for ${product} | draft | medium | founder + AI | ${date} |`)
    .join("\n");

  const contents: Record<string, string> = {
    "AGENTS.md": `# Project Instructions

- Repository Markdown and Git are authoritative.
- Preserve founder-claimed and repo-evidence-backed labels.
- Execute one bounded task at a time and stop on failed verification or an open gate.
`,
    "docs/raw/founder-interview.md": `# Founder Interview

Source mode: founder-fed
Captured: ${date}

| Prompt | Answer | Claim status |
| --- | --- | --- |
| Product name | ${product} | founder-claimed |
| Primary user | ${user} | founder-claimed |
| Primary workflow | ${workflow} | founder-claimed |
| Value moment | ${value} | founder-claimed |
| Hard constraint | ${constraint} | founder-claimed |

These answers are founder-supplied inputs, not customer validation.
`,
    "docs/source-index.md": `# Source Index

| Document | Purpose | Status | Confidence | Owner | Last Reviewed |
| --- | --- | --- | --- | --- | --- |
${documents}
| tasks/sprint-0.md | Foundation tracker | active | medium | founder + AI | ${date} |
| ${firstTask} | First bounded validation task | ready | medium | AI | ${date} |
`,
    "docs/mvp-scope.md": `# MVP Scope

Status: draft
Owner: founder
Confidence: low
Source mode: founder-fed
Prototype confidence: medium
Last updated: ${date}

## Primary Customer

${user}

## Primary Workflow

${workflow}

## Value Moment

${value}

## Requirements And Constraints

| Requirement or Constraint | Kind | Evidence Status | Design Consequence |
| --- | --- | --- | --- |
| ${workflow} | user outcome | founder-claimed | Keep the first workflow focused on one outcome |
| ${constraint} | hard | founder-claimed | Reject solutions that violate this boundary |

## Guarantees To Preserve

- ${constraint}

## Included

| Capability | User Outcome | Risk Reduced | Evidence |
| --- | --- | --- | --- |
| One reversible prototype workflow | ${value} | Building beyond the stated value moment | docs/raw/founder-interview.md |

## Excluded

- Pricing, positioning, and customer-demand claims not supplied here.

## Manual Before Automated

- Founder validates the draft product truth and first task before implementation.

## Readiness Notes

Run preflight and resolve every founder or external gate before product features.

## Validation Required Before Product Truth

- Direct founder validation and later customer evidence.

## Learning Objective

Test whether ${value.toLowerCase()} is useful to ${user}.
`,
    "docs/blueprint-status.md": `# Blueprint Status

Status: sprint-0-ready
Current phase: preflight
Project state: blank/new
Source mode: founder-fed
Prototype confidence: medium
Active task: ${firstTask}
Current gate: validate the execution baseline
Last evidence: founder inputs captured in docs/raw/founder-interview.md
Last updated: ${date}

## Readiness

| Gate | Status | Evidence | Notes |
| --- | --- | --- | --- |
| Founder intent captured | ready | docs/raw/founder-interview.md | Founder supplied |
| Claims tagged | ready | docs/raw/founder-interview.md | Founder claims remain labeled |
| Evidence recorded | draft | docs/evidence/preflight.md | Repository inventory only |
| Canonical docs exist | ready | docs/source-index.md | Created by confirmed plan |
| MVP extracted | needs-validation | docs/mvp-scope.md | Founder validation required |
| First task is bounded and verifiable | ready | ${firstTask} | AI-owned validation |

## Next Action

Run preflight, review its gate, then execute ${firstTask} only if selected.
`,
    "docs/decision-log.md": `# Decision Log

| Date | Decision | Requirement Basis | Tradeoff / Guarantee Impact | Owner | Evidence |
| --- | --- | --- | --- | --- | --- |
| ${date} | Use founder-fed source mode for ${product} | Founder supplied the bootstrap answers | Product claims remain unvalidated until founder/customer evidence upgrades them | founder | docs/raw/founder-interview.md |
| ${date} | Preserve the hard constraint: ${constraint} | Founder bootstrap decision | Candidate implementations that violate it are excluded | founder | docs/mvp-scope.md |
`,
    "docs/conflicts.md": `# Conflicts

No material conflict was identified during the initial founder-fed bootstrap.

Re-run preflight and record any later contradiction here before execution.
`,
    "docs/execution-rules.md": `# Execution Rules

## Authority Order

1. AGENTS.md and nested instructions.
2. docs/source-index.md.
3. docs/mvp-scope.md.
4. docs/release-gates.md.
5. The selected task.

## Effect Boundary

- Inspect before mutation.
- Preview exact paths and contents before planning writes.
- Execute one AI-owned ready task only after explicit confirmation.
- Repository evidence, not provider self-report, determines completion.
- Stop on founder, external, conflict, stale-source, or failed-verification gates.
`,
    "docs/release-gates.md": `# Release Gates

Status: sprint-0
Last updated: ${date}

| Gate | Required Evidence | Status |
| --- | --- | --- |
| Founder draft | docs/raw/founder-interview.md | ready |
| Product scope | founder validation of docs/mvp-scope.md | needs-validation |
| Execution baseline | ${firstTask} evidence and checks | ready |

No production release is authorized by bootstrap.
`,
    "docs/evidence/preflight.md": `# Preflight Evidence

Date: ${date}
Source mode: founder-fed
Project classification: blank/new

## Repository Evidence

- The selected Git repository lacked one or more canonical Build Right artifacts.
- The founder reviewed an exact create-only plan before applying it.

## Unsupported Important Claims

- Customer demand, urgency, willingness to pay, and product positioning.

## Evidence To Gather Next

- Founder validation of the draft MVP.
- Repository checks from the first bounded task.
`,
    "tasks/sprint-0.md": `# Sprint 0

Status: active
Purpose: establish a trustworthy execution baseline before product feature work.

## Tasks

| ID | Title | Status | Evidence |
| --- | --- | --- | --- |
| 001 | Establish execution baseline | ready | ${firstTask} |

## Gate

Do not start product feature work until Task 001 has verified evidence or an explicit blocker.
`,
    [firstTask]: `# 001: Establish Execution Baseline

Status: ready
Type: validation
Owner: AI

Assumption basis: founder-claimed
Requirement basis: docs/mvp-scope.md
Reversibility: easy
Learning objective: prove the repository can run its declared validation surface without violating the founder constraint
Source under test: repo-local path

## Goal

Establish the smallest repeatable validation baseline for ${product}.

## Non-Goals

- Implement product features.
- Change the founder-owned MVP boundary.

## Required Reading

- AGENTS.md
- docs/mvp-scope.md
- docs/execution-rules.md

## Acceptance Criteria

- [ ] Existing build, type, lint, and test commands are inventoried without inventing missing tooling.
- [ ] One repeatable validation command or an explicit blocker is recorded.
- [ ] Evidence states what is proved, simulated, and still unknown.

## Baseline Evidence

The confirmed bootstrap created repository authority; validation has not yet run.

## Verification

- Run the repository's existing checks, if present.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |

## Blockers

- None yet.

## Follow-Ups

- Plan product features only after this task is terminal.
`,
  };
  return contents[path];
}

export function buildBootstrapDrafts(
  project: ProjectSnapshot,
  inputs: FounderBootstrapInputs,
  date = new Date().toISOString().slice(0, 10),
): ArtifactDraft[] {
  const invalid = validateFounderInputs(inputs);
  if (invalid.length) return [];
  return deriveBootstrapInventory(project).missingPaths.map((path) => ({
    path,
    content: contentByPath(path, inputs, date),
  }));
}
