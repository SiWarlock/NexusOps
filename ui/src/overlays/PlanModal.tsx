import { useState } from "react";
import { Check, ShieldCheck, X } from "lucide-react";
import { Button, IconButton } from "../design-system/kit";
import { StatusPill } from "../status/StatusPill";
import type { ActionAck, ActionPlan, ActionPlanStep, Approval } from "../contracts/index";
import type { GatewayPort } from "../gateway-client/types";
import { useSubmitIntent, type IntentResult } from "../intent/submit-intent";
import { useCanSubmitIntent } from "../connection/read-only";
import { Overlay } from "./Overlay";
import { ResultNotice } from "./GatewayModal";

/**
 * The N-step ActionPlan approval modal (§11.5 Gateway Review Modal; O-3). A PURE RENDERER of the
 * daemon's frozen §6.2 `ActionPlan` (1..N steps) + plan-level `Approval` — it derives NO risk /
 * eligibility of its own (LESSON 17): the header shows the daemon's plan-level `overall_risk`, each
 * step shows the daemon's catalog-authoritative per-step `action_request.risk_level` (honest-absent
 * "—" when not present, never a fabricated digit — forbidden #2), and per-step `policy_decision` is
 * daemon-NULL today (8.1 follow-on) → an honest "pending" surface, never a synthesized consequence.
 *
 * Mutation discipline (cat-1, INV-SEC-1): every approve/deny SUBMITS an intent through the existing,
 * user-signed-off L2-C seam (`approve(approval_id, step_id?)` / `deny(approval_id, reason)`) — the
 * daemon Gateway is the real chokepoint, the `canSubmitIntent && mutationsEnabled` gate is
 * defense-in-depth. NO new gate / NO new go-live (Q5; the approve/deny path IS the human control that
 * ENFORCES #10 "Brain proposes, never executes" — the gated capability is the *submitter*). "Approve
 * all eligible" submits the PLAN-LEVEL approve (no step_id) — the daemon owns critical-exclusion
 * (2.1c L3), the UI computes no eligibility. The post-action surface reuses `ResultNotice`:
 * daemon-status-never-"done" (Q3) + each §6.4 reject routed to its distinct §11.5 card
 * (`fencing_conflict` never re-approvable, #6). The single-action `GatewayModal` (the N=1 case) is
 * unchanged. The `policy_grant` "save as policy" standing-grant stays disabled-pinned (own cat-1).
 */
