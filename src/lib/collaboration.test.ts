import { describe, expect, it } from "vitest";

import {
  assertSecretFreeCollaborationProjection,
  type CollaborationProjection,
} from "./collaboration";

function localProjection(): CollaborationProjection {
  return {
    mode: "localOnly",
    session: null,
    local: {
      taskPath: "tasks/issues/013-contract.md",
      taskSha256: "sha256:task",
      repositoryId: "sha256:repository",
      gitHead: null,
      gitIndexSha256: "sha256:index",
      gitWorktreeSha256: "sha256:worktree",
      gitDirty: true,
    },
    remote: null,
    reconciliation: "localOnly",
    repair: null,
  };
}

describe("collaboration contracts", () => {
  it("represents local-only state without a session, remote task, or effect", () => {
    const projection = localProjection();

    expect(() => assertSecretFreeCollaborationProjection(projection)).not.toThrow();
    expect(projection.session).toBeNull();
    expect(projection.remote).toBeNull();
    expect(projection.reconciliation).toBe("localOnly");
  });

  it.each([
    "https://example.test/?edit=secret",
    "Authorization: secret",
    "Bearer secret",
    "workspace?k=secret",
  ])("rejects capability-like projection content: %s", (unsafeValue) => {
    const projection = localProjection();
    projection.local.repositoryId = unsafeValue;

    expect(() => assertSecretFreeCollaborationProjection(projection)).toThrow(
      /forbidden content/,
    );
  });

  it.each([
    "https://example.test/workspaces/w?opaque=credential",
    "raw provider payload opaque-value",
    "{\"remote\":\"file body\"}",
  ])("rejects arbitrary URL, payload, and body content: %s", (unsafeValue) => {
    const projection: unknown = {
      ...localProjection(),
      repair: {
        code: "repair",
        message: unsafeValue,
        nextAction: "Reconnect",
      },
    };

    expect(() => assertSecretFreeCollaborationProjection(projection)).toThrow(
      /forbidden content/,
    );
  });

  it("keeps local hashes separate from the remote integer version", () => {
    const projection = localProjection();
    projection.mode = "viewer";
    projection.remote = {
      taskId: "BR-013",
      taskPath: "tasks/BR-013.md",
      baseVersion: 7,
    };

    expect(projection.local.taskSha256).toMatch(/^sha256:/);
    expect(projection.remote.baseVersion).toBe(7);
  });

  it("rejects an untyped opaque session handle", () => {
    const projection: unknown = {
      ...localProjection(),
      mode: "viewer",
      session: {
        sessionId: "Q2FwYWJpbGl0eVZhbHVlMTIzNDU2Nzg5",
        workspaceId: "workspace-1",
        webOrigin: "https://sync.example.test",
        apiOrigin: "https://sync-api.example.test",
        access: "viewer",
        actor: "pax-workbench",
      },
    };

    expect(() => assertSecretFreeCollaborationProjection(projection)).toThrow(
      /forbidden content/,
    );
  });

  it("rejects opaque free-form repair output", () => {
    const projection: unknown = {
      ...localProjection(),
      repair: {
        code: "retry",
        message: "opaque-value-9f8e7d6c5b4a",
        nextAction: "Reconnect",
      },
    };

    expect(() => assertSecretFreeCollaborationProjection(projection)).toThrow(
      /forbidden content/,
    );
  });

  it.each([
    { evidenceIds: ["Q2FwYWJpbGl0eVZhbHVlMTIzNDU2Nzg5"] },
    { handoffId: "opaque-value-9f8e7d6c5b4a" },
    { missingEffects: ["opaque-provider-effect"] },
  ])("rejects opaque successful handoff output: %j", (value) => {
    expect(() => assertSecretFreeCollaborationProjection(value)).toThrow(
      /forbidden content/,
    );
  });

  it("accepts the closed deterministic post-run effect order", () => {
    expect(() =>
      assertSecretFreeCollaborationProjection({
        missingEffects: ["evidenceWrite", "taskUpdate", "handoffWrite", "statusWrite"],
      }),
    ).not.toThrow();
  });
});
