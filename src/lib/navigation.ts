import type { ProjectFile } from "../types";

export type NavigationStatusFilter =
  | "all"
  | "ready"
  | "active"
  | "complete"
  | "blocked"
  | "other";

export interface ProjectNavigationGroup {
  id: string;
  label: string;
  trackerPath: string | null;
  files: ProjectFile[];
}

const issuePathPattern = /tasks\/issues\/[a-z0-9][a-z0-9-]*\.md/giu;

export function taskPathsFromTracker(markdown: string) {
  return [...new Set(markdown.match(issuePathPattern) ?? [])];
}

function matchesStatus(file: ProjectFile, filter: NavigationStatusFilter) {
  if (filter === "all") return true;
  const status = file.status?.toLowerCase() ?? "other";
  if (filter === "other") {
    return !["ready", "active", "complete", "blocked"].includes(status);
  }
  return status === filter;
}

function matchesQuery(file: ProjectFile, query: string) {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return `${file.name} ${file.path} ${file.status ?? ""}`
    .toLowerCase()
    .includes(needle);
}

export function projectNavigationGroups(input: {
  files: ProjectFile[];
  trackerMarkdown: Record<string, string>;
  query: string;
  status: NavigationStatusFilter;
}) {
  const visible = input.files.filter(
    (file) => matchesStatus(file, input.status) && matchesQuery(file, input.query),
  );
  const visibleByPath = new Map(visible.map((file) => [file.path, file]));
  const taskPaths = new Set(input.files.filter((file) => file.path.startsWith("tasks/issues/")).map((file) => file.path));
  const assigned = new Set<string>();
  const trackers = input.files
    .filter((file) => /^tasks\/sprint-[^/]+\.md$/u.test(file.path))
    .sort((left, right) => left.path.localeCompare(right.path, undefined, { numeric: true }));
  const groups: ProjectNavigationGroup[] = [];

  for (const tracker of trackers) {
    const taskPaths = taskPathsFromTracker(input.trackerMarkdown[tracker.path] ?? "");
    taskPaths.forEach((path) => assigned.add(path));
    const files = [
      visibleByPath.get(tracker.path),
      ...taskPaths.map((path) => visibleByPath.get(path)),
    ].filter((file): file is ProjectFile => Boolean(file));
    if (files.length) {
      groups.push({
        id: tracker.path,
        label: tracker.name,
        trackerPath: tracker.path,
        files,
      });
    }
  }

  const unassignedTasks = visible.filter(
    (file) => taskPaths.has(file.path) && !assigned.has(file.path),
  );
  if (unassignedTasks.length) {
    groups.push({
      id: "unassigned-tasks",
      label: "Unassigned tasks",
      trackerPath: null,
      files: unassignedTasks,
    });
  }

  const authority = visible.filter(
    (file) =>
      !file.path.startsWith("tasks/issues/")
      && !/^tasks\/sprint-[^/]+\.md$/u.test(file.path)
      && file.kind !== "evidence",
  );
  if (authority.length) {
    groups.unshift({
      id: "authority",
      label: "Project authority",
      trackerPath: null,
      files: authority,
    });
  }

  const evidence = visible.filter((file) => file.kind === "evidence");
  if (evidence.length) {
    groups.push({
      id: "evidence",
      label: "Evidence",
      trackerPath: null,
      files: evidence,
    });
  }

  return groups;
}
