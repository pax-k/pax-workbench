import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertTriangle,
  Check,
  CircleDot,
  Eye,
  GitMerge,
  Link2,
  LockKeyhole,
  Network,
  Play,
  ShieldCheck,
  Unplug,
  UploadCloud,
  Wrench,
  X,
} from "lucide-react";

import { collaborationEffects } from "../lib/collaboration-effects";
import {
  classifyCollaborationError,
  collaborationEffectLabel,
  compactBindingHash,
  type CollaborationSurfaceState,
  type MissingCollaborationEffect,
  type SanitizedSessionMetadata,
} from "../lib/collaboration";
import {
  collaborationAccessLabel as accessLabel,
  collaborationSurfaceCopy as surfaceCopy,
  completionMissingEffects,
  deriveCollaborationPanelProjection,
  recoveryCollaborationSurface as recoverySurface,
  sharedResultProjection as sharedResultState,
  type CollaborationBusyAction as BusyAction,
  type CollaborationRepairKind as RepairKind,
  type SafePublishPreview,
  type SafeSharedPreview,
} from "../lib/collaboration-panel-model";
import type { ProductCollaborationInput } from "../lib/product-workflow";
import type {
  BoundedTaskResult,
  GoalRecovery,
  Ha2haJoinResult,
  RunEvent,
  RuntimeMode,
  SharedBoundedTaskResult,
} from "../types";

type CollaborationEvent = Omit<RunEvent, "id" | "time">;

export interface CollaborationPanelProps {
  root: string;
  projectName: string;
  nativeAvailable: boolean;
  disabled: boolean;
  disabledReason?: string;
  goalRecovery: GoalRecovery | null;
  onEvent: (event: CollaborationEvent) => void;
  onRepositoryResult: (result: BoundedTaskResult) => void;
  onSharedResult: (result: SharedBoundedTaskResult) => void;
  onGoalRecovery: (recovery: GoalRecovery) => void;
  onBusyChange: (busy: boolean) => void;
  onProjectionChange: (projection: ProductCollaborationInput) => void;
}

