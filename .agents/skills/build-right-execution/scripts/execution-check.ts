import { join, relative, resolve } from "node:path";

type Mode = "next-task" | "task-contract" | "stop-gates" | "all";
type OutputFormat = "markdown" | "json";

type Args = {
  cwd: string;
  mode: Mode;
  task?: string;
  format: OutputFormat;
  help: boolean;
};

type TaskInfo = {
  path: string;
  status: string;
  title: string;
  owner?: string;
};

type CheckResult = {
  cwd: string;
  mode: Mode;
  selectedTask: TaskInfo | null;
  readyTaskCandidates: TaskInfo[];
  contractMissing: string[];
  gateReasons: string[];
  recommendation: string;
};

const validModes = new Set<Mode>(["next-task", "task-contract", "stop-gates", "all"]);
const validFormats = new Set<OutputFormat>(["markdown", "json"]);

const requiredFields = [
  "Status:",
  "Type:",
  "Owner:",
  "Assumption basis:",
  "Requirement basis:",
  "Reversibility:",
  "Learning objective:",
  "Source under test:",
];

const requiredSections = [
  "## Goal",
  "## Non-Goals",
  "## Required Reading",
  "## Acceptance Criteria",
  "## Baseline Evidence",
  "## Verification",
  "## Evidence Log",
  "## Blockers",
  "## Follow-Ups",
];

function parseArgs(argv: string[]): Args {
  const args: Args = {
    cwd: ".",
    mode: "next-task",
    format: "markdown",
    help: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }

    if (arg === "--cwd" || arg === "--mode" || arg === "--task" || arg === "--format") {
      const value = argv[index + 1];
      if (!value) {
        throw new Error(`${arg} requires a value`);
      }
      index += 1;

      if (arg === "--cwd") {
        args.cwd = value;
      } else if (arg === "--mode") {
        if (!validModes.has(value as Mode)) {
          throw new Error(`invalid --mode: ${value}`);
        }
        args.mode = value as Mode;
      } else if (arg === "--task") {
        args.task = value;
      } else {
        if (!validFormats.has(value as OutputFormat)) {
          throw new Error(`invalid --format: ${value}`);
        }
        args.format = value as OutputFormat;
      }
      continue;
    }

    throw new Error(`unknown argument: ${arg}`);
  }

  args.cwd = resolve(args.cwd);
  return args;
}

function usage(): string {
  return `Usage: bun execution-check.ts [--cwd <path>] [--mode next-task|task-contract|stop-gates|all] [--task <path>] [--format markdown|json]

Read-only helper for Build Right execution. Reports next-task candidates,
missing task contract fields, stop/ask gate signals, and a proceed/stop
recommendation.`;
}

async function exists(cwd: string, path: string): Promise<boolean> {
  return Bun.file(join(cwd, path)).exists();
}

async function readIfExists(cwd: string, path: string): Promise<string> {
  const file = Bun.file(join(cwd, path));
  if (!(await file.exists())) {
    return "";
  }
  return file.text();
}

async function globFiles(cwd: string, pattern: string): Promise<string[]> {
  const glob = new Bun.Glob(pattern);
  const files: string[] = [];
  for await (const file of glob.scan({ cwd, onlyFiles: true, dot: true })) {
    files.push(file);
  }
  return files.sort();
}

