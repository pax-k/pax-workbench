import { useState } from "react";
import { AlertTriangle, Check, FileDiff, ShieldCheck, X } from "lucide-react";
import {
  applyArtifactPlan,
  describeProjectError,
  executeHelper,
  previewArtifactPlan,
  projectErrorCode,
  readProjectFile,
} from "../lib/bridge";
import {
  buildPlanningDrafts,
  planningCanPropose,
  validateFeatureRequest,
} from "../lib/feature-planning";
import type {
  ArtifactApplyResult,
  ArtifactDraft,
  ArtifactPlanPreview,
  HelperResult,
  ProjectSnapshot,
} from "../types";

export interface FeaturePlanningProps {
  project: ProjectSnapshot;
  nativeAvailable: boolean;
  disabled: boolean;
  onBusyChange: (busy: boolean) => void;
  onProjectChange: (project: ProjectSnapshot) => void;
  onNotice: (message: string) => void;
}

export function FeaturePlanning({
  project,
  nativeAvailable,
  disabled,
  onBusyChange,
  onProjectChange,
  onNotice,
}: FeaturePlanningProps) {
  const [feature, setFeature] = useState("");
  const [planning, setPlanning] = useState<HelperResult | null>(null);
  const [drafts, setDrafts] = useState<ArtifactDraft[]>([]);
  const [preview, setPreview] = useState<ArtifactPlanPreview | null>(null);
  const [applied, setApplied] = useState<ArtifactApplyResult | null>(null);
  const [postPlanning, setPostPlanning] = useState<HelperResult | null>(null);
  const [resolver, setResolver] = useState<HelperResult | null>(null);
  const [error, setError] = useState<{ code: string | null; message: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const validation = validateFeatureRequest(feature);

  function beginBusy() {
    setBusy(true);
    onBusyChange(true);
    setError(null);
  }
  function endBusy() {
    setBusy(false);
    onBusyChange(false);
  }
  function resetAfterFeatureChange(value: string) {
    setFeature(value);
    setPlanning(null);
    setDrafts([]);
    setPreview(null);
    setApplied(null);
    setPostPlanning(null);
    setResolver(null);
    setError(null);
  }
  async function runPlanning() {
    if (validation) return;
    beginBusy();
    try {
      const result = await executeHelper(project.root, {
        helperId: "feature-planning-check",
        featureRequest: feature.trim(),
      });
      setPlanning(result);
      onProjectChange(result.project);
      onNotice(result.decision ? `Planning: ${result.decision.decision}. ${result.decision.nextAction}` : `Planning stopped: ${result.outcome}.`);
    } catch (cause) {
      setError({ code: projectErrorCode(cause), message: describeProjectError(cause) });
    } finally {
      endBusy();
    }
  }
  async function prepareDrafts() {
    if (!planning?.decision || !planningCanPropose(planning.decision)) return;
    beginBusy();
    try {
      const tracker = planning.decision.recommendedDestination;
      if (!tracker) throw new Error("Planning helper did not return a tracker destination.");
      const file = await readProjectFile(project.root, tracker);
      const next = buildPlanningDrafts(project, feature, planning.decision, file.content, file.version);
      if (!next.length) throw new Error("Repository truth cannot produce a bounded task and tracker proposal.");
      setDrafts(next);
      onNotice(`Editable proposal ready for ${next.length} allowlisted planning artifacts.`);
    } catch (cause) {
      setError({ code: projectErrorCode(cause), message: describeProjectError(cause) });
    } finally {
      endBusy();
    }
  }
  async function preparePreview() {
    beginBusy();
    try {
      const result = await previewArtifactPlan(project.root, drafts);
      setPreview(result);
      onNotice("Exact planning diff ready; no files have been written.");
    } catch (cause) {
      setError({ code: projectErrorCode(cause), message: describeProjectError(cause) });
    } finally {
      endBusy();
    }
  }
  async function confirmApply() {
    if (!preview) return;
    beginBusy();
    try {
      const result = await applyArtifactPlan(project.root, preview.previewToken, true);
      setApplied(result);
      setPreview(null);
      onProjectChange(result.project);
      if (!result.success) {
        throw new Error(`${result.failureMessage ?? "Planning apply stopped."} Unapplied: ${result.unappliedPaths.join(", ") || "none"}.`);
      }
      const planned = await executeHelper(project.root, {
        helperId: "feature-planning-check",
        featureRequest: feature.trim(),
      });
      setPostPlanning(planned);
      const resolved = await executeHelper(project.root, { helperId: "continue-check" });
      setResolver(resolved);
      onProjectChange(resolved.project);
      onNotice(`Planning applied and read back. Resolver: ${resolved.decision?.decision ?? resolved.outcome}.`);
    } catch (cause) {
      setError({ code: projectErrorCode(cause), message: describeProjectError(cause) });
    } finally {
      endBusy();
    }
  }

  const decision = planning?.decision;
  return (
    <section className="feature-planning" aria-labelledby="feature-planning-title">
      <header className="discover-intro">
        <div>
          <span className="eyebrow">Plan · one founder outcome</span>
          <h2 id="feature-planning-title">Shape one feature into repository truth.</h2>
          <p>The local helper identifies questions and gates before an editable task-and-tracker proposal can reach confirmation.</p>
        </div>
      </header>
      <label className="planning-request">
        <strong>Describe one feature</strong>
        <textarea aria-label="Feature request" value={feature} maxLength={2000} disabled={busy} onChange={(event) => resetAfterFeatureChange(event.target.value)} />
        <small>{validation ?? "Founder-provided request; it is not customer validation."}</small>
      </label>
      <p>
        <button className="discover-primary" onClick={runPlanning} disabled={disabled || busy || !nativeAvailable || Boolean(validation)}>Run repository planning check</button>
        {(planning || drafts.length > 0) && <button className="save-button" onClick={() => resetAfterFeatureChange("")} disabled={busy}><X size={14} /> Cancel and clear</button>}
      </p>
      {decision && (
        <section className="planning-decision" aria-label="Typed planning decision">
          <div><span className="eyebrow">Decision</span><h3>{decision.decision} · {decision.confidence}</h3><p>{decision.nextAction}</p><code>{decision.recommendedDestination}</code></div>
          <div className="planning-actions">
            <div><strong>Founder questions</strong>{decision.founderQuestions?.length ? decision.founderQuestions.map((item) => <p key={item}>{item}</p>) : <p>None</p>}</div>
            <div><strong>Blocking gates / conflicts</strong>{decision.blockingGates?.length ? decision.blockingGates.map((item) => <p key={`${item.source}-${item.reason}`}><code>{item.type}</code> {item.source}: {item.reason}</p>) : <p>None</p>}</div>
            <div><strong>Research triggers</strong>{decision.researchTriggers?.length ? decision.researchTriggers.map((item) => <p key={item}>{item}</p>) : <p>None</p>}</div>
            <div><strong>Ready candidates</strong>{decision.readyTaskCandidates?.length ? decision.readyTaskCandidates.map((item) => <p key={item.path}><code>{item.path}</code> · {item.status}</p>) : <p>None</p>}</div>
          </div>
          {planningCanPropose(decision) && drafts.length === 0 && <button className="discover-primary" onClick={prepareDrafts} disabled={busy}>Draft bounded task and tracker change</button>}
        </section>
      )}
      {drafts.length > 0 && !preview && !applied && (
        <section className="discover-preview" aria-label="Editable planning proposal">
          <div className="discover-section-heading"><span className="eyebrow">Untrusted proposal · edit before preview</span><small>Planning Markdown only</small></div>
          <div className="planning-editors">{drafts.map((draft, index) => <label key={draft.path}><code>{draft.path}</code><textarea aria-label={`Proposed content for ${draft.path}`} value={draft.content} onChange={(event) => setDrafts((current) => current.map((item, itemIndex) => itemIndex === index ? { ...item, content: event.target.value } : item))} /></label>)}</div>
          <button className="discover-primary" onClick={preparePreview} disabled={busy || drafts.some((draft) => !draft.content.trim())}><FileDiff size={15} /> Preview exact diffs</button>
        </section>
      )}
      {preview && (
        <section className="discover-preview" aria-label="Planning mutation confirmation">
          <div className="discover-section-heading"><span className="eyebrow">Exact allowlisted diff</span><small>No write yet</small></div>
          <div className="discover-preview-files">{preview.targets.map((target) => <details key={target.path} open><summary><code>{target.path}</code><span>{target.effect}</span></summary><pre>{target.diff || "No content change"}</pre></details>)}</div>
          <div className="discover-confirm"><div><ShieldCheck size={16} /><span><strong>Local planning mutation only</strong><small>No source, task execution, commit, push, publish, or HA2HA effect.</small></span></div><p><button className="save-button" onClick={() => setPreview(null)} disabled={busy}>Cancel preview</button> <button className="discover-primary" onClick={confirmApply} disabled={busy}>Confirm and apply</button></p></div>
        </section>
      )}
      {applied?.success && postPlanning && resolver && (
        <section className="discover-result" aria-label="Planning verification receipt"><Check size={17} /><div><strong>Repository write and readback complete</strong><p>Changed: {applied.committedPaths.join(", ")}.</p><p>Planning: {postPlanning.decision?.decision ?? postPlanning.outcome}. Resolver: {resolver.decision?.decision ?? resolver.outcome} — {resolver.decision?.nextAction ?? resolver.failure}</p><p>Shared publications: 0.</p></div></section>
      )}
      {error && <section className="discover-error" role="alert"><AlertTriangle size={17} /><div><strong>{error.code ?? "planning_failed"}</strong><p>{error.message}</p></div></section>}
    </section>
  );
}
