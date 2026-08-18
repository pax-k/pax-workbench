import type { ArtifactDraft, ProjectFileContent } from "../types";

export interface FounderGateInputs {
  context: string;
  scopeConfirmed: boolean;
}

function normalized(value: string) {
  return value.trim().replace(/\r\n?/g, "\n");
}

export function validateFounderGateInputs(input: FounderGateInputs) {
  if (normalized(input.context).length < 20) {
    return "Provide at least 20 characters of founder context.";
  }
  if (!input.scopeConfirmed) {
    return "Confirm that the current MVP scope reflects the founder decision.";
  }
  return null;
}

export function buildFounderGateDrafts(
  input: FounderGateInputs,
  mvpScope: ProjectFileContent,
  blueprintStatus: ProjectFileContent,
  date = new Date().toISOString().slice(0, 10),
): ArtifactDraft[] {
  const validationError = validateFounderGateInputs(input);
  if (validationError) return [];

  const context = normalized(input.context);
  const mvp = mvpScope.content
    .replace(
      /## Validation Required Before Product Truth/gu,
      "## Founder-Validated Product Truth Boundary",
    )
    .replace(/\n*$/u, "");
  const blueprint = blueprintStatus.content
    .replace(/\bneeds-validation\b/gu, "founder-validated")
    .replace(/\n*$/u, "");

  return [
    {
      path: "docs/raw/founder-dump.md",
      content: `# Founder Context Dump

Source mode: founder-fed
Captured: ${date}

${context}

This is founder-supplied context, not customer validation.
`,
    },
    {
      path: mvpScope.path,
      expectedVersion: mvpScope.version,
      content: `${mvp}

## Founder Validation

Confirmed in Build Right Studio on ${date}. This confirms the founder-owned MVP
scope only; customer demand, pricing, and production readiness remain unproved.
`,
    },
    {
      path: blueprintStatus.path,
      expectedVersion: blueprintStatus.version,
      content: `${blueprint}

## Founder Gate

Founder reviewed the MVP scope in Build Right Studio on ${date}. Customer and
release evidence remain separate gates.
`,
    },
  ];
}
