import { useEffect, useState } from "react";
import { Check, ShieldCheck, X } from "lucide-react";
import { Button, IconButton } from "../design-system/kit";
import { StatusPill } from "../status/StatusPill";
import type {
  ActionAck,
  ActionPreview,
  Approval,
  PolicyDecision,
} from "../contracts/index";
import type { GatewayPort } from "../gateway-client/types";
import { createIntentSeam, useSubmitIntent, type IntentResult } from "../intent/submit-intent";
import { useCanSubmitIntent } from "../connection/read-only";
import { describeRejection } from "../safety/model";
import { Eyebrow } from "../views/cockpit";
import { Overlay } from "./Overlay";

/**
 * The Action Gateway approval modal — the live, daemon-driven human-approval card
 * (slice 2 of the intent seam). INV-SEC-1: it SUBMITS intents only (Approve/Deny go
 * through the 043 `useSubmitIntent` seam → the single `GatewayPort`); the daemon
 * Gateway is the real chokepoint, the UI gate is defense-in-depth. The card renders
 * the DAEMON's `PolicyDecision` + `ActionPreview` (never UI-derived risk, never
 * invented consequences — forbidden #2); the post-action render shows the
 * daemon-reported `ActionAck.status` (never an optimistic "done"); a rejection routes
 * each §6.4 code to its distinct §11.5 treatment (`fencing_conflict` is never
 * re-approvable — #6). It holds NO intent state (no cache/retry over the stateless seam).
 */
