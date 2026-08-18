import { useEffect, useMemo, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import {
  Activity,
  Braces,
  Check,
  ChevronDown,
  CircleDot,
  Code2,
  FileText,
  FolderGit2,
  GitBranch,
  History,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
  RefreshCw,
  Save,
  Search,
  ShieldCheck,
  Sparkles,
  SquareTerminal,
} from "lucide-react";
import { CollaborationPanel } from "./components/CollaborationPanel";
import { DiscoverBootstrap } from "./components/DiscoverBootstrap";
import { FeaturePlanning } from "./components/FeaturePlanning";
import { FounderGateResolution } from "./components/FounderGateResolution";
import { LocalGitHandoff } from "./components/LocalGitHandoff";
import { ProjectFileNavigation } from "./components/ProjectFileNavigation";
import { ReviewReceipt } from "./components/ReviewReceipt";
import { cancelBoundedTask, cancelHelper, cancelRuntime, cancelSkillSetup, clearGoalState, describeProjectError, executeBoundedTask, executeHelper, executeRuntime, executeSkillSetup, inspectPostRunReview, isTauriRuntime, previewBoundedTask, previewSkillSetup, projectErrorCode, projectErrorCommitted } from "./lib/bridge";
import { demoProject, demoTaskMarkdown, initialEvents, simulatedEvents } from "./lib/demo";
import { deriveBootstrapInventory } from "./lib/discover-bootstrap";
import { parseTask } from "./lib/markdown";
import { projectSessionEffects } from "./lib/project-effects";
import {
  deriveProjectSessionProjection,
  isExecutionHelperTaskPath,
  repositoryNeedsSetup,
  skillSetupOperationFor,
} from "./lib/project-session";
import {
  deriveProductWorkflowProjection,
  selectRepairGuidance,
  type ProductCollaborationInput,
} from "./lib/product-workflow";
import { deriveProductActionPresentation } from "./lib/action-hierarchy";
import { deriveGoalShellProjection } from "./lib/goal-shell";
import { deriveReviewReceipt } from "./lib/review-receipt";
import {
  readRecentProjects,
  rememberRecentProject,
  writeRecentProjects,
  type RecentProjectPreference,
} from "./lib/recent-projects";
import type { BoundedTaskPreview, BoundedTaskResult, GoalRecovery, HelperId, HelperInvocation, HelperResult, PostRunReviewEvidence, ProjectSnapshot, RunEvent, RuntimeMode, RuntimeResult, RuntimeStreamMessage, SharedBoundedTaskResult, SkillSetupPreview, SkillSetupResult, SkillSummary, WorkflowCheckpoint } from "./types";

export { isExecutionHelperTaskPath, skillSetupOperationFor } from "./lib/project-session";

type WorkbenchView = "edit" | "preview" | "structured";

const phaseIcons = {
  Discover: Search,
  Plan: Braces,
  Build: SquareTerminal,
  Principles: ShieldCheck,
  Unknown: FileText,
};

const missingSkill: SkillSummary = {
  id: "not-installed",
  name: "Skills not installed",
  phase: "Unknown",
  purpose: "This repository has no project-scoped Build Right skills yet.",
  reads: [],
  writes: [],
  decisions: ["install-skills"],
  helpers: [],
  requiredEvidence: [],
  stopStates: ["install-skills"],
  renderer: "generic-markdown",
  executable: false,
  source: "not installed",
  installedPath: ".agents/skills",
};

function isHelperId(value: string): value is HelperId {
  return value === "preflight-check" || value === "feature-planning-check" || value === "continue-check" || value === "execution-check";
}

function SkillCard({ skill, active, onSelect }: { skill: SkillSummary; active: boolean; onSelect: () => void }) {
  const Icon = phaseIcons[skill.phase];
  return (
    <button className={`skill-card ${active ? "is-active" : ""}`} onClick={onSelect}>
      <span className="skill-icon"><Icon size={14} /></span>
      <span>
        <strong>{skill.phase}</strong>
        <small>{skill.name}</small>
      </span>
      <span className="skill-source">{skill.renderer === "generic-markdown" ? "viewer only" : skill.source.split("/").at(-1)}</span>
    </button>
  );
}

function RunInspector({ events, running, operationRunning, taskLabel, requirementBasis, onCollapse }: { events: RunEvent[]; running: boolean; operationRunning: boolean; taskLabel: string; requirementBasis: string; onCollapse: () => void }) {
  const provenance = new Set(events.map((event) => event.provenance ?? (event.simulated ? "simulated" : "real")));
  const inspectorLabel = provenance.size > 1
    ? "MIXED"
    : provenance.has("real")
      ? "REAL LOCAL"
      : provenance.has("manual")
        ? "MANUAL"
        : provenance.has("adapter")
          ? "ADAPTER"
        : "SIMULATED";
  return (
    <aside className="run-inspector" aria-label="Agent run inspector">
      <div className="pane-heading">
        <div>
          <span className="eyebrow">Run {taskLabel} / local activity</span>
          <h2>Agent run</h2>
        </div>
        <button className="icon-button" aria-label="Collapse run inspector" onClick={onCollapse}><PanelRightClose size={16} /></button>
      </div>

      <div className="run-state">
        <span className={`pulse ${running || operationRunning ? "is-running" : ""}`} />
        <div><strong>{operationRunning ? "Running bounded local operation" : running ? `Running ${taskLabel}` : "Ready to continue"}</strong><small>Bounded by {requirementBasis}</small></div>
        <span className="simulation-chip">{inspectorLabel}</span>
      </div>

      <div className="event-list">
        {events.map((event, index) => (
          <article className={`event event-${event.kind}`} key={event.id}>
            <div className="event-rail"><span>{index + 1}</span></div>
            <div className="event-body">
              <time>{event.time}</time>
              <strong>{event.label}</strong>
              <p>{event.detail}</p>
              <small>{event.provenance === "real" ? "real local effect" : event.provenance === "adapter" ? "real adapter evidence" : event.provenance === "manual" ? "manual action" : "simulated"}</small>
            </div>
          </article>
        ))}
      </div>

      <div className="run-actions">
        <span className="inspector-readonly"><History size={15} /> Product outcomes and verification evidence appear here.</span>
      </div>
    </aside>
  );
}

function ExecutionRibbon({ items, proof, activeStep, onSelect }: { items: WorkflowCheckpoint[]; proof: string; activeStep: string; onSelect: (id: string) => void }) {
  return (
    <footer className="execution-ribbon" aria-label="Execution workflow">
      <div className="ribbon-title"><Activity size={15} /><span>Execution line</span></div>
      <div className="checkpoint-track">
        {items.map((checkpoint, index) => (
          <button
            key={checkpoint.id}
            className={`checkpoint is-${checkpoint.state} ${activeStep === checkpoint.id ? "is-selected" : ""}`}
            onClick={() => onSelect(checkpoint.id)}
          >
            <span className="checkpoint-node">{checkpoint.state === "done" ? <Check size={12} /> : index + 1}</span>
            <span className="checkpoint-copy"><strong>{checkpoint.label}</strong><small>{checkpoint.detail}</small></span>
          </button>
        ))}
      </div>
      <div className="ribbon-proof"><ShieldCheck size={15} /><span>{proof}</span></div>
    </footer>
  );
}

export default function App() {
  const [project, setProject] = useState<ProjectSnapshot>(demoProject);
  const [isDemo, setIsDemo] = useState(true);
  const [markdown, setMarkdown] = useState(demoTaskMarkdown);
  const [activeFilePath, setActiveFilePath] = useState("tasks/issues/001-build-local-workbench-mvp.md");
  const [navigationHistory, setNavigationHistory] = useState({
    paths: ["tasks/issues/001-build-local-workbench-mvp.md"],
    index: 0,
  });
  const [projectNavigationOpen, setProjectNavigationOpen] = useState(true);
  const [evidenceInspectorOpen, setEvidenceInspectorOpen] = useState(
    () => typeof window === "undefined" || window.innerWidth > 1100,
  );
  const [contentVersion, setContentVersion] = useState<string | null>(null);
  const [loadedMarkdown, setLoadedMarkdown] = useState(demoTaskMarkdown);
  const [staleConflict, setStaleConflict] = useState(false);
  const [pendingNavigationPath, setPendingNavigationPath] = useState<string | null>(null);
  const [pendingProjectSwitch, setPendingProjectSwitch] = useState(false);
  const [view, setView] = useState<WorkbenchView>("edit");
  const [selectedSkill, setSelectedSkill] = useState("build-right-execution");
  const [events, setEvents] = useState<RunEvent[]>(initialEvents);
  const [running, setRunning] = useState(false);
  const [activeStep, setActiveStep] = useState("task");
  const [notice, setNotice] = useState("Demo projection · repository writes disabled in browser");
  const [setupPreview, setSetupPreview] = useState<SkillSetupPreview | null>(null);
  const [setupResult, setSetupResult] = useState<SkillSetupResult | null>(null);
  const [setupRunning, setSetupRunning] = useState(false);
  const [helperResult, setHelperResult] = useState<HelperResult | null>(null);
  const [helperRunning, setHelperRunning] = useState(false);
  const [runtimePrompt, setRuntimePrompt] = useState("Inspect the selected repository and report one bounded next action. Do not modify files or advance repository state.");
  const [runtimeResult, setRuntimeResult] = useState<RuntimeResult | null>(null);
  const [runtimeRunning, setRuntimeRunning] = useState(false);
  const [runtimeRunId, setRuntimeRunId] = useState<string | null>(null);
  const [boundedPreview, setBoundedPreview] = useState<BoundedTaskPreview | null>(null);
  const [boundedResult, setBoundedResult] = useState<BoundedTaskResult | null>(null);
  const [reviewEvidence, setReviewEvidence] = useState<PostRunReviewEvidence | null>(null);
  const [reviewEvidenceFailure, setReviewEvidenceFailure] = useState<string | null>(null);
  const [reviewDecision, setReviewDecision] = useState<"handoff" | "revision" | null>(null);
  const [sharedResult, setSharedResult] = useState<SharedBoundedTaskResult | null>(null);
  const [controllerRunning, setControllerRunning] = useState(false);
  const [collaborationRunning, setCollaborationRunning] = useState(false);
  const [bootstrapRunning, setBootstrapRunning] = useState(false);
  const [planningRunning, setPlanningRunning] = useState(false);
  const [collaborationProjection, setCollaborationProjection] =
    useState<ProductCollaborationInput>({
      mode: "localOnly",
      session: null,
      reconciliation: "localOnly",
    });
  const [goalRecovery, setGoalRecovery] = useState<GoalRecovery | null>(null);
  const [recentProjects, setRecentProjects] = useState<RecentProjectPreference[]>(() =>
    readRecentProjects(typeof window === "undefined" ? null : window.localStorage)
  );
  const setupRootRef = useRef<string | null>(null);
  const helperRootRef = useRef<string | null>(null);
  const runtimeRootRef = useRef<string | null>(null);
  const runtimeRunIdRef = useRef<string | null>(null);
  const repositoryGenerationRef = useRef(0);
  const setupGenerationRef = useRef<number | null>(null);
  const helperGenerationRef = useRef<number | null>(null);
  const runtimeGenerationRef = useRef<number | null>(null);
  const operationRunning = setupRunning
    || bootstrapRunning
    || planningRunning
    || helperRunning
    || runtimeRunning
    || controllerRunning
    || collaborationRunning;
  const task = useMemo(() => parseTask(markdown), [markdown]);
  const workflowSkills = project.skills.filter((skill) => skill.phase !== "Principles");
  const principlesSkill = project.skills.find((skill) => skill.phase === "Principles") ?? null;
  const activeSkill = workflowSkills.find((skill) => skill.id === selectedSkill) ?? workflowSkills[0] ?? missingSkill;
  const bootstrapInventory = useMemo(() => deriveBootstrapInventory(project), [project]);
  const bootstrapVisible = !isDemo
    && (!goalRecovery || goalRecovery.state === "missing")
    && (
      bootstrapRunning
      || (!bootstrapInventory.complete
        && (project.files.length === 0 || activeSkill.phase === "Discover"))
    );
  const planningVisible = !bootstrapVisible
    && !isDemo
    && activeSkill.id === "build-right-feature-planning";
  const preflightAvailable = project.skills.some(
    (skill) =>
      skill.id === "build-right-preflight"
      && skill.renderer === "operating-card"
      && skill.helpers.includes("preflight-check"),
  );
  const currentPreflightResult =
    helperResult?.helperId === "preflight-check"
    && helperResult.project.root === project.root
      ? helperResult
      : null;
  const preflightReadyForExecution =
    currentPreflightResult?.success === true
    && currentPreflightResult.decision?.decision === "ready-for-execution";
  const founderGateVisible =
    currentPreflightResult?.success === true
    && currentPreflightResult.decision?.decision === "ask-founder";
  const selectedExecutionEnvelopeReady =
    boundedPreview?.executable === true
    && Boolean(boundedPreview.selectedTask)
    && boundedPreview.blockingGates.length === 0;
  const collaborationEligible =
    !isDemo
    && bootstrapInventory.complete
    && preflightReadyForExecution
    && selectedExecutionEnvelopeReady;
  const collaborationDisabledReason = bootstrapVisible
    ? "Available after local setup"
    : !preflightReadyForExecution
      ? "Available after ready preflight"
      : !selectedExecutionEnvelopeReady
        ? "Available after task selection"
        : undefined;
  const projectSession = useMemo(
    () =>
      deriveProjectSessionProjection({
        isDemo,
        activeFilePath,
        markdown,
        loadedMarkdown,
        staleConflict,
        pendingNavigationPath,
        pendingProjectSwitch,
        operationRunning,
      }),
    [
      activeFilePath,
      isDemo,
      loadedMarkdown,
      markdown,
      operationRunning,
      pendingNavigationPath,
      pendingProjectSwitch,
      staleConflict,
    ],
  );
  const resolverSelectedTask = boundedPreview?.selectedTask
    ?? boundedResult?.loopState.nextTask
    ?? boundedResult?.selectedTask
    ?? goalRecovery?.checkpointTask
    ?? null;
  const resolverSelectedTaskReady = Boolean(
    resolverSelectedTask
    && project.files.some(
      (file) =>
        file.path === resolverSelectedTask
        && ["ready", "active", "in_progress"].includes((file.status ?? "").toLowerCase()),
    ),
  );
  const repositoryHasReadyTask = project.files.some(
    (file) =>
      file.kind === "task"
      && ["ready", "active", "in_progress"].includes((file.status ?? "").toLowerCase()),
  );
  const workflowGoalLoop = boundedResult?.loopState
    ?? (boundedPreview?.executable
      ? boundedPreview.loopState
      : boundedPreview
        ? { ...boundedPreview.loopState, state: "externalStop" as const }
        : null);
  const productWorkflow = useMemo(
    () =>
      deriveProductWorkflowProjection({
        projectSelected: !isDemo,
        projectNeedsSetup: !isDemo && (repositoryNeedsSetup(project) || !preflightAvailable),
        preflightRequired:
          !isDemo
          && bootstrapInventory.complete
          && preflightAvailable
          && !currentPreflightResult?.decision,
        founderInputRequired: currentPreflightResult?.decision?.decision === "ask-founder",
        planningReady: !isDemo && !repositoryHasReadyTask && !resolverSelectedTaskReady,
        selectedTaskReady: repositoryHasReadyTask || resolverSelectedTaskReady,
        operationRunning,
        resultNeedsReview: Boolean(boundedResult),
        goalLoop: workflowGoalLoop,
        recovery: goalRecovery,
        collaboration: collaborationProjection,
      }),
    [
      boundedPreview,
      boundedResult,
      bootstrapInventory.complete,
      collaborationProjection,
      currentPreflightResult,
      goalRecovery,
      isDemo,
      operationRunning,
      preflightAvailable,
      project,
      repositoryHasReadyTask,
      resolverSelectedTaskReady,
      workflowGoalLoop,
    ],
  );
  const goalShell = useMemo(
    () =>
      deriveGoalShellProjection({
        project,
        projectSelected: !isDemo,
        workflow: productWorkflow,
        recovery: goalRecovery,
        preview: boundedPreview,
        result: boundedResult,
      }),
    [boundedPreview, boundedResult, goalRecovery, isDemo, productWorkflow, project],
  );
  const productAction = useMemo(
    () => deriveProductActionPresentation(productWorkflow),
    [productWorkflow],
  );
  const runtimeRepair = useMemo(() => {
    const runtime = boundedResult?.runtime;
    if (!runtime || runtime.success || runtime.provenance.simulated) return null;
    return selectRepairGuidance({
      failureClass: runtime.outcome === "cancelled" ? "cancellation" : "runtime",
      code: runtime.outcome,
      message: runtime.failure ?? runtime.outcome,
      evidence: [{
        source: "runtime",
        code: runtime.outcome,
        summary: runtime.failure ?? "Bounded runtime returned a typed failure",
      }],
    });
  }, [boundedResult]);
  const reviewReceipt = useMemo(
    () => boundedResult
      ? deriveReviewReceipt({
          result: boundedResult,
          gitEvidence: reviewEvidence,
          gitEvidenceFailure: reviewEvidenceFailure,
          sharedResult,
          recovery: goalRecovery,
        })
      : null,
    [
      boundedResult,
      goalRecovery,
      reviewEvidence,
      reviewEvidenceFailure,
      sharedResult,
    ],
  );

  useEffect(() => {
    if (!boundedResult || isDemo || !isTauriRuntime()) {
      setReviewEvidence(null);
      setReviewEvidenceFailure(null);
      return;
    }
    const root = boundedResult.project.root;
    const generation = repositoryGenerationRef.current;
    setReviewEvidence(null);
    setReviewEvidenceFailure(null);
    setReviewDecision(null);
    void inspectPostRunReview(root)
      .then((evidence) => {
        if (generation === repositoryGenerationRef.current) setReviewEvidence(evidence);
      })
      .catch((error) => {
        if (generation === repositoryGenerationRef.current) {
          setReviewEvidenceFailure(
            `Current Git review evidence unavailable: ${describeProjectError(error)}`,
          );
        }
      });
  }, [boundedResult, isDemo]);

  useEffect(() => {
    if (isDemo) return;
    setRecentProjects((current) => {
      const next = rememberRecentProject(current, {
        root: project.root,
        lastOpenedAt: Date.now(),
        selectedSkill,
        view,
      });
      writeRecentProjects(window.localStorage, next);
      return next;
    });
  }, [isDemo, project.root, selectedSkill, view]);

  function applyInspectedProject(
    next: ProjectSnapshot,
    preference?: Pick<RecentProjectPreference, "selectedSkill" | "view">,
  ) {
    setProject(next);
    setIsDemo(false);
    setActiveFilePath("");
    setNavigationHistory({ paths: [], index: -1 });
    setContentVersion(null);
    setStaleConflict(false);
    setPendingNavigationPath(null);
    setPendingProjectSwitch(false);
    setSetupPreview(null);
    setSetupResult(null);
    setHelperResult(null);
    setRuntimeResult(null);
    setBoundedPreview(null);
    setBoundedResult(null);
    setReviewEvidence(null);
    setReviewEvidenceFailure(null);
    setReviewDecision(null);
    setSharedResult(null);
    setGoalRecovery(null);
    setCollaborationProjection({
      mode: "localOnly",
      session: null,
      reconciliation: "localOnly",
    });
    if (preference) {
      setSelectedSkill(preference.selectedSkill);
      setView(preference.view);
    }
    const selectionPrompt = "# Advanced repository inspection\n\nChoose an authority file only when you need to inspect or edit its raw Markdown.";
    setMarkdown(selectionPrompt);
    setLoadedMarkdown(selectionPrompt);
    setEvents([{
      id: "project-opened",
      time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
      label: "Project inspected",
      detail: `Inspected ${next.root}; goal actions await repository and recovery evidence.`,
      kind: "read",
      provenance: "real",
    }]);
  }

  async function openProject(discardCurrentDraft = false) {
    if (operationRunning) {
      setNotice("Project switch blocked while a bounded local or collaboration operation is running.");
      return;
    }
    if (!isTauriRuntime()) {
      setNotice("Native folder selection is available inside the Tauri app. Showing the repository-backed demo projection.");
      return;
    }
    if (!discardCurrentDraft && !isDemo && activeFilePath && projectSession.draft === "dirty") {
      setPendingProjectSwitch(true);
      setPendingNavigationPath(null);
      setNotice(`Project switch blocked: ${activeFilePath} has unsaved edits. Keep the draft or discard it explicitly.`);
      return;
    }
    const generation = ++repositoryGenerationRef.current;
    try {
      const next = await projectSessionEffects.choose();
      if (generation !== repositoryGenerationRef.current) return;
      if (next) {
        applyInspectedProject(next);
        const errorSuffix = next.errors.length ? ` Inspection issues: ${next.errors.map((error) => error.code).join(", ")}.` : "";
        setNotice(`Opened and inspected ${next.root}. Repository and goal evidence now drive the shell. Select a Markdown file to edit only for advanced inspection.${errorSuffix}`);
        try {
          const recovery = await projectSessionEffects.recover(next.root);
          if (generation === repositoryGenerationRef.current) setGoalRecovery(recovery);
        } catch (error) {
          if (generation === repositoryGenerationRef.current) {
            setNotice(`Opened ${next.root}, but durable goal recovery failed safely: ${describeProjectError(error)} No process started.`);
          }
        }
      }
    } catch (error) {
      if (generation !== repositoryGenerationRef.current) return;
      setNotice(`Project inspection failed: ${describeProjectError(error)}`);
    }
  }

  async function reopenRecentProject(preference: RecentProjectPreference) {
    if (operationRunning) {
      setNotice("Recent project reopen is blocked while an operation is running.");
      return;
    }
    if (!isTauriRuntime()) {
      setNotice("Recent project reinspection is available inside the native app.");
      return;
    }
    if (!isDemo && activeFilePath && projectSession.draft === "dirty") {
      setNotice(`Recent project reopen blocked: ${activeFilePath} has unsaved edits.`);
      return;
    }
    const generation = ++repositoryGenerationRef.current;
    try {
      const next = await projectSessionEffects.refresh(preference.root);
      if (generation !== repositoryGenerationRef.current) return;
      applyInspectedProject(next, preference);
      setNotice(`Reopened and re-inspected ${preference.root}. No helper or Codex process started.`);
      try {
        const recovery = await projectSessionEffects.recover(next.root);
        if (generation === repositoryGenerationRef.current) setGoalRecovery(recovery);
      } catch (error) {
        if (generation === repositoryGenerationRef.current) {
          setNotice(`Reopened ${next.root}, but durable goal recovery failed safely: ${describeProjectError(error)} No process started.`);
        }
      }
    } catch (error) {
      if (generation === repositoryGenerationRef.current) {
        setNotice(`Recent project reinspection failed: ${describeProjectError(error)}`);
      }
    }
  }

  function runShellPrimaryAction() {
    switch (productWorkflow.primaryAction) {
      case "openOrCreateProject":
        void openProject();
        return;
      case "resumeVerifiedGoal":
      case "reviewSelectedTask":
      case "reviewNextIteration":
        void prepareBoundedExecution();
        return;
      case "confirmOperation":
        void runBoundedTask("live");
        return;
      case "completeSetup":
        void inspectSkillSetup();
        return;
      case "runPreflight":
        void runHelper("preflight-check");
        return;
      case "answerFounderQuestions":
        setSelectedSkill("build-right-preflight");
        setNotice(productAction.consequence);
        return;
      case "previewPlanningChanges":
        setSelectedSkill("build-right-feature-planning");
        setNotice(productAction.consequence);
        return;
      default:
        setNotice(goalShell.guidance);
    }
  }

  async function loadAuthorityFile(path: string, recordHistory = true) {
    if (operationRunning) {
      setNotice("File navigation is blocked while skill setup is running.");
      return;
    }
    const root = project.root;
    const generation = ++repositoryGenerationRef.current;
    const file = await projectSessionEffects.readFile(root, path);
    if (generation !== repositoryGenerationRef.current) return;
    setActiveFilePath(path);
    if (recordHistory) {
      setNavigationHistory((current) => {
        const retained = current.paths.slice(0, current.index + 1);
        if (retained.at(-1) === path) return current;
        const paths = [...retained, path].slice(-30);
        return { paths, index: paths.length - 1 };
      });
    }
    setMarkdown(file.content);
    setLoadedMarkdown(file.content);
    setContentVersion(file.version);
    setStaleConflict(false);
    setPendingNavigationPath(null);
    setNotice(`Opened ${path}`);
  }

  async function openAuthorityFile(path: string) {
    if (operationRunning) {
      setNotice("File navigation is blocked while skill setup is running.");
      return;
    }
    if (isDemo || !isTauriRuntime()) {
      setNotice("Demo file navigation is illustrative. Open a native repository to load another file.");
      return;
    }
    if (path === activeFilePath) return;
    if (activeFilePath && projectSession.draft === "dirty") {
      setPendingNavigationPath(path);
      setNotice(`Navigation blocked: ${activeFilePath} has unsaved edits. Keep the draft or discard it explicitly.`);
      return;
    }
    try {
      await loadAuthorityFile(path);
    } catch (error) {
      setNotice(`File read failed: ${describeProjectError(error)}`);
    }
  }

  async function navigateDocumentHistory(direction: -1 | 1) {
    const nextIndex = navigationHistory.index + direction;
    const path = navigationHistory.paths[nextIndex];
    if (!path || operationRunning || projectSession.draft === "dirty") return;
    setNavigationHistory((current) => ({ ...current, index: nextIndex }));
    try {
      await loadAuthorityFile(path, false);
    } catch (error) {
      setNavigationHistory((current) => ({
        ...current,
        index: Math.max(0, Math.min(current.paths.length - 1, current.index - direction)),
      }));
      setNotice(`Document history failed: ${describeProjectError(error)}`);
    }
  }

  async function saveDocument() {
    if (operationRunning) {
      setNotice("Save is blocked while skill setup is running.");
      return;
    }
    if (!isTauriRuntime()) {
      setNotice("Draft retained in this preview. Native mode writes to the selected repository.");
      return;
    }
    if (!activeFilePath || !contentVersion) {
      setNotice("Save blocked: select an existing project Markdown file first.");
      return;
    }
    const root = project.root;
    const generation = ++repositoryGenerationRef.current;
    try {
      const result = await projectSessionEffects.writeFile(root, activeFilePath, markdown, contentVersion);
      if (generation !== repositoryGenerationRef.current) return;
      setProject(result.project);
      setMarkdown(result.file.content);
      setLoadedMarkdown(result.file.content);
      setContentVersion(result.file.version);
      setStaleConflict(false);
      setPendingNavigationPath(null);
      setNotice(`Saved ${activeFilePath}; file and repository state refreshed.`);
    } catch (error) {
      if (generation !== repositoryGenerationRef.current) return;
      const code = projectErrorCode(error);
      if (code === "stale_version" || code === "path_changed" || projectErrorCommitted(error)) {
        setStaleConflict(true);
        setNotice(`Save conflict: ${describeProjectError(error)} Draft preserved; reload only when you choose to discard it.`);
      } else {
        setNotice(`Save blocked: ${describeProjectError(error)}`);
      }
    }
  }

  async function discardAndContinue() {
    if (operationRunning) {
      setNotice("Repository navigation is blocked while skill setup is running.");
      return;
    }
    try {
      if (pendingProjectSwitch) {
        setPendingProjectSwitch(false);
        await openProject(true);
        return;
      }
      if (pendingNavigationPath) {
        await loadAuthorityFile(pendingNavigationPath);
        return;
      }
      if (activeFilePath) {
        const root = project.root;
        const path = activeFilePath;
        const generation = ++repositoryGenerationRef.current;
        const refreshedProject = await projectSessionEffects.refresh(root);
        const file = await projectSessionEffects.readFile(root, path);
        if (generation !== repositoryGenerationRef.current) return;
        setProject(refreshedProject);
        setMarkdown(file.content);
        setLoadedMarkdown(file.content);
        setContentVersion(file.version);
        setStaleConflict(false);
        setPendingNavigationPath(null);
        setNotice(`Opened ${path}`);
      }
    } catch (error) {
      setNotice(`Reload failed: ${describeProjectError(error)} Draft preserved.`);
    }
  }

  function keepDraft() {
    setPendingNavigationPath(null);
    setPendingProjectSwitch(false);
    setStaleConflict(false);
    setNotice(`Draft preserved for ${activeFilePath}.`);
  }

  async function refreshRepository() {
    if (operationRunning) {
      setNotice("Refresh is blocked while skill setup is running.");
      return;
    }
    if (isDemo || !isTauriRuntime()) {
      setNotice("Native refresh is available after opening a repository in the Tauri app.");
      return;
    }
    if (activeFilePath && projectSession.draft === "dirty") {
      setNotice("Refresh blocked: save or discard the current Markdown edits first.");
      return;
    }
    const root = project.root;
    const generation = ++repositoryGenerationRef.current;
    try {
      const refreshedProject = await projectSessionEffects.refresh(root);
      if (generation !== repositoryGenerationRef.current) return;
      let file: Awaited<ReturnType<typeof projectSessionEffects.readFile>> | null = null;
      if (activeFilePath) {
        file = await projectSessionEffects.readFile(root, activeFilePath);
        if (generation !== repositoryGenerationRef.current) return;
      }
      setProject(refreshedProject);
      if (file) {
        setMarkdown(file.content);
        setLoadedMarkdown(file.content);
        setContentVersion(file.version);
      }
      setNotice(`Refreshed repository truth for ${project.root}`);
    } catch (error) {
      if (generation !== repositoryGenerationRef.current) return;
      setNotice(`Refresh failed: ${describeProjectError(error)}`);
    }
  }

  async function runHelper(helperId: HelperId) {
    if (operationRunning) {
      setNotice("Helper invocation is blocked while another local operation is running.");
      return;
    }
    if (helperId === "feature-planning-check") {
      setNotice("Use the guided Plan surface to bind the helper to one feature request.");
      return;
    }
    const invocation: HelperInvocation = helperId === "execution-check"
      ? { helperId, mode: "task-contract", taskPath: activeFilePath }
      : { helperId };
    if (helperId === "execution-check" && !project.files.some((file) => file.kind === "task" && file.path === activeFilePath && isExecutionHelperTaskPath(file.path))) {
      setNotice("Execution check requires the currently selected inventoried Markdown task.");
      return;
    }
    if (isDemo || !isTauriRuntime()) {
      const time = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      setEvents((current) => [...current,
        { id: `demo-helper-request-${current.length}`, time, label: "Helper request", detail: `${helperId} requested in demo mode.`, kind: "decision", simulated: true, provenance: "simulated" },
        { id: `demo-helper-start-${current.length}`, time, label: "Helper start", detail: "No native process was started; this event is simulated.", kind: "command", simulated: true, provenance: "simulated" },
        { id: `demo-helper-output-${current.length}`, time, label: "Helper output", detail: "Simulated bounded JSON output.", kind: "evidence", simulated: true, provenance: "simulated" },
        { id: `demo-helper-decision-${current.length}`, time, label: "Helper decision", detail: "ready-for-execution · confidence high", kind: "decision", simulated: true, provenance: "simulated" },
        { id: `demo-helper-refresh-${current.length}`, time, label: "Repository refresh", detail: "Simulated refresh only; repository truth was not re-read.", kind: "read", simulated: true, provenance: "simulated" },
      ]);
      setNotice(`Demo ${helperId} lifecycle captured; no helper process executed.`);
      setActiveStep(helperId === "preflight-check" ? "discover" : "next");
      return;
    }
    const root = project.root;
    const generation = ++repositoryGenerationRef.current;
    helperRootRef.current = root;
    helperGenerationRef.current = generation;
    setHelperRunning(true);
    setHelperResult(null);
    setEvents((current) => [...current, {
      id: `helper-request-${current.length}`,
      time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
      label: "Helper request",
      detail: `User requested allowlisted ${helperId}; native execution evidence is pending.`,
      kind: "decision",
      provenance: "manual",
    }]);
    try {
      const result = await executeHelper(root, invocation);
      if (helperRootRef.current !== root || helperGenerationRef.current !== generation || generation !== repositoryGenerationRef.current) return;
      setProject(result.project);
      setHelperResult(result);
      const time = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      const cancelled = result.outcome === "cancelled";
      setEvents((current) => [...current,
        ...(result.executed ? [{ id: `helper-start-${current.length}`, time, label: "Helper start", detail: `${result.executable} ${result.argv.join(" ")}`, kind: "command" as const, provenance: "real" as const }] : []),
        ...((result.stdout || result.stderr) ? [{ id: `helper-output-${current.length}`, time, label: "Helper output", detail: `${result.stderr || result.stdout}${result.stderrTruncated || result.stdoutTruncated ? " (bounded output truncated)" : ""}`, kind: "evidence" as const, provenance: result.executed ? "real" as const : "adapter" as const }] : []),
        {
          id: `helper-terminal-${current.length}`,
          time,
          label: cancelled ? "Helper cancellation" : result.success ? "Helper decision" : "Helper failure",
          detail: result.decision ? `${result.decision.decision} · confidence ${result.decision.confidence} · ${result.decision.nextAction}` : `${result.outcome}: ${result.failure ?? "No decision returned"}`,
          kind: result.success ? "decision" : "verify",
          provenance: result.executed ? "real" : "adapter",
        },
        { id: `helper-refresh-${current.length}`, time, label: "Repository refresh", detail: `Repository truth refreshed after ${result.outcome}.`, kind: "read", provenance: "real" },
      ]);
      setNotice(result.success
        ? `${helperId}: ${result.decision?.decision} · confidence ${result.decision?.confidence}; repository truth refreshed.`
        : `${helperId} ${result.outcome}; repository truth refreshed. ${result.failure ?? "Inspect bounded output."}`);
    } catch (error) {
      if (helperRootRef.current !== root || helperGenerationRef.current !== generation || generation !== repositoryGenerationRef.current) return;
      let refreshed: ProjectSnapshot | null = null;
      try {
        refreshed = await projectSessionEffects.refresh(root);
      } catch {
        // The invocation error remains primary; the inspector records refresh failure below.
      }
      if (helperRootRef.current !== root || helperGenerationRef.current !== generation || generation !== repositoryGenerationRef.current) return;
      if (refreshed) setProject(refreshed);
      setEvents((current) => [...current,
        { id: `helper-failure-${current.length}`, time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }), label: "Helper failure", detail: describeProjectError(error), kind: "verify", provenance: "adapter" },
        { id: `helper-refresh-${current.length}`, time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }), label: "Repository refresh", detail: refreshed ? "Repository truth refreshed after rejected invocation." : "Repository refresh also failed after rejected invocation.", kind: "read", provenance: refreshed ? "real" : "adapter" },
      ]);
      setNotice(`${helperId} failed before a structured terminal result: ${describeProjectError(error)}`);
    } finally {
      if (helperRootRef.current === root && helperGenerationRef.current === generation) {
        helperRootRef.current = null;
        helperGenerationRef.current = null;
        setHelperRunning(false);
      }
    }
  }

  async function cancelActiveHelper() {
    const root = helperRootRef.current;
    const generation = helperGenerationRef.current;
    if (!helperRunning || !root || generation === null) return;
    try {
      const cancellation = await cancelHelper(root);
      if (helperRootRef.current !== root || helperGenerationRef.current !== generation || repositoryGenerationRef.current !== generation) return;
      setEvents((current) => [...current, {
        id: `helper-cancel-request-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: "Helper cancellation requested",
        detail: cancellation.message,
        kind: "decision",
        provenance: "manual",
      }]);
      setNotice(cancellation.message);
    } catch (error) {
      if (helperRootRef.current !== root || helperGenerationRef.current !== generation || repositoryGenerationRef.current !== generation) return;
      setNotice(`Helper cancellation failed: ${describeProjectError(error)}`);
    }
  }

  async function runRuntime(mode: RuntimeMode) {
    if (operationRunning) {
      setNotice("Runtime invocation is blocked while another local operation is running.");
      return;
    }
    if (mode === "live" && (isDemo || !isTauriRuntime())) {
      setNotice("A confirmed live Codex run requires a selected native repository.");
      return;
    }
    if (mode === "live" && !window.confirm("Run the displayed prompt once through Codex in ephemeral, read-only sandbox mode?")) {
      setNotice("Live Codex run cancelled before execution. No provider process started.");
      return;
    }
    if ((isDemo || !isTauriRuntime()) && mode === "fixture") {
      const time = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      setEvents((current) => [...current, {
        id: `runtime-fixture-${current.length}`,
        time,
        label: "Runtime fixture",
        detail: "Deterministic Codex JSONL fixture normalized in demo mode; no provider process executed.",
        kind: "evidence",
        simulated: true,
        provenance: "simulated",
      }]);
      setNotice("Dry Codex JSONL fixture captured. No executable was probed or spawned.");
      return;
    }
    const root = project.root;
    const generation = ++repositoryGenerationRef.current;
    runtimeRootRef.current = root;
    runtimeGenerationRef.current = generation;
    runtimeRunIdRef.current = null;
    setRuntimeRunId(null);
    setRuntimeRunning(true);
    setRuntimeResult(null);
    setEvents((current) => [...current, {
      id: `runtime-request-${current.length}`,
      time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
      label: mode === "fixture" ? "Runtime fixture requested" : "Live runtime confirmed",
      detail: mode === "fixture" ? "Requested deterministic adapter fixture; no spawn is permitted." : "User confirmed one bounded prompt; native execution evidence is pending.",
      kind: "decision",
      provenance: "manual",
    }]);
    try {
      const onRuntimeMessage = (message: RuntimeStreamMessage) => {
        if (runtimeRootRef.current !== root || runtimeGenerationRef.current !== generation) return;
        const time = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
        if (message.type === "started") {
          runtimeRunIdRef.current = message.handle.runId;
          setRuntimeRunId(message.handle.runId);
          setEvents((current) => [...current, {
            id: `runtime-handle-${message.handle.runId}`,
            time,
            label: "Runtime handle issued",
            detail: `Native run ${message.handle.runId}; cancellation is scoped to this run.`,
            kind: "decision",
            provenance: "adapter",
          }]);
          return;
        }
        if (runtimeRunIdRef.current !== message.runId) return;
        const event = message.event;
        setEvents((current) => [...current, {
          id: `runtime-event-${message.runId}-${event.sequence}`,
          time,
          label: `Runtime ${event.kind}`,
          detail: event.summary,
          kind: event.kind === "error" || event.kind === "malformed" ? "verify" : "evidence",
          simulated: mode === "fixture",
          provenance: mode === "fixture" ? "simulated" : "adapter",
        }]);
      };
      const result = await executeRuntime(root, {
        mode,
        prompt: mode === "live" ? runtimePrompt : undefined,
        confirmed: mode === "live",
      }, onRuntimeMessage);
      if (runtimeRootRef.current !== root || runtimeGenerationRef.current !== generation || repositoryGenerationRef.current !== generation) return;
      setRuntimeResult(result);
      const time = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      setEvents((current) => [...current,
        ...(result.executed ? [{
          id: `runtime-start-${current.length}`,
          time,
          label: "Codex runtime start",
          detail: `${result.provenance.executable} ${result.provenance.argv.join(" ")}`,
          kind: "command" as const,
          provenance: "real" as const,
        }] : []),
        {
          id: `runtime-terminal-${current.length}`,
          time,
          label: result.success ? "Runtime completed" : "Runtime failure",
          detail: `${result.outcome}; repositoryAuthorityAdvanced=${result.repositoryAuthorityAdvanced}. ${result.failure ?? "Provider events are evidence only."}`,
          kind: result.success ? "evidence" : "verify",
          provenance: result.provenance.simulated ? "simulated" : "adapter",
        },
      ]);
      setNotice(result.success
        ? `${mode === "fixture" ? "Dry fixture" : "Live Codex run"} completed; provider self-report did not advance repository authority.`
        : `Runtime ${result.outcome}: ${result.failure ?? "Inspect the typed result."}`);
    } catch (error) {
      if (runtimeRootRef.current !== root || runtimeGenerationRef.current !== generation || repositoryGenerationRef.current !== generation) return;
      setNotice(`Runtime failed before a structured terminal result: ${describeProjectError(error)}`);
    } finally {
      if (runtimeRootRef.current === root && runtimeGenerationRef.current === generation) {
        runtimeRootRef.current = null;
        runtimeGenerationRef.current = null;
        runtimeRunIdRef.current = null;
        setRuntimeRunId(null);
        setRuntimeRunning(false);
      }
    }
  }

  async function cancelActiveRuntime() {
    const runId = runtimeRunIdRef.current;
    const generation = runtimeGenerationRef.current;
    if (!runtimeRunning || !runId || generation === null) return;
    try {
      const cancellation = await cancelRuntime(runId);
      if (runtimeRunIdRef.current !== runId || runtimeGenerationRef.current !== generation || repositoryGenerationRef.current !== generation) return;
      setNotice(cancellation.message);
    } catch (error) {
      if (runtimeRunIdRef.current !== runId || runtimeGenerationRef.current !== generation || repositoryGenerationRef.current !== generation) return;
      setNotice(`Runtime cancellation failed: ${describeProjectError(error)}`);
    }
  }

  async function prepareBoundedExecution() {
    if (operationRunning) return;
    if (isDemo || !isTauriRuntime()) {
      setNotice("The bounded controller requires a selected native repository.");
      return;
    }
    if (activeFilePath && projectSession.draft === "dirty") {
      setNotice("Bounded execution blocked: save or discard the current Markdown edits first.");
      return;
    }
    const root = project.root;
    const generation = ++repositoryGenerationRef.current;
    setControllerRunning(true);
    setBoundedResult(null);
    try {
      const preview = await previewBoundedTask(root);
      if (generation !== repositoryGenerationRef.current) return;
      setBoundedPreview(preview);
      setEvents((current) => [...current, {
        id: `bounded-preview-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: "Single-task decision",
        detail: `${preview.decision} · confidence ${preview.confidence} · ${preview.selectedTask ?? "no selected task"}; ${preview.blockingGates.length} blocking gate(s).`,
        kind: "decision",
        provenance: "real",
      }]);
      setNotice(preview.executable
        ? `Resolver selected exactly ${preview.selectedTask}. Review the confirmation preview before one invocation.`
        : `Resolver stopped at ${preview.decision}: ${preview.nextAction}`);
    } catch (error) {
      if (generation !== repositoryGenerationRef.current) return;
      setBoundedPreview(null);
      setNotice(`Bounded execution stopped before selection: ${describeProjectError(error)}`);
    } finally {
      if (generation === repositoryGenerationRef.current) setControllerRunning(false);
    }
  }

  async function runBoundedTask(mode: RuntimeMode) {
    if (!boundedPreview?.executable || !boundedPreview.selectedTask || operationRunning) return;
    const root = project.root;
    const selected = boundedPreview.selectedTask;
    const generation = ++repositoryGenerationRef.current;
    setControllerRunning(true);
    setRuntimeRunning(true);
    setBoundedResult(null);
    setSharedResult(null);
    runtimeRootRef.current = root;
    runtimeGenerationRef.current = generation;
    runtimeRunIdRef.current = null;
    try {
      const onRuntimeMessage = (message: RuntimeStreamMessage) => {
        if (runtimeRootRef.current !== root || runtimeGenerationRef.current !== generation) return;
        if (message.type === "started") {
          runtimeRunIdRef.current = message.handle.runId;
          setRuntimeRunId(message.handle.runId);
          return;
        }
        if (runtimeRunIdRef.current !== message.runId) return;
        setEvents((current) => [...current, {
          id: `bounded-runtime-${message.runId}-${message.event.sequence}`,
          time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
          label: `Bounded runtime ${message.event.kind}`,
          detail: message.event.summary,
          kind: message.event.kind === "error" || message.event.kind === "malformed" ? "verify" : "evidence",
          simulated: mode === "fixture",
          provenance: mode === "fixture" ? "simulated" : "adapter",
        }]);
      };
      const result = await executeBoundedTask(root, {
        mode,
        previewToken: boundedPreview.previewToken,
        selectedTask: selected,
        confirmed: true,
      }, onRuntimeMessage);
      if (generation !== repositoryGenerationRef.current) return;
      setProject(result.project);
      if (result.taskEvidence && activeFilePath === selected) {
        setMarkdown(result.taskEvidence.content);
        setLoadedMarkdown(result.taskEvidence.content);
        setContentVersion(result.taskEvidence.version);
      }
      setBoundedResult(result);
      setBoundedPreview(null);
      setEvents((current) => [...current, {
        id: `bounded-refresh-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: result.outcome === "verified" ? "Repository verification passed" : result.outcome === "waitExternal" ? "Wait external" : "Repository verification failed",
        detail: `${result.reason} Loop state: ${result.loopState.state}; one confirmed invocation ended.`,
        kind: result.outcome === "verified" ? "evidence" : "verify",
        provenance: "real",
      }]);
      setNotice(result.loopState.state === "continueAvailable"
        ? `${result.reason} Review the refreshed next iteration and confirm again to continue.`
        : `${result.reason} The goal loop stopped at ${result.loopState.state}.`);
      try {
        setGoalRecovery(await projectSessionEffects.recover(root));
      } catch (error) {
        setNotice(`${result.reason} Goal recovery refresh failed safely: ${describeProjectError(error)}`);
      }
    } catch (error) {
      if (generation !== repositoryGenerationRef.current) return;
      try {
        setProject(await projectSessionEffects.refresh(root));
      } catch {
        // The primary controller failure remains visible below.
      }
      setBoundedPreview(null);
      setNotice(`Bounded execution stopped after failure: ${describeProjectError(error)} No other task was selected.`);
    } finally {
      if (generation === repositoryGenerationRef.current) {
        runtimeRootRef.current = null;
        runtimeGenerationRef.current = null;
        runtimeRunIdRef.current = null;
        setRuntimeRunId(null);
        setRuntimeRunning(false);
        setControllerRunning(false);
      }
    }
  }

  async function cancelActiveBoundedTask() {
    const runId = runtimeRunIdRef.current;
    if (!controllerRunning || !runId) return;
    try {
      const cancellation = await cancelBoundedTask(runId);
      setNotice(cancellation.message);
    } catch (error) {
      setNotice(`Bounded task cancellation failed: ${describeProjectError(error)}`);
    }
  }

  async function discardRecoveredGoal() {
    try {
      await clearGoalState();
      setGoalRecovery(null);
      setNotice("Persisted orchestration receipt discarded. Repository authority files were not changed.");
    } catch (error) {
      setNotice(`Could not discard persisted orchestration receipt: ${describeProjectError(error)}`);
    }
  }

  async function inspectSkillSetup() {
    if (operationRunning) {
      setNotice("Another skill setup invocation is already running.");
      return;
    }
    if (isDemo || !isTauriRuntime()) {
      setNotice("Skill setup preview is available after opening a repository in the Tauri app.");
      return;
    }
    const operation = skillSetupOperationFor(project.skills);
    const root = project.root;
    const generation = ++repositoryGenerationRef.current;
    try {
      const preview = await previewSkillSetup(root, operation);
      if (generation !== repositoryGenerationRef.current) return;
      setSetupPreview(preview);
      setSetupResult(null);
      setEvents((current) => [...current, {
        id: `skill-setup-preview-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: "Skill setup previewed",
        detail: `${operation} ${preview.source} with ${preview.cliVersion}; no mutation executed.`,
        kind: "read",
        provenance: "manual",
      }]);
      setNotice(`Previewed ${operation}; no command or mutation was executed.`);
    } catch (error) {
      if (generation !== repositoryGenerationRef.current) return;
      setEvents((current) => [...current, {
        id: `skill-setup-preview-failed-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: "Skill setup preview failed",
        detail: describeProjectError(error),
        kind: "verify",
        provenance: "manual",
      }]);
      setNotice(`Skill setup preview failed: ${describeProjectError(error)}`);
    }
  }

  async function cancelActiveSkillSetup() {
    if (!setupRunning) {
      setSetupPreview(null);
      setEvents((current) => [...current, {
        id: `skill-setup-cancelled-preview-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: "Skill setup cancelled before execution",
        detail: "The read-only preview was closed; no command or mutation was executed.",
        kind: "decision",
        provenance: "manual",
      }]);
      setNotice("Skill setup cancelled. No command or mutation was executed.");
      return;
    }
    const root = setupRootRef.current;
    if (!root) return;
    const generation = setupGenerationRef.current;
    try {
      const cancellation = await cancelSkillSetup(root);
      if (generation !== setupGenerationRef.current || generation !== repositoryGenerationRef.current) return;
      setEvents((current) => [...current, {
        id: `skill-setup-cancel-requested-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: "Skill setup cancellation requested",
        detail: cancellation.message,
        kind: "decision",
        provenance: "manual",
      }]);
      setNotice(cancellation.message);
    } catch (error) {
      if (generation !== setupGenerationRef.current || generation !== repositoryGenerationRef.current) return;
      setEvents((current) => [...current, {
        id: `skill-setup-cancel-failed-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: "Skill setup cancellation failed",
        detail: describeProjectError(error),
        kind: "verify",
        provenance: "manual",
      }]);
      setNotice(`Skill setup cancellation failed: ${describeProjectError(error)}`);
    }
  }

  async function confirmSkillSetup() {
    if (!setupPreview || operationRunning) return;
    const setupRoot = project.root;
    const generation = ++repositoryGenerationRef.current;
    setupRootRef.current = setupRoot;
    setupGenerationRef.current = generation;
    setSetupRunning(true);
    setEvents((current) => [...current, {
        id: `skill-setup-confirmed-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: "Skill setup invocation requested",
        detail: `User confirmed and requested ${setupPreview.operation} for ${setupRoot}. Native execution evidence is pending.`,
        kind: "decision",
        provenance: "manual",
      }]);
    try {
      const result = await executeSkillSetup(setupRoot, setupPreview.operation, true, setupPreview.previewToken);
      if (setupRootRef.current !== setupRoot || setupGenerationRef.current !== generation || repositoryGenerationRef.current !== generation) return;
      setProject(result.project);
      setSetupResult(result);
      setSetupPreview(null);
      const cancelled = result.outcome === "cancelled" || result.outcome === "cancelledBeforeExecution";
      setEvents((current) => [
        ...current,
        ...(result.executed ? [{
          id: `skill-setup-executed-${current.length}`,
          time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
          label: "Skill setup process executed",
          detail: `${setupPreview.executable} ${setupPreview.argv.join(" ")}`,
          kind: "command" as const,
          provenance: "real" as const,
        }] : []),
        {
          id: `skill-setup-${result.outcome}-${current.length + (result.executed ? 1 : 0)}`,
          time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
          label: !result.executed ? "Skill setup not executed" : cancelled ? "Skill setup cancelled" : result.success ? "Skill setup completed" : "Skill setup failed",
          detail: `${result.outcome}; executed=${result.executed}; ${result.changedPaths.length} changed path(s); repository and provenance refreshed.`,
          kind: result.success ? "evidence" : "verify",
          provenance: result.executed ? "real" : "adapter",
        },
      ]);
      setNotice(result.success
        ? `Skill ${result.operation} completed; provenance and repository truth refreshed.`
        : `Skill ${result.operation} failed; repository truth refreshed. ${result.repair?.nextAction ?? "Inspect the result."}`);
    } catch (error) {
      if (setupRootRef.current !== setupRoot || setupGenerationRef.current !== generation || repositoryGenerationRef.current !== generation) return;
      setEvents((current) => [...current, {
        id: `skill-setup-failed-${current.length}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        label: "Skill setup invocation failed",
        detail: `Native invocation returned no structured execution evidence: ${describeProjectError(error)}`,
        kind: "verify",
        provenance: "manual",
      }]);
      setNotice(`Skill setup failed before execution: ${describeProjectError(error)}`);
    } finally {
      if (setupRootRef.current === setupRoot && setupGenerationRef.current === generation) {
        setupRootRef.current = null;
        setupGenerationRef.current = null;
        setSetupRunning(false);
      }
    }
  }

  function simulateRun() {
    setRunning(true);
    window.setTimeout(() => {
      setEvents((current) => [...current, ...simulatedEvents]);
      setActiveStep("verify");
      setRunning(false);
      setNotice("Simulation captured. No agent or shell command was executed.");
    }, 650);
  }

  function appendCollaborationEvent(event: Omit<RunEvent, "id" | "time">) {
    setEvents((current) => [
      ...current,
      {
        ...event,
        id: `collaboration-${current.length}-${Date.now()}`,
        time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
      },
    ]);
  }

  function applySharedRepositoryResult(result: BoundedTaskResult) {
    setProject(result.project);
    setBoundedResult(result);
    setBoundedPreview(null);
    if (result.taskEvidence && activeFilePath === result.taskEvidence.path) {
      setMarkdown(result.taskEvidence.content);
      setLoadedMarkdown(result.taskEvidence.content);
      setContentVersion(result.taskEvidence.version);
    }
  }

  return (
    <main
      className="app-shell"
      data-project-session={projectSession.selection}
      data-workflow-state={productWorkflow.state}
      data-workflow-mode={productWorkflow.mode}
      data-goal-shell-state={goalShell.state}
      data-shared-reconciliation={productWorkflow.shared?.reconciliation ?? "none"}
      data-navigation-open={String(projectNavigationOpen)}
      data-inspector-open={String(evidenceInspectorOpen)}
    >
      <header className="titlebar">
        <div className="brand-mark" aria-hidden="true"><span /><span /><span /></div>
        <div className="brand"><strong>build right</strong><span>studio</span></div>
        <button className="project-switcher" onClick={() => openProject()} disabled={operationRunning}>
          <FolderGit2 size={16} />
          <span><small>Project</small><strong>{project.name}</strong></span>
          <ChevronDown size={14} />
        </button>
        <CollaborationPanel
          key={project.root}
          root={project.root}
          projectName={project.name}
          nativeAvailable={!isDemo && isTauriRuntime()}
          disabled={operationRunning || !collaborationEligible}
          disabledReason={collaborationDisabledReason}
          goalRecovery={goalRecovery}
          onEvent={appendCollaborationEvent}
          onRepositoryResult={applySharedRepositoryResult}
          onSharedResult={setSharedResult}
          onGoalRecovery={setGoalRecovery}
          onBusyChange={setCollaborationRunning}
          onProjectionChange={setCollaborationProjection}
        />
        <div className="goal-label"><span>Goal</span><strong>{isDemo ? "Prove the local workbench" : goalShell.title}</strong></div>
        {helperRunning && <button className="preflight-button" onClick={cancelActiveHelper}>Cancel helper</button>}
        <div className="layout-controls" aria-label="Workspace panes">
          <button
            className="icon-button"
            aria-label={projectNavigationOpen ? "Collapse project navigation" : "Expand project navigation"}
            aria-pressed={projectNavigationOpen}
            onClick={() => setProjectNavigationOpen((open) => !open)}
          >
            {projectNavigationOpen ? <PanelLeftClose size={15} /> : <PanelLeftOpen size={15} />}
          </button>
          <button
            className="icon-button"
            aria-label={evidenceInspectorOpen ? "Collapse evidence inspector" : "Expand evidence inspector"}
            aria-pressed={evidenceInspectorOpen}
            onClick={() => setEvidenceInspectorOpen((open) => !open)}
          >
            {evidenceInspectorOpen ? <PanelRightClose size={15} /> : <PanelRightOpen size={15} />}
          </button>
        </div>
        <button className="icon-button" aria-label="Refresh repository" onClick={refreshRepository} disabled={operationRunning}><RefreshCw size={15} /></button>
        <div className="branch-state"><GitBranch size={14} /><span>{project.branch}</span>{project.dirty && <i>dirty</i>}</div>
      </header>

      <div
        className="workspace-grid"
        data-navigation-open={String(projectNavigationOpen)}
        data-inspector-open={String(evidenceInspectorOpen)}
      >
        <nav className="project-nav" aria-label="Project navigation">
          {recentProjects.length > 0 && (
            <div className="nav-section recent-projects" aria-label="Recent projects">
              <span className="eyebrow">Recent projects · reinspect on open</span>
              {recentProjects.map((entry) => (
                <button
                  className="nav-row"
                  key={entry.root}
                  onClick={() => reopenRecentProject(entry)}
                  title={entry.root}
                  disabled={operationRunning}
                >
                  <FolderGit2 size={14} />
                  <span>Reopen {entry.root.split("/").at(-1)}</span>
                </button>
              ))}
            </div>
          )}
          <ProjectFileNavigation
            project={project}
            activeFilePath={activeFilePath}
            selectedTaskPath={goalShell.selectedTaskPath}
            nativeAvailable={!isDemo && isTauriRuntime()}
            disabled={operationRunning}
            canGoBack={navigationHistory.index > 0 && projectSession.draft !== "dirty"}
            canGoForward={
              navigationHistory.index >= 0
              && navigationHistory.index < navigationHistory.paths.length - 1
              && projectSession.draft !== "dirty"
            }
            onBack={() => void navigateDocumentHistory(-1)}
            onForward={() => void navigateDocumentHistory(1)}
            onOpen={(path) => void openAuthorityFile(path)}
          />

          <div className="nav-section skill-section">
            <div className="section-label"><span className="eyebrow">Operating modes</span><Sparkles size={13} /></div>
            {(workflowSkills.length ? workflowSkills : [missingSkill]).map((skill) => <SkillCard key={skill.id} skill={skill} active={skill.id === activeSkill.id} onSelect={() => setSelectedSkill(skill.id)} />)}
            {principlesSkill && (
              <details className="principles-reference">
                <summary>Engineering principles</summary>
                <p>{principlesSkill.purpose}</p>
                <small>Contextual reference · {principlesSkill.source}</small>
              </details>
            )}
          </div>

          <div className="nav-section sprint-section">
            <div className="section-label"><span className="eyebrow">Active goal</span><span className="sprint-count">{goalShell.statusLabel}</span></div>
            <button className="task-row is-active" onClick={runShellPrimaryAction} disabled={operationRunning}>
              <CircleDot size={14} />
              <span>
                <strong>{goalShell.selectedTaskPath?.split("/").at(-1)?.replace(/\.md$/u, "") ?? "Resolver pending"}</strong>
                <small>{goalShell.primaryActionLabel}</small>
              </span>
              <i>{goalShell.selectedTaskStatus ?? goalShell.state}</i>
            </button>
            <small className="meter-copy">{goalShell.guidance}</small>
          </div>

          <div className="source-note"><Code2 size={14} /><span><strong>Repo-native</strong><small>{project.root}</small></span></div>
        </nav>

        <section className="document-workbench">
          <div className="document-header">
            <div>
              <nav className="document-path" aria-label="Document breadcrumbs">
                  <span>Project</span>
                <span>{bootstrapVisible ? "Discover" : planningVisible ? "Plan" : productAction.phase}</span>
                <span>{activeFilePath || goalShell.selectedTaskPath || "No project Markdown selected"}</span>
              </nav>
              <h1>{bootstrapVisible ? <><span>00</span> Discover project authority</> : planningVisible ? <><span>01</span> Plan one feature</> : <><span>{task.id}</span> {task.title}</>}</h1>
              <div className="document-meta">{bootstrapVisible ? <><b className="status-active">founder input</b><span>{bootstrapInventory.missingPaths.length} files missing</span><span>Source: founder-fed</span></> : planningVisible ? <><b className="status-active">planning only</b><span>Local Markdown</span><span>Source: founder-fed</span></> : <><b className="status-active">{task.status}</b><span>Owner: {task.owner}</span><span>Requirement: {task.requirementBasis}</span></>}</div>
            </div>
            {!bootstrapVisible && !planningVisible && <button className="save-button" onClick={saveDocument} disabled={operationRunning || !activeFilePath}><Save size={15} /> Save</button>}
          </div>

          {!bootstrapVisible && !planningVisible && !setupPreview && !setupResult && (
            <section className={`goal-shell-summary state-${goalShell.state}`} aria-label="Goal-centered project status">
              <div className="goal-shell-copy">
                <span className="eyebrow">{productAction.phase} · {goalShell.sharedContextLabel}</span>
                <strong>{productAction.label}</strong>
                <p>{goalShell.guidance}</p>
                <div className="action-instrumentation" aria-label="Action classification">
                  <span className={`effect-chip effect-${productAction.classification}`}>{productAction.effectLabel}</span>
                  <span>{productAction.consequence}</span>
                  <span>{productAction.confirmationLabel}</span>
                </div>
              </div>
              <button className="product-primary-action" onClick={runShellPrimaryAction} disabled={operationRunning}>
                {productAction.label}
              </button>
            </section>
          )}

          {!bootstrapVisible && !planningVisible && <div className="workbench-toolbar">
            <div className="view-tabs" role="tablist" aria-label="Document view">
              {(["edit", "preview", "structured"] as const).map((tab) => (
                <button key={tab} role="tab" aria-selected={view === tab} className={view === tab ? "is-active" : ""} onClick={() => setView(tab)}>{tab}</button>
              ))}
            </div>
            <span className="source-authority"><FileText size={13} /> Raw Markdown is authoritative</span>
          </div>}

          <div className="document-canvas">
            {bootstrapVisible && (
              <DiscoverBootstrap
                key={project.root}
                project={project}
                nativeAvailable={!isDemo && isTauriRuntime()}
                preflightAvailable={preflightAvailable}
                disabled={operationRunning && !bootstrapRunning}
                onBusyChange={setBootstrapRunning}
                onProjectChange={setProject}
                onPreflight={setHelperResult}
                onNotice={setNotice}
              />
            )}
            {planningVisible && (
              <FeaturePlanning
                key={project.root}
                project={project}
                nativeAvailable={isTauriRuntime()}
                disabled={operationRunning && !planningRunning}
                onBusyChange={setPlanningRunning}
                onProjectChange={setProject}
                onNotice={setNotice}
              />
            )}
            {founderGateVisible && currentPreflightResult?.decision && (
              <FounderGateResolution
                key={`${project.root}-founder-gate`}
                project={project}
                decision={currentPreflightResult.decision}
                disabled={operationRunning && !bootstrapRunning}
                onBusyChange={setBootstrapRunning}
                onProjectChange={setProject}
                onPreflight={setHelperResult}
                onNotice={setNotice}
              />
            )}
            {!planningVisible && goalRecovery && goalRecovery.state !== "missing" && (
              <section className="structured-view" aria-label="Goal recovery state">
                <div className="structured-block">
                  <span className="eyebrow">Durable orchestration receipt · repository files remain authority</span>
                  <h2>{goalRecovery.state}</h2>
                  <p>{goalRecovery.reason}</p>
                  {goalRecovery.objective && <p>Objective: {goalRecovery.objective}</p>}
                  {goalRecovery.stopConditions.length > 0 && <p>Allowed stop conditions: {goalRecovery.stopConditions.join("; ")}</p>}
                  <p>Checkpoint task: <code>{goalRecovery.checkpointTask ?? "none"}</code> · event cursor: {goalRecovery.eventCursor ?? 0}</p>
                  <p>Automatic Codex execution started: {String(goalRecovery.automaticExecutionStarted)}</p>
                </div>
                <div className="authority-callout"><ShieldCheck size={17} /><div><strong>Fresh confirmation required</strong><p>Recovery never trusts persisted task, sprint, status, or gate projections. Preparing again rereads repository truth; execution remains a separate explicit confirmation.</p></div></div>
                <p><button className="quiet-action" onClick={discardRecoveredGoal} disabled={operationRunning}>Discard orchestration receipt</button></p>
              </section>
            )}
            {boundedPreview && (
              <section className="structured-view" aria-label="Bounded task confirmation">
                <div className="structured-block">
                  <span className="eyebrow">Full resolver decision</span>
                  <h2>{boundedPreview.decision} · confidence {boundedPreview.confidence}</h2>
                  <p>{boundedPreview.nextAction}</p>
                  <p>Selected task: <code>{boundedPreview.selectedTask ?? "none"}</code></p>
                  <p>Blocking gates: {boundedPreview.blockingGates.length ? boundedPreview.blockingGates.join("; ") : "none"}</p>
                  <p>Goal-loop state: {boundedPreview.loopState.state} · fresh confirmation required: {String(boundedPreview.loopState.explicitConfirmationRequired)}</p>
                </div>
                {boundedPreview.executable && <div className="structured-block"><span className="eyebrow">Goal</span><p>{boundedPreview.goal}</p></div>}
                {boundedPreview.executable && <div className="structured-block"><span className="eyebrow">Non-goals</span>{boundedPreview.nonGoals.map((item) => <p key={item}>{item}</p>)}</div>}
                {boundedPreview.executable && <div className="structured-block"><span className="eyebrow">Source / expected effects</span><p><code>{boundedPreview.sourceUnderTest}</code></p>{boundedPreview.expectedEffects.map((item) => <p key={item}>{item}</p>)}</div>}
                <div className="authority-callout"><ShieldCheck size={17} /><div><strong>Live host warning</strong><p>{boundedPreview.liveHostWarning}</p></div></div>
                {boundedPreview.executable && <div className="authority-callout"><ShieldCheck size={17} /><div><strong>macOS Local Network consent</strong><p>The signed app launches one Codex child for this task. macOS attributes that child's provider connection to Build Right Studio and may ask for Local Network access. Allow it only if you intend to run this confirmed task.</p></div></div>}
                <p><button className="quiet-action" onClick={() => setBoundedPreview(null)} disabled={operationRunning}>Close task review</button></p>
              </section>
            )}
            {boundedResult && !boundedPreview && (
              <section aria-label="Bounded task result">
                {reviewReceipt && (
                  <ReviewReceipt
                    receipt={reviewReceipt}
                    decision={reviewDecision}
                    onDecision={setReviewDecision}
                    onContinue={
                      boundedResult.loopState.state === "continueAvailable"
                        ? () => void prepareBoundedExecution()
                        : null
                    }
                    onStop={() => setBoundedResult(null)}
                  />
                )}
                {reviewReceipt && reviewDecision === "handoff" && isTauriRuntime() && (
                  <LocalGitHandoff
                    key={boundedResult.runtime?.runId ?? boundedResult.selectedTask ?? "review"}
                    root={boundedResult.project.root}
                    receiptPaths={reviewReceipt.changedFiles.map((file) => file.path)}
                    onProjectUpdate={(nextProject) => {
                      setProject(nextProject);
                      void inspectPostRunReview(nextProject.root)
                        .then(setReviewEvidence)
                        .catch((error) => {
                          setReviewEvidenceFailure(
                            `Current Git review evidence unavailable: ${describeProjectError(error)}`,
                          );
                        });
                    }}
                  />
                )}
                {boundedResult.refreshFailures.length > 0 && (
                  <details className="review-raw" aria-label="Controller refresh failures">
                    <summary>Typed controller refresh failures</summary>
                    {boundedResult.refreshFailures.map((failure, index) => (
                      <p key={`${failure.surface}-${failure.code}-${index}`}>
                        <strong>{failure.surface}</strong> · <code>{failure.code}</code> · {failure.message}
                      </p>
                    ))}
                  </details>
                )}
                {runtimeRepair && <div className="authority-callout"><ShieldCheck size={17} /><div><strong>Typed runtime repair · {runtimeRepair.confidence}</strong><p>{runtimeRepair.message}</p></div></div>}
              </section>
            )}
            {setupPreview && (
              <section className="structured-view" aria-label="Skill setup preview">
                <div className="structured-block">
                  <span className="eyebrow">Explicit mutation preview</span>
                  <h2>{setupPreview.operation === "install" ? "Install" : "Update"} Build Right skills</h2>
                  <p>Target: <code>{setupPreview.targetProject}</code></p>
                  <p>Source: <code>{setupPreview.source}</code> · CLI: <code>{setupPreview.cliVersion}</code></p>
                  <p>Exact argv: <code>{[setupPreview.executable, ...setupPreview.argv].join(" ")}</code></p>
                </div>
                <div className="structured-block">
                  <span className="eyebrow">Expected change boundary</span>
                  {setupPreview.expectedChangedPaths.map((path) => <p key={path}><code>{path}</code></p>)}
                </div>
                <div className="structured-block">
                  <span className="eyebrow">Current → proposed provenance</span>
                  {setupPreview.hashChanges.map((change) => (
                    <p key={change.skillId}><code>{change.skillId}</code>: {change.currentHash ?? "not installed"} → {change.proposedHash ?? change.proposedState}</p>
                  ))}
                </div>
                <div className="authority-callout">
                  <ShieldCheck size={17} />
                  <div><strong>Confirmation required</strong><p>Preview is read-only. Execution uses only this fixed operation, source, skill registry, and argv.</p></div>
                </div>
                <p><button className="save-button" onClick={confirmSkillSetup} disabled={operationRunning}>{setupRunning ? "Executing…" : `Confirm ${setupPreview.operation}`}</button> <button className="save-button" onClick={cancelActiveSkillSetup}>{setupRunning ? "Cancel running setup" : "Cancel"}</button></p>
              </section>
            )}
            {setupResult && !setupPreview && (
              <section className="structured-view" aria-label="Skill setup result">
                <div className="structured-block">
                  <span className="eyebrow">Structured setup result</span>
                  <h2>{setupResult.success ? "Setup completed" : "Setup needs repair"}</h2>
                  <p>Executed: {String(setupResult.executed)} · Exit status: {setupResult.exitStatus ?? "unavailable"}</p>
                  {setupResult.repair && <p>{setupResult.repair.code}: {setupResult.repair.message} {setupResult.repair.nextAction}</p>}
                </div>
                <div className="structured-block">
                  <span className="eyebrow">Changed paths</span>
                  {setupResult.changedPaths.length ? setupResult.changedPaths.map((path) => <p key={path}><code>{path}</code></p>) : <p>No setup files changed.</p>}
                </div>
                <div className="structured-block">
                  <span className="eyebrow">Post-execution provenance</span>
                  {setupResult.after.map((state) => <p key={state.skillId}><code>{state.skillId}</code>: {state.lockHash ?? "missing hash"}</p>)}
                </div>
                {(setupResult.stdout || setupResult.stderr) && <div className="structured-block"><span className="eyebrow">Bounded process output</span><p><code>{setupResult.stderr || setupResult.stdout}</code>{(setupResult.stderrTruncated || setupResult.stdoutTruncated) && " (truncated)"}</p></div>}
                <button className="save-button" onClick={() => setSetupResult(null)}>Close result</button>
              </section>
            )}
            {helperResult && !founderGateVisible && !boundedPreview && !boundedResult && !setupPreview && !setupResult && (
              <section className="structured-view" aria-label="Helper result">
                <div className="structured-block">
                  <span className="eyebrow">Typed helper result · {helperResult.executed ? "real local" : "native adapter"}</span>
                  <h2>{helperResult.success ? "Helper decision" : "Helper failure"}</h2>
                  <p><code>{helperResult.helperId}</code> · {helperResult.outcome} · exit {helperResult.exitStatus ?? "unavailable"}</p>
                  {helperResult.decision && <p>{helperResult.decision.decision} · confidence {helperResult.decision.confidence} · {helperResult.decision.nextAction}</p>}
                  {helperResult.failure && <p>{helperResult.failure}</p>}
                </div>
                <div className="structured-block">
                  <span className="eyebrow">Bounded process output</span>
                  <p><code>{helperResult.stderr || helperResult.stdout || "No process output."}</code>{(helperResult.stderrTruncated || helperResult.stdoutTruncated) && " (truncated)"}</p>
                </div>
                {helperResult.decision && <div className="structured-block"><span className="eyebrow">Evidence / warnings</span>{helperResult.decision.evidence.map((item) => <p key={item}>{item}</p>)}{helperResult.decision.warnings.map((item) => <p key={item}>{item}</p>)}</div>}
                <button className="save-button" onClick={() => setHelperResult(null)}>Close result</button>
              </section>
            )}
            {!bootstrapVisible && !planningVisible && !boundedPreview && !boundedResult && !setupPreview && !setupResult && !helperResult && view === "edit" && (
              <div className="editor-wrap">
                <div className="line-numbers" aria-hidden="true">{markdown.split("\n").map((_, index) => <span key={index}>{index + 1}</span>)}</div>
                <textarea aria-label="Markdown source" value={markdown} onChange={(event) => setMarkdown(event.target.value)} spellCheck={false} />
              </div>
            )}
            {!bootstrapVisible && !planningVisible && !boundedPreview && !boundedResult && !setupPreview && !setupResult && !helperResult && view === "preview" && <article className="markdown-preview"><ReactMarkdown skipHtml>{markdown}</ReactMarkdown></article>}
            {!bootstrapVisible && !planningVisible && !boundedPreview && !boundedResult && !setupPreview && !setupResult && !helperResult && view === "structured" && (
              <div className="structured-view">
                <div className="structured-block"><span className="eyebrow">Goal</span><p>{task.goal || "No goal section found."}</p></div>
                <div className="structured-block"><span className="eyebrow">Acceptance criteria</span>{task.acceptanceCriteria.map((item) => <div className="criterion" key={item.text}><span className={item.checked ? "is-checked" : ""}>{item.checked && <Check size={12} />}</span><p>{item.text}</p></div>)}</div>
                <div className="authority-callout"><ShieldCheck size={17} /><div><strong>Projection only</strong><p>Structured fields are derived from Markdown. Switch to Edit to change authority.</p></div></div>
              </div>
            )}
          </div>

          <details className="operating-card">
            <summary><span>{activeSkill.phase}</span><strong>{activeSkill.name}</strong><small>Operating contract</small></summary>
            <div className="operating-card-grid">
              <div className="mode-copy"><span className="eyebrow">Selected operating mode</span><h3>{activeSkill.name}</h3><p>{activeSkill.purpose}</p></div>
              <div className="mode-decisions mode-choices"><span className="eyebrow">Possible decisions</span><div>{activeSkill.decisions.map((decision) => <code key={decision}>{decision}</code>)}</div></div>
              <div className="mode-decisions mode-surfaces"><span className="eyebrow">Reads / writes</span><div>{[...activeSkill.reads, ...activeSkill.writes].map((surface, index) => <code key={`${surface}-${index}`}>{surface}</code>)}</div></div>
              <div className="mode-decisions mode-helpers"><span className="eyebrow">Declared helpers</span><div>{activeSkill.helpers.length ? activeSkill.helpers.map((helper) => <code key={helper}>{helper}</code>) : <code>none</code>}</div></div>
              <div className="mode-decisions mode-evidence"><span className="eyebrow">Evidence / stops</span><div>{[...activeSkill.requiredEvidence, ...activeSkill.stopStates].map((item, index) => <code key={`${item}-${index}`}>{item}</code>)}</div></div>
              <div className="mode-source"><span className="eyebrow">Skill source</span><strong>{activeSkill.source}</strong><small>{activeSkill.installedPath}{activeSkill.lockHash ? ` · ${activeSkill.lockHash.slice(0, 12)}` : " · generic non-executable fallback"}</small></div>
            </div>
          </details>
          <details className="developer-tools" aria-label="Developer tools and diagnostics">
            <summary><Braces size={15} /><span><strong>Developer tools</strong><small>Fixtures, probes, raw payloads, and troubleshooting</small></span></summary>
            <div className="developer-tools-grid">
              <section className="diagnostic-card">
                <span className="effect-chip effect-diagnostic">Diagnostic</span>
                <h3>Deterministic UI and controller fixtures</h3>
                <p>These controls test adapters. They do not fulfill product work or advance repository authority.</p>
                <div className="diagnostic-actions">
                  <button onClick={simulateRun} disabled={running || operationRunning || events.length > initialEvents.length}>{running ? "Running…" : "Simulate checkpoint"}</button>
                  {boundedPreview?.executable && <button onClick={() => runBoundedTask("fixture")} disabled={operationRunning}>Run controller fixture</button>}
                  <button onClick={inspectSkillSetup} disabled={operationRunning}>Inspect skill setup adapter</button>
                  <button onClick={() => runHelper("preflight-check")} disabled={operationRunning}>Run raw preflight helper</button>
                  {activeSkill.helpers
                    .filter((helper): helper is HelperId => helper !== "preflight-check" && isHelperId(helper))
                    .map((helper) => (
                      <button
                        key={helper}
                        onClick={() => runHelper(helper)}
                        disabled={
                          operationRunning
                          || (helper === "execution-check" && !isExecutionHelperTaskPath(activeFilePath))
                        }
                      >
                        Run raw {helper}
                      </button>
                    ))}
                </div>
              </section>
              <section className="diagnostic-card" aria-label="Codex runtime adapter">
                <span className="effect-chip effect-diagnostic">Diagnostic</span>
                <h3>Generic runtime probe</h3>
                <p>Provider events are evidence only. This probe cannot change tracker, checkpoint, or release authority.</p>
                <textarea aria-label="Runtime prompt" value={runtimePrompt} maxLength={4096} onChange={(event) => setRuntimePrompt(event.target.value)} disabled={operationRunning} />
                <div className="diagnostic-actions">
                  <button onClick={() => runRuntime("fixture")} disabled={operationRunning}>Run dry runtime fixture</button>
                  <button onClick={() => runRuntime("live")} disabled={operationRunning}>Confirm read-only runtime probe</button>
                  {runtimeRunning && <button onClick={cancelActiveRuntime} disabled={!runtimeRunId}>Cancel runtime probe</button>}
                </div>
              </section>
            </div>
            {runtimeResult && (
              <section className="diagnostic-result" aria-label="Runtime diagnostic result">
                <div><span className="eyebrow">Typed runtime diagnostic · {runtimeResult.provenance.simulated ? "fixture" : "adapter"}</span><h3>{runtimeResult.outcome}</h3><p>Executed: {String(runtimeResult.executed)} · exit {runtimeResult.exitStatus ?? "unavailable"}</p></div>
                <details><summary>Raw bounded payloads and adapter metadata</summary>
                  <p><code>{runtimeResult.provenance.executable}</code> · {runtimeResult.provenance.runtimeVersion ?? "version unavailable"}</p>
                  <p>Exact argv: <code>{runtimeResult.provenance.argv.join(" ") || "fixture: no argv and no spawn"}</code></p>
                  {runtimeResult.events.map((event) => <p key={event.sequence}><strong>{event.kind}</strong> · {event.summary}<br /><code>{event.rawPayload.encoding}:{event.rawPayload.data}</code></p>)}
                </details>
                <button className="quiet-action" onClick={() => setRuntimeResult(null)}>Clear diagnostic result</button>
              </section>
            )}
          </details>
        </section>

        <RunInspector events={events} running={running} operationRunning={operationRunning} taskLabel={goalShell.selectedTaskPath ? goalShell.selectedTaskPath.split("/").at(-1) ?? "resolver-selected task" : "goal"} requirementBasis={goalShell.statusLabel} onCollapse={() => setEvidenceInspectorOpen(false)} />
      </div>

      <div className="notice-bar" role="status">
        <span>{notice}</span>
        {(staleConflict || pendingNavigationPath || pendingProjectSwitch) && (
          <span>
            <button onClick={discardAndContinue} disabled={operationRunning}>
              {pendingProjectSwitch ? "Discard and switch project" : pendingNavigationPath ? "Discard and open" : "Reload disk version"}
            </button>
            <button onClick={keepDraft} disabled={operationRunning}>Keep draft</button>
          </span>
        )}
      </div>
      <ExecutionRibbon items={goalShell.checkpoints} proof={goalShell.statusLabel} activeStep={activeStep} onSelect={setActiveStep} />
    </main>
  );
}
