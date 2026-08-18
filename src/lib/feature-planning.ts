import type { ArtifactDraft, HelperDecision, ProjectSnapshot } from "../types";

const sprintTrackerPattern = /^tasks\/sprint-\d+\.md$/;

export function validateFeatureRequest(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return "Describe one feature before running planning.";
  if (new TextEncoder().encode(trimmed).length > 2_000) return "Feature request must be at most 2000 bytes.";
  if ([...trimmed].some((character) => character === "\0" || (/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/).test(character))) {
    return "Feature request contains unsupported control characters.";
  }
  return null;
}

function slugify(value: string) {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 54) || "planned-feature";
}

function titleFor(value: string) {
  const sentence = value.trim().split(/\n|[.!?](?:\s|$)/)[0]?.trim() || "Planned feature";
  return sentence.charAt(0).toUpperCase() + sentence.slice(1);
}

function nextTaskId(project: ProjectSnapshot) {
  const ids = project.files.flatMap((file) => {
    const match = file.path.match(/^tasks\/issues\/(\d{3})-/);
    return match ? [Number(match[1])] : [];
  });
  return String(Math.max(0, ...ids) + 1).padStart(3, "0");
}

export function planningCanPropose(decision: HelperDecision) {
  return decision.decision === "update-sprint"
    && (decision.blockingGates?.length ?? 0) === 0
    && (decision.founderQuestions?.length ?? 0) === 0
    && (decision.researchTriggers?.length ?? 0) === 0;
}

export function buildPlanningDrafts(
  project: ProjectSnapshot,
  featureRequest: string,
  decision: HelperDecision,
  trackerContent: string,
  trackerVersion: string,
  date = new Date().toISOString().slice(0, 10),
): ArtifactDraft[] {
  if (validateFeatureRequest(featureRequest) || !planningCanPropose(decision)) return [];
  const tracker = decision.recommendedDestination && sprintTrackerPattern.test(decision.recommendedDestination)
    ? decision.recommendedDestination
    : project.files.map((file) => file.path).filter((path) => sprintTrackerPattern.test(path)).sort().at(-1);
  if (!tracker || !project.files.some((file) => file.path === tracker)) return [];

  const id = nextTaskId(project);
  const title = titleFor(featureRequest);
  const path = `tasks/issues/${id}-${slugify(title)}.md`;
  if (project.files.some((file) => file.path === path)) return [];
  const task = `# ${id}: ${title}

Status: ready
Type: feature
Owner: AI

Assumption basis: founder feature request captured in Build Right Studio on ${date}
Requirement basis: founder-approved feature request; ${tracker}
Reversibility: moderate
Learning objective: prove the requested outcome with repository and live evidence
Source under test: repo-local path

## Goal

${featureRequest.trim()}

## Non-Goals

- Expand beyond the described feature without founder confirmation.
- Publish, deploy, or change collaboration state implicitly.

## Required Reading

- docs/mvp-scope.md
- docs/execution-rules.md
- ${tracker}

## Acceptance Criteria

- [ ] The described founder outcome works through the real product surface.
- [ ] Repository-backed automated checks pass for every touched boundary.
- [ ] A live/manual trial records the exact evidence and remaining limitations.
- [ ] Authority docs and tracker state match the verified implementation.

## Baseline Evidence

The founder supplied this feature request through the guided Plan surface.
Implementation evidence does not exist yet; this task must establish it.

## Verification

- Focused automated tests for changed behavior.
- Full repository checks.
- Live/manual acceptance for the primary user path.

## Evidence Log

| Date | Evidence | Result | Notes |
| --- | --- | --- | --- |

## Blockers

- Stop on unresolved founder, dependency, credential, destructive, deployment, or external-service gates.

## Follow-Ups

- Record only follow-up work proved necessary by implementation or live verification.
`;
  const row = `| ${id} | ${title.replaceAll("|", "\\|")} | ready | none | ${path} |`;
  const lines = trackerContent.split("\n");
  const taskHeading = lines.findIndex((line) => /^## Tasks\s*$/.test(line.trim()));
  if (taskHeading < 0) return [];
  let insertAt = taskHeading + 1;
  while (insertAt < lines.length && !/^##\s/.test(lines[insertAt])) insertAt += 1;
  while (insertAt > taskHeading + 1 && lines[insertAt - 1].trim() === "") insertAt -= 1;
  lines.splice(insertAt, 0, row);
  const updatedTracker = `${lines.join("\n").replace(/\n*$/, "")}\n`;

  return [
    { path, content: task },
    { path: tracker, content: updatedTracker, expectedVersion: trackerVersion },
  ];
}
