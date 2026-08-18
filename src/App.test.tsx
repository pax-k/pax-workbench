import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "./lib/bridge";
import { createLocalSessionHandle } from "./lib/collaboration";
import { bootstrapArtifactPaths } from "./lib/discover-bootstrap";
import App, { isExecutionHelperTaskPath, skillSetupOperationFor } from "./App";
import type { BoundedTaskOutcome, BoundedTaskPreview, BoundedTaskResult, HelperResult, ProjectSnapshot, RuntimeResult, SkillSetupResult, SkillSummary } from "./types";

vi.mock("./lib/bridge", () => ({
  applyHa2haPublish: vi.fn(),
  applyLocalGitHandoff: vi.fn(),
  cancelBoundedTask: vi.fn(),
  cancelHelper: vi.fn(),
  cancelRuntime: vi.fn(),
  cancelSkillSetup: vi.fn(),
  clearGoalState: vi.fn(),
  chooseProject: vi.fn(),
  connectMdsyncSession: vi.fn(),
  describeProjectError: (error: unknown) => String(error),
  disconnectMdsyncSession: vi.fn(),
  executeHelper: vi.fn(),
  executeBoundedTask: vi.fn(),
  executeRuntime: vi.fn(),
  executeSharedBoundedTask: vi.fn(),
  executeSkillSetup: vi.fn(),
  inspectPostRunReview: vi.fn(),
  isTauriRuntime: vi.fn(() => false),
  joinHa2haWorkspace: vi.fn(),
  projectErrorCode: (error: unknown) => typeof error === "object" && error !== null && "code" in error ? String(error.code) : null,
  projectErrorCommitted: (error: unknown) => typeof error === "object" && error !== null && "committed" in error && error.committed === true,
  previewHa2haPublish: vi.fn(),
  previewLocalGitHandoff: vi.fn(),
  previewSkillSetup: vi.fn(),
  previewBoundedTask: vi.fn(),
  previewSharedBoundedTask: vi.fn(),
  readProjectFile: vi.fn(),
  recoverGoalState: vi.fn(),
  refreshProject: vi.fn(),
  repairCollaborationCompletion: vi.fn(),
  writeProjectFile: vi.fn(),
}));

const openedSetupProject = {
  root: "/tmp/setup-project",
  name: "setup-project",
  branch: "main",
  dirty: false,
  files: [],
  skills: [],
  errors: [],
};

const authorityFiles = bootstrapArtifactPaths.map((path) => ({
  path,
  name: path.split("/").at(-1) ?? path,
  kind: path.includes("/issues/") ? "task" as const : "document" as const,
}));