function parseTask(path: string, text: string): TaskInfo {
  const title = text.match(/^#\s+(.+)$/m)?.[1]?.trim() ?? path;
  const status = text.match(/^Status:\s*(.+)$/m)?.[1]?.trim() ?? "missing";
  const owner = text.match(/^Owner:\s*(.+)$/m)?.[1]?.trim();
  return { path, status, title, owner };
}

async function taskFiles(cwd: string): Promise<string[]> {
  const files = [
    ...(await globFiles(cwd, "tasks/issues/*.md")),
    ...(await globFiles(cwd, "tasks/*.md")),
    ...(await globFiles(cwd, "issues/*.md")),
  ];
  return [...new Set(files)]
    .filter((file) => !file.endsWith("sprint-0.md"))
    .filter((file) => !file.endsWith("post-release-backlog.md"))
    .sort();
}

async function loadTasks(cwd: string): Promise<TaskInfo[]> {
  const files = await taskFiles(cwd);
  const tasks: TaskInfo[] = [];
  for (const file of files) {
    tasks.push(parseTask(file, await readIfExists(cwd, file)));
  }
  return tasks;
}

function resolveTaskPath(cwd: string, task?: string): string | undefined {
  if (!task) {
    return undefined;
  }

  const absolute = resolve(cwd, task);
  if (absolute.startsWith(cwd)) {
    return relative(cwd, absolute);
  }
  return task;
}

function missingContractFields(text: string): string[] {
  const missing: string[] = [];
  for (const field of requiredFields) {
    if (!text.includes(field)) {
      missing.push(field);
    }
  }
  for (const section of requiredSections) {
    if (!text.includes(section)) {
      missing.push(section);
    }
  }
  return missing;
}

function blockText(text: string, heading: string): string {
  const escaped = heading.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = text.match(new RegExp(`${escaped}\\n([\\s\\S]*?)(?:\\n## |$)`));
  return match?.[1]?.trim() ?? "";
}

function nonEmptyBlockers(text: string): boolean {
  const blockers = blockText(text, "## Blockers");
  if (!blockers) {
    return false;
  }
  const normalized = blockers.toLowerCase().replace(/[.\s]+$/g, "").trim();
  return !["- none", "none", "- none yet", "none yet", "- no blockers", "no blockers"].includes(
    normalized,
  );
}

function isAiOwned(owner?: string): boolean {
  return ["ai", "agent", "codex"].includes((owner ?? "").trim().toLowerCase());
}

function splitTableLine(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function isTableSeparator(cells: string[]): boolean {
  return cells.length > 0 && cells.every((cell) => /^:?-{3,}:?$/.test(cell));
}

function parseTable(sectionText: string): Record<string, string>[] {
  const lines = sectionText
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith("|") && line.endsWith("|"));
  if (lines.length < 2) {
    return [];
  }

  const headerLine = lines[0];
  if (headerLine === undefined) {
    return [];
  }

  const headers = splitTableLine(headerLine);
  if (!headers.includes("Conflict") || !headers.includes("Status")) {
    return [];
  }

  return lines
    .slice(1)
    .map(splitTableLine)
    .filter((cells) => !isTableSeparator(cells))
    .map((cells) => {
      const row: Record<string, string> = {};
      headers.forEach((header, index) => {
        row[header] = cells[index] ?? "";
      });
      return row;
    });
}

function openConflictReasons(text: string): string[] {
  return parseTable(blockText(text, "## Conflicts"))
    .filter((row) => {
      const conflict = (row.Conflict ?? "").trim();
      const status = (row.Status ?? "").trim().toLowerCase();
      return (
        conflict &&
        conflict !== "None" &&
        !conflict.includes("<") &&
        !["resolved", "closed", "complete", "done", "none"].includes(status)
      );
    })
    .map((row) => `open conflict: ${row.Conflict ?? "unknown conflict"}`);
}

async function gateReasons(cwd: string, selectedTask: TaskInfo | null, taskText: string): Promise<string[]> {
  const reasons: string[] = [];
  if (!(await exists(cwd, "docs/execution-rules.md"))) {
    reasons.push("execution rules are missing");
  }
  if (!(await exists(cwd, "docs/release-gates.md"))) {
    reasons.push("release or validation gates are missing");
  }
  if (!(await exists(cwd, "docs/mvp-scope.md")) && !(await exists(cwd, "docs/blueprint-status.md"))) {
    reasons.push("product truth, MVP scope, or blueprint status is missing");
  }
  if (!selectedTask) {
    reasons.push("no selected task");
  } else if (!["ready", "active"].includes(selectedTask.status)) {
    reasons.push(`selected task status is ${selectedTask.status}`);
  } else if (!isAiOwned(selectedTask.owner)) {
    reasons.push(`selected task is owned by ${selectedTask.owner ?? "unknown"}, not AI`);
  }
  if (taskText && nonEmptyBlockers(taskText)) {
    reasons.push("selected task has blockers");
  }

  const openQuestions = await readIfExists(cwd, "docs/open-questions.md");
  if (/\bfounder\b|\bprimary user\b|\bmvp\b|\bpositioning\b|\bpromise\b/i.test(openQuestions)) {
    reasons.push("open founder-owned questions are recorded");
  }

  const releaseGates = await readIfExists(cwd, "docs/release-gates.md");
  if (/\bunproven\b|\bmissing\b|\bexternal\b|\bdirectory indexing\b/i.test(releaseGates)) {
    reasons.push("release gates contain unproven or external-state language");
  }

  const conflicts = await readIfExists(cwd, "docs/conflicts.md");
  reasons.push(...openConflictReasons(conflicts));

  return reasons;
}

async function runCheck(args: Args): Promise<CheckResult> {
  const tasks = await loadTasks(args.cwd);
  const readyTaskCandidates = tasks.filter((task) => task.status === "ready");
  const requestedTask = resolveTaskPath(args.cwd, args.task);
  const selectedTask =
    (requestedTask ? tasks.find((task) => task.path === requestedTask) : null) ??
    readyTaskCandidates[0] ??
    null;

  const taskText = selectedTask ? await readIfExists(args.cwd, selectedTask.path) : "";
  const contractMissing =
    (args.mode === "task-contract" || args.mode === "all") && taskText
      ? missingContractFields(taskText)
      : [];
  const reasons =
    args.mode === "stop-gates" || args.mode === "all"
      ? await gateReasons(args.cwd, selectedTask, taskText)
      : [];

  let recommendation = "No ready task candidate found. Stop or create the smallest AI-owned blocker.";
  if (selectedTask && selectedTask.status === "ready" && contractMissing.length === 0 && reasons.length === 0) {
    recommendation = `Proceed with one bounded task: ${selectedTask.path}`;
  } else if (selectedTask && contractMissing.length > 0) {
    recommendation = "Reconcile missing task contract fields before editing.";
  } else if (selectedTask && reasons.length > 0) {
    recommendation = "Stop at the gate or resolve the blocker before advancing.";
  } else if (selectedTask) {
    recommendation = `Inspect selected task before proceeding: ${selectedTask.path}`;
  }

  return {
    cwd: args.cwd,
    mode: args.mode,
    selectedTask,
    readyTaskCandidates,
    contractMissing,
    gateReasons: reasons,
    recommendation,
  };
}

function renderMarkdown(result: CheckResult): string {
  const lines = [
    "# Build Right Execution Check",
    "",
    `CWD: ${result.cwd}`,
    `Mode: ${result.mode}`,
    "",
    "## Selected Task",
    result.selectedTask
      ? `- ${result.selectedTask.path} (${result.selectedTask.status}) - ${result.selectedTask.title}`
      : "- none",
    "",
    "## Ready Task Candidates",
    ...(
      result.readyTaskCandidates.length > 0
        ? result.readyTaskCandidates.map((task) => `- ${task.path} - ${task.title}`)
        : ["- none"]
    ),
  ];

  if (result.contractMissing.length > 0) {
    lines.push("", "## Missing Task Contract Fields", ...result.contractMissing.map((item) => `- ${item}`));
  }

  if (result.gateReasons.length > 0) {
    lines.push("", "## Stop/Ask Gate Signals", ...result.gateReasons.map((item) => `- ${item}`));
  }

  lines.push("", "## Recommendation", result.recommendation);
  return `${lines.join("\n")}\n`;
}

try {
  const args = parseArgs(Bun.argv.slice(2));
  if (args.help) {
    console.log(usage());
    process.exit(0);
  }

  const result = await runCheck(args);
  if (args.format === "json") {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log(renderMarkdown(result));
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  console.error(usage());
  process.exit(1);
}