export function PlanModal({
  plan,
  approval,
  port,
  onClose,
}: {
  plan: ActionPlan;
  approval: Approval;
  port: GatewayPort;
  onClose: () => void;
}) {
  const seam = useSubmitIntent(port);
  // The SAME effective gate as the single-action modal (L2-B/056) — no new go-live: a live link AND
  // the L2 enable. While false the controls stay disabled + the seam returns readOnly (never an
  // enabled-button-that-throws, §11.7); the daemon Gateway is the INV-SEC-1 chokepoint regardless.
  const canSubmit = useCanSubmitIntent() && port.mutationsEnabled;
  const [result, setResult] = useState<IntentResult<ActionAck> | null>(null);
  const [denyReason, setDenyReason] = useState("");
  const [submitting, setSubmitting] = useState(false);

  // Approve-all: the PLAN-LEVEL approve (NO step_id) — the daemon owns which steps are eligible
  // (critical-exclusion is catalog-authoritative, 2.1c L3); the UI never computes eligibility.
  async function approveAll() {
    setSubmitting(true);
    setResult(await seam.approve(approval));
    setSubmitting(false);
  }
  // Per-step approve: carries the step_id over the §6.1 wire.
  async function approveStep(stepId: string) {
    setSubmitting(true);
    setResult(await seam.approve(approval, stepId));
    setSubmitting(false);
  }
  // Deny: plan-level, with the reason trimmed so a whitespace-only value never becomes the recorded
  // audit reason (LESSON 33; defense-in-depth — the daemon Gateway is the real reason-content guard).
  async function deny() {
    setSubmitting(true);
    setResult(await seam.deny(approval, denyReason.trim() || "Denied by the operator"));
    setSubmitting(false);
  }

  return (
    <Overlay onClose={onClose} align="center" width={640} label="Action plan review">
      <div style={CARD} data-testid="plan-modal">
        <div style={HEADER}>
          <span aria-hidden="true" style={{ color: "var(--accent-ink)", display: "inline-flex" }}>
            <ShieldCheck size={16} />
          </span>
          <span style={{ font: "var(--fw-semibold) var(--fs-sub) var(--font-sans)" }}>Action plan</span>
          {/* The daemon's PLAN-LEVEL overall_risk — never recomputed from the steps (LESSON 17). */}
          <span
            data-testid="plan-risk"
            style={{ marginLeft: 6, font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)" }}
          >
            risk {plan.overall_risk}/4
          </span>
          <span style={{ marginLeft: "auto" }}>
            <IconButton label="Close" onClick={onClose}>
              <X size={15} />
            </IconButton>
          </span>
        </div>

        <div style={{ padding: 16 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6, flexWrap: "wrap" }}>
            <StatusPill machine="Approval" status={approval.status} />
            <strong data-testid="plan-title" style={{ font: "var(--fw-semibold) var(--fs-label) var(--font-sans)" }}>
              {plan.title}
            </strong>
          </div>
          <div
            style={{
              display: "flex",
              gap: 12,
              marginBottom: 12,
              font: "var(--fs-meta) var(--font-mono)",
              color: "var(--text-faint)",
              flexWrap: "wrap",
            }}
          >
            <code data-testid="plan-id" style={CODE}>{plan.plan_id}</code>
            <span data-testid="plan-approval-mode">mode: {plan.approval_mode}</span>
            <span>{plan.steps.length} steps</span>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {plan.steps.map((step, i) => (
              <StepRow
                key={step.step_id}
                index={i}
                step={step}
                canSubmit={canSubmit}
                submitting={submitting}
                // Once a plan-level decision lands the whole surface is read-only — the per-step
                // approve must NOT offer a second mutation against the same approval (the footer
                // already switches to Close; the rows match).
                locked={result !== null}
                onApprove={() => void approveStep(step.step_id)}
              />
            ))}
          </div>

          {/* Post-action: the daemon-reported status / the distinct reject card — never optimistic
              "done" (reused from the single-action modal — one source for §6.4 routing). */}
          {result ? <ResultNotice result={result} onReapprove={() => setResult(null)} /> : null}
        </div>

        <div style={FOOTER}>
          <label style={POLICY_GRANT} title="A standing grant is a separate, cat-1-gated slice — disabled here">
            {/* Deferred: the policy_grant standing-grant has its own cat-1 checkpoint (mirrors the
                single-action always-allow). */}
            <input type="checkbox" disabled data-testid="plan-policy-grant" style={{ accentColor: "var(--accent-solid)" }} />{" "}
            Save as policy for this project
          </label>
          {!result ? (
            <>
              <input
                aria-label="Deny reason"
                data-testid="plan-deny-reason"
                value={denyReason}
                onChange={(e) => setDenyReason(e.target.value)}
                placeholder="reason (optional)"
                style={REASON_INPUT}
              />
              <Button
                variant="ghost"
                size="md"
                data-testid="plan-deny"
                disabled={!canSubmit || submitting}
                onClick={() => void deny()}
              >
                Deny plan
              </Button>
              <Button
                variant="secondary"
                size="md"
                data-testid="approve-all"
                icon={<Check size={14} />}
                disabled={!canSubmit || submitting}
                onClick={() => void approveAll()}
              >
                Approve all eligible
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

/** One step row: index, status, label, the daemon's per-step risk (honest-absent "—"), action_type,
 *  target (resource_refs ids), required/skippable flags, honest-absent per-step policy, and a per-step
 *  Approve control (gated identically to the plan-level controls). */
function StepRow({
  index,
  step,
  canSubmit,
  submitting,
  locked,
  onApprove,
}: {
  index: number;
  step: ActionPlanStep;
  canSubmit: boolean;
  submitting: boolean;
  /** A plan-level result has landed → the row is read-only (no second per-step mutation). */
  locked: boolean;
  onApprove: () => void;
}) {
  return (
    <div data-testid="plan-step" data-step-id={step.step_id} style={STEP_ROW}>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={STEP_INDEX}>{index + 1}</span>
        <StatusPill machine="ActionRequest" status={step.status} />
        <span data-testid="step-label" style={{ font: "var(--fw-semibold) var(--fs-label) var(--font-sans)" }}>
          {step.label}
        </span>
        {/* The daemon's catalog-authoritative per-step risk — honest-absent "—", never a fabricated
            digit (forbidden #2 / LESSON 17). Per-step live preview_action is a deferred follow-on. */}
        <span
          data-testid="step-risk"
          style={{ marginLeft: "auto", font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)" }}
        >
          risk {stepRisk(step)}
        </span>
      </div>
      <div style={STEP_META}>
        <code data-testid="step-action-type" style={CODE}>{step.action_request.action_type}</code>
        <span data-testid="step-target">→ {stepTarget(step)}</span>
        <span data-testid="step-flags">
          {step.required ? "required" : "optional"}
          {step.can_skip ? " · skippable" : ""}
        </span>
        {/* Per-step policy_decision is daemon-NULL today (8.1 follow-on) → honest pending, never a
            synthesized consequence (forbidden #2). */}
        <span data-testid="step-policy" style={{ color: "var(--text-faint)" }}>policy: pending</span>
        <Button
          variant="ghost"
          size="sm"
          data-testid="approve-step"
          disabled={!canSubmit || submitting || locked}
          onClick={onApprove}
          style={{ marginLeft: "auto" }}
        >
          Approve step
        </Button>
      </div>
    </div>
  );
}

/** The daemon's per-step risk_level (integer 0–4), or "—" when absent (never an invented digit). */
function stepRisk(step: ActionPlanStep): string {
  const r = step.action_request?.risk_level;
  return typeof r === "number" ? String(r) : "—";
}

/** The step's target resource ids (verbatim from the daemon's action_request), or "—". */
function stepTarget(step: ActionPlanStep): string {
  const ids = step.action_request?.resource_refs?.map((ref) => ref.id) ?? [];
  return ids.length ? ids.join(", ") : "—";
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
const STEP_ROW: React.CSSProperties = {
  background: "var(--surface-sunken)",
  borderRadius: "var(--r-2)",
  padding: "10px 12px",
  boxShadow: "var(--elev-inset)",
  display: "flex",
  flexDirection: "column",
  gap: 4,
};
const STEP_META: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 10,
  font: "var(--fs-meta) var(--font-mono)",
  color: "var(--text-faint)",
  flexWrap: "wrap",
};
const STEP_INDEX: React.CSSProperties = {
  font: "var(--fw-semibold) var(--fs-meta) var(--font-mono)",
  color: "var(--text-muted)",
  minWidth: 16,
  textAlign: "center",
};
const FOOTER: React.CSSProperties = {
  padding: "12px 16px",
  borderTop: "1px solid var(--border-default)",
  display: "flex",
  alignItems: "center",
  gap: 8,
};
const POLICY_GRANT: React.CSSProperties = {
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
