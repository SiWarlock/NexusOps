import { ChevronRight, GitPullRequest, Pause } from "lucide-react";
import type { ProjectActivityRow, SessionRow, UsageRow } from "../../contracts/index";
import { Button, HarnessBadge, MetaChip, ProfileBadge } from "../../design-system/kit";
import { StatusPill } from "../../status/StatusPill";
import { describeStatus } from "../../status/descriptors";
import { RiskBadge } from "../../design-system/kit";
import { sessionDisplayFixture } from "../../shell/display-meta";
import { SessionsTable } from "../sessions/SessionsTable";
import { Eyebrow } from "../cockpit";

/**
 * The Session Terminal view (ported from kit-views2.jsx SessionTerminal).
 * The header (task › branch crumb, status pill + title, ownership chips) is
 * REAL session + display-side-map data; the terminal WELL is daemon-gated —
 * the §9.1 PTY/terminal-channel contract (6.3d/e) hasn't landed, so the well
 * is an honest placeholder (PTY is display-only by invariant #9; transcript
 * lines are never invented). The permission prompt renders (disabled) only
 * when the session genuinely waits on permission. With no session selected,
 * the view hosts the Sessions table (real projection data + filters) as the
 * picker — the table stays the structured/AT surface for sessions.
 */
export function SessionTerminal({
  session,
  sessions,
  projects,
}: {
  session: SessionRow | null;
  sessions: SessionRow[];
  projects: ProjectActivityRow[];
  usage?: UsageRow[];
}) {
  if (!session) {
    return (
      <section aria-label="Session Terminal" className="terminal-view" style={{ overflowY: "auto" }}>
        <div style={{ padding: "14px 16px 0" }}>
          <Eyebrow style={{ marginBottom: 8 }}>Pick a session</Eyebrow>
          <p style={{ margin: "0 0 10px", font: "var(--fs-meta) var(--font-sans)", color: "var(--text-muted)" }}>
            Use the sidebar workspace tree to open a session's terminal. The table
            below is the structured session list (filter + sort).
          </p>
        </div>
        <SessionsTable sessions={sessions} projects={projects} />
      </section>
    );
  }

  const descriptor = describeStatus("Session", session.status);
  const display = sessionDisplayFixture[session.session_id];
  const waitingPermission = session.status === "waiting_on_permission";

  return (
    <section
      aria-label="Session Terminal"
      style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--surface-canvas)" }}
    >
      {/* header — real session data + display side-map chips */}
      <div style={{ padding: "11px 16px", borderBottom: "1px solid var(--border-default)" }}>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            font: "var(--fs-meta) var(--font-sans)",
            color: "var(--text-faint)",
            marginBottom: 7,
          }}
        >
          <span>{display?.task?.id ?? "Session"}</span>
          <ChevronRight size={12} aria-hidden="true" />
          <span style={{ color: "var(--text-secondary)" }}>{display?.branch ?? session.session_id}</span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <StatusPill machine="Session" status={session.status} size="md" />
          <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)" }}>
            {session.title ?? session.session_id}
          </h1>
          <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
            <span title="PR navigation arrives with the integration contract (Phase 7)">
              <Button variant="ghost" size="sm" icon={<GitPullRequest size={14} />} disabled>
                Open PR
              </Button>
            </span>
            <span title="Pause is a session mutation (intent seam — daemon-gated)">
              <Button variant="secondary" size="sm" icon={<Pause size={14} />} disabled>
                Pause
              </Button>
            </span>
          </div>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 9, flexWrap: "wrap" }}>
          {display?.harness ? <HarnessBadge harness={display.harness} /> : null}
          {display?.profile ? (
            <ProfileBadge name={display.profile} provider={display.provider} />
          ) : null}
          {display?.branch ? <MetaChip tone="branch">{display.branch}</MetaChip> : null}
          {display?.worktree ? <MetaChip tone="worktree">{display.worktree}</MetaChip> : null}
          {display?.pr ? <MetaChip tone="pr">{display.pr}</MetaChip> : null}
        </div>
      </div>
      {/* terminal well — daemon-gated (§9.1 PTY display-only; 6.3d/e) */}
      <div
        style={{
          flex: 1,
          overflowY: "auto",
          padding: "12px 16px",
          background: "var(--surface-sunken)",
          boxShadow: "var(--elev-inset)",
          fontFamily: "var(--font-mono)",
          fontSize: "var(--fs-body)",
        }}
      >
        <div
          data-testid="terminal-well-pending"
          style={{ color: "var(--text-faint)", lineHeight: "22px" }}
        >
          <div>· terminal channel pending — the live PTY stream lands with the</div>
          <div>  daemon terminal contract (6.3d/e). Status above is real;</div>
          <div>  transcript lines are never invented.</div>
          {display?.current ? (
            <div style={{ marginTop: 12, color: "var(--text-muted)" }}>
              last reported activity: <span style={{ color: "var(--text-secondary)" }}>{display.current}</span>
            </div>
          ) : null}
        </div>
        {waitingPermission ? (
          <div
            style={{
              marginTop: 12,
              border: "1px solid var(--attention-line)",
              background: "var(--attention-surface)",
              borderRadius: "var(--r-3)",
              padding: "12px 13px",
              fontFamily: "var(--font-sans)",
            }}
            data-testid="terminal-permission-prompt"
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
              <RiskBadge level="medium" />
              <span style={{ font: "var(--fw-semibold) var(--fs-body) var(--font-sans)", color: "var(--attention-ink)" }}>
                Permission required
              </span>
            </div>
            <div style={{ font: "var(--fs-body)/1.5 var(--font-sans)", color: "var(--text-secondary)", marginBottom: 11 }}>
              {descriptor.label} — the session is blocked on a Gateway approval.
            </div>
            <div style={{ display: "flex", gap: 7 }}>
              <span title="Approve flows arrive with the Gateway overlay (intent seam — daemon-gated)">
                <Button variant="secondary" size="sm" disabled>
                  Approve once
                </Button>
              </span>
              <span title="Standing permissions arrive with the policy engine (daemon Phase 2)">
                <Button variant="secondary" size="sm" disabled>
                  Always allow
                </Button>
              </span>
              <span title="Deny arrives with the Gateway overlay (intent seam — daemon-gated)">
                <Button variant="ghost" size="sm" disabled>
                  Deny
                </Button>
              </span>
            </div>
          </div>
        ) : null}
      </div>
    </section>
  );
}
