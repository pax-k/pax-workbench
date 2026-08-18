import type {
  ParsedTask,
  ProjectSnapshot,
  SkillSetupOperation,
  SkillSummary,
  WorkflowCheckpoint,
} from "../types";

const buildRightSkillIds = [
  "build-right-preflight",
  "build-right-feature-planning",
  "build-right-execution",
  "build-right-engineering-principles",
] as const;

export interface ProjectSessionProjection {
  mode: "demo" | "repository";
  selectedPath: string | null;
  selection: "absent" | "selected" | "stale";
  draft: "clean" | "dirty";
  pendingAction: "none" | "navigate" | "switchProject" | "reload";
  mutationBlocked: boolean;
  automaticSelectionPerformed: false;
}

export interface ProjectSessionInput {
  isDemo: boolean;
  activeFilePath: string;
  markdown: string;
  loadedMarkdown: string;
  staleConflict: boolean;
  pendingNavigationPath: string | null;
  pendingProjectSwitch: boolean;
  operationRunning: boolean;
}

export function deriveProjectSessionProjection(
  input: ProjectSessionInput,
): ProjectSessionProjection {
  const dirty = input.markdown !== input.loadedMarkdown;
  return {
    mode: input.isDemo ? "demo" : "repository",
    selectedPath: input.activeFilePath || null,
    selection: input.staleConflict
      ? "stale"
      : input.activeFilePath
        ? "selected"
        : "absent",
    draft: dirty ? "dirty" : "clean",
    pendingAction: input.pendingProjectSwitch
      ? "switchProject"
      : input.pendingNavigationPath
        ? "navigate"
        : input.staleConflict
          ? "reload"
          : "none",
    mutationBlocked: input.operationRunning || dirty || input.staleConflict,
    automaticSelectionPerformed: false,
  };
}

export function deriveWorkflowCheckpoints(task: ParsedTask): WorkflowCheckpoint[] {
  const normalizedStatus = task.status.toLowerCase();
  const taskState = ["active", "in_progress"].includes(normalizedStatus)
    ? "active"
    : normalizedStatus === "ready"
      ? "ready"
      : "waiting";
  return [
    { id: "discover", label: "Preflight", detail: "Project indexed", state: "done" },
    {
      id: "plan",
      label: "Plan",
      detail: task.requirementBasis === "unknown" ? "Basis not found" : "Requirement traced",
      state: task.requirementBasis === "unknown" ? "waiting" : "done",
    },
    {
      id: "task",
      label: task.id === "—" ? "Selected file" : `Task ${task.id}`,
      detail: `${task.status} · ${task.owner}`,
      state: taskState,
    },
    { id: "verify", label: "Verify", detail: "Evidence not resolved", state: "ready" },
    { id: "next", label: "Next action", detail: "Resolver pending", state: "waiting" },
  ];
}

export function isExecutionHelperTaskPath(path: string) {
  const parts = path.split("/");
  const fileName = parts.at(-1) ?? "";
  const supportedRoot = (parts.length === 3 && parts[0] === "tasks" && parts[1] === "issues")
    || (parts.length === 2 && (parts[0] === "tasks" || parts[0] === "issues"));
  return supportedRoot
    && fileName.endsWith(".md")
    && !fileName.endsWith("sprint-0.md")
    && !fileName.endsWith("post-release-backlog.md");
}

export function skillSetupOperationFor(skills: SkillSummary[]): SkillSetupOperation {
  const hasCompleteFirstPartySet = buildRightSkillIds.every((id) => skills.some((skill) =>
    skill.id === id
    && skill.source === "pax-k/build-right"
    && skill.installedPath === `.agents/skills/${id}/SKILL.md`
    && Boolean(skill.lockHash),
  ));
  return hasCompleteFirstPartySet ? "update" : "install";
}

export function repositoryNeedsSetup(project: ProjectSnapshot): boolean {
  const required = new Set(["docs/source-index.md", "docs/mvp-scope.md", "docs/release-gates.md"]);
  for (const file of project.files) required.delete(file.path);
  return required.size > 0;
}
