import { ChevronRight, GitPullRequest, Pause } from "lucide-react";
import type { ProjectActivityRow, SessionRow, UsageRow } from "../../contracts/index";
import { Button, HarnessBadge, MetaChip, ProfileBadge } from "../../design-system/kit";
import { StatusPill } from "../../status/StatusPill";
import { describeStatus } from "../../status/descriptors";
import { RiskBadge } from "../../design-system/kit";
import { sessionDisplayFixture } from "../../shell/display-meta";
import { SessionsTable } from "../sessions/SessionsTable";
import { LaunchAgentControl } from "../sessions/LaunchAgentControl";
import { KillSessionControl } from "../sessions/KillSessionControl";
import { Eyebrow } from "../cockpit";
import type { GatewayPort } from "../../gateway-client/types";
import { TerminalDisplay } from "./TerminalDisplay";
import { isEndedSession } from "./session-lifecycle";

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
  gateway,
  activeProjectId = null,
}: {
  session: SessionRow | null;
  sessions: SessionRow[];
  projects: ProjectActivityRow[];
  usage?: UsageRow[];
  gateway: GatewayPort;
  /** The active project (the launch target for the WAVE-1 "Launch agent" control). */
  activeProjectId?: string | null;
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
        <SessionsTable
          sessions={sessions}
          projects={projects}
          headerActions={
            <LaunchAgentControl gateway={gateway} activeProjectId={activeProjectId} />
          }
          rowActions={
            // eslint-disable-next-line react/no-unstable-nested-components -- a render-prop slot, not a defined-in-render component: KillSessionControl is a stable top-level import (no remount); the arrow keeps SessionsTable presentational (forbidden #2).
            (row) => (
              <KillSessionControl gateway={gateway} sessionId={row.id} status={row.status} />
            )
          }
        />
      </section>
    );
  }

  const descriptor = describeStatus("Session", session.status);
  const display = sessionDisplayFixture[session.session_id];
  const waitingPermission = session.status === "waiting_on_permission";
  // The well state is driven by the Session PROJECTION + the (fixture) terminal
  // handle — NEVER by the output bytes (#9). Ended → honest ended state; a live
  // terminal handle → the xterm well; otherwise the honest placeholder.
  const terminalId = display?.terminalId;
  const ended = isEndedSession(session.status);

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
            {session.display_name ?? session.session_id}
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
      {/* terminal well — daemon-gated (§9.1 PTY display-only; 6.3d/e). Flex column so
          a live xterm fills the space ABOVE a still-visible permission card. */}
      <div
        style={{
          flex: 1,
          minHeight: 0,
          display: "flex",
          flexDirection: "column",
          padding: "12px 16px",
          background: "var(--surface-sunken)",
          boxShadow: "var(--elev-inset)",
          fontFamily: "var(--font-mono)",
          fontSize: "var(--fs-body)",
        }}
      >
        {ended ? (
          // ENDED: an honest end-state sourced from session.status (the projection),
          // never a faked "still running". exit_code/signal detail has no frozen
          // UI-readable source yet → deferred (P4 projection wiring).
          <div
            data-testid="terminal-ended"
            style={{ color: "var(--text-faint)", lineHeight: "22px" }}
          >
            <div>· session {descriptor.label.toLowerCase()} — the terminal process has ended.</div>
            <div>  exit detail (code / signal) arrives with the daemon projection (P4).</div>
          </div>
        ) : terminalId ? (
          // LIVE: the xterm well, fed the daemon's §6.4 terminal_output stream
          // (display-only #9 — see TerminalDisplay). flex:1 + minHeight:0 so xterm
          // fills the space + scrolls internally, leaving the card below visible.
          <div style={{ flex: 1, minHeight: 0 }}>
            <TerminalDisplay gateway={gateway} terminalId={terminalId} />
          </div>
        ) : (
          // NO live terminal handle: the honest placeholder (never invent a transcript).
          <div
            data-testid="terminal-well-pending"
            style={{ color: "var(--text-faint)", lineHeight: "22px" }}
          >
            <div>· no live terminal stream for this session.</div>
            <div>  the well attaches when the daemon reports a terminal handle (P4).</div>
            <div>  status above is real; transcript lines are never invented.</div>
            {display?.current ? (
              <div style={{ marginTop: 12, color: "var(--text-muted)" }}>
                last reported activity: <span style={{ color: "var(--text-secondary)" }}>{display.current}</span>
              </div>
            ) : null}
          </div>
        )}
        {waitingPermission ? (
          <div
            style={{
              marginTop: 12,
              flexShrink: 0, // stay visible below a flex:1 live terminal
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
