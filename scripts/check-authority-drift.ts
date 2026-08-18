import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export type AuthorityIssue = {
  code:
    | "active-task"
    | "broken-reference"
    | "dependency"
    | "indexed-document"
    | "predecessor-sprint"
    | "readme-claim"
    | "readme-command"
    | "release-gate"
    | "status";
  path: string;
  message: string;
};

type TaskRow = {
  id: string;
  status: string;
  dependsOn: string;
  evidencePath: string | null;
};

type SprintRecord = {
  number: number;
  path: string;
  status: string | null;
  tasks: TaskRow[];
};

const TASK_STATUSES = new Set([
  "active",
  "blocked",
  "canceled",
  "complete",
  "deferred",
  "moved",
  "needs-founder",
  "planned",
  "ready",
  "split",
  "superseded",
  "wait-external",
]);
const SPRINT_STATUSES = new Set(["active", "complete", "planned"]);
const TERMINAL_TASK_STATUSES = new Set([
  "canceled",
  "complete",
  "deferred",
  "moved",
  "split",
  "superseded",
]);
const LOCAL_MARKDOWN_ROOTS = ["README.md", "docs", "tasks"];

function normalizePath(root: string, absolutePath: string): string {
  return relative(root, absolutePath).replaceAll("\\", "/");
}

function read(root: string, path: string): string | null {
  const absolute = join(root, path);
  return existsSync(absolute) ? readFileSync(absolute, "utf8") : null;
}

function markdownFiles(root: string): string[] {
  const files: string[] = [];
  const visit = (absolutePath: string) => {
    if (!existsSync(absolutePath)) return;
    const entries = readdirSync(absolutePath, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.name === "node_modules" || entry.name === "target" || entry.name === "dist") continue;
      const child = join(absolutePath, entry.name);
      if (entry.isDirectory()) visit(child);
      else if (entry.isFile() && entry.name.endsWith(".md")) files.push(normalizePath(root, child));
    }
  };

  for (const localRoot of LOCAL_MARKDOWN_ROOTS) {
    const absolute = join(root, localRoot);
    if (!existsSync(absolute)) continue;
    if (localRoot.endsWith(".md")) files.push(localRoot);
    else visit(absolute);
  }
  return files.sort();
}

function statusOf(content: string): string | null {
  return content.match(/^Status:\s*([^\n]+)$/m)?.[1].trim().toLowerCase() ?? null;
}

