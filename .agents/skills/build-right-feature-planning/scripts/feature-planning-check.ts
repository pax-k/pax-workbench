import { join, resolve } from "node:path";

type OutputFormat = "markdown" | "json";
type Decision =
  | "route-preflight"
  | "ask-founder"
  | "run-research"
  | "delegate-review"
  | "update-roadmap"
  | "update-sprint"
  | "create-ready-tasks"
  | "blocked";

type Args = {
  cwd: string;
  feature: string;
  format: OutputFormat;
  help: boolean;
};

type Gate = {
  type: "missing-preflight" | "founder-owned" | "open-conflict" | "external-state" | "invalid-task" | "sprint-closure";
  source: string;
  reason: string;
};

type TaskCandidate = {
  id: string;
  title: string;
  status: string;
  owner: string;
  path: string;
  tracker: string;
};

type PlanningResult = {
  cwd: string;
  decision: Decision;
  confidence: "high" | "medium" | "low";
  featureRequest: string;
  recommendedDestination: string;
  blockingGates: Gate[];
  founderQuestions: string[];
  researchTriggers: string[];
  readyTaskCandidates: TaskCandidate[];
  nextAction: string;
  scannedArtifacts: string[];
};

const validFormats = new Set<OutputFormat>(["markdown", "json"]);
const terminalStatuses = new Set(["resolved", "closed", "complete", "done", "none", "n/a"]);
const completedTrackerStatuses = new Set(["complete", "completed", "done"]);
const terminalTaskStatuses = new Set([
  "complete",
  "completed",
  "done",
  "deferred",
  "moved",
  "canceled",
  "cancelled",
  "split",
  "superseded",
]);
const readyStatuses = new Set(["ready", "active"]);

