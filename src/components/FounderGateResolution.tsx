import { useState } from "react";
import { AlertTriangle, Check, ShieldCheck } from "lucide-react";
import {
  applyArtifactPlan,
  describeProjectError,
  executeHelper,
  previewArtifactPlan,
  projectErrorCode,
  readProjectFile,
} from "../lib/bridge";
import {
  buildFounderGateDrafts,
  validateFounderGateInputs,
} from "../lib/founder-gate-resolution";
import type {
  ArtifactPlanPreview,
  HelperDecision,
  HelperResult,
  ProjectSnapshot,
} from "../types";

interface FounderGateResolutionProps {
  project: ProjectSnapshot;
  decision: HelperDecision;
  disabled: boolean;
  onBusyChange: (busy: boolean) => void;
  onProjectChange: (project: ProjectSnapshot) => void;
  onPreflight: (result: HelperResult) => void;
  onNotice: (message: string) => void;
}

export function FounderGateResolution({
  project,
  decision,
  disabled,
  onBusyChange,
  onProjectChange,
  onPreflight,
  onNotice,
}: FounderGateResolutionProps) {
  const [context, setContext] = useState("");
  const [scopeConfirmed, setScopeConfirmed] = useState(false);
  const [preview, setPreview] = useState<ArtifactPlanPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<{ code: string | null; message: string } | null>(null);
  const validationError = validateFounderGateInputs({ context, scopeConfirmed });

  async function prepare() {
    if (validationError) return;
    setBusy(true);
    onBusyChange(true);
    setError(null);
    try {
      const [mvp, blueprint] = await Promise.all([
        readProjectFile(project.root, "docs/mvp-scope.md"),
        readProjectFile(project.root, "docs/blueprint-status.md"),
      ]);
      const drafts = buildFounderGateDrafts({ context, scopeConfirmed }, mvp, blueprint);
      const next = await previewArtifactPlan(project.root, drafts);
      setPreview(next);
      onNotice(`Founder gate preview ready for ${next.targets.length} exact paths.`);
    } catch (cause) {
      setError({ code: projectErrorCode(cause), message: describeProjectError(cause) });
    } finally {
      setBusy(false);
      onBusyChange(false);
    }
  }

  async function confirm() {
    if (!preview) return;
    setBusy(true);
    onBusyChange(true);
    setError(null);
    try {
      const applied = await applyArtifactPlan(project.root, preview.previewToken, true);
      setPreview(null);
      onProjectChange(applied.project);
      if (!applied.success) {
        setError({
          code: applied.failureCode,
          message: applied.failureMessage ?? "Founder gate write stopped before full verification.",
        });
        return;
      }
      const checked = await executeHelper(project.root, {
        helperId: "preflight-check",
        mode: "all",
      });
      onPreflight(checked);
      onProjectChange(checked.project);
      onNotice(`Preflight: ${checked.decision?.decision ?? checked.outcome}.`);
    } catch (cause) {
      setError({ code: projectErrorCode(cause), message: describeProjectError(cause) });
    } finally {
      setBusy(false);
      onBusyChange(false);
    }
  }

  return (
    <section className="discover-bootstrap founder-gate" aria-labelledby="founder-gate-title">
      <header className="discover-intro">
        <div>
          <span className="eyebrow">Discover · founder decision required</span>
          <h2 id="founder-gate-title">Resolve the smallest founder gate.</h2>
          <p>Record founder context and confirm the current MVP scope. Every repository effect is previewed before a separate write.</p>
        </div>
      </header>

      <div className="discover-ledger">
        <form className="discover-questions" onSubmit={(event) => { event.preventDefault(); void prepare(); }}>
          <div className="discover-section-heading"><span className="eyebrow">Founder input</span><small>Not customer validation</small></div>
          <label className="discover-question">
            <span className="discover-question-index">01</span>
            <span className="discover-question-copy"><strong>Founder context</strong><small>What product truth or constraint is missing from the current authority?</small><em>FOUNDER CLAIM</em></span>
            <textarea aria-label="Founder context" value={context} maxLength={4000} disabled={busy} onChange={(event) => { setContext(event.target.value); setPreview(null); }} />
          </label>
          <label className="founder-confirmation">
            <input type="checkbox" checked={scopeConfirmed} disabled={busy} onChange={(event) => { setScopeConfirmed(event.target.checked); setPreview(null); }} />
            <span>I reviewed <code>docs/mvp-scope.md</code> and confirm it reflects my current founder-owned MVP decision.</span>
          </label>
          <button className="discover-primary" type="submit" disabled={disabled || busy || Boolean(validationError)}>Preview founder gate resolution</button>
          {validationError && <p className="discover-inline-gate">{validationError}</p>}
        </form>
        <aside className="discover-plan" aria-label="Founder gate evidence">
          <div className="discover-section-heading"><span className="eyebrow">Typed preflight evidence</span><small>{decision.confidence} confidence</small></div>
          {(decision.founderQuestions ?? []).map((item) => <p key={item}><strong>{item}</strong></p>)}
          {decision.warnings.map((item) => <p key={item}>{item}</p>)}
          <div className="discover-shared-gate"><ShieldCheck size={16} /><div><strong>Local authority only</strong><p>No implementation, Git, collaboration, or remote effect is part of this resolution.</p></div></div>
        </aside>
      </div>

      {preview && (
        <section className="discover-preview" aria-label="Founder gate preview">
          <div className="discover-section-heading"><div><span className="eyebrow">Exact version-bound plan</span><h3>Review {preview.targets.length} files</h3></div></div>
          <div className="discover-preview-files">{preview.targets.map((target) => <details key={target.path}><summary><code>{target.path}</code><span>{target.effect}</span></summary><pre>{target.content}</pre></details>)}</div>
          <div className="discover-confirm"><div><Check size={16} /><span><strong>One-use confirmation</strong><small>Create one context file; update two exact versions</small></span></div><button className="discover-primary" type="button" disabled={busy} onClick={() => void confirm()}>Confirm founder gate resolution</button></div>
        </section>
      )}
      {error && <div className="discover-error" role="alert"><AlertTriangle size={17} /><div><strong>{error.code ?? "Founder gate stopped"}</strong><p>{error.message}</p></div></div>}
    </section>
  );
}