function tableCells(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function taskRows(content: string): TaskRow[] {
  const rows: TaskRow[] = [];
  for (const line of content.split(/\r?\n/)) {
    if (!line.trim().startsWith("|")) continue;
    const cells = tableCells(line);
    if (
      cells.length < 4
      || cells[0] === "ID"
      || /^-+$/.test(cells[0])
      || !/^[0-9]+[A-Z]?$/.test(cells[0])
    ) continue;
    const hasDependencyColumn = cells.length >= 5;
    const evidenceCell = hasDependencyColumn ? cells[4] : cells[3];
    rows.push({
      id: cells[0],
      status: cells[2].toLowerCase(),
      dependsOn: hasDependencyColumn ? cells[3] : "—",
      evidencePath: evidenceCell.match(/tasks\/issues\/[A-Za-z0-9._/-]+\.md/)?.[0] ?? null,
    });
  }
  return rows;
}

function sprintRecords(root: string): SprintRecord[] {
  const tasksRoot = join(root, "tasks");
  if (!existsSync(tasksRoot)) return [];
  return readdirSync(tasksRoot)
    .map((name) => ({ name, match: name.match(/^sprint-(\d+)\.md$/) }))
    .filter((entry): entry is { name: string; match: RegExpMatchArray } => Boolean(entry.match))
    .map(({ name, match }) => {
      const path = `tasks/${name}`;
      const content = read(root, path) ?? "";
      return {
        number: Number(match[1]),
        path,
        status: statusOf(content),
        tasks: taskRows(content),
      };
    })
    .sort((a, b) => a.number - b.number);
}

function taskStatusMap(root: string, sprints: SprintRecord[]): Map<string, string> {
  const statuses = new Map<string, string>();
  for (const sprint of sprints) {
    for (const row of sprint.tasks) {
      if (row.evidencePath) statuses.set(row.evidencePath, row.status);
    }
  }
  const issuesRoot = join(root, "tasks/issues");
  if (!existsSync(issuesRoot)) return statuses;
  for (const name of readdirSync(issuesRoot)) {
    if (!name.endsWith(".md")) continue;
    const path = `tasks/issues/${name}`;
    const content = read(root, path);
    if (content) {
      const status = statusOf(content);
      if (status) statuses.set(path, status);
    }
  }
  return statuses;
}

function checkIndexedDocuments(root: string, issues: AuthorityIssue[]) {
  const path = "docs/source-index.md";
  const content = read(root, path);
  if (!content) {
    issues.push({ code: "indexed-document", path, message: "source index is missing" });
    return;
  }
  for (const line of content.split(/\r?\n/)) {
    if (!line.trim().startsWith("|")) continue;
    const document = tableCells(line)[0];
    if (!/^(?:docs|tasks)\/[A-Za-z0-9._/-]+\.md$/.test(document)) continue;
    if (!existsSync(join(root, document))) {
      issues.push({
        code: "indexed-document",
        path,
        message: `indexed document does not exist: ${document}`,
      });
    }
  }
}

function checkMarkdownReferences(root: string, issues: AuthorityIssue[]) {
  for (const path of markdownFiles(root)) {
    const content = read(root, path) ?? "";
    const linkPattern = /\]\(([^)\s]+)\)/g;
    for (const match of content.matchAll(linkPattern)) {
      const target = match[1].split("#")[0];
      if (!target || /^(?:https?:|mailto:|#)/.test(target)) continue;
      const decoded = decodeURIComponent(target);
      const absolute = resolve(root, dirname(path), decoded);
      if (!existsSync(absolute)) {
        issues.push({
          code: "broken-reference",
          path,
          message: `local Markdown reference does not exist: ${target}`,
        });
      }
    }
  }
}

function dependencyTokens(value: string): string[] {
  return value
    .split(",")
    .map((token) => token.trim().replace(/\s+complete$/i, ""))
    .filter(Boolean)
    .filter((token) => token !== "—" && token !== "-")
    .filter((token) => !/^founder available$/i.test(token))
    .filter((token) => !/^sprint \d+ complete$/i.test(token))
    .flatMap((token) => {
      const range = token.match(/^([0-9]+[A-Z]?)-([0-9]+[A-Z]?)$/);
      if (range) return [range[1], range[2]];
      return /^[0-9]+[A-Z]?$/.test(token) ? [token] : [];
    });
}

function checkSprintState(root: string, sprints: SprintRecord[], issues: AuthorityIssue[]) {
  const allIds = new Set(sprints.flatMap((sprint) => sprint.tasks.map((task) => task.id)));
  for (const sprint of sprints) {
    if (!sprint.status || !SPRINT_STATUSES.has(sprint.status)) {
      issues.push({
        code: "status",
        path: sprint.path,
        message: `invalid sprint status: ${sprint.status ?? "missing"}`,
      });
    }

    for (const task of sprint.tasks) {
      if (!TASK_STATUSES.has(task.status)) {
        issues.push({
          code: "status",
          path: sprint.path,
          message: `task ${task.id} has invalid status: ${task.status}`,
        });
      }
      if (!task.evidencePath || !existsSync(join(root, task.evidencePath))) {
        issues.push({
          code: "indexed-document",
          path: sprint.path,
          message: `task ${task.id} evidence file is missing: ${task.evidencePath ?? "none"}`,
        });
      }
      for (const dependency of dependencyTokens(task.dependsOn)) {
        if (!allIds.has(dependency)) {
          issues.push({
            code: "dependency",
            path: sprint.path,
            message: `task ${task.id} references unknown dependency ID ${dependency}`,
          });
        }
      }
    }

    if (sprint.status === "complete") {
      for (const task of sprint.tasks) {
        if (!TERMINAL_TASK_STATUSES.has(task.status)) {
          issues.push({
            code: "status",
            path: sprint.path,
            message: `complete sprint contains non-terminal task ${task.id} (${task.status})`,
          });
        }
      }
    }
  }

  for (const sprint of sprints.filter((candidate) => candidate.status === "active")) {
    for (const predecessor of sprints.filter((candidate) => candidate.number < sprint.number)) {
      if (predecessor.status !== "complete") {
        issues.push({
          code: "predecessor-sprint",
          path: sprint.path,
          message: `active sprint ${sprint.number} has non-terminal predecessor sprint ${predecessor.number} (${predecessor.status ?? "missing"})`,
        });
      }
    }
  }
}

function checkIssueStatuses(root: string, issues: AuthorityIssue[]) {
  const issuesRoot = join(root, "tasks/issues");
  if (!existsSync(issuesRoot)) return;
  for (const name of readdirSync(issuesRoot)) {
    if (!name.endsWith(".md")) continue;
    const path = `tasks/issues/${name}`;
    const status = statusOf(read(root, path) ?? "");
    if (!status || !TASK_STATUSES.has(status)) {
      issues.push({
        code: "status",
        path,
        message: `invalid task status: ${status ?? "missing"}`,
      });
    }
  }
}

function checkActiveTask(root: string, taskStatuses: Map<string, string>, issues: AuthorityIssue[]) {
  const path = "docs/blueprint-status.md";
  const content = read(root, path);
  if (!content) return;
  const activeTask = content.match(/^Active task:\s*(\S+\.md)$/m)?.[1] ?? null;
  if (!activeTask) {
    issues.push({ code: "active-task", path, message: "active task pointer is missing" });
    return;
  }
  const status = taskStatuses.get(activeTask);
  if (!status) {
    issues.push({ code: "active-task", path, message: `active task does not exist: ${activeTask}` });
  } else if (status !== "ready" && status !== "active") {
    const activeTaskContent = read(root, activeTask) ?? "";
    const hasSelectableTask = [...taskStatuses.values()].some(
      (taskStatus) => taskStatus === "ready" || taskStatus === "active",
    );
    const isFounderGate = status === "planned"
      && /^Owner:\s*.*founder/im.test(activeTaskContent)
      && /Founder participation is required/i.test(activeTaskContent)
      && !hasSelectableTask;
    if (isFounderGate) return;
    issues.push({
      code: "active-task",
      path,
      message: `active task points to non-selectable status ${status}: ${activeTask}`,
    });
  }
}

function expectedGateStatus(taskStatus: string): string | null {
  if (taskStatus === "planned") return "planned";
  if (taskStatus === "blocked" || taskStatus === "needs-founder" || taskStatus === "wait-external") return "blocked";
  if (taskStatus === "ready" || taskStatus === "active" || TERMINAL_TASK_STATUSES.has(taskStatus)) return "ready";
  return null;
}

function checkReleaseGates(root: string, taskStatuses: Map<string, string>, issues: AuthorityIssue[]) {
  const path = "docs/release-gates.md";
  const content = read(root, path);
  if (!content) return;
  for (const line of content.split(/\r?\n/)) {
    if (!line.trim().startsWith("|")) continue;
    const cells = tableCells(line);
    if (cells.length !== 4 || cells[0] === "Gate" || /^-+$/.test(cells[0])) continue;
    const gateStatus = cells[3].toLowerCase();
    for (const match of cells[2].matchAll(/tasks\/issues\/[A-Za-z0-9._/-]+\.md/g)) {
      const taskPath = match[0];
      const taskStatus = taskStatuses.get(taskPath);
      if (!taskStatus) {
        issues.push({ code: "release-gate", path, message: `gate references unknown task: ${taskPath}` });
        continue;
      }
      const expected = expectedGateStatus(taskStatus);
      if (expected && gateStatus !== expected) {
        issues.push({
          code: "release-gate",
          path,
          message: `gate ${cells[0]} is ${gateStatus} but ${taskPath} (${taskStatus}) requires ${expected}`,
        });
      }
    }
  }
}

function checkReadme(
  root: string,
  sprints: SprintRecord[],
  taskStatuses: Map<string, string>,
  issues: AuthorityIssue[],
) {
  const path = "README.md";
  const content = read(root, path);
  const packageContent = read(root, "package.json");
  if (!content || !packageContent) return;
  const packageJson = JSON.parse(packageContent) as { scripts?: Record<string, string>; dependencies?: Record<string, string> };
  const scripts = packageJson.scripts ?? {};

  for (const match of content.matchAll(/\bbun run ([A-Za-z0-9:_-]+)/g)) {
    if (!scripts[match[1]]) {
      issues.push({
        code: "readme-command",
        path,
        message: `README references undefined package script: ${match[1]}`,
      });
    }
  }

  const checkScript = scripts.check ?? "";
  for (const required of ["authority:check", "typecheck", "test", "build"]) {
    if (!checkScript.includes(`bun run ${required}`)) {
      issues.push({
        code: "readme-command",
        path: "package.json",
        message: `check script does not include bun run ${required}`,
      });
    }
  }

  if (content.includes("Tauri 2") && !String(packageJson.dependencies?.["@tauri-apps/api"] ?? "").includes("2.")) {
    issues.push({ code: "readme-claim", path, message: "README claims Tauri 2 but package dependency is not major version 2" });
  }
  const cargoManifest = read(root, "src-tauri/Cargo.toml") ?? "";
  if (content.includes("Tauri 2") && cargoManifest && !/tauri\s*=\s*\{[^}]*version\s*=\s*"2/i.test(cargoManifest)) {
    issues.push({ code: "readme-claim", path, message: "README claims Tauri 2 but the Rust manifest does not declare Tauri major version 2" });
  }
  if (/development-signed native|Apple-development-signed local trial/i.test(content)) {
    const signedTrialTask = "tasks/issues/012-prove-mvp-end-to-end.md";
    if (taskStatuses.get(signedTrialTask) !== "complete" || !existsSync(join(root, "docs/evidence/manual-trials.md"))) {
      issues.push({
        code: "readme-claim",
        path,
        message: "README claims a signed native trial without terminal Task 012 and manual-trial evidence",
      });
    }
  }

  for (const sprint of sprints) {
    const completedClaim = new RegExp(`completed Sprint ${sprint.number}`, "i").test(content);
    const activeClaim = new RegExp(`Sprint ${sprint.number} is the active`, "i").test(content);
    if (completedClaim && sprint.status !== "complete") {
      issues.push({
        code: "readme-claim",
        path,
        message: `README claims Sprint ${sprint.number} complete but tracker is ${sprint.status ?? "missing"}`,
      });
    }
    if (activeClaim && sprint.status !== "active") {
      issues.push({
        code: "readme-claim",
        path,
        message: `README claims Sprint ${sprint.number} active but tracker is ${sprint.status ?? "missing"}`,
      });
    }
  }

  const releaseGates = read(root, "docs/release-gates.md") ?? "";
  for (const boundary of ["Proved", "Simulated", "Not proved", "Post-MVP"]) {
    if (!releaseGates.includes(`| ${boundary} |`)) {
      issues.push({
        code: "readme-claim",
        path: "docs/release-gates.md",
        message: `evidence boundary is missing: ${boundary}`,
      });
    }
  }
}

export function checkAuthorityDrift(rootInput: string): AuthorityIssue[] {
  const root = resolve(rootInput);
  const issues: AuthorityIssue[] = [];
  const sprints = sprintRecords(root);
  const taskStatuses = taskStatusMap(root, sprints);

  checkIndexedDocuments(root, issues);
  checkMarkdownReferences(root, issues);
  checkSprintState(root, sprints, issues);
  checkIssueStatuses(root, issues);
  checkActiveTask(root, taskStatuses, issues);
  checkReleaseGates(root, taskStatuses, issues);
  checkReadme(root, sprints, taskStatuses, issues);

  return issues
    .filter(
      (issue, index, all) =>
        all.findIndex(
          (candidate) =>
            candidate.code === issue.code
            && candidate.path === issue.path
            && candidate.message === issue.message,
        ) === index,
    )
    .sort((a, b) => a.path.localeCompare(b.path) || a.code.localeCompare(b.code) || a.message.localeCompare(b.message));
}

function runCli() {
  const root = process.argv[2] ? resolve(process.argv[2]) : process.cwd();
  const issues = checkAuthorityDrift(root);
  if (issues.length > 0) {
    console.error(`Authority drift check failed with ${issues.length} issue(s):`);
    for (const issue of issues) console.error(`- [${issue.code}] ${issue.path}: ${issue.message}`);
    process.exitCode = 1;
    return;
  }
  console.log("Authority drift check passed.");
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) runCli();
