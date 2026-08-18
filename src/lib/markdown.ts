import type { ParsedTask } from "../types";

function section(markdown: string, heading: string): string {
  const lines = markdown.split("\n");
  const headingIndex = lines.findIndex(
    (line) => line.trim().toLowerCase() === `## ${heading.toLowerCase()}`,
  );
  if (headingIndex === -1) return "";
  const nextHeadingOffset = lines
    .slice(headingIndex + 1)
    .findIndex((line) => /^##\s+/.test(line.trim()));
  const endIndex = nextHeadingOffset === -1 ? lines.length : headingIndex + 1 + nextHeadingOffset;
  return lines.slice(headingIndex + 1, endIndex).join("\n").trim();
}

function field(markdown: string, name: string): string {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return markdown.match(new RegExp(`^${escaped}:\\s*(.+)$`, "im"))?.[1]?.trim() ?? "unknown";
}

export function parseTask(markdown: string): ParsedTask {
  const titleMatch = markdown.match(/^#\s+(?:(\d+):\s*)?(.+)$/m);
  const criteria = section(markdown, "Acceptance Criteria")
    .split("\n")
    .map((line) => line.match(/^\s*-\s+\[([ xX])\]\s+(.+)$/))
    .filter((match): match is RegExpMatchArray => Boolean(match))
    .map((match) => ({ checked: match[1].toLowerCase() === "x", text: match[2].trim() }));

  return {
    id: titleMatch?.[1] ?? "—",
    title: titleMatch?.[2]?.trim() ?? "Untitled task",
    status: field(markdown, "Status"),
    owner: field(markdown, "Owner"),
    requirementBasis: field(markdown, "Requirement basis"),
    goal: section(markdown, "Goal").replace(/\n+/g, " ").trim(),
    acceptanceCriteria: criteria,
  };
}

export function extractSprintRows(markdown: string) {
  return markdown
    .split("\n")
    .map((line) => line.match(/^\|\s*(\d+)\s*\|\s*([^|]+)\|\s*([^|]+)\|/))
    .filter((match): match is RegExpMatchArray => Boolean(match))
    .map((match) => ({ id: match[1], title: match[2].trim(), status: match[3].trim() }));
}
