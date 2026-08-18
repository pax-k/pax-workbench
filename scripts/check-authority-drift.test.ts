import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { describe, expect, it } from "vitest";
import { checkAuthorityDrift } from "./check-authority-drift";

const TASK_001 = "tasks/issues/001-example.md";
const TASK_028A = "tasks/issues/028a-example.md";

function write(root: string, path: string, content: string) {
  mkdirSync(join(root, path, ".."), { recursive: true });
  writeFileSync(join(root, path), content);
}

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), "pax-authority-"));
  write(
    root,
    "package.json",
    JSON.stringify({
      scripts: {
        "authority:check": "bun scripts/check-authority-drift.ts",
        typecheck: "tsc -b",
        test: "vitest run",
        build: "vite build",
        check: "bun run authority:check && bun run typecheck && bun run test && bun run build",
        dev: "vite",
        tauri: "tauri",
      },
      dependencies: { "@tauri-apps/api": "^2.8.0" },
    }),
  );
  write(
    root,
    "README.md",
    "# App\n\nCompleted Sprint 0. Sprint 1 is the active phase.\n\n[Tauri 2 scope](docs/mvp-scope.md)\n\n`bun run check`\n`bun run dev`\n`bun run tauri dev`\n",
  );
  write(
    root,
    "docs/source-index.md",
    "| Document | Purpose |\n| --- | --- |\n| docs/mvp-scope.md | Scope |\n| docs/blueprint-status.md | State |\n| docs/release-gates.md | Gates |\n| tasks/sprint-0.md | Predecessor |\n| tasks/sprint-1.md | Active |\n",
  );
  write(root, "docs/mvp-scope.md", "# Scope\n");
  write(root, "docs/blueprint-status.md", `# State\n\nStatus: active\nActive task: ${TASK_028A}\n`);
  write(
    root,
    "docs/release-gates.md",
    `# Gates\n\n| Gate | Required Evidence | Command or Proof | Status |\n| --- | --- | --- | --- |\n| Prior | proof | ${TASK_001} | ready |\n| Current | proof | ${TASK_028A} | ready |\n\n| Classification | Current Boundary |\n| --- | --- |\n| Proved | local |\n| Simulated | fixtures |\n| Not proved | users |\n| Post-MVP | release |\n`,
  );
  write(root, TASK_001, "# 001\n\nStatus: complete\n");
  write(root, TASK_028A, "# 028A\n\nStatus: ready\n");
  write(
    root,
    "tasks/sprint-0.md",
    `# Sprint 0\n\nStatus: complete\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 001 | Example | complete | Sprint 0 complete | ${TASK_001} |\n`,
  );
  write(
    root,
    "tasks/sprint-1.md",
    `# Sprint 1\n\nStatus: active\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 028A | Example | ready | Sprint 0 complete | ${TASK_028A} |\n`,
  );
  return root;
}

describe("authority drift checker", () => {
  it("accepts a synchronized fixture including a nonnumeric task ID", () => {
    expect(checkAuthorityDrift(fixture())).toEqual([]);
  });

  it("detects missing indexed documents and broken local links", () => {
    const root = fixture();
    write(root, "docs/source-index.md", "| Document |\n| --- |\n| docs/missing.md |\n");
    write(root, "README.md", "# App\n\n[Missing](docs/also-missing.md)\n");
    const codes = checkAuthorityDrift(root).map((issue) => issue.code);
    expect(codes).toContain("indexed-document");
    expect(codes).toContain("broken-reference");
  });

  it("detects invalid statuses and dependency IDs", () => {
    const root = fixture();
    write(
      root,
      "tasks/sprint-1.md",
      `# Sprint 1\n\nStatus: active\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 028A | Example | invented | 999 | ${TASK_028A} |\n`,
    );
    const issues = checkAuthorityDrift(root);
    expect(issues.some((issue) => issue.code === "status")).toBe(true);
    expect(issues.some((issue) => issue.code === "dependency")).toBe(true);
  });

  it("detects stale active-task pointers and release-gate mismatches", () => {
    const root = fixture();
    write(root, "docs/blueprint-status.md", `# State\n\nStatus: active\nActive task: ${TASK_001}\n`);
    write(
      root,
      "docs/release-gates.md",
      `# Gates\n\n| Gate | Required Evidence | Command or Proof | Status |\n| --- | --- | --- | --- |\n| Current | proof | ${TASK_028A} | planned |\n\n| Classification | Current Boundary |\n| --- | --- |\n| Proved | local |\n| Simulated | fixtures |\n| Not proved | users |\n| Post-MVP | release |\n`,
    );
    const codes = checkAuthorityDrift(root).map((issue) => issue.code);
    expect(codes).toContain("active-task");
    expect(codes).toContain("release-gate");
  });

  it("allows an explicit planned founder gate only when no task is selectable", () => {
    const root = fixture();
    write(root, TASK_028A, `# 028A: Manual trial

Status: planned
Owner: founder + AI

## Blockers

- Founder participation is required.
`);
    write(root, "tasks/sprint-1.md", `# Sprint 1

Status: active

| ID | Title | Status | Depends On | Evidence |
| --- | --- | --- | --- | --- |
| 028A | Manual trial | planned |  | ${TASK_028A} |
`);
    write(root, "docs/blueprint-status.md", `# State

Status: active
Active task: ${TASK_028A}
`);

    const issues = checkAuthorityDrift(root);

    expect(issues.filter((issue) => issue.code === "active-task")).toEqual([]);
  });

  it("detects a non-terminal predecessor sprint", () => {
    const root = fixture();
    write(
      root,
      "tasks/sprint-0.md",
      `# Sprint 0\n\nStatus: planned\n\n| ID | Title | Status | Depends On | Evidence |\n| --- | --- | --- | --- | --- |\n| 001 | Example | planned | Sprint 0 complete | ${TASK_001} |\n`,
    );
    expect(checkAuthorityDrift(root).some((issue) => issue.code === "predecessor-sprint")).toBe(true);
  });

  it("detects unsupported README commands and stale sprint claims", () => {
    const root = fixture();
    write(root, "README.md", "# App\n\nCompleted Sprint 1.\n\n`bun run missing`\n");
    const codes = checkAuthorityDrift(root).map((issue) => issue.code);
    expect(codes).toContain("readme-command");
    expect(codes).toContain("readme-claim");
  });
});
