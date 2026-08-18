import { useEffect, useMemo, useState } from "react";
import {
  BookOpen,
  Braces,
  ChevronLeft,
  ChevronRight,
  FileCheck2,
  FileText,
  Search,
} from "lucide-react";
import { readProjectFile } from "../lib/bridge";
import {
  projectNavigationGroups,
  type NavigationStatusFilter,
} from "../lib/navigation";
import type { ProjectSnapshot } from "../types";

export function ProjectFileNavigation({
  project,
  activeFilePath,
  selectedTaskPath,
  nativeAvailable,
  disabled,
  canGoBack,
  canGoForward,
  onBack,
  onForward,
  onOpen,
}: {
  project: ProjectSnapshot;
  activeFilePath: string;
  selectedTaskPath: string | null;
  nativeAvailable: boolean;
  disabled: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  onBack: () => void;
  onForward: () => void;
  onOpen: (path: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState<NavigationStatusFilter>("all");
  const [trackerMarkdown, setTrackerMarkdown] = useState<Record<string, string>>({});
  const [openGroups, setOpenGroups] = useState<Record<string, boolean>>({});

  useEffect(() => {
    setQuery("");
    setStatus("all");
    setTrackerMarkdown({});
    setOpenGroups({});
    if (!nativeAvailable) return;
    let cancelled = false;
    const trackers = project.files.filter((file) => /^tasks\/sprint-[^/]+\.md$/u.test(file.path));
    void Promise.all(
      trackers.map(async (tracker) => {
        try {
          const file = await readProjectFile(project.root, tracker.path);
          return [tracker.path, file.content] as const;
        } catch {
          return [tracker.path, ""] as const;
        }
      }),
    ).then((entries) => {
      if (!cancelled) setTrackerMarkdown(Object.fromEntries(entries));
    });
    return () => {
      cancelled = true;
    };
  }, [nativeAvailable, project.files, project.root]);

  const groups = useMemo(
    () => projectNavigationGroups({ files: project.files, trackerMarkdown, query, status }),
    [project.files, query, status, trackerMarkdown],
  );

  return (
    <section className="file-navigation" aria-label="Repository files">
      <div className="file-navigation-heading">
        <span className="eyebrow">Project truth</span>
        <div className="history-actions" aria-label="Document history">
          <button aria-label="Previous document" onClick={onBack} disabled={disabled || !canGoBack}>
            <ChevronLeft size={14} />
          </button>
          <button aria-label="Next document" onClick={onForward} disabled={disabled || !canGoForward}>
            <ChevronRight size={14} />
          </button>
        </div>
      </div>
      <div className="navigation-filters">
        <label>
          <Search size={13} />
          <input
            aria-label="Search project files"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Find task or document"
          />
        </label>
        <select
          aria-label="Filter project files by status"
          value={status}
          onChange={(event) => setStatus(event.target.value as NavigationStatusFilter)}
        >
          <option value="all">All states</option>
          <option value="ready">Ready</option>
          <option value="active">Active</option>
          <option value="complete">Complete</option>
          <option value="blocked">Blocked</option>
          <option value="other">Other</option>
        </select>
      </div>
      <div className="navigation-groups">
        {groups.map((group) => {
          const containsCurrent = group.files.some(
            (file) => file.path === activeFilePath || file.path === selectedTaskPath,
          );
          return (
            <details
              key={group.id}
              open={openGroups[group.id] ?? (containsCurrent || group.id === "authority")}
              onToggle={(event) => {
                const open = event.currentTarget.open;
                setOpenGroups((current) =>
                  current[group.id] === open ? current : { ...current, [group.id]: open }
                );
              }}
            >
              <summary>
                <span>{group.label}</span>
                <small>{group.files.length}</small>
              </summary>
              {group.files.map((file) => {
                const Icon = file.kind === "task"
                  ? Braces
                  : file.kind === "evidence"
                    ? FileCheck2
                    : BookOpen;
                return (
                  <button
                    aria-label={`Open file ${file.path}`}
                    className={`nav-row ${file.path === activeFilePath ? "is-current" : ""} ${file.path === selectedTaskPath ? "is-selected-task" : ""}`}
                    key={file.path}
                    onClick={() => onOpen(file.path)}
                    title={file.path}
                    disabled={disabled}
                  >
                    <Icon size={14} />
                    <span>{file.name}</span>
                    {file.status && <b>{file.status}</b>}
                  </button>
                );
              })}
            </details>
          );
        })}
        {project.files.length === 0 && (
          <div className="nav-empty"><FileText size={14} /><span>No project Markdown found</span></div>
        )}
        {project.files.length > 0 && groups.length === 0 && (
          <div className="nav-empty"><Search size={14} /><span>No files match this filter</span></div>
        )}
      </div>
    </section>
  );
}
