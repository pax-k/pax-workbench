import { useState } from "react";
import { Check, GitCommitHorizontal, ShieldCheck } from "lucide-react";
import {
  applyLocalGitHandoff,
  describeProjectError,
  previewLocalGitHandoff,
} from "../lib/bridge";
import type {
  LocalGitHandoffPreview,
  LocalGitHandoffResult,
  ProjectSnapshot,
} from "../types";

export function LocalGitHandoff({
  root,
  receiptPaths,
  onProjectUpdate,
}: {
  root: string;
  receiptPaths: string[];
  onProjectUpdate: (project: ProjectSnapshot) => void;
}) {
  const [inspection, setInspection] = useState<LocalGitHandoffPreview | null>(null);
  const [confirmation, setConfirmation] = useState<LocalGitHandoffPreview | null>(null);
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [message, setMessage] = useState("");
  const [result, setResult] = useState<LocalGitHandoffResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const inspect = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const preview = await previewLocalGitHandoff(root, receiptPaths, [], "");
      setInspection(preview);
      setConfirmation(null);
      setSelectedPaths([]);
    } catch (nextError) {
      setError(describeProjectError(nextError));
    } finally {
      setBusy(false);
    }
  };

  const togglePath = (path: string) => {
    setConfirmation(null);
    setResult(null);
    setSelectedPaths((current) =>
      current.includes(path)
        ? current.filter((candidate) => candidate !== path)
        : [...current, path].sort()
    );
  };

  const previewSelection = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const preview = await previewLocalGitHandoff(
        root,
        receiptPaths,
        selectedPaths,
        message,
      );
      setConfirmation(preview);
    } catch (nextError) {
      setConfirmation(null);
      setError(describeProjectError(nextError));
    } finally {
      setBusy(false);
    }
  };

  const commit = async () => {
    if (!confirmation?.previewToken) return;
    setBusy(true);
    setError(null);
    try {
      const nextResult = await applyLocalGitHandoff(
        root,
        confirmation.previewToken,
        true,
      );
      setResult(nextResult);
      setConfirmation(null);
      onProjectUpdate(nextResult.project);
    } catch (nextError) {
      setConfirmation(null);
      setError(describeProjectError(nextError));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="git-handoff" aria-label="Local Git handoff">
      <div className="git-handoff-heading">
        <GitCommitHorizontal size={18} />
        <div>
          <span className="eyebrow">Separate local Git effect</span>
          <h3>Prepare one reviewed local commit</h3>
          <p>This never pushes, publishes, resolves conflicts, or changes task completion.</p>
        </div>
      </div>

      {!inspection && (
        <button className="quiet-action" onClick={() => void inspect()} disabled={busy}>
          {busy ? "Inspecting current Git state…" : "Inspect local commit options"}
        </button>
      )}

      {inspection && !result && (
        <>
          <dl className="git-handoff-baseline">
            <div><dt>HEAD</dt><dd>{inspection.baseline.head}</dd></div>
            <div><dt>Index fingerprint</dt><dd>{inspection.baseline.index}</dd></div>
            <div><dt>Worktree fingerprint</dt><dd>{inspection.baseline.worktree}</dd></div>
            <div><dt>Remote effects</dt><dd>none</dd></div>
          </dl>

          <fieldset>
            <legend>Eligible current review-receipt paths</legend>
            {inspection.candidates.length ? inspection.candidates.map((candidate) => (
              <label key={candidate.path} className="git-handoff-candidate">
                <input
                  type="checkbox"
                  checked={selectedPaths.includes(candidate.path)}
                  onChange={() => togglePath(candidate.path)}
                  disabled={busy}
                />
                <code>{candidate.status}</code>
                <span>{candidate.path}</span>
              </label>
            )) : <p>No eligible changed paths are available.</p>}
          </fieldset>

          {inspection.exclusions.length > 0 && (
            <details className="git-handoff-exclusions">
              <summary>{inspection.exclusions.length} excluded current or stale path(s)</summary>
              {inspection.exclusions.map((item) => (
                <p key={`${item.code}-${item.path}`}>
                  <code>{item.status}</code> <strong>{item.path}</strong> · {item.reason}
                </p>
              ))}
            </details>
          )}

          <label className="git-handoff-message">
            <span>Reviewed commit message</span>
            <input
              value={message}
              onChange={(event) => {
                setMessage(event.target.value);
                setConfirmation(null);
                setResult(null);
              }}
              maxLength={512}
              placeholder="Commit reviewed task result"
              disabled={busy}
            />
          </label>

          <div className="git-handoff-buttons">
            <button
              className="quiet-action"
              onClick={() => void previewSelection()}
              disabled={busy || selectedPaths.length === 0 || message.trim() !== message || !message}
            >
              Preview selected local commit
            </button>
            <button className="quiet-action" onClick={() => void inspect()} disabled={busy}>
              Refresh Git state
            </button>
          </div>
        </>
      )}

      {confirmation?.previewToken && !result && (
        <section className="git-handoff-confirmation" aria-label="Local commit confirmation">
          <span className="eyebrow"><ShieldCheck size={13} /> Exact mutation preview</span>
          {confirmation.stagedEffects.map((effect) => <p key={effect}>{effect}</p>)}
          <p>Token expires at {new Date(confirmation.expiresAtMs ?? 0).toLocaleTimeString()} and can be used once.</p>
          <button className="quiet-action" onClick={() => void commit()} disabled={busy}>
            {busy ? "Creating local commit…" : "Confirm and create local commit"}
          </button>
          <button className="quiet-action" onClick={() => setConfirmation(null)} disabled={busy}>
            Cancel commit
          </button>
        </section>
      )}

      {result && (
        <section className={`git-handoff-result ${result.success ? "is-success" : "is-failure"}`} role="status">
          <strong>{result.success ? <><Check size={14} /> Local commit verified</> : `Local commit outcome: ${result.outcome}`}</strong>
          <p>New HEAD: <code>{result.newHead ?? "unverified"}</code></p>
          <p>Committed paths: {result.committedPaths.length ? result.committedPaths.join(", ") : "none verified"}</p>
          {result.stagedPaths.length > 0 && <p>Paths still staged: {result.stagedPaths.join(", ")}</p>}
          {result.repair && <p>{result.repair.message} {result.repair.nextAction}</p>}
          <p>Remote effects: none. Repository completion remains unchanged.</p>
        </section>
      )}

      {error && <p className="git-handoff-error" role="alert">{error}</p>}
    </section>
  );
}
