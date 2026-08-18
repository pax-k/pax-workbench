import { useMemo, useState } from "react";
import {
  AlertTriangle,
  Check,
  ChevronRight,
  FilePlus2,
  ShieldCheck,
  X,
} from "lucide-react";

import {
  buildBootstrapDrafts,
  deriveBootstrapInventory,
  founderInputContract,
  validateFounderInputs,
  type FounderBootstrapInputs,
} from "../lib/discover-bootstrap";
import {
  applyArtifactPlan,
  describeProjectError,
  executeHelper,
  previewArtifactPlan,
  projectErrorCode,
} from "../lib/bridge";
import type {
  ArtifactApplyResult,
  ArtifactPlanPreview,
  HelperResult,
  ProjectSnapshot,
} from "../types";

const initialInputs: FounderBootstrapInputs = {
  productName: "",
  primaryUser: "",
  primaryWorkflow: "",
  valueMoment: "",
  hardConstraint: "",
};

export interface DiscoverBootstrapProps {
  project: ProjectSnapshot;
  nativeAvailable: boolean;
  preflightAvailable: boolean;
  disabled: boolean;
  onBusyChange: (busy: boolean) => void;
  onProjectChange: (project: ProjectSnapshot) => void;
  onPreflight: (result: HelperResult) => void;
  onNotice: (message: string) => void;
}