const setupPreview = {
  operation: "install" as const,
  targetProject: "/tmp/setup-project",
  source: "pax-k/build-right" as const,
  executable: "bun" as const,
  cliVersion: "skills@1.5.19" as const,
  argv: ["x", "skills@1.5.19", "add", "pax-k/build-right", "--skill", "build-right-preflight", "--agent", "codex", "--yes", "--copy"],
  skillIds: ["build-right-preflight"],
  expectedChangedPaths: ["skills-lock.json", ".agents/skills/build-right-preflight/"],
  hashChanges: [{ skillId: "build-right-preflight", currentHash: null, proposedHash: null, proposedState: "resolvedOnExecution" as const }],
  explicitConfirmationRequired: true as const,
  previewToken: "sha256:preview-baseline",
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function openDeveloperTools() {
  const tools = screen.getByRole("group", { name: "Developer tools and diagnostics" });
  if (!tools.hasAttribute("open")) {
    fireEvent.click(within(tools).getByText("Developer tools"));
  }
  return tools;
}

function setupResult(project: ProjectSnapshot, outcome: SkillSetupResult["outcome"] = "completed"): SkillSetupResult {
  const success = outcome === "completed";
  return {
    operation: "install",
    outcome,
    executed: true,
    success,
    exitStatus: success ? 0 : null,
    stdout: success ? "setup complete" : "",
    stderr: "",
    stdoutTruncated: false,
    stderrTruncated: false,
    changedPaths: ["skills-lock.json", ".agents/skills/build-right-preflight/SKILL.md"],
    before: [{ skillId: "build-right-preflight", installedPath: ".agents/skills/build-right-preflight/SKILL.md", installed: false, lockHash: null }],
    after: [{ skillId: "build-right-preflight", installedPath: ".agents/skills/build-right-preflight/SKILL.md", installed: true, lockHash: "post-hash" }],
    repair: success ? null : { code: `skill_setup_${outcome}`, message: outcome, nextAction: "review" },
    project,
  };
}

function helperResult(project: ProjectSnapshot, overrides: Partial<HelperResult> = {}): HelperResult {
  return {
    helperId: "preflight-check",
    mode: null,
    taskPath: null,
    executable: "bun",
    argv: [".agents/skills/build-right-preflight/scripts/preflight-check.ts", "--cwd", project.root, "--mode", "all", "--format", "json"],
    outcome: "completed",
    executed: true,
    success: true,
    exitStatus: 0,
    stdout: "{\"decision\":\"ready-for-execution\"}",
    stderr: "",
    stdoutTruncated: false,
    stderrTruncated: false,
    decision: { decision: "ready-for-execution", confidence: "high", nextAction: "Execute one task", evidence: ["docsMarkdownFiles: 4"], warnings: [] },
    failure: null,
    project,
    ...overrides,
  };
}

function runtimeResult(mode: "fixture" | "live", overrides: Partial<RuntimeResult> = {}): RuntimeResult {
  const fixtureKinds = ["session", "turn", "message", "unknown", "usage"] as const;
  const events = mode === "fixture"
    ? fixtureKinds.map((kind, sequence) => ({
      sequence,
      kind,
      providerType: `fixture.${kind}`,
      summary: `Fixture stream ${sequence + 1}`,
      rawPayload: { encoding: "utf8" as const, data: `{"type":"fixture.${kind}"}` },
      provenance: "fixture" as const,
    }))
    : [{ sequence: 0, kind: "message" as const, providerType: "item.completed", summary: "Fixture response", rawPayload: { encoding: "utf8" as const, data: "{\"type\":\"item.completed\"}" }, provenance: "provider" as const }];
  return {
    runId: mode === "live" ? "0123456789abcdef0123456789abcdef" : "fedcba9876543210fedcba9876543210",
    outcome: "completed",
    executed: mode === "live",
    success: true,
    exitStatus: mode === "live" ? 0 : null,
    events,
    stdout: { encoding: "utf8", data: "{\"type\":\"item.completed\"}\n" },
    stderr: { encoding: "utf8", data: "" },
    stdoutTruncated: false,
    stderrTruncated: false,
    failure: null,
    capabilities: { eventStream: true, cancellation: true, timeout: true, rawPayload: true, fixture: true, live: true, repositoryAuthority: false },
    provenance: {
      adapter: "runtime-port/v1",
      provider: "codex-jsonl/v1",
      mode,
      executable: "/fixed/codex",
      runtimeVersion: mode === "live" ? "codex-cli 0.144.4" : "fixture-schema 1",
      projectRoot: "/tmp/setup-project",
      argv: mode === "live" ? ["exec", "--json", "prompt"] : [],
      simulated: mode === "fixture",
    },
    repositoryAuthorityAdvanced: false,
    ...overrides,
  };
}

function emitRuntimeFixture(onMessage: Parameters<typeof bridge.executeRuntime>[2], mode: "fixture" | "live") {
  const result = runtimeResult(mode);
  onMessage({ type: "started", handle: { runId: result.runId, capabilities: result.capabilities, provenance: result.provenance } });
  for (const event of result.events) onMessage({ type: "event", runId: result.runId, event });
  return result;
}

const boundedPreview: BoundedTaskPreview = {
  decision: "execute-task",
  confidence: "high",
  nextAction: "Execute ready task 009",
  blockingGates: [],
  selectedTask: "tasks/issues/009-test.md",
  executable: true,
  goal: "Prove one bounded task.",
  nonGoals: ["Do not select another task."],
  sourceUnderTest: "repo-local path",
  expectedEffects: ["Codex may edit files inside the selected repository"],
  liveHostWarning: "Live bounded execution uses Codex workspace-write with host permissions.",
  prompt: "native-owned prompt",
  previewToken: "sha256:bounded-preview",
  loopState: {
    state: "awaitingConfirmation",
    nextTask: "tasks/issues/009-test.md",
    blockingGates: [],
    expectedEffects: ["Codex may edit files inside the selected repository"],
    explicitConfirmationRequired: true,
    automaticExecutionStarted: false,
    reason: "One task awaits confirmation",
  },
};

function boundedTaskResult(project: ProjectSnapshot, outcome: BoundedTaskOutcome): BoundedTaskResult {
  const verified = outcome === "verified";
  const resolverDecision = outcome === "waitExternal" ? "wait-external" : "no-ready-task";
  return {
    outcome,
    selectedTask: boundedPreview.selectedTask,
    runtime: runtimeResult("fixture"),
    project,
    taskEvidence: {
      path: boundedPreview.selectedTask!,
      content: verified
        ? "# 009: Test\n\nStatus: complete\nOwner: AI\n\n## Acceptance Criteria\n\n- [x] proved\n\n## Evidence Log\n\n| command | pass |\n\n## Verification Summary\n\nPassed."
        : "# 009: Test\n\nStatus: active\nOwner: AI\n\n## Acceptance Criteria\n\n- [ ] proved",
      version: "sha256:post-run",
    },
    resolver: helperResult(project, { helperId: "continue-check", decision: { decision: resolverDecision, confidence: "high", nextAction: outcome === "waitExternal" ? "Wait for external review" : "Stop", evidence: [], warnings: [] } }),
    stopGates: helperResult(project, { helperId: "execution-check", mode: "stop-gates", taskPath: boundedPreview.selectedTask, decision: { decision: "stop", confidence: "high", nextAction: "Stop", evidence: [], warnings: [] } }),
    refreshFailures: [],
    repositoryVerified: verified,
    reason: verified ? "Repository evidence passed" : outcome === "waitExternal" ? "Wait for external review" : "Repository verification failed",
    loopState: {
      state: verified ? "noReadyTaskStop" : outcome === "waitExternal" ? "externalStop" : "failureStop",
      nextTask: null,
      blockingGates: outcome === "waitExternal" ? ["external review"] : [],
      expectedEffects: [],
      explicitConfirmationRequired: false,
      automaticExecutionStarted: false,
      reason: verified ? "No next task" : outcome === "waitExternal" ? "Wait for external review" : "Repository verification failed",
    },
  };
}

function validSkill(id: string): SkillSummary {
  return {
    id,
    name: id,
    phase: "Build",
    purpose: "test",
    reads: [],
    writes: [],
    decisions: [],
    helpers: [],
    requiredEvidence: [],
    stopStates: [],
    renderer: "operating-card",
    executable: false,
    source: "pax-k/build-right",
    installedPath: `.agents/skills/${id}/SKILL.md`,
    lockHash: `hash-${id}`,
  };
}

function readyExecutionSkills(): SkillSummary[] {
  return [
    { ...validSkill("build-right-preflight"), phase: "Discover", helpers: ["preflight-check"] },
    { ...validSkill("build-right-execution"), helpers: ["continue-check", "execution-check"] },
  ];
}

describe("Build Right Studio workbench", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(false);
    vi.mocked(bridge.cancelHelper).mockResolvedValue({ cancellationRequested: true, message: "Helper cancellation requested" });
    vi.mocked(bridge.cancelBoundedTask).mockResolvedValue({ cancellationRequested: true, message: "Bounded task cancellation requested" });
    vi.mocked(bridge.cancelRuntime).mockResolvedValue({ cancellationRequested: true, message: "Runtime cancellation requested" });
    vi.mocked(bridge.cancelSkillSetup).mockResolvedValue({ cancellationRequested: true, message: "Cancellation requested" });
    vi.mocked(bridge.clearGoalState).mockResolvedValue();
    vi.mocked(bridge.disconnectMdsyncSession).mockResolvedValue();
    vi.mocked(bridge.inspectPostRunReview).mockResolvedValue({
      scopeNote: "Current Git working tree; may include pre-existing changes.",
      changedFiles: [],
      truncated: false,
    });
    vi.mocked(bridge.recoverGoalState).mockResolvedValue({
      state: "missing",
      objective: null,
      repository: null,
      runId: null,
      eventCursor: null,
      checkpointTask: null,
      evidenceReferences: [],
      collaboration: null,
      stopConditions: [],
      reason: "No persisted goal exists",
      explicitConfirmationRequired: false,
      automaticExecutionStarted: false,
    });
  });

  it("matches execution-check's supported direct task inventory", () => {
    expect(isExecutionHelperTaskPath("tasks/issues/007-test.md")).toBe(true);
    expect(isExecutionHelperTaskPath("tasks/root-task.md")).toBe(true);
    expect(isExecutionHelperTaskPath("issues/root-issue.md")).toBe(true);
    expect(isExecutionHelperTaskPath("tasks/sprint-0.md")).toBe(false);
    expect(isExecutionHelperTaskPath("tasks/post-release-backlog.md")).toBe(false);
    expect(isExecutionHelperTaskPath("tasks/issues/nested/007-test.md")).toBe(false);
  });

  it.each(["resumable", "staleTask", "interrupted"] as const)("projects %s restart truth and requires a fresh two-step confirmation with zero automatic Codex", async (state) => {
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(openedSetupProject);
    vi.mocked(bridge.recoverGoalState).mockResolvedValue({
      state,
      objective: "Resume safely",
      repository: { canonicalPath: openedSetupProject.root, repositoryId: "sha256:repo" },
      runId: "0123456789abcdef0123456789abcdef",
      eventCursor: 3,
      checkpointTask: "tasks/issues/010.md",
      evidenceReferences: [{ path: "tasks/issues/010.md", sha256: "sha256:task" }],
      collaboration: null,
      stopConditions: ["founder-owned decision required", "external state required"],
      reason: `${state} recovery reason`,
      explicitConfirmationRequired: state === "resumable",
      automaticExecutionStarted: false,
    });
    vi.mocked(bridge.previewBoundedTask).mockResolvedValue(boundedPreview);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    const recovery = await screen.findByRole("region", { name: "Goal recovery state" });
    expect(within(recovery).getByRole("heading", { name: state })).toBeInTheDocument();
    expect(within(recovery).getByText(/Automatic Codex execution started: false/)).toBeInTheDocument();
    expect(bridge.previewBoundedTask).not.toHaveBeenCalled();
    expect(bridge.executeBoundedTask).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Resume from verified checkpoint" }));
    const confirmation = await screen.findByRole("region", { name: "Bounded task confirmation" });
    expect(screen.getByRole("button", { name: "Confirm and execute one task" })).toBeInTheDocument();
    expect(within(confirmation).getByText(/macOS attributes that child's provider connection/)).toBeInTheDocument();
    expect(bridge.previewBoundedTask).toHaveBeenCalledTimes(1);
    expect(bridge.executeBoundedTask).not.toHaveBeenCalled();
    expect(bridge.executeRuntime).not.toHaveBeenCalled();
  });

  it("reopens a recent project by reinspecting repository and recovery truth without starting helpers or Codex", async () => {
    const inspected = {
      ...openedSetupProject,
      files: [{ path: "tasks/issues/026-shell.md", name: "026 shell", kind: "task" as const, status: "ready" }],
    };
    window.localStorage.setItem("build-right-studio/recent-projects/v1", JSON.stringify([{
      root: inspected.root,
      lastOpenedAt: 42,
      selectedSkill: "build-right-execution",
      view: "structured",
    }]));
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.refreshProject).mockResolvedValue(inspected);
    vi.mocked(bridge.recoverGoalState).mockResolvedValue({
      state: "resumable",
      objective: "Resume shell task",
      repository: { canonicalPath: inspected.root, repositoryId: "sha256:repo" },
      runId: "run",
      eventCursor: 2,
      checkpointTask: "tasks/issues/026-shell.md",
      evidenceReferences: [],
      collaboration: null,
      stopConditions: [],
      reason: "truthful checkpoint",
      explicitConfirmationRequired: true,
      automaticExecutionStarted: false,
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Reopen setup-project" }));

    await waitFor(() => expect(bridge.refreshProject).toHaveBeenCalledWith(inspected.root));
    expect(bridge.chooseProject).not.toHaveBeenCalled();
    expect(bridge.recoverGoalState).toHaveBeenCalledWith(inspected.root);
    expect(await screen.findByRole("main")).toHaveAttribute("data-goal-shell-state", "resumable");
    expect(screen.getByText("Resume shell task")).toBeInTheDocument();
    expect(bridge.executeHelper).not.toHaveBeenCalled();
    expect(bridge.previewBoundedTask).not.toHaveBeenCalled();
    expect(bridge.executeBoundedTask).not.toHaveBeenCalled();
    expect(bridge.executeRuntime).not.toHaveBeenCalled();
  });

  it("keeps bounded preparation closed when repository recovery affirms goal completion", async () => {
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(openedSetupProject);
    vi.mocked(bridge.recoverGoalState).mockResolvedValue({
      state: "completed",
      objective: "Finished objective",
      repository: { canonicalPath: openedSetupProject.root, repositoryId: "sha256:repo" },
      runId: "run",
      eventCursor: 8,
      checkpointTask: "tasks/issues/026-shell.md",
      evidenceReferences: [],
      collaboration: null,
      stopConditions: [],
      reason: "Repository terminal truth affirms completion",
      explicitConfirmationRequired: false,
      automaticExecutionStarted: false,
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));

    expect(await screen.findByRole("main")).toHaveAttribute("data-goal-shell-state", "complete");
    expect(screen.getByRole("button", { name: "Review completed goal" })).toBeEnabled();
    expect(screen.queryByRole("button", { name: "Review resolver-selected task" })).not.toBeInTheDocument();
    expect(bridge.previewBoundedTask).not.toHaveBeenCalled();
  });

  it("keeps manual Markdown selection as inspection and does not let it become shell task authority", async () => {
    const centeredProject: ProjectSnapshot = {
      ...openedSetupProject,
      name: "centered",
      files: [
        { path: "docs/source-index.md", name: "Source index", kind: "document" },
        { path: "docs/mvp-scope.md", name: "MVP scope", kind: "document" },
        { path: "docs/release-gates.md", name: "Release gates", kind: "document" },
        { path: "tasks/issues/999-inspect.md", name: "Inspect only", kind: "task", status: "ready" },
      ],
      skills: readyExecutionSkills(),
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(centeredProject);
    vi.mocked(bridge.readProjectFile).mockResolvedValue({
      path: "tasks/issues/999-inspect.md",
      content: "# 999: Inspect only\n\nStatus: ready\n\n## Goal\n\nThis editor text is not the active goal.",
      version: "sha256:inspect",
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText(/Select a Markdown file to edit only for advanced inspection/);
    const shell = screen.getByRole("main");
    expect(shell).toHaveAttribute("data-goal-shell-state", "ready");
    expect(screen.getByText("Advance centered")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Open file tasks/issues/999-inspect.md" }));
    await waitFor(() => expect(bridge.readProjectFile).toHaveBeenCalledWith(
      centeredProject.root,
      "tasks/issues/999-inspect.md",
    ));
    expect(shell).toHaveAttribute("data-goal-shell-state", "ready");
    expect(screen.getByText("Advance centered")).toBeInTheDocument();
    expect(screen.getByText("Resolver pending")).toBeInTheDocument();
  });

  it.each([
    ["verified", "Repository verification passed"],
    ["verificationFailed", "Repository verification failed"],
    ["waitExternal", "Wait external"],
  ] as const)("runs one deterministic bounded controller fixture and stops at %s", async (outcome, terminalLabel) => {
    const project: ProjectSnapshot = {
      ...openedSetupProject,
      files: [...authorityFiles, { path: boundedPreview.selectedTask!, name: "009 test", kind: "task", status: "ready" }],
      skills: readyExecutionSkills(),
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(project);
    vi.mocked(bridge.executeHelper).mockResolvedValue(helperResult(project));
    vi.mocked(bridge.previewBoundedTask).mockResolvedValue(boundedPreview);
    vi.mocked(bridge.executeBoundedTask).mockImplementation(async (_root, _invocation, onMessage) => {
      const runtime = runtimeResult("fixture");
      onMessage({ type: "started", handle: { runId: runtime.runId, capabilities: runtime.capabilities, provenance: runtime.provenance } });
      return boundedTaskResult(project, outcome);
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));
    await screen.findByRole("button", { name: "Review resolver-selected task" });
    fireEvent.click(screen.getByRole("button", { name: "Review resolver-selected task" }));

    const preview = await screen.findByRole("region", { name: "Bounded task confirmation" });
    expect(within(preview).getByText(/execute-task · confidence high/)).toBeInTheDocument();
    expect(within(preview).getByText(/tasks\/issues\/009-test.md/)).toBeInTheDocument();
    expect(within(preview).getByText(/workspace-write with host permissions/)).toBeInTheDocument();
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run controller fixture" }));

    await waitFor(() => expect(bridge.executeBoundedTask).toHaveBeenCalledTimes(1));
    expect(bridge.executeBoundedTask).toHaveBeenCalledWith("/tmp/setup-project", {
      mode: "fixture",
      previewToken: "sha256:bounded-preview",
      selectedTask: "tasks/issues/009-test.md",
      confirmed: true,
    }, expect.any(Function));
    const result = await screen.findByRole("region", { name: "Bounded task result" });
    const headline = outcome === "verified"
      ? "Repository verification passed"
      : outcome === "verificationFailed"
        ? "Repository verification failed"
        : "Run stopped on a blocker";
    expect(within(result).getByRole("heading", { name: headline })).toBeInTheDocument();
    expect(screen.getAllByText(terminalLabel).length).toBeGreaterThan(0);
    expect(within(result).getByText(/do not revert, stage, commit, push, publish, or rerun Codex/i)).toBeInTheDocument();
    expect(within(result).queryByRole("button", { name: "Review next task" })).not.toBeInTheDocument();
    fireEvent.click(within(result).getByRole("button", { name: "Accept for handoff" }));
    expect(within(result).getByRole("status")).toHaveTextContent("accepted for a separate handoff action");
    expect(bridge.executeBoundedTask).toHaveBeenCalledTimes(1);
    expect(bridge.writeProjectFile).not.toHaveBeenCalled();
    expect(bridge.previewBoundedTask).toHaveBeenCalledTimes(1);
    expect(bridge.executeRuntime).not.toHaveBeenCalled();
  });

  it("runs two iterations only after two separate confirmations and then stops", async () => {
    const secondPreview: BoundedTaskPreview = {
      ...boundedPreview,
      selectedTask: "tasks/issues/010-test.md",
      goal: "Prove the second bounded task.",
      previewToken: "sha256:second-preview",
      loopState: { ...boundedPreview.loopState, nextTask: "tasks/issues/010-test.md" },
    };
    const project: ProjectSnapshot = {
      ...openedSetupProject,
      files: [
        ...authorityFiles,
        { path: boundedPreview.selectedTask!, name: "009 test", kind: "task", status: "ready" },
        { path: secondPreview.selectedTask!, name: "010 test", kind: "task", status: "ready" },
      ],
      skills: readyExecutionSkills(),
    };
    const firstResult: BoundedTaskResult = {
      ...boundedTaskResult(project, "verified"),
      loopState: {
        state: "continueAvailable",
        nextTask: secondPreview.selectedTask,
        blockingGates: [],
        expectedEffects: secondPreview.expectedEffects,
        explicitConfirmationRequired: true,
        automaticExecutionStarted: false,
        reason: "Checkpoint recorded; next ready AI task selected",
      },
    };
    const secondResult: BoundedTaskResult = {
      ...boundedTaskResult(project, "verified"),
      selectedTask: secondPreview.selectedTask,
      taskEvidence: {
        path: secondPreview.selectedTask!,
        content: "# 010: Test\n\nStatus: complete\n\n## Acceptance Criteria\n\n- [x] proved\n\n## Evidence Log\n\n| command | pass |\n\n## Verification Summary\n\nPassed.",
        version: "sha256:second-post-run",
      },
      loopState: {
        state: "noReadyTaskStop",
        nextTask: null,
        blockingGates: [],
        expectedEffects: [],
        explicitConfirmationRequired: false,
        automaticExecutionStarted: false,
        reason: "No ready AI-owned task remains",
      },
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(project);
    vi.mocked(bridge.executeHelper).mockResolvedValue(helperResult(project));
    vi.mocked(bridge.previewBoundedTask)
      .mockResolvedValueOnce(boundedPreview)
      .mockResolvedValueOnce(secondPreview);
    vi.mocked(bridge.executeBoundedTask)
      .mockResolvedValueOnce(firstResult)
      .mockResolvedValueOnce(secondResult);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));
    await screen.findByRole("button", { name: "Review resolver-selected task" });
    fireEvent.click(screen.getByRole("button", { name: "Review resolver-selected task" }));
    const firstConfirmation = await screen.findByRole("region", { name: "Bounded task confirmation" });
    expect(bridge.executeBoundedTask).not.toHaveBeenCalled();
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run controller fixture" }));
    await waitFor(() => expect(bridge.executeBoundedTask).toHaveBeenCalledTimes(1));

    const firstTransition = await screen.findByRole("region", { name: "Bounded task result" });
    expect(within(firstTransition).getByText("continueAvailable")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Review next task" }));
    const secondConfirmation = await screen.findByRole("region", { name: "Bounded task confirmation" });
    expect(within(secondConfirmation).getByText(/tasks\/issues\/010-test.md/)).toBeInTheDocument();
    expect(bridge.executeBoundedTask).toHaveBeenCalledTimes(1);
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run controller fixture" }));

    await waitFor(() => expect(bridge.executeBoundedTask).toHaveBeenCalledTimes(2));
    const terminal = await screen.findByRole("region", { name: "Bounded task result" });
    expect(within(terminal).getByText("noReadyTaskStop")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Review next task" })).not.toBeInTheDocument();
    expect(bridge.previewBoundedTask).toHaveBeenCalledTimes(2);
  });

  it("renders typed resolver stop evidence without offering execution", async () => {
    const project: ProjectSnapshot = {
      ...openedSetupProject,
      files: [...authorityFiles, { path: boundedPreview.selectedTask!, name: "009 test", kind: "task", status: "ready" }],
      skills: readyExecutionSkills(),
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(project);
    vi.mocked(bridge.executeHelper).mockResolvedValue(helperResult(project));
    vi.mocked(bridge.previewBoundedTask).mockResolvedValue({
      ...boundedPreview,
      decision: "wait-external",
      confidence: "medium",
      nextAction: "Wait for external review",
      blockingGates: ["tasks/issues/010.md: external review"],
      selectedTask: null,
      executable: false,
      goal: "",
      nonGoals: [],
      sourceUnderTest: "",
      expectedEffects: [],
      prompt: "",
    });
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));
    await screen.findByRole("button", { name: "Review resolver-selected task" });
    fireEvent.click(screen.getByRole("button", { name: "Review resolver-selected task" }));
    const preview = await screen.findByRole("region", { name: "Bounded task confirmation" });
    expect(within(preview).getByText(/wait-external · confidence medium/)).toBeInTheDocument();
    expect(within(preview).getAllByText(/external review/)).toHaveLength(2);
    expect(screen.queryByRole("button", { name: "Confirm and execute one task" })).not.toBeInTheDocument();
    expect(bridge.executeBoundedTask).not.toHaveBeenCalled();
  });

  it("renders only obtained refresh surfaces and typed partial failures", async () => {
    const project: ProjectSnapshot = {
      ...openedSetupProject,
      files: [...authorityFiles, { path: boundedPreview.selectedTask!, name: "009 test", kind: "task", status: "ready" }],
      skills: readyExecutionSkills(),
    };
    const partial: BoundedTaskResult = {
      ...boundedTaskResult(project, "verificationFailed"),
      outcome: "stopped",
      runtime: null,
      taskEvidence: null,
      resolver: null,
      stopGates: null,
      refreshFailures: [
        { surface: "taskEvidence", code: "controller_task_untrusted", message: "task path changed" },
        { surface: "resolver", code: "refresh_cancelled", message: "resolver not launched" },
        { surface: "stopGates", code: "refresh_cancelled", message: "stop gates not launched" },
      ],
      reason: "Controller stopped with partial repository evidence",
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(project);
    vi.mocked(bridge.executeHelper).mockResolvedValue(helperResult(project));
    vi.mocked(bridge.previewBoundedTask).mockResolvedValue(boundedPreview);
    vi.mocked(bridge.executeBoundedTask).mockResolvedValue(partial);
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));
    await screen.findByRole("button", { name: "Review resolver-selected task" });
    fireEvent.click(screen.getByRole("button", { name: "Review resolver-selected task" }));
    const preview = await screen.findByRole("region", { name: "Bounded task confirmation" });
    const runFixture = within(openDeveloperTools()).getByRole("button", { name: "Run controller fixture" });
    await waitFor(() => expect(runFixture).toBeEnabled());
    fireEvent.click(runFixture);
    await waitFor(() => expect(bridge.executeBoundedTask).toHaveBeenCalledTimes(1));
    const receipt = await screen.findByRole("region", { name: "Post-run review receipt" });
    expect(within(receipt).getByRole("heading", { name: "Repository verification failed" })).toBeInTheDocument();
    expect(within(receipt).getByText("Controller stopped with partial repository evidence")).toBeInTheDocument();
    expect(await within(receipt).findByText(/Current Git working tree/)).toBeInTheDocument();
    const failures = screen.getByLabelText("Controller refresh failures");
    expect(within(failures).getByText("controller_task_untrusted")).toBeInTheDocument();
    expect(within(failures).getAllByText("refresh_cancelled")).toHaveLength(2);
  });

  it("does not guess a macOS Local Network repair path for an untyped live failure", async () => {
    const project: ProjectSnapshot = {
      ...openedSetupProject,
      files: [...authorityFiles, { path: boundedPreview.selectedTask!, name: "009 test", kind: "task", status: "ready" }],
      skills: readyExecutionSkills(),
    };
    const failed = boundedTaskResult(project, "verificationFailed");
    failed.runtime = runtimeResult("live", { success: false, outcome: "timedOut", failure: "No provider events arrived" });
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(project);
    vi.mocked(bridge.executeHelper).mockResolvedValue(helperResult(project));
    vi.mocked(bridge.previewBoundedTask).mockResolvedValue(boundedPreview);
    vi.mocked(bridge.executeBoundedTask).mockResolvedValue(failed);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));
    await screen.findByRole("button", { name: "Review resolver-selected task" });
    fireEvent.click(screen.getByRole("button", { name: "Review resolver-selected task" }));
    const preview = await screen.findByRole("region", { name: "Bounded task confirmation" });
    fireEvent.click(screen.getByRole("button", { name: "Confirm and execute one task" }));

    const result = await screen.findByRole("region", { name: "Bounded task result" });
    expect(within(result).queryByText("Check Local Network access")).not.toBeInTheDocument();
    expect(within(result).queryByText(/System Settings.*Privacy.*Security.*Local Network/)).not.toBeInTheDocument();
  });

  it("derives setup operation from the exact valid first-party set and ignores extras", () => {
    const ids = [
      "build-right-preflight",
      "build-right-feature-planning",
      "build-right-execution",
      "build-right-engineering-principles",
    ];
    const exact = ids.map(validSkill);
    expect(skillSetupOperationFor(exact)).toBe("update");
    expect(skillSetupOperationFor([...exact, validSkill("unrelated")])).toBe("update");
    expect(skillSetupOperationFor(exact.slice(0, 3))).toBe("install");
    expect(skillSetupOperationFor(ids.map((id) => ({ ...validSkill(id), source: "other/source" })))).toBe("install");
    expect(skillSetupOperationFor(ids.map((id) => ({ ...validSkill(id), installedPath: `.agents/other/${id}` })))).toBe("install");
    expect(skillSetupOperationFor(ids.map((id) => ({ ...validSkill(id), lockHash: undefined })))).toBe("install");
    expect(skillSetupOperationFor(ids.map((id) => validSkill(`unrelated-${id}`)))).toBe("install");
  });

  it("renders repo authority and labels simulated activity", () => {
    render(<App />);
    expect(screen.getByRole("main")).toHaveAttribute("data-project-session", "selected");
    expect(screen.getByRole("main")).toHaveAttribute("data-workflow-state", "noProject");
    expect(screen.getByRole("main")).toHaveAttribute("data-workflow-mode", "localSolo");
    expect(screen.getByText("pax-workbench")).toBeInTheDocument();
    expect(screen.getByText("Raw Markdown is authoritative")).toBeInTheDocument();
    expect(screen.getByText("SIMULATED")).toBeInTheDocument();
    expect(within(openDeveloperTools()).getByRole("button", { name: "Simulate checkpoint" })).toBeEnabled();
  });

  it("labels the complete demo helper lifecycle as simulated", async () => {
    render(<App />);
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));

    for (const label of ["Helper request", "Helper start", "Helper output", "Helper decision", "Repository refresh"]) {
      const event = await screen.findByText(label);
      expect(within(event.closest("article")!).getByText("simulated")).toBeInTheDocument();
    }
    expect(bridge.executeHelper).not.toHaveBeenCalled();
    expect(screen.getByText(/no helper process executed/i)).toBeInTheDocument();
  });

  it("keeps dry and confirmed live runtime runs distinct and never advances repository authority", async () => {
    const fixtureTerminal = deferred<RuntimeResult>();
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(openedSetupProject);
    vi.mocked(bridge.executeRuntime)
      .mockImplementationOnce((_root, _invocation, onMessage) => {
        emitRuntimeFixture(onMessage, "fixture");
        return fixtureTerminal.promise;
      })
      .mockImplementationOnce(async (_root, _invocation, onMessage) => emitRuntimeFixture(onMessage, "live"));
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");

    const tools = openDeveloperTools();
    fireEvent.click(within(tools).getByRole("button", { name: "Run dry runtime fixture" }));
    await waitFor(() => expect(bridge.executeRuntime).toHaveBeenCalledWith("/tmp/setup-project", {
      mode: "fixture",
      prompt: undefined,
      confirmed: false,
    }, expect.any(Function)));
    for (const sequence of [1, 2, 3, 4, 5]) {
      expect(await screen.findByText(`Fixture stream ${sequence}`)).toBeInTheDocument();
    }
    expect(screen.queryByRole("region", { name: "Runtime result" })).not.toBeInTheDocument();
    fixtureTerminal.resolve(runtimeResult("fixture"));
    expect(await screen.findByText(/Typed runtime diagnostic · fixture/)).toBeInTheDocument();
    fireEvent.click(screen.getByText("Raw bounded payloads and adapter metadata"));
    expect(screen.getByText(/fixture: no argv and no spawn/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Clear diagnostic result" }));

    fireEvent.change(screen.getByRole("textbox", { name: "Runtime prompt" }), { target: { value: "Inspect only and report evidence." } });
    fireEvent.click(within(tools).getByRole("button", { name: "Confirm read-only runtime probe" }));
    expect(confirm).toHaveBeenCalledOnce();
    await waitFor(() => expect(bridge.executeRuntime).toHaveBeenLastCalledWith("/tmp/setup-project", {
      mode: "live",
      prompt: "Inspect only and report evidence.",
      confirmed: true,
    }, expect.any(Function)));
    expect(await screen.findByText(/Typed runtime diagnostic · adapter/)).toBeInTheDocument();
    fireEvent.click(screen.getByText("Raw bounded payloads and adapter metadata"));
    expect(screen.getByText(/codex-cli 0\.144\.4/)).toBeInTheDocument();
    expect(bridge.refreshProject).not.toHaveBeenCalled();
    confirm.mockRestore();
  });

  it("keeps runtime cancellation callable while one live invocation is pending", async () => {
    const execution = deferred<RuntimeResult>();
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(openedSetupProject);
    vi.mocked(bridge.executeRuntime).mockImplementation((_root, _invocation, onMessage) => {
      const result = runtimeResult("live");
      onMessage({ type: "started", handle: { runId: result.runId, capabilities: result.capabilities, provenance: result.provenance } });
      onMessage({ type: "event", runId: result.runId, event: { ...result.events[0], summary: "Streamed before terminal" } });
      return execution.promise;
    });
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    const tools = openDeveloperTools();
    fireEvent.click(within(tools).getByRole("button", { name: "Confirm read-only runtime probe" }));
    expect(await screen.findByText("Streamed before terminal")).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Runtime result" })).not.toBeInTheDocument();
    expect(await within(tools).findByRole("button", { name: "Cancel runtime probe" })).toBeEnabled();
    fireEvent.click(within(tools).getByRole("button", { name: "Cancel runtime probe" }));
    await waitFor(() => expect(bridge.cancelRuntime).toHaveBeenCalledWith("0123456789abcdef0123456789abcdef"));
    execution.resolve(runtimeResult("live", { outcome: "cancelled", success: false, failure: "cancelled" }));
    expect(await screen.findByText(/Runtime cancelled: cancelled/)).toBeInTheDocument();
    confirm.mockRestore();
  });

  it("blocks setup confirmation while a helper owns the shared local-operation boundary", async () => {
    const nativeProject: ProjectSnapshot = {
      ...openedSetupProject,
      skills: [{ ...validSkill("build-right-preflight"), phase: "Discover", helpers: ["preflight-check"] }],
    };
    const helperExecution = deferred<HelperResult>();
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(nativeProject);
    vi.mocked(bridge.previewSkillSetup).mockResolvedValue(setupPreview);
    vi.mocked(bridge.executeHelper).mockReturnValue(helperExecution.promise);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Inspect skill setup adapter" }));
    const confirmSetup = await screen.findByRole("button", { name: "Confirm install" });
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));
    await waitFor(() => expect(bridge.executeHelper).toHaveBeenCalledTimes(1));
    expect(confirmSetup).toBeDisabled();
    fireEvent.click(confirmSetup);
    expect(bridge.executeSkillSetup).not.toHaveBeenCalled();

    helperExecution.resolve(helperResult(nativeProject));
    await waitFor(() => expect(confirmSetup).toBeEnabled());
  });

  it("releases frontend operation state when the shared native lease rejects a runtime start", async () => {
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(openedSetupProject);
    vi.mocked(bridge.executeRuntime).mockRejectedValue({ code: "operation_in_progress", message: "Another local operation is already in progress" });
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    const tools = openDeveloperTools();
    fireEvent.click(within(tools).getByRole("button", { name: "Confirm read-only runtime probe" }));
    expect(await screen.findByText(/Runtime failed before a structured terminal result/)).toBeInTheDocument();
    await waitFor(() => expect(within(tools).getByRole("button", { name: "Confirm read-only runtime probe" })).toBeEnabled());
    expect(screen.getByRole("button", { name: /Project setup-project/ })).toBeEnabled();
    expect(within(tools).queryByRole("button", { name: "Cancel runtime probe" })).not.toBeInTheDocument();
    confirm.mockRestore();
  });

  it("runs and cancels an allowlisted native helper while blocking competing controls", async () => {
    const nativeProject: ProjectSnapshot = {
      ...openedSetupProject,
      files: [...authorityFiles, { path: "tasks/issues/007-test.md", name: "007 test", kind: "task", status: "ready" }],
      skills: [{ ...validSkill("build-right-preflight"), phase: "Discover", helpers: ["preflight-check"] }],
    };
    const execution = deferred<HelperResult>();
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(nativeProject);
    vi.mocked(bridge.executeHelper).mockReturnValue(execution.promise);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));

    await waitFor(() => expect(bridge.executeHelper).toHaveBeenCalledWith("/tmp/setup-project", { helperId: "preflight-check" }));
    expect(screen.getByText("Helper request")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Project setup-project/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Refresh repository" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Cancel helper" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Cancel helper" }));
    await waitFor(() => expect(bridge.cancelHelper).toHaveBeenCalledWith("/tmp/setup-project"));

    execution.resolve(helperResult({ ...nativeProject, dirty: true }, {
      outcome: "cancelled",
      success: false,
      decision: null,
      failure: "Helper invocation was cancelled and its process group was reaped",
    }));
    expect(await screen.findByText("Helper cancellation")).toBeInTheDocument();
    expect(screen.getByText("Helper start")).toBeInTheDocument();
    expect(screen.getByText("Helper output")).toBeInTheDocument();
    expect(screen.getByText("Repository refresh")).toBeInTheDocument();
    const start = screen.getByText("Helper start").closest("article");
    expect(start && within(start).getByText("real local effect")).toBeInTheDocument();
    expect(screen.getByText("dirty")).toBeInTheDocument();
  });

  it("sends execution-check only the closed mode and selected inventoried task path", async () => {
    const nativeProject: ProjectSnapshot = {
      ...openedSetupProject,
      files: [...authorityFiles, { path: "tasks/issues/007-test.md", name: "007 test", kind: "task", status: "ready" }],
      skills: readyExecutionSkills(),
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(nativeProject);
    vi.mocked(bridge.readProjectFile).mockResolvedValue({ path: "tasks/issues/007-test.md", content: "# 007: Test\n\nStatus: ready", version: "sha256:task" });
    vi.mocked(bridge.executeHelper).mockResolvedValue(helperResult(nativeProject, {
      helperId: "execution-check",
      mode: "task-contract",
      taskPath: "tasks/issues/007-test.md",
      argv: ["fixed-native-argv"],
    }));

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    const tools = openDeveloperTools();
    expect(within(tools).getByRole("button", { name: "Run raw execution-check" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Open file tasks/issues/007-test.md" }));
    await screen.findByDisplayValue(/# 007: Test/);
    fireEvent.click(within(tools).getByRole("button", { name: "Run raw execution-check" }));

    await waitFor(() => expect(bridge.executeHelper).toHaveBeenCalledWith("/tmp/setup-project", {
      helperId: "execution-check",
      mode: "task-contract",
      taskPath: "tasks/issues/007-test.md",
    }));
    expect(await screen.findByRole("region", { name: "Helper result" })).toBeInTheDocument();
  });

  it("renders unsupported helper platforms as adapter failures without process or cancellation claims", async () => {
    const nativeProject: ProjectSnapshot = {
      ...openedSetupProject,
      skills: [{ ...validSkill("build-right-preflight"), phase: "Discover", helpers: ["preflight-check"] }],
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(nativeProject);
    vi.mocked(bridge.executeHelper).mockResolvedValue(helperResult(nativeProject, {
      argv: [],
      outcome: "unsupportedPlatform",
      executed: false,
      success: false,
      exitStatus: null,
      stdout: "",
      decision: null,
      failure: "Deterministic helper timeout and cancellation are supported only on Unix platforms; no process was started",
    }));

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));

    expect(await screen.findByText(/Typed helper result · native adapter/)).toBeInTheDocument();
    expect(screen.queryByText("Helper start")).not.toBeInTheDocument();
    expect(screen.queryByText("Helper cancellation")).not.toBeInTheDocument();
    expect(screen.getAllByText(/no process was started/i)).toHaveLength(3);
  });

  it("previews exact skill setup without executing and cancellation remains effect-free", async () => {
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(openedSetupProject);
    vi.mocked(bridge.previewSkillSetup).mockResolvedValue(setupPreview);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Inspect skill setup adapter" }));

    expect(await screen.findByRole("region", { name: "Skill setup preview" })).toBeInTheDocument();
    expect(screen.getByText(/bun x skills@1.5.19 add pax-k\/build-right/)).toBeInTheDocument();
    expect(screen.getByText(/resolvedOnExecution/)).toBeInTheDocument();
    expect(bridge.executeSkillSetup).not.toHaveBeenCalled();
    const previewEvent = screen.getByText("Skill setup previewed").closest("article");
    expect(previewEvent && within(previewEvent).getByText("manual action")).toBeInTheDocument();
    expect(screen.getByText("MIXED")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(bridge.executeSkillSetup).not.toHaveBeenCalled();
    expect(await screen.findByText(/cancelled.*No command or mutation was executed/i)).toBeInTheDocument();
  });

  it("keeps skill setup and readiness preflight on the dominant founder path", async () => {
    const authorityProject: ProjectSnapshot = {
      ...openedSetupProject,
      files: authorityFiles,
    };
    const installedProject: ProjectSnapshot = {
      ...authorityProject,
      dirty: true,
      skills: [
        { ...validSkill("build-right-preflight"), phase: "Discover", helpers: ["preflight-check"] },
      ],
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(authorityProject);
    vi.mocked(bridge.previewSkillSetup).mockResolvedValue(setupPreview);
    vi.mocked(bridge.executeSkillSetup).mockResolvedValue(setupResult(installedProject));
    vi.mocked(bridge.executeHelper).mockResolvedValue(helperResult(installedProject));

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    const setupStatus = await screen.findByRole("region", { name: "Goal-centered project status" });
    fireEvent.click(within(setupStatus).getByRole("button", { name: "Complete project setup" }));

    expect(await screen.findByRole("region", { name: "Skill setup preview" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Confirm install" }));
    const setupReceipt = await screen.findByRole("region", { name: "Skill setup result" });
    fireEvent.click(within(setupReceipt).getByRole("button", { name: "Close result" }));

    const preflightStatus = await screen.findByRole("region", { name: "Goal-centered project status" });
    expect(within(preflightStatus).getByText("Readiness inspection")).toBeInTheDocument();
    fireEvent.click(within(preflightStatus).getByRole("button", { name: "Run readiness preflight" }));

    await waitFor(() => expect(bridge.executeHelper).toHaveBeenCalledWith(
      installedProject.root,
      { helperId: "preflight-check" },
    ));
    expect(await screen.findByRole("region", { name: "Helper result" })).toBeInTheDocument();
  });

  it("executes only after confirmation and consumes the refreshed project result", async () => {
    const refreshed = { ...openedSetupProject, dirty: true };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(openedSetupProject);
    vi.mocked(bridge.previewSkillSetup).mockResolvedValue(setupPreview);
    vi.mocked(bridge.executeSkillSetup).mockResolvedValue({
      operation: "install",
      outcome: "completed",
      executed: true,
      success: true,
      exitStatus: 0,
      stdout: "setup complete",
      stderr: "",
      stdoutTruncated: false,
      stderrTruncated: false,
      changedPaths: ["skills-lock.json", ".agents/skills/build-right-preflight/SKILL.md"],
      before: [{ skillId: "build-right-preflight", installedPath: ".agents/skills/build-right-preflight/SKILL.md", installed: false, lockHash: null }],
      after: [{ skillId: "build-right-preflight", installedPath: ".agents/skills/build-right-preflight/SKILL.md", installed: true, lockHash: "post-hash" }],
      repair: null,
      project: refreshed,
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Inspect skill setup adapter" }));
    fireEvent.click(await screen.findByRole("button", { name: "Confirm install" }));

    await waitFor(() => expect(bridge.executeSkillSetup).toHaveBeenCalledWith("/tmp/setup-project", "install", true, "sha256:preview-baseline"));
    expect(await screen.findByRole("region", { name: "Skill setup result" })).toBeInTheDocument();
    expect(screen.getByText(/post-hash/)).toBeInTheDocument();
    expect(screen.getByText("dirty")).toBeInTheDocument();
  });

  it("labels a stale-preview rejection as adapter evidence without claiming a local effect", async () => {
    const refreshed = { ...openedSetupProject, dirty: true };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(openedSetupProject);
    vi.mocked(bridge.previewSkillSetup).mockResolvedValue(setupPreview);
    vi.mocked(bridge.executeSkillSetup).mockResolvedValue({
      ...setupResult(refreshed, "stalePreview"),
      executed: false,
      changedPaths: [],
      repair: {
        code: "stale_skill_setup_preview",
        message: "Repository skill provenance changed after preview",
        nextAction: "Refresh the setup preview",
      },
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Inspect skill setup adapter" }));
    fireEvent.click(await screen.findByRole("button", { name: "Confirm install" }));

    expect(await screen.findByText("Skill setup not executed")).toBeInTheDocument();
    expect(bridge.executeSkillSetup).toHaveBeenCalledWith("/tmp/setup-project", "install", true, "sha256:preview-baseline");
    expect(screen.queryByText("Skill setup process executed")).not.toBeInTheDocument();
    const resultEvent = screen.getByText("Skill setup not executed").closest("article");
    expect(resultEvent && within(resultEvent).getByText("real adapter evidence")).toBeInTheDocument();
    expect(resultEvent && within(resultEvent).queryByText("real local effect")).not.toBeInTheDocument();
    expect(screen.getByText(/stale_skill_setup_preview/)).toBeInTheDocument();
  });

  it("projects sanitized native collaboration activity into the run inspector as real adapter evidence", async () => {
    const opaqueAlias = "opaque-app-integration-018";
    const handoff = `https://sync.example.test/workspaces/workspace-safe-018?edit=${opaqueAlias}`;
    const collaborationReadyProject = {
      ...openedSetupProject,
      files: bootstrapArtifactPaths.map((path) => ({
        path,
        name: path.split("/").at(-1) ?? path,
        kind: path.includes("/issues/") ? "task" as const : "document" as const,
        status: path === "tasks/issues/001-establish-execution-baseline.md" ? "ready" : undefined,
      })),
      skills: readyExecutionSkills(),
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(collaborationReadyProject);
    vi.mocked(bridge.executeHelper).mockResolvedValue(helperResult(collaborationReadyProject));
    vi.mocked(bridge.previewBoundedTask).mockResolvedValue({
      ...boundedPreview,
      selectedTask: "tasks/issues/001-establish-execution-baseline.md",
    });
    vi.mocked(bridge.connectMdsyncSession).mockResolvedValue({
      sessionId: createLocalSessionHandle(`local-session-${"5".repeat(32)}`),
      workspaceId: "workspace-safe-018",
      webOrigin: "https://sync.example.test",
      apiOrigin: "https://sync-api.example.test",
      access: "collaborator",
      actor: "build-right-studio",
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    expect(screen.getByRole("button", { name: /Collaboration.*Available after ready preflight/i })).toBeDisabled();
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" }));
    expect(await screen.findByRole("button", { name: /Collaboration.*Available after task selection/i })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Review resolver-selected task" }));
    await screen.findByText(/Resolver selected exactly tasks\/issues\/001-establish-execution-baseline.md/);
    fireEvent.click(screen.getByRole("button", { name: /Collaboration.*Local solo/i }));
    const panel = screen.getByRole("dialog", { name: "Collaboration authority" });
    fireEvent.change(within(panel).getByLabelText("Workspace handoff"), {
      target: { value: handoff },
    });
    fireEvent.click(within(panel).getByRole("button", { name: "Connect in native memory" }));

    await within(panel).findByRole("heading", { name: "Collaborator" });
    const event = await screen.findByText("Native collaboration session");
    const article = event.closest("article");
    expect(article && within(article).getByText("real adapter evidence")).toBeInTheDocument();
    expect(article?.textContent).not.toContain(opaqueAlias);
    expect(document.documentElement.outerHTML).not.toContain(opaqueAlias);
    expect(screen.queryByText("SIMULATED")).not.toBeInTheDocument();
  });

  it("keeps running cancellation available, rejects duplicate confirm, and blocks competing repository controls", async () => {
    const runningProject = {
      ...openedSetupProject,
      files: [{ path: "docs/plan.md", name: "Plan", kind: "document" as const }],
    };
    const execution = deferred<SkillSetupResult>();
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(runningProject);
    vi.mocked(bridge.previewSkillSetup).mockResolvedValue(setupPreview);
    vi.mocked(bridge.executeSkillSetup).mockReturnValue(execution.promise);

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("setup-project");
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Inspect skill setup adapter" }));
    const confirm = await screen.findByRole("button", { name: "Confirm install" });
    fireEvent.click(confirm);
    fireEvent.click(confirm);

    await waitFor(() => expect(bridge.executeSkillSetup).toHaveBeenCalledTimes(1));
    expect(screen.getByText("Skill setup invocation requested")).toBeInTheDocument();
    expect(screen.queryByText("Skill setup process started")).not.toBeInTheDocument();
    expect(screen.queryByText("Skill setup process executed")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel running setup" })).toBeEnabled();
    expect(screen.getByRole("button", { name: /Project setup-project/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Refresh repository" })).toBeDisabled();
    expect(within(openDeveloperTools()).getByRole("button", { name: "Run raw preflight helper" })).toBeDisabled();
    expect(within(openDeveloperTools()).getByRole("button", { name: "Inspect skill setup adapter" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Open file docs/plan.md" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Cancel running setup" }));
    await waitFor(() => expect(bridge.cancelSkillSetup).toHaveBeenCalledWith("/tmp/setup-project"));

    execution.resolve(setupResult({ ...runningProject, dirty: true }, "cancelled"));
    expect(await screen.findByText("Skill setup cancelled")).toBeInTheDocument();
    const requestedEvent = screen.getByText("Skill setup invocation requested").closest("article");
    const executedEvent = screen.getByText("Skill setup process executed").closest("article");
    expect(requestedEvent && within(requestedEvent).getByText("manual action")).toBeInTheDocument();
    expect(executedEvent && within(executedEvent).getByText("real local effect")).toBeInTheDocument();
    expect(screen.getByText("MIXED")).toBeInTheDocument();
  });

  it("rejects stale project and setup-preview results by repository generation", async () => {
    const first = deferred<ProjectSnapshot | null>();
    const second = deferred<ProjectSnapshot | null>();
    const stalePreview = deferred<typeof setupPreview>();
    const firstProject = { ...openedSetupProject, root: "/tmp/first", name: "first" };
    const secondProject = { ...openedSetupProject, root: "/tmp/second", name: "second" };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject)
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise)
      .mockResolvedValueOnce(secondProject);

    render(<App />);
    const initialSwitcher = screen.getByRole("button", { name: /Project pax-workbench/ });
    fireEvent.click(initialSwitcher);
    fireEvent.click(initialSwitcher);
    second.resolve(secondProject);
    await screen.findByText("second");
    first.resolve(firstProject);
    await waitFor(() => expect(screen.queryByText("first")).not.toBeInTheDocument());

    vi.mocked(bridge.previewSkillSetup).mockReturnValue(stalePreview.promise);
    fireEvent.click(within(openDeveloperTools()).getByRole("button", { name: "Inspect skill setup adapter" }));
    fireEvent.click(screen.getByRole("button", { name: /Project second/ }));
    await waitFor(() => expect(bridge.chooseProject).toHaveBeenCalledTimes(3));
    stalePreview.resolve({ ...setupPreview, targetProject: "/tmp/second" });
    await waitFor(() => expect(screen.queryByRole("region", { name: "Skill setup preview" })).not.toBeInTheDocument());
  });

  it("projects task Markdown into a structured read-only view", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("tab", { name: "structured" }));
    expect(screen.getByText("Projection only")).toBeInTheDocument();
    expect(screen.getByText(/Create a runnable workbench/)).toBeInTheDocument();
    expect(screen.getByText("Workbench interface passes its validation ladder.")).toBeInTheDocument();
  });

  it("requires explicit file selection and refreshes repository truth after save", async () => {
    const openedProject = {
      root: "/tmp/workbench-project",
      name: "workbench-project",
      branch: "main",
      dirty: false,
      files: [...authorityFiles, { path: "tasks/issues/005.md", name: "Task five", kind: "task" as const, status: "ready" }],
      skills: [],
      errors: [],
    };
    const savedProject = {
      ...openedProject,
      dirty: true,
      files: openedProject.files.map((file) =>
        file.path === "tasks/issues/005.md" ? { ...file, status: "active" } : file
      ),
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(openedProject);
    vi.mocked(bridge.readProjectFile).mockResolvedValue({
      path: "tasks/issues/005.md",
      content: "# 005: Task five\n\nStatus: ready\nOwner: AI\n",
      version: "sha256:v1",
    });
    vi.mocked(bridge.writeProjectFile).mockResolvedValue({
      file: {
        path: "tasks/issues/005.md",
        content: "# 005: Task five\n\nStatus: active\nOwner: AI\n",
        version: "sha256:v2",
      },
      project: savedProject,
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText(/Select a Markdown file to edit/);
    expect(bridge.readProjectFile).not.toHaveBeenCalledWith(
      "/tmp/workbench-project",
      "tasks/issues/005.md",
    );

    fireEvent.click(screen.getByRole("button", { name: "Open file tasks/issues/005.md" }));
    const editor = await screen.findByRole("textbox", { name: "Markdown source" });
    fireEvent.change(editor, { target: { value: "# 005: Task five\n\nStatus: active\nOwner: AI\n" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(bridge.writeProjectFile).toHaveBeenCalledWith(
      "/tmp/workbench-project",
      "tasks/issues/005.md",
      "# 005: Task five\n\nStatus: active\nOwner: AI\n",
      "sha256:v1",
    ));
    expect(await screen.findByText(/file and repository state refreshed/)).toBeInTheDocument();
    expect(screen.getByText("dirty")).toBeInTheDocument();
  });

  it("preserves a stale draft until the user explicitly reloads the disk version", async () => {
    const project = {
      root: "/tmp/workbench-project",
      name: "workbench-project",
      branch: "main",
      dirty: true,
      files: [{ path: "docs/plan.md", name: "Plan", kind: "document" as const }],
      skills: [],
      errors: [],
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(project);
    vi.mocked(bridge.refreshProject).mockResolvedValue(project);
    vi.mocked(bridge.readProjectFile)
      .mockResolvedValueOnce({ path: "docs/plan.md", content: "# Original\n", version: "sha256:v1" })
      .mockResolvedValueOnce({ path: "docs/plan.md", content: "# Changed on disk\n", version: "sha256:v2" });
    vi.mocked(bridge.writeProjectFile).mockRejectedValue({
      code: "stale_version",
      message: "changed elsewhere",
      path: "docs/plan.md",
      committed: false,
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText(/Select a Markdown file to edit/);
    fireEvent.click(screen.getByRole("button", { name: "Open file docs/plan.md" }));
    const editor = await screen.findByRole("textbox", { name: "Markdown source" });
    fireEvent.change(editor, { target: { value: "# My draft\n" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(await screen.findByRole("button", { name: "Reload disk version" })).toBeInTheDocument();
    expect(editor).toHaveValue("# My draft\n");
    fireEvent.click(screen.getByRole("button", { name: "Reload disk version" }));
    await waitFor(() => expect(editor).toHaveValue("# Changed on disk\n"));
  });

  it("guards file navigation while the selected Markdown has unsaved edits", async () => {
    const project = {
      root: "/tmp/workbench-project",
      name: "workbench-project",
      branch: "main",
      dirty: false,
      files: [
        { path: "docs/one.md", name: "One", kind: "document" as const },
        { path: "docs/two.md", name: "Two", kind: "document" as const },
      ],
      skills: [],
      errors: [],
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(project);
    vi.mocked(bridge.readProjectFile)
      .mockResolvedValueOnce({ path: "docs/one.md", content: "# One\n", version: "sha256:one" })
      .mockResolvedValueOnce({ path: "docs/two.md", content: "# Two\n", version: "sha256:two" });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText(/Select a Markdown file to edit/);
    fireEvent.click(screen.getByRole("button", { name: "Open file docs/one.md" }));
    const editor = await screen.findByRole("textbox", { name: "Markdown source" });
    fireEvent.change(editor, { target: { value: "# Unsaved\n" } });
    fireEvent.click(screen.getByRole("button", { name: "Open file docs/two.md" }));

    expect(await screen.findByRole("button", { name: "Discard and open" })).toBeInTheDocument();
    expect(editor).toHaveValue("# Unsaved\n");
    expect(bridge.readProjectFile).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Discard and open" }));
    await waitFor(() => expect(editor).toHaveValue("# Two\n"));
  });

  it("guards project switching until the user keeps or explicitly discards the draft", async () => {
    const firstProject = {
      root: "/tmp/first-project",
      name: "first-project",
      branch: "main",
      dirty: false,
      files: [{ path: "docs/plan.md", name: "Plan", kind: "document" as const }],
      skills: [],
      errors: [],
    };
    const secondProject = {
      ...firstProject,
      root: "/tmp/second-project",
      name: "second-project",
      files: [],
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject)
      .mockResolvedValueOnce(firstProject)
      .mockResolvedValueOnce(secondProject);
    vi.mocked(bridge.readProjectFile).mockResolvedValue({
      path: "docs/plan.md",
      content: "# Original\n",
      version: "sha256:original",
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText(/Select a Markdown file to edit/);
    fireEvent.click(screen.getByRole("button", { name: "Open file docs/plan.md" }));
    const editor = await screen.findByRole("textbox", { name: "Markdown source" });
    fireEvent.change(editor, { target: { value: "# Unsaved project draft\n" } });

    fireEvent.click(screen.getByRole("button", { name: /Project first-project/ }));
    expect(await screen.findByRole("button", { name: "Discard and switch project" })).toBeInTheDocument();
    expect(bridge.chooseProject).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Keep draft" }));
    expect(editor).toHaveValue("# Unsaved project draft\n");
    expect(bridge.chooseProject).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: /Project first-project/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Discard and switch project" }));
    await screen.findByText("second-project");
    expect(bridge.chooseProject).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("heading", { name: "Draft the terrain before the agent moves." })).toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "Markdown source" })).not.toBeInTheDocument();
    expect(editor).not.toBeInTheDocument();
  });

  it("keeps one primary canvas while pane controls and breadcrumbs remain explicit", async () => {
    const project = {
      root: "/tmp/navigation-project",
      name: "navigation-project",
      branch: "main",
      dirty: false,
      files: [
        ...authorityFiles,
        { path: "docs/one.md", name: "One", kind: "document" as const },
        { path: "tasks/issues/029-layout.md", name: "029 layout", kind: "task" as const, status: "active" },
      ],
      skills: [],
      errors: [],
    };
    vi.mocked(bridge.isTauriRuntime).mockReturnValue(true);
    vi.mocked(bridge.chooseProject).mockResolvedValue(project);
    vi.mocked(bridge.readProjectFile).mockImplementation(async (_root, path) => ({
      path,
      content: "# Tracker\n",
      version: `sha256:${path}`,
    }));

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Project pax-workbench/ }));
    await screen.findByText("navigation-project");
    const shell = await screen.findByRole("main");
    expect(shell).toHaveAttribute("data-navigation-open", "true");
    expect(shell).toHaveAttribute("data-inspector-open", "false");

    const breadcrumbs = screen.getByRole("navigation", { name: "Document breadcrumbs" });
    expect(within(breadcrumbs).getByText("Project")).toBeInTheDocument();
    expect(within(breadcrumbs).getByText("No project Markdown selected")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Collapse project navigation" }));
    expect(shell).toHaveAttribute("data-navigation-open", "false");
    fireEvent.click(screen.getByRole("button", { name: "Expand evidence inspector" }));
    expect(shell).toHaveAttribute("data-inspector-open", "true");
  });
});
