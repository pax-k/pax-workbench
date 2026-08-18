import { describe, expect, it } from "vitest";
import {
  RECENT_PROJECTS_STORAGE_KEY,
  parseRecentProjects,
  readRecentProjects,
  rememberRecentProject,
  writeRecentProjects,
} from "./recent-projects";

const valid = {
  root: "/tmp/product",
  lastOpenedAt: 42,
  selectedSkill: "build-right-execution",
  view: "structured" as const,
};

describe("recent project preferences", () => {
  it("round-trips only the explicit UI preference allowlist", () => {
    const writes = new Map<string, string>();
    expect(writeRecentProjects({ setItem: (key, value) => void writes.set(key, value) }, [valid])).toBe(true);
    expect(JSON.parse(writes.get(RECENT_PROJECTS_STORAGE_KEY)!)).toEqual([valid]);
    expect(readRecentProjects({ getItem: (key) => writes.get(key) ?? null })).toEqual([valid]);
  });

  it.each([
    { ...valid, taskStatus: "complete" },
    { ...valid, goalAuthority: { state: "resumable" } },
    { ...valid, capabilityUrl: "https://secret.example/token" },
    { ...valid, remoteContents: "private" },
    { ...valid, providerPayload: { output: "secret" } },
  ])("rejects entries containing authority, capability, remote, or provider fields", (entry) => {
    expect(parseRecentProjects(JSON.stringify([entry]))).toEqual([]);
  });

  it("rejects unsafe roots and malformed preferences, deduplicates, and bounds history", () => {
    const many = Array.from({ length: 12 }, (_, index) => ({
      ...valid,
      root: `/tmp/product-${index}`,
      lastOpenedAt: index,
    }));
    expect(parseRecentProjects(JSON.stringify([
      ...many,
      { ...valid, root: "relative/path" },
      { ...valid, selectedSkill: "../unsafe" },
    ]))).toHaveLength(8);
    expect(rememberRecentProject([valid], { ...valid, lastOpenedAt: 99 })).toEqual([
      { ...valid, lastOpenedAt: 99 },
    ]);
  });
});