export function DiscoverBootstrap({
  project,
  nativeAvailable,
  preflightAvailable,
  disabled,
  onBusyChange,
  onProjectChange,
  onPreflight,
  onNotice,
}: DiscoverBootstrapProps) {
  const inventory = useMemo(() => deriveBootstrapInventory(project), [project]);
  const [inputs, setInputs] = useState(initialInputs);
  const [preview, setPreview] = useState<ArtifactPlanPreview | null>(null);
  const [result, setResult] = useState<ArtifactApplyResult | null>(null);
  const [preflight, setPreflight] = useState<HelperResult | null>(null);
  const [error, setError] = useState<{ code: string | null; message: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const missingInputs = validateFounderInputs(inputs);
  const mutationDisabled = disabled || busy || !nativeAvailable;

  function updateInput(key: keyof FounderBootstrapInputs, value: string) {
    setInputs((current) => ({ ...current, [key]: value }));
    setPreview(null);
    setResult(null);
    setPreflight(null);
    setError(null);
  }

  async function preparePreview() {
    const drafts = buildBootstrapDrafts(project, inputs);
    if (!drafts.length) {
      setError({
        code: "founder_input_required",
        message: "Complete the five founder-supplied fields before previewing repository authority.",
      });
      return;
    }
    setBusy(true);
    onBusyChange(true);
    setError(null);
    try {
      const next = await previewArtifactPlan(project.root, drafts);
      setPreview(next);
      setResult(null);
      onNotice(`Artifact preview ready for ${next.targets.length} exact create-only paths.`);
    } catch (cause) {
      setError({ code: projectErrorCode(cause), message: describeProjectError(cause) });
    } finally {
      setBusy(false);
      onBusyChange(false);
    }
  }

  async function confirmApply() {
    if (!preview) return;
    setBusy(true);
    onBusyChange(true);
    setError(null);
    try {
      const applied = await applyArtifactPlan(project.root, preview.previewToken, true);
      setResult(applied);
      setPreview(null);
      onProjectChange(applied.project);
      if (!applied.success) {
        setError({
          code: applied.failureCode,
          message: `${applied.failureMessage ?? "Artifact creation stopped."} Committed: ${applied.committedPaths.join(", ") || "none"}. Unapplied: ${applied.unappliedPaths.join(", ") || "none"}.`,
        });
        return;
      }
      if (!preflightAvailable) {
        onNotice("Authority created. Install the project-scoped Build Right skills, then run preflight.");
        return;
      }
      const checked = await executeHelper(project.root, {
        helperId: "preflight-check",
        mode: "all",
      });
      setPreflight(checked);
      onPreflight(checked);
      onProjectChange(checked.project);
      onNotice(
        checked.decision
          ? `Preflight: ${checked.decision.decision}. ${checked.decision.nextAction}`
          : `Preflight stopped: ${checked.outcome}.`,
      );
    } catch (cause) {
      setError({ code: projectErrorCode(cause), message: describeProjectError(cause) });
    } finally {
      setBusy(false);
      onBusyChange(false);
    }
  }

  return (
    <section className="discover-bootstrap" aria-labelledby="discover-bootstrap-title">
      <header className="discover-intro">
        <div>
          <span className="eyebrow">Discover · local authority first</span>
          <h2 id="discover-bootstrap-title">Draft the terrain before the agent moves.</h2>
          <p>
            Supply five founder-owned facts and decisions. The workbench will
            show every Markdown file before a separate confirmed create.
          </p>
        </div>
        <div className="discover-meter" aria-label={`${inventory.existingPaths.length} of ${inventory.existingPaths.length + inventory.missingPaths.length} bootstrap artifacts present`}>
          <strong>{inventory.existingPaths.length}</strong>
          <span>of {inventory.existingPaths.length + inventory.missingPaths.length}<br />authority files present</span>
        </div>
      </header>

      <div className="discover-ledger">
        <form className="discover-questions" onSubmit={(event) => { event.preventDefault(); void preparePreview(); }}>
          <div className="discover-section-heading">
            <span className="eyebrow">Founder input ledger</span>
            <small>Answers remain draft until confirmed</small>
          </div>
          {founderInputContract.map((field, index) => (
            <label key={field.key} className="discover-question">
              <span className="discover-question-index">{String(index + 1).padStart(2, "0")}</span>
              <span className="discover-question-copy">
                <strong>{field.label}</strong>
                <small>{field.prompt}</small>
                <em>{field.evidence}</em>
              </span>
              <input
                aria-label={field.label}
                value={inputs[field.key]}
                maxLength={240}
                disabled={busy}
                onChange={(event) => updateInput(field.key, event.target.value)}
              />
            </label>
          ))}
          <button
            className="discover-primary"
            type="submit"
            disabled={mutationDisabled || missingInputs.length > 0}
          >
            <FilePlus2 size={16} />
            Preview {inventory.missingPaths.length} authority files
            <ChevronRight size={15} />
          </button>
          {!nativeAvailable && (
            <p className="discover-inline-gate">
              Open a real Git repository in the native app to create authority files.
            </p>
          )}
        </form>

        <aside className="discover-plan" aria-label="Bootstrap artifact inventory">
          <div className="discover-section-heading">
            <span className="eyebrow">Repository evidence</span>
            <small>Derived from current inventory</small>
          </div>
          <ol className="discover-paths">
            {inventory.existingPaths.map((path) => (
              <li className="is-present" key={path}><Check size={13} /><code>{path}</code><span>present</span></li>
            ))}
            {inventory.missingPaths.map((path) => (
              <li key={path}><span className="path-node" /><code>{path}</code><span>missing</span></li>
            ))}
          </ol>
          <div className="discover-shared-gate">
            <ShieldCheck size={16} />
            <div>
              <strong>Shared mode waits for local truth</strong>
              <p>No session, publish, claim, or remote write is part of bootstrap.</p>
            </div>
          </div>
        </aside>
      </div>

      {preview && (
        <section className="discover-preview" aria-label="Artifact creation preview">
          <div className="discover-section-heading">
            <div>
              <span className="eyebrow">Exact create-only plan</span>
              <h3>Review {preview.targets.length} files before creation</h3>
            </div>
            <button type="button" className="icon-button" aria-label="Cancel artifact preview" onClick={() => setPreview(null)} disabled={busy}><X size={15} /></button>
          </div>
          <div className="discover-preview-files">
            {preview.targets.map((target) => (
              <details key={target.path}>
                <summary><code>{target.path}</code><span>{target.effect}</span></summary>
                <pre>{target.content}</pre>
              </details>
            ))}
          </div>
          <div className="discover-confirm">
            <div><ShieldCheck size={16} /><span><strong>One-use confirmation</strong><small>Current Git baseline · local plan mutation · no overwrite · no collaboration</small></span></div>
            <button className="discover-primary" type="button" onClick={() => void confirmApply()} disabled={mutationDisabled}>
              {busy ? "Creating…" : "Confirm and create files"}
            </button>
          </div>
        </section>
      )}

      {error && (
        <div className="discover-error" role="alert">
          <AlertTriangle size={17} />
          <div><strong>{error.code ?? "Bootstrap stopped"}</strong><p>{error.message}</p></div>
        </div>
      )}

      {result?.success && (
        <div className="discover-result" role="status">
          <Check size={18} />
          <div>
            <strong>Repository authority created</strong>
            <p>{result.committedPaths.length} new and {result.alreadyCommittedPaths.length} already matching files verified.</p>
            {preflight
              ? <p>Preflight: <code>{preflight.decision?.decision ?? preflight.outcome}</code> · {preflight.decision?.nextAction ?? preflight.failure}</p>
              : <p>{preflightAvailable ? "Preflight result unavailable." : "Next: install Build Right skills, then run preflight."}</p>}
          </div>
        </div>
      )}
    </section>
  );
}