export function CollaborationPanel({
  root,
  projectName,
  nativeAvailable,
  disabled,
  disabledReason,
  goalRecovery,
  onEvent,
  onRepositoryResult,
  onSharedResult,
  onGoalRecovery,
  onBusyChange,
  onProjectionChange,
}: CollaborationPanelProps) {
  const [open, setOpen] = useState(false);
  const [session, setSession] = useState<SanitizedSessionMetadata | null>(null);
  const [joined, setJoined] = useState<Ha2haJoinResult | null>(null);
  const [surfaceState, setSurfaceState] = useState<CollaborationSurfaceState>(() =>
    recoverySurface(goalRecovery),
  );
  const [busy, setBusy] = useState<BusyAction | null>(null);
  const [notice, setNotice] = useState(surfaceCopy[recoverySurface(goalRecovery)].detail);
  const [publishPreview, setPublishPreview] = useState<SafePublishPreview | null>(null);
  const [sharedPreview, setSharedPreview] = useState<SafeSharedPreview | null>(null);
  const [missingEffects, setMissingEffects] = useState<MissingCollaborationEffect[]>(
    goalRecovery?.collaboration?.missingEffects ?? [],
  );
  const [repairKind, setRepairKind] = useState<RepairKind>(
    goalRecovery?.collaboration && goalRecovery.collaboration.state !== "reconciled"
      ? "completion"
      : null,
  );
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLElement>(null);
  const workspaceUrlRef = useRef<HTMLInputElement>(null);
  const actorRef = useRef<HTMLInputElement>(null);
  const projectGenerationRef = useRef(0);

  useEffect(() => {
    projectGenerationRef.current += 1;
    setSession(null);
    setJoined(null);
    setPublishPreview(null);
    setSharedPreview(null);
    setBusy(null);
    setSurfaceState(recoverySurface(goalRecovery));
    setNotice(surfaceCopy[recoverySurface(goalRecovery)].detail);
    setMissingEffects(goalRecovery?.collaboration?.missingEffects ?? []);
    setRepairKind(
      goalRecovery?.collaboration && goalRecovery.collaboration.state !== "reconciled"
        ? "completion"
        : null,
    );
    if (workspaceUrlRef.current) workspaceUrlRef.current.value = "";
    if (actorRef.current) actorRef.current.value = "build-right-studio";
  }, [root]);

  useEffect(() => {
    if (session || !goalRecovery?.collaboration) return;
    const nextSurface = recoverySurface(goalRecovery);
    setSurfaceState(nextSurface);
    setMissingEffects(goalRecovery.collaboration.missingEffects);
    setRepairKind(goalRecovery.collaboration.state === "reconciled" ? null : "completion");
    setNotice(surfaceCopy[nextSurface].detail);
  }, [goalRecovery, session]);

  useEffect(() => {
    if (!open) return;
    panelRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setOpen(false);
        triggerRef.current?.focus();
        return;
      }
      if (event.key !== "Tab" || !panelRef.current) return;
      const focusable = [
        ...panelRef.current.querySelectorAll<HTMLElement>(
          'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), summary, [href], [tabindex]:not([tabindex="-1"])',
        ),
      ].filter((element) => {
        if (element.closest('[hidden], [aria-hidden="true"]')) return false;
        const closedDetails = element.closest("details:not([open])");
        return !closedDetails
          || (element.tagName === "SUMMARY" && element.parentElement === closedDetails);
      });
      if (focusable.length === 0) {
        event.preventDefault();
        panelRef.current.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement;
      if (event.shiftKey && (active === first || active === panelRef.current)) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && active === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open]);

  useEffect(() => {
    onBusyChange(Boolean(busy));
    return () => onBusyChange(false);
  }, [busy, onBusyChange]);

  const recoveryBinding = goalRecovery?.collaboration?.intent ?? null;
  const panelProjection = useMemo(
    () =>
      deriveCollaborationPanelProjection({
        session,
        joined,
        sharedPreview,
        goalRecovery,
        surfaceState,
        repairKind,
        missingEffects,
        busy,
      }),
    [
      busy,
      goalRecovery,
      joined,
      missingEffects,
      repairKind,
      session,
      sharedPreview,
      surfaceState,
    ],
  );
  const {
    triggerLabel,
    localTaskPath,
    localTaskHash,
    remoteTaskPath,
    remoteVersion,
    currentAccess,
    isViewer,
    completionDebt,
    canRepair,
  } = panelProjection;

  useEffect(() => {
    onProjectionChange(panelProjection.product);
  }, [onProjectionChange, panelProjection.product]);

  function emit(event: CollaborationEvent) {
    onEvent(event);
  }

  function closePanel() {
    setOpen(false);
    triggerRef.current?.focus();
  }

  function safeFailure(error: unknown, action: string) {
    const failure = classifyCollaborationError(error);
    setSurfaceState(failure.state);
    setNotice(failure.detail);
    setSharedPreview(null);
    emit({
      label: `${action} stopped`,
      detail: failure.detail,
      kind: "verify",
      provenance: "adapter",
    });
  }

  async function connect() {
    if (!nativeAvailable || disabled || busy) return;
    let workspaceUrl = workspaceUrlRef.current?.value ?? "";
    let actor = actorRef.current?.value ?? "";
    if (workspaceUrlRef.current) workspaceUrlRef.current.value = "";
    if (actorRef.current) actorRef.current.value = "";
    if (!workspaceUrl || !actor) {
      workspaceUrl = "";
      actor = "";
      setSurfaceState("disconnected");
      setNotice("A workspace handoff and actor are required. Nothing was retained.");
      return;
    }
    const generation = projectGenerationRef.current;
    setBusy("connect");
    try {
      const request = collaborationEffects.connect(root, workspaceUrl, actor);
      workspaceUrl = "";
      actor = "";
      const next = await request;
      if (generation !== projectGenerationRef.current) return;
      setSession(next);
      setJoined(null);
      setPublishPreview(null);
      setSharedPreview(null);
      setSurfaceState("disconnected");
      setRepairKind(
        goalRecovery?.collaboration && goalRecovery.collaboration.state !== "reconciled"
          ? "completion"
          : null,
      );
      setNotice(
        `${accessLabel(next.access)} access established in native memory. Inspect the envelope before shared work.`,
      );
      emit({
        label: "Native collaboration session",
        detail: `Sanitized ${accessLabel(next.access)} session established; the handoff was cleared and no remote task was trusted.`,
        kind: "read",
        provenance: "adapter",
      });
    } catch (error) {
      if (generation !== projectGenerationRef.current) return;
      workspaceUrl = "";
      actor = "";
      safeFailure(error, "Native session");
    } finally {
      if (generation === projectGenerationRef.current) setBusy(null);
    }
  }

  async function disconnect() {
    if (!session || disabled || busy) return;
    const generation = projectGenerationRef.current;
    setBusy("disconnect");
    try {
      await collaborationEffects.disconnect(root, session.sessionId);
      if (generation !== projectGenerationRef.current) return;
      setSession(null);
      setJoined(null);
      setPublishPreview(null);
      setSharedPreview(null);
      const nextSurface = completionDebt ? "repairRequired" : "disconnected";
      setSurfaceState(nextSurface);
      setNotice(
        completionDebt
          ? "Disconnected. Local completion remains authoritative; reconnect as Collaborator to repair missing remote effects."
          : "Disconnected from shared coordination. Local solo behavior is unchanged.",
      );
      emit({
        label: "Collaboration disconnected",
        detail: "The native session was destroyed. Local repository authority was not changed.",
        kind: "decision",
        provenance: "adapter",
      });
    } catch (error) {
      if (generation !== projectGenerationRef.current) return;
      safeFailure(error, "Disconnect");
    } finally {
      if (generation === projectGenerationRef.current) setBusy(null);
    }
  }

  async function inspectEnvelope() {
    if (!session || disabled || busy) return;
    const generation = projectGenerationRef.current;
    setBusy("inspect");
    try {
      const result = await collaborationEffects.inspect(root, session.sessionId);
      if (generation !== projectGenerationRef.current) return;
      setJoined(result);
      setPublishPreview(null);
      setSharedPreview(null);
      const nextState: CollaborationSurfaceState = result.reconciled
        ? "reconciled"
        : result.repair
          ? "stale"
          : "disconnected";
      setSurfaceState(nextState);
      setNotice(
        result.inspectionOnly
          ? "Envelope reconciled for read-only inspection. Shared execution remains denied."
          : result.reconciled
            ? "Envelope reconciled. Preview the exact shared mutation before execution."
            : "Envelope inspection stopped before a trusted binding was established.",
      );
      emit({
        label: "HA2HA envelope inspected",
        detail: `${accessLabel(result.access)} inspection returned a sanitized ${result.reconciled ? "reconciled" : "unreconciled"} task coordinate.`,
        kind: "read",
        provenance: "adapter",
      });
    } catch (error) {
      if (generation !== projectGenerationRef.current) return;
      setJoined(null);
      safeFailure(error, "Envelope inspection");
    } finally {
      if (generation === projectGenerationRef.current) setBusy(null);
    }
  }

  async function preparePublish() {
    if (!session || session.access !== "collaborator" || disabled || busy) return;
    const generation = projectGenerationRef.current;
    setBusy("publish-preview");
    try {
      const summary = await collaborationEffects.previewPublish(root, session.sessionId);
      if (generation !== projectGenerationRef.current) return;
      setPublishPreview(summary);
      setNotice("Publish preview is read-only. Confirm the one-envelope projection explicitly.");
      emit({
        label: "HA2HA publish preview",
        detail: "One resolver-selected local task was projected; no remote write or provider process started.",
        kind: "decision",
        provenance: "adapter",
      });
    } catch (error) {
      if (generation !== projectGenerationRef.current) return;
      safeFailure(error, "Publish preview");
    } finally {
      if (generation === projectGenerationRef.current) setBusy(null);
    }
  }

  async function publish() {
    if (!session || !publishPreview || disabled || busy) return;
    const generation = projectGenerationRef.current;
    setBusy("publish");
    try {
      const result = await collaborationEffects.publish(
        root,
        session.sessionId,
        publishPreview.previewToken,
      );
      if (generation !== projectGenerationRef.current) return;
      setPublishPreview(null);
      setSurfaceState(result.complete ? "disconnected" : "repairRequired");
      setRepairKind(result.complete ? null : "publish");
      setNotice(
        result.complete
          ? `Published one envelope in ${result.writes.length} bounded writes. Inspect it before shared execution.`
          : `Envelope publication stopped after ${result.writes.length} bounded writes. No execution started.`,
      );
      emit({
        label: result.complete ? "HA2HA envelope published" : "HA2HA publication incomplete",
        detail: result.complete
          ? `The confirmed one-task envelope completed in ${result.writes.length} sanitized remote writes.`
          : `Publication stopped after ${result.writes.length} sanitized remote writes; no provider process started.`,
        kind: result.complete ? "evidence" : "verify",
        provenance: "adapter",
      });
    } catch (error) {
      if (generation !== projectGenerationRef.current) return;
      setPublishPreview(null);
      safeFailure(error, "Envelope publication");
    } finally {
      if (generation === projectGenerationRef.current) setBusy(null);
    }
  }

  async function prepareSharedExecution() {
    if (!session || !joined?.reconciled || disabled || busy) return;
    const generation = projectGenerationRef.current;
    setBusy("shared-preview");
    try {
      const summary = await collaborationEffects.previewShared(
        root,
        session.sessionId,
      );
      if (generation !== projectGenerationRef.current) return;
      setSharedPreview(summary);
      setSurfaceState("reconciled");
      setNotice(
        summary.executable
          ? "Shared preview is bound to exact local and remote versions. Execution still requires explicit action."
          : "Viewer inspection is complete. Shared execution is denied before mutation.",
      );
      emit({
        label: "Shared execution preview",
        detail: summary.executable
          ? "Sanitized local and remote bindings were previewed; no remote claim or provider process started."
          : "Read-only access was confirmed; no remote mutation or provider process is permitted.",
        kind: "decision",
        provenance: "adapter",
      });
    } catch (error) {
      if (generation !== projectGenerationRef.current) return;
      safeFailure(error, "Shared preview");
    } finally {
      if (generation === projectGenerationRef.current) setBusy(null);
    }
  }

  async function executeShared(mode: RuntimeMode) {
    if (
      !session
      || !sharedPreview?.executable
      || !sharedPreview.explicitConfirmationRequired
      || disabled
      || busy
    ) return;
    const generation = projectGenerationRef.current;
    setBusy("shared-execute");
    emit({
      label: mode === "fixture" ? "Shared fixture confirmed" : "Shared execution confirmed",
      detail: mode === "fixture"
        ? "A deterministic controller fixture was explicitly requested; provider execution is simulated."
        : "One exact local/remote binding was explicitly confirmed; native claim evidence is pending.",
      kind: "decision",
      provenance: mode === "fixture" ? "simulated" : "manual",
      simulated: mode === "fixture",
    });
    try {
      const result = await collaborationEffects.executeShared(
        root,
        session.sessionId,
        sharedPreview,
        mode,
        (message) => {
          if (generation !== projectGenerationRef.current) return;
          emit({
            label: message.type === "started"
              ? "Shared runtime handle"
              : `Shared runtime ${message.event.kind}`,
            detail: message.type === "started"
              ? "The native adapter issued a run-scoped handle."
              : `Sanitized event ${message.event.sequence} observed; provider payload is not shown in collaboration activity.`,
            kind: message.type === "event"
              && (message.event.kind === "error" || message.event.kind === "malformed")
              ? "verify"
              : "evidence",
            provenance: mode === "fixture" ? "simulated" : "adapter",
            simulated: mode === "fixture",
          });
        },
      );
      if (generation !== projectGenerationRef.current) return;
      const terminal = sharedResultState(result);
      onSharedResult(result);
      setSurfaceState(terminal.state);
      setRepairKind(terminal.repairKind);
      setMissingEffects(terminal.missingEffects);
      setNotice(terminal.detail);
      setSharedPreview(null);
      if (result.bounded) onRepositoryResult(result.bounded);
      emit({
        label: terminal.state === "reconciled"
          ? "Shared iteration reconciled"
          : terminal.state === "conflict"
            ? "Shared conflict stop"
            : terminal.state === "repairRequired"
              ? "Shared repair stop"
              : "Shared iteration stopped",
        detail: `${terminal.detail} Runtime started: ${String(result.codexStarted)}.`,
        kind: terminal.state === "reconciled" ? "evidence" : "verify",
        provenance: mode === "fixture" ? "simulated" : "adapter",
        simulated: mode === "fixture",
      });
      try {
        const recovery = await collaborationEffects.recover(root);
        if (generation === projectGenerationRef.current) onGoalRecovery(recovery);
      } catch {
        if (generation === projectGenerationRef.current) {
          setNotice(`${terminal.detail} Durable collaboration recovery could not be refreshed.`);
        }
      }
    } catch (error) {
      if (generation !== projectGenerationRef.current) return;
      setSharedPreview(null);
      safeFailure(error, "Shared execution");
    } finally {
      if (generation === projectGenerationRef.current) setBusy(null);
    }
  }

  async function repairCompletion() {
    if (!session || !canRepair || disabled || busy) return;
    const generation = projectGenerationRef.current;
    setBusy("repair");
    try {
      const result = await collaborationEffects.repair(root, session.sessionId);
      if (generation !== projectGenerationRef.current) return;
      if (result.codexStarted) {
        setSurfaceState("repairRequired");
        setNotice("Repair invariant failed closed: runtime execution was reported during remote repair.");
        return;
      }
      const remaining = completionMissingEffects(result.completion);
      const reconciled = result.completion.status === "synchronized"
        && !result.sharedIterationBlocked;
      setSurfaceState(reconciled ? "reconciled" : "repairRequired");
      setRepairKind(reconciled ? null : "completion");
      setMissingEffects(remaining);
      setNotice(
        reconciled
          ? "Missing remote effects were reconciled. Codex was not started."
          : "Some remote effects remain missing. Local completion is unchanged and Codex was not started.",
      );
      emit({
        label: reconciled ? "Collaboration repair reconciled" : "Collaboration repair incomplete",
        detail: `${result.reconciledEffects.length} bounded remote effect(s) inspected or applied; Codex started: false.`,
        kind: reconciled ? "evidence" : "verify",
        provenance: "adapter",
      });
      try {
        const recovery = await collaborationEffects.recover(root);
        if (generation === projectGenerationRef.current) onGoalRecovery(recovery);
      } catch {
        if (generation === projectGenerationRef.current) {
          setNotice("Remote repair returned, but durable collaboration recovery could not be refreshed.");
        }
      }
    } catch (error) {
      if (generation !== projectGenerationRef.current) return;
      safeFailure(error, "Collaboration repair");
      setRepairKind("completion");
    } finally {
      if (generation === projectGenerationRef.current) setBusy(null);
    }
  }

  return (
    <>
      <button
        ref={triggerRef}
        className={`collaboration-trigger state-${surfaceState}`}
        type="button"
        aria-controls="collaboration-authority-panel"
        aria-expanded={open}
        disabled={disabled}
        title={disabledReason}
        onClick={() => setOpen((current) => !current)}
      >
        <Network size={15} />
        <span>
          <small>Collaboration</small>
          <strong>{disabledReason ?? triggerLabel}</strong>
        </span>
        <span className="collaboration-trigger-state">{surfaceCopy[surfaceState].label}</span>
      </button>

      {open && (
        <div className="collaboration-layer">
          <button
            className="collaboration-scrim"
            type="button"
            aria-label="Close collaboration panel"
            onClick={closePanel}
          />
          <section
            id="collaboration-authority-panel"
            ref={panelRef}
            className={`collaboration-panel state-${surfaceState}`}
            role="dialog"
            aria-modal="true"
            aria-labelledby="collaboration-authority-title"
            aria-describedby="collaboration-authority-description"
            tabIndex={-1}
          >
            <header className="collaboration-heading">
              <div>
                <span className="eyebrow">One local task · optional remote envelope</span>
                <h2 id="collaboration-authority-title">Collaboration authority</h2>
                <p id="collaboration-authority-description">Understand whether shared work is safe to inspect, execute, or repair.</p>
              </div>
              <button className="icon-button" type="button" aria-label="Close collaboration panel" onClick={closePanel}>
                <X size={16} />
              </button>
            </header>

            <ol className="authority-rail" aria-label="Local to remote authority path">
              <li className="authority-node is-local">
                <span className="authority-node-icon"><ShieldCheck size={15} /></span>
                <div>
                  <small>Local truth</small>
                  <strong>Authoritative</strong>
                  <span title={localTaskPath}>{localTaskPath}</span>
                  {localTaskHash && <code>{compactBindingHash(localTaskHash)}</code>}
                </div>
              </li>
              <li className={`authority-node access-${currentAccess ?? "disconnected"}`}>
                <span className="authority-node-icon">{isViewer ? <Eye size={15} /> : session ? <LockKeyhole size={15} /> : <Unplug size={15} />}</span>
                <div>
                  <small>Access</small>
                  <strong>{accessLabel(currentAccess)}</strong>
                  <span>{session?.actor ?? recoveryBinding?.actor ?? "No actor in native memory"}</span>
                </div>
              </li>
              <li className={`authority-node state-${surfaceState}`}>
                <span className="authority-node-icon">{surfaceState === "conflict" || surfaceState === "stale" ? <AlertTriangle size={15} /> : remoteTaskPath ? <Link2 size={15} /> : <CircleDot size={15} />}</span>
                <div>
                  <small>Binding</small>
                  <strong>{remoteTaskPath ? "One remote task" : "Not inspected"}</strong>
                  <span title={remoteTaskPath ?? undefined}>{remoteTaskPath ?? "No remote task trusted"}</span>
                  {remoteVersion !== null && <code>version {remoteVersion}</code>}
                </div>
              </li>
              <li className={`authority-node state-${surfaceState}`}>
                <span className="authority-node-icon">{surfaceState === "reconciled" ? <Check size={15} /> : surfaceState === "repairRequired" || surfaceState === "syncPending" ? <Wrench size={15} /> : <GitMerge size={15} />}</span>
                <div>
                  <small>Sync / repair</small>
                  <strong>{surfaceCopy[surfaceState].label}</strong>
                  <span>{surfaceCopy[surfaceState].detail}</span>
                </div>
              </li>
            </ol>

            <div className={`collaboration-status state-${surfaceState}`} role="status" aria-live="polite">
              <span />
              <div><strong>{surfaceCopy[surfaceState].label}</strong><p>{notice}</p></div>
            </div>

            {!nativeAvailable && (
              <div className="collaboration-callout">
                <ShieldCheck size={16} />
                <div><strong>Local solo preview</strong><p>Shared coordination is native-only. No hosted request is made in the browser projection.</p></div>
              </div>
            )}

            {nativeAvailable && !session && (
              <form
                className="collaboration-connect"
                onSubmit={(event) => {
                  event.preventDefault();
                  void connect();
                }}
              >
                <div className="collaboration-section-heading">
                  <div><span className="eyebrow">Native session boundary</span><h3>Connect privately</h3></div>
                  <span>Local solo remains available</span>
                </div>
                <div className="collaboration-field">
                  <label htmlFor="collaboration-workspace-handoff">Workspace handoff</label>
                  <input
                    id="collaboration-workspace-handoff"
                    ref={workspaceUrlRef}
                    type="password"
                    name="workspace-handoff"
                    aria-describedby="collaboration-workspace-handoff-help"
                    autoComplete="off"
                    spellCheck={false}
                    maxLength={4096}
                    placeholder="Paste once · cleared immediately"
                    disabled={disabled || Boolean(busy)}
                  />
                  <small id="collaboration-workspace-handoff-help">Consumed only by the native connector. It is never stored, rendered, logged, or copied.</small>
                </div>
                <div className="collaboration-field">
                  <label htmlFor="collaboration-actor-handle">Actor handle</label>
                  <input
                    id="collaboration-actor-handle"
                    ref={actorRef}
                    type="text"
                    name="collaboration-actor"
                    autoComplete="off"
                    spellCheck={false}
                    maxLength={128}
                    defaultValue="build-right-studio"
                    disabled={disabled || Boolean(busy)}
                  />
                </div>
                <button className="collaboration-primary" type="submit" disabled={disabled || Boolean(busy)}>
                  <LockKeyhole size={14} /> {busy === "connect" ? "Connecting…" : "Connect in native memory"}
                </button>
              </form>
            )}

            {session && (
              <section className="collaboration-session" aria-label="Sanitized shared session">
                <div className="collaboration-section-heading">
                  <div><span className="eyebrow">Sanitized native summary</span><h3>{accessLabel(session.access)}</h3></div>
                  <button className="collaboration-quiet" type="button" onClick={() => void disconnect()} disabled={disabled || Boolean(busy)}>
                    <Unplug size={13} /> {busy === "disconnect" ? "Disconnecting…" : "Disconnect"}
                  </button>
                </div>
                <p className="read-only-note"><ShieldCheck size={13} /> {accessLabel(session.access)} access · native-memory session</p>
                <details className="collaboration-diagnostics">
                  <summary>Technical session coordinates</summary>
                  <dl className="session-summary">
                    <div><dt>Workspace</dt><dd>{session.workspaceId}</dd></div>
                    <div><dt>Actor</dt><dd>{session.actor}</dd></div>
                    <div><dt>Access</dt><dd>{accessLabel(session.access)}</dd></div>
                  </dl>
                </details>
                <div className="collaboration-actions">
                  <button className="collaboration-primary" type="button" onClick={() => void inspectEnvelope()} disabled={disabled || Boolean(busy)}>
                    <Eye size={14} /> {busy === "inspect" ? "Inspecting…" : "Join and inspect envelope"}
                  </button>
                  <button className="collaboration-quiet" type="button" onClick={() => void preparePublish()} disabled={disabled || Boolean(busy) || session.access !== "collaborator"}>
                    <UploadCloud size={14} /> {busy === "publish-preview" ? "Preparing…" : "Preview publish"}
                  </button>
                </div>
                {isViewer && <p className="read-only-note"><Eye size={13} /> Viewer access can inspect. Claim, publish, repair, and shared execution remain denied.</p>}
              </section>
            )}

            {publishPreview && (
              <section className="collaboration-preview" aria-label="HA2HA publish confirmation">
                <div className="collaboration-section-heading">
                  <div><span className="eyebrow">Explicit remote mutation</span><h3>Publish one execution envelope</h3></div>
                  <code>{compactBindingHash(publishPreview.localTaskSha256)}</code>
                </div>
                <p>Local task: <code>{publishPreview.localTaskPath}</code></p>
                <p>Remote task: <code>{publishPreview.taskPath}</code></p>
                <ul>{publishPreview.expectedEffects.map((effect) => <li key={effect}>{effect}</li>)}</ul>
                <div className="collaboration-actions">
                  <button className="collaboration-primary" type="button" onClick={() => void publish()} disabled={disabled || Boolean(busy)}>
                    <UploadCloud size={14} /> {busy === "publish" ? "Publishing…" : "Publish this one envelope"}
                  </button>
                  <button className="collaboration-quiet" type="button" onClick={() => setPublishPreview(null)} disabled={Boolean(busy)}>Cancel</button>
                </div>
              </section>
            )}

            {joined?.reconciled && !publishPreview && (
              <section className="collaboration-preview" aria-label="Inspected HA2HA envelope">
                <div className="collaboration-section-heading">
                  <div><span className="eyebrow">Inspected envelope</span><h3>{joined.inspectionOnly ? "Safe to inspect" : "Bound to local truth"}</h3></div>
                  <span className={`access-chip access-${joined.access}`}>{accessLabel(joined.access)}</span>
                </div>
                <p>One remote execution envelope is reconciled with the selected local task.</p>
                <details className="collaboration-diagnostics">
                  <summary>Technical task binding</summary>
                  <p>Local task: <code>{joined.local.taskPath}</code></p>
                  <p>Remote task: <code>{joined.task.taskPath}</code> · version {joined.task.baseVersion}</p>
                </details>
                <button className="collaboration-primary" type="button" onClick={() => void prepareSharedExecution()} disabled={disabled || Boolean(busy)}>
                  <GitMerge size={14} /> {busy === "shared-preview" ? "Checking both sides…" : "Preview shared execution"}
                </button>
              </section>
            )}

            {sharedPreview && (
              <section className="collaboration-preview shared-execution-preview" aria-label="Shared execution confirmation">
                <div className="collaboration-section-heading">
                  <div><span className="eyebrow">Exact local + remote effects</span><h3>{sharedPreview.executable ? "Safe to execute once" : "Inspection only"}</h3></div>
                  <span className={`access-chip access-${sharedPreview.binding.session.access}`}>{accessLabel(sharedPreview.binding.session.access)}</span>
                </div>
                <details className="collaboration-diagnostics">
                  <summary>Technical source and version binding</summary>
                  <dl className="binding-grid">
                    <div><dt>Local task</dt><dd>{sharedPreview.binding.local.taskPath}</dd></div>
                    <div><dt>Local hash</dt><dd>{compactBindingHash(sharedPreview.binding.local.taskSha256)}</dd></div>
                    <div><dt>Remote task</dt><dd>{sharedPreview.binding.remote.taskPath}</dd></div>
                    <div><dt>Remote version</dt><dd>{sharedPreview.binding.remote.baseVersion}</dd></div>
                    <div className="binding-mutation"><dt>Expected mutation</dt><dd>{sharedPreview.binding.expectedRemoteMutation.fromState} → {sharedPreview.binding.expectedRemoteMutation.toState} by {sharedPreview.binding.expectedRemoteMutation.updatedBy}</dd></div>
                  </dl>
                </details>
                <div className="preview-effects">
                  <strong>Expected local effects</strong>
                  <ul>{sharedPreview.expectedEffects.map((effect) => <li key={effect}>{effect}</li>)}</ul>
                </div>
                <details>
                  <summary>Stop conditions</summary>
                  <ul>{sharedPreview.stopConditions.map((condition) => <li key={condition}>{condition}</li>)}</ul>
                </details>
                {sharedPreview.executable ? (
                  <div className="collaboration-actions">
                    <button className="collaboration-primary" type="button" onClick={() => void executeShared("live")} disabled={disabled || Boolean(busy)}>
                      <Play size={14} /> {busy === "shared-execute" ? "Running…" : "Confirm and execute one shared task"}
                    </button>
                    <button className="collaboration-quiet" type="button" onClick={() => setSharedPreview(null)} disabled={Boolean(busy)}>Cancel</button>
                  </div>
                ) : (
                  <p className="read-only-note"><Eye size={13} /> Read-only denial occurred before remote mutation and before Codex.</p>
                )}
                {sharedPreview.executable && (
                  <details className="collaboration-diagnostics">
                    <summary>Developer diagnostics</summary>
                    <button className="collaboration-quiet" type="button" onClick={() => void executeShared("fixture")} disabled={disabled || Boolean(busy)}>
                      Run simulated shared fixture
                    </button>
                  </details>
                )}
              </section>
            )}

            {(surfaceState === "conflict" || surfaceState === "stale") && (
              <section className={`collaboration-callout state-${surfaceState}`} aria-label={`${surfaceCopy[surfaceState].label} collaboration stop`}>
                <AlertTriangle size={16} />
                <div>
                  <strong>{surfaceCopy[surfaceState].label} before execution</strong>
                  <p>{notice} Refresh and inspect both authority sources before creating a new confirmation.</p>
                </div>
              </section>
            )}

            {repairKind === "claim" && (
              <section className="collaboration-callout state-repairRequired" aria-label="Remote claim repair required">
                <Wrench size={16} />
                <div>
                  <strong>Remote claim needs inspection</strong>
                  <p>The remote task may already be claimed, but Codex did not start. Inspect and explicitly reconcile or release that claim before preparing again.</p>
                </div>
              </section>
            )}

            {repairKind === "publish" && (
              <section className="collaboration-callout state-repairRequired" aria-label="Partial publish repair required">
                <Wrench size={16} />
                <div>
                  <strong>Envelope publication needs inspection</strong>
                  <p>Some remote files may exist, but no task was executed. Inspect the remote workspace and create a fresh publish preview only after resolving the partial envelope.</p>
                </div>
              </section>
            )}

            {completionDebt && (
              <section className="collaboration-repair" aria-label="Collaboration completion repair">
                <div className="collaboration-section-heading">
                  <div><span className="eyebrow">Explicit repair only</span><h3>Repair missing remote effects</h3></div>
                  <Wrench size={16} />
                </div>
                <div className="repair-authority-copy">
                  <strong>Local work may already be complete.</strong>
                  <p>Preview only the missing remote effects below. Repair applies only those effects, requires an explicit action, and never reruns Codex.</p>
                </div>
                <ul className="missing-effects">
                  {missingEffects.map((effect) => <li key={effect}><Check size={12} /> {collaborationEffectLabel(effect)}</li>)}
                  {missingEffects.length === 0 && <li><CircleDot size={12} /> Reconnect to reconstruct the missing-effect preview.</li>}
                </ul>
                {!session && <p className="read-only-note"><Unplug size={13} /> Disconnected after restart. Reconnect the exact Collaborator workspace and actor before repair.</p>}
                {session && session.access !== "collaborator" && <p className="read-only-note"><Eye size={13} /> Viewer access cannot repair remote state.</p>}
                <button className="collaboration-primary" type="button" onClick={() => void repairCompletion()} disabled={!canRepair || disabled}>
                  <Wrench size={14} /> {busy === "repair" ? "Repairing…" : "Apply only missing remote effects"}
                </button>
              </section>
            )}

            <footer className="collaboration-footer">
              <ShieldCheck size={14} />
              <p><strong>Build Right stays authoritative.</strong> Shared state coordinates access, binding, evidence, and repair; it never completes local work by itself.</p>
              <span>{projectName}</span>
            </footer>
          </section>
        </div>
      )}
    </>
  );
}