function parseArgs(argv: string[]): Args {
  const args: Args = {
    cwd: ".",
    feature: "",
    format: "markdown",
    help: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
    if (arg === "--cwd" || arg === "--feature" || arg === "--format") {
      const value = argv[index + 1];
      if (!value) {
        throw new Error(`${arg} requires a value`);
      }
      index += 1;
      if (arg === "--cwd") {
        args.cwd = value;
      } else if (arg === "--feature") {
        args.feature = value;
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
  return `Usage: bun feature-planning-check.ts [--cwd <path>] [--feature <request>] [--format markdown|json]

Read-only Build Right feature planning helper. Scans docs, sprint trackers,
post-release backlog, and issue tasks, then returns one planning decision:
route-preflight, ask-founder, run-research, delegate-review, update-roadmap,
update-sprint, create-ready-tasks, or blocked.

Reports Planning decision, Confidence, Feature request, Recommended destination,
Blocking gates, Founder questions, Research triggers, Ready task candidates,
and Next action.`;
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

function normalize(value: string | undefined): string {
  return (value ?? "").trim().toLowerCase();
}

function statusLine(text: string): string {
  return text.match(/^Status:\s*(.+)$/m)?.[1]?.trim() ?? "missing";
}

function fieldLine(text: string, field: string): string {
  return text.match(new RegExp(`^${field}:\\s*(.+)$`, "m"))?.[1]?.trim() ?? "";
}

function titleLine(path: string, text: string): string {
  return text.match(/^#\s+(.+)$/m)?.[1]?.trim() ?? path;
}

function section(text: string, heading: string): string {
  const lines = text.split("\n");
  const headingLine = `## ${heading}`;
  const start = lines.findIndex((line) => line.trim() === headingLine);
  if (start === -1) {
    return "";
  }

  const body: string[] = [];
  for (const line of lines.slice(start + 1)) {
    if (/^##\s+/.test(line)) {
      break;
    }
    body.push(line);
  }
  return body.join("\n").trim();
}

function nextActionBlock(text: string): string {
  return section(text, "Next Action").replace(/\s+/g, " ").trim();
}

function splitTableLine(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
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
  if (!headerLine) {
    return [];
  }
  const headers = splitTableLine(headerLine);
  return lines.slice(2).map((line) => {
    const values = splitTableLine(line);
    const row: Record<string, string> = {};
    headers.forEach((header, index) => {
      row[header] = values[index] ?? "";
    });
    return row;
  });
}

function isAiOwned(owner: string): boolean {
  return ["ai", "agent", "codex"].includes(normalize(owner));
}

function isCompleteTrackerStatus(status: string): boolean {
  return completedTrackerStatuses.has(normalize(status));
}

function isTerminalTaskStatus(status: string): boolean {
  return terminalTaskStatuses.has(normalize(status));
}

async function loadTrackerTasks(cwd: string, trackers: string[]): Promise<TaskCandidate[]> {
  const tasks: TaskCandidate[] = [];
  for (const tracker of trackers) {
    const trackerText = await readIfExists(cwd, tracker);
    for (const row of parseTable(section(trackerText, "Tasks"))) {
      const id = row.ID ?? row.Id ?? row.id ?? "";
      const path = row.Evidence ?? row.Task ?? row.Path ?? "";
      const taskText = path ? await readIfExists(cwd, path) : "";
      tasks.push({
        id,
        title: row.Title ?? titleLine(path || tracker, taskText),
        status: normalize(row.Status ?? statusLine(taskText)),
        owner: fieldLine(taskText, "Owner") || row.Owner || "",
        path,
        tracker,
      });
    }
  }
  return tasks;
}

function conflictGates(conflicts: string): Gate[] {
  const gates: Gate[] = [];
  for (const row of parseTable(section(conflicts, "Conflicts"))) {
    const conflict = row.Conflict ?? "";
    const status = normalize(row.Status);
    if (!conflict || conflict.includes("<") || normalize(conflict) === "none" || terminalStatuses.has(status)) {
      continue;
    }
    gates.push({
      type: "open-conflict",
      source: "docs/conflicts.md",
      reason: conflict,
    });
  }
  return gates;
}

async function sprintClosureGates(cwd: string, sprintTrackers: string[]): Promise<Gate[]> {
  const gates: Gate[] = [];
  for (const tracker of sprintTrackers) {
    const trackerText = await readIfExists(cwd, tracker);
    if (!isCompleteTrackerStatus(statusLine(trackerText))) {
      continue;
    }
    for (const row of parseTable(section(trackerText, "Tasks"))) {
      const status = normalize(row.Status);
      if (isTerminalTaskStatus(status)) {
        continue;
      }
      const taskId = row.ID ?? row.Id ?? row.id ?? row.Title ?? "unknown task";
      gates.push({
        type: "sprint-closure",
        source: tracker,
        reason: `${tracker} is complete but ${taskId} is ${row.Status || "missing"}; complete, defer, move, cancel, split, or supersede it before advancing.`,
      });
    }
  }
  return gates;
}

function researchTriggers(feature: string): string[] {
  const triggers: Array<[RegExp, string]> = [
    [/\b(competitor|alternative|market|pricing|price|benchmark)\b/i, "public market or pricing evidence"],
    [/\b(regulatory|legal|compliance|policy|app store|payment|payments)\b/i, "public policy or platform constraints"],
    [/\b(vendor|integration|api|oauth|webhook|sms|email provider)\b/i, "public integration feasibility"],
    [/\b(seo|search indexing|directory|public launch)\b/i, "public distribution constraints"],
  ];
  return triggers.filter(([pattern]) => pattern.test(feature)).map(([, label]) => label);
}

function delegationTriggers(feature: string): string[] {
  const triggers: Array<[RegExp, string]> = [
    [/\b(architecture|migration|refactor|data model|database|auth|security)\b/i, "technical feasibility review"],
    [/\b(cross-cutting|large|many modules|multi-package)\b/i, "scope and conflict review"],
  ];
  return triggers.filter(([pattern]) => pattern.test(feature)).map(([, label]) => label);
}

async function resolvePlanning(args: Args): Promise<PlanningResult> {
  const feature = args.feature.trim();
  const blueprint = await readIfExists(args.cwd, "docs/blueprint-status.md");
  const mvpScope = await readIfExists(args.cwd, "docs/mvp-scope.md");
  const executionRules = await readIfExists(args.cwd, "docs/execution-rules.md");
  const openQuestions = await readIfExists(args.cwd, "docs/open-questions.md");
  const conflicts = await readIfExists(args.cwd, "docs/conflicts.md");
  const backlog = await readIfExists(args.cwd, "tasks/post-release-backlog.md");
  const sprintTrackers = await globFiles(args.cwd, "tasks/sprint-*.md");
  const issueFiles = await globFiles(args.cwd, "tasks/issues/*.md");
  const trackers = [...sprintTrackers, ...(backlog ? ["tasks/post-release-backlog.md"] : [])];
  const tasks = await loadTrackerTasks(args.cwd, trackers);
  const scannedArtifacts = [
    ...(blueprint ? ["docs/blueprint-status.md"] : []),
    ...(mvpScope ? ["docs/mvp-scope.md"] : []),
    ...(executionRules ? ["docs/execution-rules.md"] : []),
    ...(openQuestions ? ["docs/open-questions.md"] : []),
    ...(conflicts ? ["docs/conflicts.md"] : []),
    ...trackers,
    ...issueFiles,
  ];

  const blockingGates: Gate[] = [];
  const missing = [
    !blueprint ? "docs/blueprint-status.md" : "",
    !mvpScope ? "docs/mvp-scope.md" : "",
    !executionRules ? "docs/execution-rules.md" : "",
    trackers.length === 0 ? "tasks/sprint-*.md or tasks/post-release-backlog.md" : "",
  ].filter(Boolean);

  if (missing.length > 0) {
    blockingGates.push({
      type: "missing-preflight",
      source: "preflight surface",
      reason: `missing ${missing.join(", ")}`,
    });
  }

  const founderQuestions: string[] = [];
  const blueprintGate = fieldLine(blueprint, "Current gate");
  const blueprintNext = nextActionBlock(blueprint);
  const openStatus = normalize(statusLine(openQuestions));
  if (openQuestions && !terminalStatuses.has(openStatus) && !["ready"].includes(openStatus)) {
    founderQuestions.push("Resolve open planning questions before changing feature scope.");
  }
  if (/\b(founder|ask|decision|priority|scope)\b/i.test(`${blueprintGate} ${blueprintNext}`)) {
    founderQuestions.push(blueprintNext || blueprintGate || "Confirm founder-owned planning gate.");
  }
  if (feature && /\b(should we|which user|buyer|package|launch|mvp)\b/i.test(feature)) {
    founderQuestions.push("Confirm feature priority, target user, and MVP boundary.");
  }

  const conflictBlocks = conflictGates(conflicts);
  const sprintClosureBlocks = await sprintClosureGates(args.cwd, sprintTrackers);
  blockingGates.push(...conflictBlocks);
  blockingGates.push(...sprintClosureBlocks);

  const invalidTasks = tasks.filter(
    (task) => readyStatuses.has(task.status) && (!task.path || !task.owner || !isAiOwned(task.owner)),
  );
  for (const task of invalidTasks) {
    blockingGates.push({
      type: "invalid-task",
      source: task.path || task.tracker,
      reason: `ready task ${task.id || task.title} is missing path or AI owner`,
    });
  }

  const research = researchTriggers(feature);
  const delegation = delegationTriggers(feature);
  const readyTaskCandidates = tasks.filter(
    (task) => task.status === "ready" && isAiOwned(task.owner) && task.path,
  );

  let decision: Decision = "update-sprint";
  let recommendedDestination = sprintTrackers[0] ?? "tasks/sprint-0.md";
  let nextAction = "Update the active sprint with planned feature work.";
  let confidence: PlanningResult["confidence"] = "medium";

  if (missing.length > 0) {
    decision = "route-preflight";
    recommendedDestination = "build-right-preflight";
    nextAction = `Route to build-right-preflight before feature planning: ${missing.join(", ")}`;
    confidence = "high";
  } else if (conflictBlocks.length > 0 || sprintClosureBlocks.length > 0 || invalidTasks.length > 0) {
    decision = "blocked";
    recommendedDestination = blockingGates[0]?.source ?? "docs/conflicts.md";
    nextAction = blockingGates[0]?.reason ?? "Resolve blocking planning gate.";
    confidence = "high";
  } else if (founderQuestions.length > 0) {
    decision = "ask-founder";
    recommendedDestination = "docs/open-questions.md";
    nextAction = founderQuestions[0] ?? "Resolve founder-owned planning gate.";
    confidence = "high";
  } else if (research.length > 0) {
    decision = "run-research";
    recommendedDestination = "docs/evidence/";
    nextAction = `Run bounded research for ${research[0] ?? "public evidence"}.`;
    confidence = "medium";
  } else if (delegation.length > 0) {
    decision = "delegate-review";
    recommendedDestination = "assets/templates/subagents/feature-feasibility-review.md";
    nextAction = `Run subagent review for ${delegation[0] ?? "feature feasibility"}.`;
    confidence = "medium";
  } else if (readyTaskCandidates.length > 0) {
    decision = "create-ready-tasks";
    const readyTask = readyTaskCandidates[0];
    recommendedDestination = readyTask?.path ?? "tasks/issues/";
    nextAction = `Ready for execution: ${readyTask?.path ?? "tasks/issues/"}`;
    confidence = "high";
  } else if (backlog && /\b(later|post-release|backlog|roadmap|defer|not now)\b/i.test(feature)) {
    decision = "update-roadmap";
    recommendedDestination = "tasks/post-release-backlog.md";
    nextAction = "Update the post-release backlog with this feature.";
    confidence = "high";
  } else if (sprintTrackers.length > 0) {
    decision = "update-sprint";
    recommendedDestination = sprintTrackers[0] ?? "tasks/sprint-0.md";
    nextAction = "Update the active sprint and create bounded task files if priority is already clear.";
    confidence = "medium";
  }

  return {
    cwd: args.cwd,
    decision,
    confidence,
    featureRequest: feature || "none",
    recommendedDestination,
    blockingGates,
    founderQuestions,
    researchTriggers: research,
    readyTaskCandidates,
    nextAction,
    scannedArtifacts,
  };
}

function listOrNone<T>(items: T[], render: (item: T) => string): string {
  return items.length === 0 ? "none" : items.map(render).join("; ");
}

function renderMarkdown(result: PlanningResult): string {
  return [
    `Planning decision: ${result.decision}`,
    `Confidence: ${result.confidence}`,
    `Feature request: ${result.featureRequest}`,
    `Recommended destination: ${result.recommendedDestination}`,
    `Blocking gates: ${listOrNone(result.blockingGates, (gate) => `${gate.type} at ${gate.source}: ${gate.reason}`)}`,
    `Founder questions: ${listOrNone(result.founderQuestions, (question) => question)}`,
    `Research triggers: ${listOrNone(result.researchTriggers, (trigger) => trigger)}`,
    `Ready task candidates: ${listOrNone(result.readyTaskCandidates, (task) => `${task.id || task.title} ${task.path}`)}`,
    `Next action: ${result.nextAction}`,
  ].join("\n");
}

async function main(): Promise<void> {
  const args = parseArgs(Bun.argv.slice(2));
  if (args.help) {
    console.log(usage());
    return;
  }

  const result = await resolvePlanning(args);
  if (args.format === "json") {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log(renderMarkdown(result));
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
