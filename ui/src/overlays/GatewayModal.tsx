import { Check, FolderGit2, ShieldCheck, ShieldOff, Terminal, X } from "lucide-react";
import { Button, IconButton, RiskBadge } from "../design-system/kit";
import { StatusPill } from "../status/StatusPill";
import type { ApprovalQueueRow } from "../contracts/index";
import { approvalDisplayFixture } from "../shell/display-meta";
import { Eyebrow } from "../views/cockpit";
import { Overlay } from "./Overlay";

/**
 * The Action Gateway approval modal (ported from kit-overlays.jsx
 * GatewayModal): the REAL approval row (title + frozen status pill) with the
 * requesting-actor line + risk from the approval display side-map. The
 * consequence preview ("what will happen") needs the daemon's ActionPlan
 * preview contract — an honest pending note, never invented consequences
 * (forbidden #2). Approve/Deny/always-allow are the FIRST mutation path —
 * disabled until the intent seam lands (§11.6; INV-SEC-1 enforcement stays
 * daemon-side).
 */
export function GatewayModal({
  approval,
  onClose,
}: {
  approval: ApprovalQueueRow;
  onClose: () => void;
}) {
  const meta = approvalDisplayFixture[approval.approval_id];
  return (
    <Overlay onClose={onClose} align="center" width={520} label="Action Gateway">
      <div
        style={{
          background: "var(--surface-card)",
          border: "1px solid var(--border-strong)",
          borderRadius: "var(--r-4)",
          boxShadow: "var(--elev-4)",
          overflow: "hidden",
        }}
        data-testid="gateway-modal"
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 9,
            padding: "13px 16px",
            borderBottom: "1px solid var(--border-default)",
          }}
        >
          <span aria-hidden="true" style={{ color: "var(--accent-ink)", display: "inline-flex" }}>
            <ShieldCheck size={16} />
          </span>
          <span style={{ font: "var(--fw-semibold) var(--fs-sub) var(--font-sans)" }}>Action Gateway</span>
          {meta?.risk ? <RiskBadge level={meta.risk} style={{ marginLeft: 4 }} /> : null}
          <span style={{ marginLeft: "auto" }}>
            <IconButton label="Close" onClick={onClose}>
              <X size={15} />
            </IconButton>
          </span>
        </div>
        <div style={{ padding: 16 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
            <StatusPill machine="Approval" status={approval.status} />
            {meta?.who ? (
              <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)" }}>
                {meta.who}
              </span>
            ) : null}
          </div>
          <div style={{ font: "var(--fw-medium) var(--fs-body-lg)/1.4 var(--font-sans)", marginBottom: 6 }}>
            <code
              style={{
                font: "var(--fs-body) var(--font-mono)",
                background: "var(--surface-sunken)",
                padding: "2px 6px",
                borderRadius: 4,
                color: "var(--accent-ink)",
              }}
            >
              {approval.title ?? approval.approval_id}
            </code>
          </div>
          <div
            style={{
              background: "var(--surface-sunken)",
              borderRadius: "var(--r-2)",
              padding: "11px 12px",
              boxShadow: "var(--elev-inset)",
              marginTop: 14,
            }}
          >
            <Eyebrow style={{ marginBottom: 8 }}>What will happen</Eyebrow>
            <div
              data-testid="gateway-conseq-pending"
              style={{ display: "flex", flexDirection: "column", gap: 4, font: "var(--fs-label)/1.5 var(--font-sans)", color: "var(--text-muted)" }}
            >
              <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                <Terminal size={14} aria-hidden="true" style={{ color: "var(--text-faint)" }} />
                The consequence preview arrives with the daemon ActionPlan
              </span>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                <FolderGit2 size={14} aria-hidden="true" style={{ color: "var(--text-faint)" }} />
                preview contract — listed from the dry-run, never invented.
              </span>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                <ShieldOff size={14} aria-hidden="true" style={{ color: "var(--text-faint)" }} />
                No mutation can fire from this build (read-only client).
              </span>
            </div>
          </div>
        </div>
        <div
          style={{
            padding: "12px 16px",
            borderTop: "1px solid var(--border-default)",
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: 7,
              font: "var(--fs-label) var(--font-sans)",
              color: "var(--text-faint)",
              marginRight: "auto",
            }}
            title="Standing permissions arrive with the policy engine (daemon Phase 2)"
          >
            <input type="checkbox" disabled style={{ accentColor: "var(--accent-solid)" }} /> Always
            allow in this project
          </label>
          <span title="Deny is a Gateway mutation (intent seam — daemon-gated)">
            <Button variant="ghost" size="md" disabled>
              Deny
            </Button>
          </span>
          <span title="Approve is a Gateway mutation (intent seam — daemon-gated)">
            <Button variant="secondary" size="md" icon={<Check size={14} />} disabled>
              Approve once
            </Button>
          </span>
        </div>
      </div>
    </Overlay>
  );
}