export function GatewayModal({
  approval,
  policyDecision,
  port,
  onClose,
}: {
  approval: Approval;
  policyDecision: PolicyDecision;
  port: GatewayPort;
  onClose: () => void;
}) {
  const seam = useSubmitIntent(port);
  // The effective mutation-submit gate (056/L2-B): a live link AND the L2 go-live enable. While
  // `mutationsEnabled` is false (the L2-B honest disabled state), approve/deny stay disabled + the
  // seam returns readOnly — the controls never offer an enabled-button-that-throws (§11.7).
  const canSubmit = useCanSubmitIntent() && port.mutationsEnabled;
  const [preview, setPreview] = useState<IntentResult<ActionPreview> | null>(null);
  const [result, setResult] = useState<IntentResult<ActionAck> | null>(null);
  const [denyReason, setDenyReason] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [previewFailed, setPreviewFailed] = useState(false);
  const arId = approval.action_request_id ?? undefined;

  // Fetch the daemon's ActionPreview on open. No optimism: while pending (or when the
  // fail-safe gate refuses in degraded mode) we render an HONEST pending note, never a
  // blank or a synthesized consequence (forbidden #2 / §11.7). A thrown (non-WireError)
  // error keeps the honest-pending contract via `.catch` — never an unhandled rejection.
  // Keyed on [port, arId, canSubmit] (NOT the per-render `seam` identity) so a no-op
  // status change doesn't churn the fetch; a real gate flip (recovery) re-fetches once.
  useEffect(() => {
    if (!arId) return;
    let active = true;
    setPreviewFailed(false);
    void createIntentSeam(port, () => canSubmit)
      .previewAction(arId)
      .then((r) => {
        if (active) setPreview(r);
      })
      .catch(() => {
        if (active) setPreviewFailed(true);
      });
    return () => {
      active = false;
    };
  }, [port, arId, canSubmit]);

  // Re-approvable (precondition_stale): clear the stale result + regenerate the preview,
  // then require a fresh approval — NEVER an auto-replay (Q7; the daemon re-validates).
  function refreshPreview() {
    setResult(null);
    setPreview(null);
    setPreviewFailed(false);
    if (arId)
      void createIntentSeam(port, () => canSubmit)
        .previewAction(arId)
        .then(setPreview)
        .catch(() => setPreviewFailed(true));
  }

  async function submit(kind: "approve" | "deny") {
    setSubmitting(true);
    const r =
      kind === "approve"
        ? await seam.approve(approval)
        : // defense-in-depth: trim so a whitespace-only reason can never become the recorded
          // audit reason — the explicit default rides instead (the daemon Gateway is the chokepoint).
          await seam.deny(approval, denyReason.trim() || "Denied by the operator");
    setResult(r);
    setSubmitting(false);
  }

  return (
    <Overlay onClose={onClose} align="center" width={520} label="Action Gateway">
      <div style={CARD} data-testid="gateway-modal">
        <div style={HEADER}>
          <span aria-hidden="true" style={{ color: "var(--accent-ink)", display: "inline-flex" }}>
            <ShieldCheck size={16} />
          </span>
          <span style={{ font: "var(--fw-semibold) var(--fs-sub) var(--font-sans)" }}>Action Gateway</span>
          <span
            data-testid="gateway-risk"
            style={{ marginLeft: 6, font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)" }}
          >
            risk {previewRisk(preview)}/4
          </span>
          <span style={{ marginLeft: "auto" }}>
            <IconButton label="Close" onClick={onClose}>
              <X size={15} />
            </IconButton>
          </span>
        </div>

        <div style={{ padding: 16 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
            <StatusPill machine="Approval" status={approval.status} />
            <code style={CODE}>{approval.approval_id}</code>
          </div>

          {/* Q4 — the requirement is READ from the daemon's PolicyDecision, never UI-derived. */}
          <div data-testid="gateway-policy" style={SUNKEN}>
            <Eyebrow style={{ marginBottom: 8 }}>Policy decision</Eyebrow>
            <div style={{ font: "var(--fs-label)/1.5 var(--font-sans)", color: "var(--text-muted)" }}>
              <div>
                Decision: <strong data-testid="policy-status">{policyDecision.status}</strong>
              </div>
              <div data-testid="policy-requirement">
                Requires:{" "}
                {policyDecision.required_approvals.length
                  ? policyDecision.required_approvals.map((a) => a.kind).join(", ")
                  : "no further approval"}
              </div>
              {policyDecision.reasons.length ? (
                <div data-testid="policy-reasons">Reasons: {policyDecision.reasons.join("; ")}</div>
              ) : null}
              {policyDecision.safer_alt ? (
                <div data-testid="policy-safer-alt" style={{ color: "var(--text-faint)" }}>
                  Safer alternative (suggestion): {policyDecision.safer_alt}
                </div>
              ) : null}
            </div>
          </div>

          {/* Q5 — the daemon's ActionPreview consequences ONLY; honest pending otherwise. */}
          <div style={{ ...SUNKEN, marginTop: 14 }}>
            <Eyebrow style={{ marginBottom: 8 }}>What will happen</Eyebrow>
            <div data-testid="gateway-preview" style={{ font: "var(--fs-label)/1.5 var(--font-sans)", color: "var(--text-muted)" }}>
              {renderPreview(preview, Boolean(arId), previewFailed)}
            </div>
          </div>

          {/* Post-action: the daemon-reported status / the distinct reject card — never optimistic "done". */}
          {result ? <ResultNotice result={result} onReapprove={refreshPreview} /> : null}
        </div>

        <div style={FOOTER}>
          <label style={ALWAYS_ALLOW} title="A standing grant is a separate, cat-1-gated slice — disabled here">
            {/* Deferred: the policy_grant standing-grant has its own cat-1 checkpoint. */}
            <input type="checkbox" disabled data-testid="always-allow" style={{ accentColor: "var(--accent-solid)" }} />{" "}
            Always allow in this project
          </label>
          {!result ? (
            <>
              <input
                aria-label="Deny reason"
                data-testid="deny-reason"
                value={denyReason}
                onChange={(e) => setDenyReason(e.target.value)}
                placeholder="reason (optional)"
                style={REASON_INPUT}
              />
              <Button
                variant="ghost"
                size="md"
                disabled={!canSubmit || submitting}
                onClick={() => void submit("deny")}
              >
                Deny
              </Button>
              <Button
                variant="secondary"
                size="md"
                icon={<Check size={14} />}
                disabled={!canSubmit || submitting}
                onClick={() => void submit("approve")}
              >
                Approve once
              </Button>
            </>
          ) : (
            <Button variant="secondary" size="md" onClick={onClose}>
              Close
            </Button>
          )}
        </div>
      </div>
    </Overlay>
  );
}

/** The daemon's preview risk_level (integer 0–4), or "—" while pending (never invented). */
function previewRisk(preview: IntentResult<ActionPreview> | null): string {
  return preview && "ok" in preview ? String(preview.ok.risk_level) : "—";
}

function renderPreview(
  preview: IntentResult<ActionPreview> | null,
  hasTarget: boolean,
  failed: boolean,
) {
  // Distinct testid per state (§11.5 distinct surface; Lesson §11) — no-target / failed /
  // loading / degraded-readonly / daemon-error are NOT conflated.
  if (!hasTarget)
    return <span data-testid="preview-no-target">No linked action to preview yet.</span>;
  if (failed)
    return <span data-testid="preview-failed">Preview unavailable (the request failed).</span>;
  if (!preview) return <span data-testid="preview-loading">Loading the daemon’s preview…</span>;
  if ("readOnly" in preview)
    return <span data-testid="preview-readonly">Preview unavailable while the daemon is unreachable.</span>;
  if ("error" in preview)
    return <span data-testid="preview-error">Preview unavailable (the daemon reported {preview.error.code}).</span>;
  const p = preview.ok;
  if (p.cannot_preview_reason)
    return <span data-testid="preview-cannot">No preview: {p.cannot_preview_reason}</span>;
  return (
    <div data-testid="preview-ready" style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <span>{p.summary}</span>
      {p.changed_resources.length ? (
        <span style={{ color: "var(--text-faint)" }}>
          Changes: {p.changed_resources.map((r) => r.id).join(", ")}
        </span>
      ) : null}
    </div>
  );
}

/** The post-action surface. `ok` → the daemon-reported status (NEVER "done"); `error`
 *  → the distinct §11.5 reject card; `readOnly` → an honest no-mutation note.
 *  Exported for the cat-1 result-branch pins (no-optimism / reject-routing / readonly). */
export function ResultNotice({
  result,
  onReapprove,
}: {
  result: IntentResult<ActionAck>;
  onReapprove: () => void;
}) {
  if ("readOnly" in result) {
    return (
      <div data-testid="result-readonly" style={{ ...SUNKEN, marginTop: 14, color: "var(--text-muted)" }}>
        Not submitted — the daemon is unreachable or version-incompatible (read-only).
      </div>
    );
  }
  if ("ok" in result) {
    // Q3 — the daemon-reported decision status, verbatim. NEVER "done"/"executed".
    return (
      <div data-testid="result-ok" style={{ ...SUNKEN, marginTop: 14 }}>
        Daemon recorded the decision: <strong data-testid="result-status">{result.ok.status}</strong>.
        {" "}A confirmed outcome will arrive from the audited projection.
      </div>
    );
  }
  const r = describeRejection(result.error);
  return (
    <div
      data-testid="result-reject"
      data-reject-kind={r.kind}
      style={{ ...SUNKEN, marginTop: 14, borderLeft: `3px solid var(--${r.severity === "critical" ? "danger" : "warn"}-solid)` }}
    >
      <div style={{ font: "var(--fw-semibold) var(--fs-label) var(--font-sans)" }}>
        <span aria-hidden="true">{r.glyph}</span> {r.label}{" "}
        <code style={{ ...CODE, marginLeft: 4 }} data-testid="reject-code">{r.code}</code>
      </div>
      <div style={{ font: "var(--fs-label)/1.5 var(--font-sans)", color: "var(--text-muted)", marginTop: 4 }}>
        {r.message}
      </div>
      {/* Re-approvable ONLY for precondition_stale — fencing/internal/denied/generic offer NO retry (#6). */}
      {r.reapprovable ? (
        <Button variant="ghost" size="sm" data-testid="reapprove" onClick={onReapprove} style={{ marginTop: 8 }}>
          Regenerate preview
        </Button>
      ) : null}
    </div>
  );
}

const CARD: React.CSSProperties = {
  background: "var(--surface-card)",
  border: "1px solid var(--border-strong)",
  borderRadius: "var(--r-4)",
  boxShadow: "var(--elev-4)",
  overflow: "hidden",
};
const HEADER: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 9,
  padding: "13px 16px",
  borderBottom: "1px solid var(--border-default)",
};
const SUNKEN: React.CSSProperties = {
  background: "var(--surface-sunken)",
  borderRadius: "var(--r-2)",
  padding: "11px 12px",
  boxShadow: "var(--elev-inset)",
};
const FOOTER: React.CSSProperties = {
  padding: "12px 16px",
  borderTop: "1px solid var(--border-default)",
  display: "flex",
  alignItems: "center",
  gap: 8,
};
const ALWAYS_ALLOW: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 7,
  font: "var(--fs-label) var(--font-sans)",
  color: "var(--text-faint)",
  marginRight: "auto",
};
const CODE: React.CSSProperties = {
  font: "var(--fs-meta) var(--font-mono)",
  background: "var(--surface-sunken)",
  padding: "2px 6px",
  borderRadius: 4,
  color: "var(--accent-ink)",
};
const REASON_INPUT: React.CSSProperties = {
  font: "var(--fs-label) var(--font-sans)",
  background: "var(--surface-sunken)",
  border: "1px solid var(--border-default)",
  borderRadius: 4,
  padding: "5px 8px",
  width: 130,
};
