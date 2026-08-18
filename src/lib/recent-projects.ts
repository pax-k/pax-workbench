export type RecentWorkbenchView = "edit" | "preview" | "structured";

export interface RecentProjectPreference {
  root: string;
  lastOpenedAt: number;
  selectedSkill: string;
  view: RecentWorkbenchView;
}

export const RECENT_PROJECTS_STORAGE_KEY = "build-right-studio/recent-projects/v1";
const MAX_RECENT_PROJECTS = 8;
const allowedKeys = new Set(["root", "lastOpenedAt", "selectedSkill", "view"]);
const views = new Set<RecentWorkbenchView>(["edit", "preview", "structured"]);

function isSafeAbsoluteRoot(value: unknown): value is string {
  return typeof value === "string"
    && value.startsWith("/")
    && value.length <= 4096
    && !/[\u0000-\u001f\u007f]/u.test(value);
}

function parseEntry(value: unknown): RecentProjectPreference | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (Object.keys(record).some((key) => !allowedKeys.has(key))) return null;
  if (!isSafeAbsoluteRoot(record.root)) return null;
  if (
    typeof record.lastOpenedAt !== "number"
    || !Number.isSafeInteger(record.lastOpenedAt)
    || record.lastOpenedAt < 0
  ) return null;
  if (
    typeof record.selectedSkill !== "string"
    || record.selectedSkill.length > 160
    || !/^[a-z0-9][a-z0-9-]*$/u.test(record.selectedSkill)
  ) return null;
  if (!views.has(record.view as RecentWorkbenchView)) return null;
  return {
    root: record.root,
    lastOpenedAt: record.lastOpenedAt,
    selectedSkill: record.selectedSkill,
    view: record.view as RecentWorkbenchView,
  };
}

export function parseRecentProjects(raw: string | null): RecentProjectPreference[] {
  if (!raw || raw.length > 64_000) return [];
  try {
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) return [];
    const unique = new Map<string, RecentProjectPreference>();
    for (const candidate of value) {
      const entry = parseEntry(candidate);
      if (entry && !unique.has(entry.root)) unique.set(entry.root, entry);
    }
    return [...unique.values()]
      .sort((left, right) => right.lastOpenedAt - left.lastOpenedAt)
      .slice(0, MAX_RECENT_PROJECTS);
  } catch {
    return [];
  }
}

export function readRecentProjects(
  storage: Pick<Storage, "getItem"> | null,
): RecentProjectPreference[] {
  if (!storage) return [];
  try {
    return parseRecentProjects(storage.getItem(RECENT_PROJECTS_STORAGE_KEY));
  } catch {
    return [];
  }
}

export function rememberRecentProject(
  current: RecentProjectPreference[],
  preference: RecentProjectPreference,
): RecentProjectPreference[] {
  const validated = parseEntry(preference);
  if (!validated) return current;
  return [validated, ...current.filter((entry) => entry.root !== validated.root)]
    .sort((left, right) => right.lastOpenedAt - left.lastOpenedAt)
    .slice(0, MAX_RECENT_PROJECTS);
}

export function writeRecentProjects(
  storage: Pick<Storage, "setItem"> | null,
  entries: RecentProjectPreference[],
): boolean {
  if (!storage) return false;
  try {
    storage.setItem(
      RECENT_PROJECTS_STORAGE_KEY,
      JSON.stringify(entries.flatMap((entry) => {
        const validated = parseEntry(entry);
        return validated ? [validated] : [];
      }).slice(0, MAX_RECENT_PROJECTS)),
    );
    return true;
  } catch {
    return false;
  }
}
