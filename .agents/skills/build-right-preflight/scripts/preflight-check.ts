import { join, resolve } from "node:path";

type Mode = "inventory" | "readiness" | "all";
type OutputFormat = "markdown" | "json";
type PreflightDecision =
  | "delegate-inventory"
  | "ask-founder"
  | "run-research"
  | "write-artifacts"
  | "create-sprint0"
  | "ready-for-execution"
  | "blocked";

type Args = {
  cwd: string;
  mode: Mode;
  format: OutputFormat;
  help: boolean;
};

type CheckResult = {
  cwd: string;
  mode: Mode;
  decision: PreflightDecision;
  nextAction: string;
  confidence: "high" | "medium" | "low";
  projectTypeSignal: "blank/new" | "existing";
  inventory: Record<string, string | number | boolean>;
  missingArtifacts: string[];
  readinessWarnings: string[];
  founderInputGaps: string[];
  recommendation: string;
};

const validModes = new Set<Mode>(["inventory", "readiness", "all"]);
const validFormats = new Set<OutputFormat>(["markdown", "json"]);

function parseArgs(argv: string[]): Args {
  const args: Args = {
    cwd: ".",
    mode: "all",
    format: "markdown",
    help: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }

    if (arg === "--cwd" || arg === "--mode" || arg === "--format") {
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
  return `Usage: bun preflight-check.ts [--cwd <path>] [--mode inventory|readiness|all] [--format markdown|json]

Read-only helper for Build Right preflight. Reports project type signals,
missing docs/tasks, readiness warnings, likely founder-input gaps, and one
recommended preflight decision.`;
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

function hasAny(text: string, values: string[]): boolean {
  const lower = text.toLowerCase();
  return values.some((value) => lower.includes(value.toLowerCase()));
}

function sourceMode(text: string): string {
  return text.match(/^Source mode:\s*(.+)$/m)?.[1]?.trim().toLowerCase() ?? "";
}

async function runCheck(args: Args): Promise<CheckResult> {
  const docs = await globFiles(args.cwd, "docs/**/*.md");
  const taskFiles = [
    ...(await globFiles(args.cwd, "tasks/**/*.md")),
    ...(await globFiles(args.cwd, "issues/**/*.md")),
  ];
  const packageFiles = [
    ...(await globFiles(args.cwd, "package.json")),
    ...(await globFiles(args.cwd, "bun.lock")),
    ...(await globFiles(args.cwd, "Cargo.toml")),
    ...(await globFiles(args.cwd, "pyproject.toml")),
    ...(await globFiles(args.cwd, "go.mod")),
  ];

  const blueprintStatus = await readIfExists(args.cwd, "docs/blueprint-status.md");
  const mvpScope = await readIfExists(args.cwd, "docs/mvp-scope.md");
  const openQuestions = await readIfExists(args.cwd, "docs/open-questions.md");
  const activeSourceMode = sourceMode(blueprintStatus) || sourceMode(mvpScope);

  const requiredArtifacts = [
    "docs/blueprint-status.md",
    "docs/source-index.md",
    "docs/mvp-scope.md",
    "docs/execution-rules.md",
    "docs/release-gates.md",
    "tasks/sprint-0.md",
  ];

  const missingArtifacts: string[] = [];
  for (const path of requiredArtifacts) {
    if (!(await exists(args.cwd, path))) {
      missingArtifacts.push(path);
    }
  }

  if (taskFiles.filter((file) => /tasks\/issues\/.+\.md$/.test(file)).length === 0) {
    missingArtifacts.push("tasks/issues/*.md");
  }

  const hasProductTruth =
    (await exists(args.cwd, "docs/mvp-scope.md")) ||
    (await exists(args.cwd, "docs/product-thesis.md")) ||
    (await exists(args.cwd, "docs/product-vision.md"));
  const hasExecutionSurface =
    (await exists(args.cwd, "docs/execution-rules.md")) &&
    taskFiles.some((file) => file.includes("tasks/"));
  const hasExistingDocsOrTasks = docs.length + taskFiles.length >= 2;
  const projectTypeSignal = hasProductTruth || hasExecutionSurface || hasExistingDocsOrTasks ? "existing" : "blank/new";

  const readinessWarnings: string[] = [];
  if (!hasProductTruth) {
    readinessWarnings.push("product truth or MVP scope is missing");
  }
  if (!(await exists(args.cwd, "docs/execution-rules.md"))) {
    readinessWarnings.push("execution rules are missing");
  }
  if (!(await exists(args.cwd, "docs/release-gates.md"))) {
    readinessWarnings.push("release or validation gates are missing");
  }
  if (taskFiles.length === 0) {
    readinessWarnings.push("task tracker is missing");
  }
  if (blueprintStatus && hasAny(blueprintStatus, ["missing", "blocked", "needs-validation"])) {
    readinessWarnings.push("blueprint status still contains missing, blocked, or needs-validation gates");
  }
  if (mvpScope && hasAny(mvpScope, ["prototype-assumption", "validation required before product truth"])) {
    readinessWarnings.push("MVP scope contains prototype assumptions or validation requirements");
  }

  const founderInputGaps: string[] = [];
  if (!(await exists(args.cwd, "docs/raw/founder-dump.md"))) {
    founderInputGaps.push("founder context dump is missing");
  }
  if (!(await exists(args.cwd, "docs/raw/founder-interview.md"))) {
    founderInputGaps.push("founder interview record is missing");
  }
  if (!mvpScope || /<one customer>|primary customer:\s*(unknown|<|$)|customer:\s*(unknown|<|$)/i.test(mvpScope)) {
    founderInputGaps.push("primary customer may need founder confirmation");
  }
  if (!mvpScope || /<one workflow>|primary workflow:\s*(unknown|<|$)|workflow:\s*(unknown|<|$)/i.test(mvpScope)) {
    founderInputGaps.push("primary workflow may need founder confirmation");
  }
  if (openQuestions && hasAny(openQuestions, ["founder", "primary user", "mvp", "positioning", "promise"])) {
    founderInputGaps.push("open founder-owned questions are recorded");
  }

  const inventory = {
    docsMarkdownFiles: docs.length,
    taskMarkdownFiles: taskFiles.length,
    packageOrBuildFiles: packageFiles.length,
    hasAgentInstructions:
      (await exists(args.cwd, "AGENTS.md")) || (await exists(args.cwd, "CLAUDE.md")),
    hasBlueprintStatus: await exists(args.cwd, "docs/blueprint-status.md"),
    hasSourceIndex: await exists(args.cwd, "docs/source-index.md"),
    hasMvpScope: await exists(args.cwd, "docs/mvp-scope.md"),
    hasExecutionRules: await exists(args.cwd, "docs/execution-rules.md"),
    hasReleaseGates: await exists(args.cwd, "docs/release-gates.md"),
    hasSprintTracker: await exists(args.cwd, "tasks/sprint-0.md"),
  };

  const visibleMissingArtifacts = args.mode === "readiness" ? [] : missingArtifacts;
  const visibleReadinessWarnings = args.mode === "inventory" ? [] : readinessWarnings;
  const visibleFounderInputGaps = args.mode === "inventory" ? [] : founderInputGaps;
  const publicEvidenceFiles = docs.filter((file) =>
    /^docs\/evidence\/.+\.md$/.test(file) &&
    /competitor|pricing|market|public|evidence-notes/.test(file),
  );
  const needsPublicResearch =
    /web-assisted|public-first-prototype/.test(activeSourceMode) && publicEvidenceFiles.length === 0;
  const needsDelegatedInventory =
    projectTypeSignal === "existing" &&
    docs.length + taskFiles.length >= 6 &&
    !(await exists(args.cwd, "docs/source-index.md"));
  const missingCoreDocs = missingArtifacts.filter((item) => !item.startsWith("tasks/"));
  const missingTaskSurface = missingArtifacts.some((item) => item.startsWith("tasks/"));

  let decision: PreflightDecision = "blocked";
  let nextAction = "Resolve readiness warnings before claiming execution readiness.";
  if (needsDelegatedInventory) {
    decision = "delegate-inventory";
    nextAction = "Run or prompt an existing-project inventory review before writing canonical artifacts.";
  } else if (founderInputGaps.length > 0) {
    decision = "ask-founder";
    nextAction = "Ask the smallest useful founder-question batch before claiming product readiness.";
  } else if (needsPublicResearch) {
    decision = "run-research";
    nextAction = "Run bounded public research and record public evidence before upgrading prototype claims.";
  } else if (missingCoreDocs.length > 0) {
    decision = "write-artifacts";
    nextAction = `Create or update missing canonical artifacts: ${missingCoreDocs.join(", ")}.`;
  } else if (missingTaskSurface) {
    decision = "create-sprint0";
    nextAction = "Create Sprint 0 and the first bounded executable task.";
  } else if (readinessWarnings.length === 0) {
    decision = "ready-for-execution";
    nextAction = "Preflight surface looks ready for a bounded execution task.";
  }

  const confidence = decision === "ready-for-execution"
    ? "high"
    : readinessWarnings.length > 0 || founderInputGaps.length > 0
      ? "medium"
      : "low";

  let recommendation = "Proceed with preflight inventory and scaffold missing artifacts.";
  if (readinessWarnings.length === 0 && founderInputGaps.length === 0) {
    recommendation = "Preflight surface looks ready for a bounded execution task.";
  } else if (founderInputGaps.length > 0) {
    recommendation = "Ask the smallest useful founder-question batch before claiming product readiness.";
  }

  return {
    cwd: args.cwd,
    mode: args.mode,
    decision,
    nextAction,
    confidence,
    projectTypeSignal,
    inventory,
    missingArtifacts: visibleMissingArtifacts,
    readinessWarnings: visibleReadinessWarnings,
    founderInputGaps: visibleFounderInputGaps,
    recommendation,
  };
}

function renderMarkdown(result: CheckResult): string {
  const lines = [
    "# Build Right Preflight Check",
    "",
    `CWD: ${result.cwd}`,
    `Mode: ${result.mode}`,
    `Decision: ${result.decision}`,
    `Confidence: ${result.confidence}`,
    `Project type signal: ${result.projectTypeSignal}`,
    "",
    "## Next Action",
    result.nextAction,
    "",
    "## Inventory",
    ...Object.entries(result.inventory).map(([key, value]) => `- ${key}: ${value}`),
  ];

  if (result.missingArtifacts.length > 0) {
    lines.push("", "## Missing Artifacts", ...result.missingArtifacts.map((item) => `- ${item}`));
  }

  if (result.readinessWarnings.length > 0) {
    lines.push("", "## Readiness Warnings", ...result.readinessWarnings.map((item) => `- ${item}`));
  }

  if (result.founderInputGaps.length > 0) {
    lines.push("", "## Founder Input Gaps", ...result.founderInputGaps.map((item) => `- ${item}`));
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
