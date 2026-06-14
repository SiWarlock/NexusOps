import { ArrowRight, CheckCheck, CircleAlert, X } from "lucide-react";
import { Button, IconButton, RiskBadge } from "../design-system/kit";
import type { ApprovalQueueRow, SessionRow } from "../contracts/index";
import { approvalDisplayFixture, sessionDisplayFixture } from "../shell/display-meta";
import { Eyebrow } from "../views/cockpit";
import { Overlay } from "./Overlay";

/**
 * The Human Input Queue drawer (⌘⇧H — ported from kit-tasks.jsx
 * HumanInputQueue): the centralized triage surface over the REAL queue —
 * pending approvals (ApprovalQueue projection) + sessions waiting on the human
 * (Session projection), grouped. "Open" navigates to the matching surface
 * (live); Approve/Deny/Defer are Gateway mutations — disabled until the intent
 * seam lands (§11.6). Risk/actor lines ride the approval display side-map
 * (projection enrichment flagged).
 */
export function HumanInputQueue({
  approvals,
  waiting,
  onClose,
  onOpenApproval,
  onOpenSession,
}: {
  approvals: ApprovalQueueRow[];
  waiting: SessionRow[];
  onClose: () => void;
  onOpenApproval: (a: ApprovalQueueRow) => void;
  onOpenSession: (s: SessionRow) => void;
}) {
  const count = approvals.length + waiting.length;
  return (
    <Overlay onClose={onClose} align="right" width={420} label="Human Input queue">
      <div
        style={{
          width: "100%",
          height: "100%",
          background: "var(--surface-panel)",
          borderLeft: "1px solid var(--border-strong)",
          boxShadow: "var(--elev-4)",
          display: "flex",
          flexDirection: "column",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 9,
            padding: "12px 14px",
            borderBottom: "1px solid var(--border-default)",
          }}
        >
          <span aria-hidden="true" style={{ color: "var(--attention-ink)", display: "inline-flex" }}>
            <CircleAlert size={16} />
          </span>
          <span style={{ font: "var(--fw-semibold) var(--fs-sub) var(--font-sans)" }}>Human Input</span>
          <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)" }}>
            {count} waiting
          </span>
          <span style={{ marginLeft: "auto" }}>
            <IconButton label="Close" size="sm" onClick={onClose}>
              <X size={15} />
            </IconButton>
          </span>
        </div>
        <div
          style={{ flex: 1, overflowY: "auto", padding: "12px 12px 16px", display: "flex", flexDirection: "column", gap: 14 }}
          data-testid="hiq-drawer"
        >
          {count === 0 ? (
            <div
              style={{
                font: "var(--fs-meta) var(--font-sans)",
                color: "var(--text-faint)",
                display: "flex",
                alignItems: "center",
                gap: 6,
                padding: "8px 2px",
              }}
            >
              <CheckCheck size={14} aria-hidden="true" style={{ color: "var(--success-ink)" }} />
              Queue clear — nothing waiting on you.
            </div>
          ) : (
            <>
              {approvals.length > 0 ? (
                <div>
                  <Eyebrow style={{ marginBottom: 8 }}>Permission requests · {approvals.length}</Eyebrow>
                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    {approvals.map((a) => {
                      const meta = approvalDisplayFixture[a.approval_id];
                      return (
                        <div
                          key={a.approval_id}
                          data-item-id={`Approval:${a.approval_id}`}
                          style={{
                            border: "1px solid var(--attention-line)",
                            background: "var(--attention-surface)",
                            borderRadius: "var(--r-2)",
                            padding: "10px 11px",
                            display: "flex",
                            flexDirection: "column",
                            gap: 8,
                          }}
                        >
                          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                            {meta?.risk ? <RiskBadge level={meta.risk} /> : null}
                            {meta?.who ? (
                              <span style={{ marginLeft: "auto", font: "var(--fs-micro) var(--font-mono)", color: "var(--text-muted)" }}>
                                {meta.who}
                              </span>
                            ) : null}
                          </div>
                          <div style={{ font: "var(--fw-medium) var(--fs-label)/1.35 var(--font-sans)", color: "var(--text-primary)" }}>
                            {a.preview_summary ?? a.approval_id}
                          </div>
                          <div style={{ display: "flex", gap: 6 }}>
                            <Button
                              variant="secondary"
                              size="sm"
                              icon={<ArrowRight size={12} />}
                              onClick={() => onOpenApproval(a)}
                            >
                              Review
                            </Button>
                            <span title="Resolution is a Gateway mutation (intent seam — daemon-gated)">
                              <Button variant="ghost" size="sm" disabled>
                                Deny
                              </Button>
                            </span>
                            <span title="Defer arrives with the queue contract (intent seam — daemon-gated)">
                              <Button variant="ghost" size="sm" disabled>
                                Defer
                              </Button>
                            </span>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ) : null}
              {waiting.length > 0 ? (
                <div>
                  <Eyebrow style={{ marginBottom: 8 }}>Sessions waiting on you · {waiting.length}</Eyebrow>
                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    {waiting.map((s) => {
                      const display = sessionDisplayFixture[s.session_id];
                      return (
                        <div
                          key={s.session_id}
                          data-item-id={`Session:${s.session_id}`}
                          style={{
                            border: "1px solid var(--attention-line)",
                            background: "var(--attention-surface)",
                            borderRadius: "var(--r-2)",
                            padding: "10px 11px",
                            display: "flex",
                            flexDirection: "column",
                            gap: 8,
                          }}
                        >
                          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                            {s.status === "waiting_on_permission" ? <RiskBadge level="medium" /> : null}
                            {display?.profile ? (
                              <span style={{ marginLeft: "auto", font: "var(--fs-micro) var(--font-mono)", color: "var(--text-muted)" }}>
                                {display.profile}
                              </span>
                            ) : null}
                          </div>
                          <div style={{ font: "var(--fw-medium) var(--fs-label)/1.35 var(--font-sans)", color: "var(--text-primary)" }}>
                            {s.title ?? s.session_id}
                          </div>
                          <div style={{ font: "var(--fs-micro) var(--font-mono)", color: "var(--text-muted)" }}>
                            {display?.current ?? s.status}
                          </div>
                          <div style={{ display: "flex", gap: 6 }}>
                            <Button
                              variant="secondary"
                              size="sm"
                              icon={<ArrowRight size={12} />}
                              onClick={() => onOpenSession(s)}
                            >
                              Open session
                            </Button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ) : null}
            </>
          )}
        </div>
      </div>
    </Overlay>
  );
}
