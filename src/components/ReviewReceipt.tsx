import { AlertTriangle, Check, ChevronRight, FileDiff, ShieldCheck, X } from "lucide-react";
import type { ReviewReceipt as ReviewReceiptModel } from "../lib/review-receipt";

export function ReviewReceipt({
  receipt,
  decision,
  onDecision,
  onContinue,
  onStop,
}: {
  receipt: ReviewReceiptModel;
  decision: "handoff" | "revision" | null;
  onDecision: (decision: "handoff" | "revision") => void;
  onContinue: (() => void) | null;
  onStop: () => void;
}) {
  return (
    <section className={`review-receipt tone-${receipt.tone}`} aria-label="Post-run review receipt">
      <header className="review-outcome">
        <span className="eyebrow">Repository-backed outcome</span>
        <div>
          <span className="review-outcome-icon">
            {receipt.tone === "completed" ? <Check size={18} /> : <AlertTriangle size={18} />}
          </span>
          <div>
            <h2>{receipt.headline}</h2>
            <p>{receipt.reason}</p>
          </div>
        </div>
        <p>
          <strong>{receipt.selectedTask ?? "No task selected"}</strong>
          {" · "}repository verified: {String(receipt.repositoryVerified)}
        </p>
      </header>

      <div className="review-grid">
        <section className="review-card review-changes">
          <span className="eyebrow"><FileDiff size={13} /> Current repository changes</span>
          <p>{receipt.changeScopeNote}</p>
          {receipt.changeEvidenceUnavailable && (
            <p className="review-unavailable">{receipt.changeEvidenceUnavailable}</p>
          )}
          {receipt.changedFiles.length ? receipt.changedFiles.map((file) => (
            <details key={`${file.status}-${file.path}`} className="review-file">
              <summary>
                <code>{file.status}</code>
                <strong>{file.path}</strong>
                <span>{file.diff ? "text diff" : "unavailable"}</span>
              </summary>
              {file.diff
                ? <pre>{file.diff}{file.truncated ? "\n[diff truncated]" : ""}</pre>
                : <p>{file.diffUnavailableReason ?? "Textual diff unavailable."}</p>}
            </details>
          )) : <p>No changed paths were reported.</p>}
        </section>

        <section className="review-card">
          <span className="eyebrow">Acceptance evidence</span>
          {receipt.criteria.length ? (
            <ul className="review-list">
              {receipt.criteria.map((criterion) => (
                <li key={criterion.text} className={criterion.passed ? "is-pass" : "is-open"}>
                  {criterion.passed ? <Check size={14} /> : <X size={14} />}
                  <span>{criterion.text}</span>
                </li>
              ))}
            </ul>
          ) : <p>Acceptance criteria evidence unavailable.</p>}
        </section>

        <section className="review-card">
          <span className="eyebrow">Commands and checks</span>
          {receipt.checks.length ? (
            <ul className="review-list">
              {receipt.checks.map((check) => (
                <li key={`${check.label}-${check.result}`} className={`is-${check.result}`}>
                  <ShieldCheck size={14} /><span>{check.label}</span><strong>{check.result}</strong>
                </li>
              ))}
            </ul>
          ) : <p>Structured command evidence unavailable; inspect raw task evidence.</p>}
        </section>

        <section className="review-card">
          <span className="eyebrow">Tracker and next resolver decision</span>
          <dl>
            <div><dt>Task status</dt><dd>{receipt.tracker.selectedTaskStatus}</dd></div>
            <div><dt>Loop state</dt><dd>{receipt.tracker.loopState}</dd></div>
            <div><dt>Next task</dt><dd>{receipt.tracker.nextTask ?? "none"}</dd></div>
          </dl>
          <p>{receipt.tracker.nextReason}</p>
        </section>

        <section className="review-card">
          <span className="eyebrow">Risks and follow-ups</span>
          {receipt.risks.length
            ? <ul>{receipt.risks.map((risk) => <li key={risk}>{risk}</li>)}</ul>
            : <p>No structured risks or follow-ups were recorded.</p>}
        </section>

        {receipt.shared && (
          <section className="review-card review-shared">
            <span className="eyebrow">Optional shared result</span>
            <dl>
              <div><dt>Access</dt><dd>{receipt.shared.access}</dd></div>
              <div><dt>Source binding</dt><dd>{receipt.shared.sourceTask}</dd></div>
              <div><dt>Source hash</dt><dd>{receipt.shared.sourceHash}</dd></div>
              <div><dt>Claim</dt><dd>{receipt.shared.claim}</dd></div>
              <div><dt>Evidence / handoff</dt><dd>{receipt.shared.completion}</dd></div>
              <div><dt>Repair state</dt><dd>{receipt.shared.repairState}</dd></div>
              <div><dt>Codex started</dt><dd>{String(receipt.shared.codexStarted)}</dd></div>
            </dl>
          </section>
        )}
      </div>

      <section className="review-actions" aria-label="Review decisions">
        <div>
          <span className="eyebrow">Explicit review decisions</span>
          <p>These choices record UI review intent only. They do not revert, stage, commit, push, publish, or rerun Codex.</p>
        </div>
        <div>
          <button className="quiet-action" onClick={() => onDecision("handoff")}>Accept for handoff</button>
          <button className="quiet-action" onClick={() => onDecision("revision")}>Request revision</button>
          {onContinue && <button className="quiet-action" onClick={onContinue}>Review next task <ChevronRight size={13} /></button>}
          <button className="quiet-action" onClick={onStop}>Stop review</button>
        </div>
        {decision && <p role="status">Review intent recorded: {decision === "handoff" ? "accepted for a separate handoff action" : "revision requested; no repository effect"}.</p>}
      </section>

      <details className="review-raw">
        <summary>Raw normalized runtime events</summary>
        {receipt.rawEvents.length
          ? receipt.rawEvents.map((event) => <p key={event.sequence}><code>{event.sequence} · {event.kind}</code> {event.summary}</p>)
          : <p>No normalized runtime events were retained.</p>}
      </details>
    </section>
  );
}
