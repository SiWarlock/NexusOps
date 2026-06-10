import { Bot, GitMerge, Pause, Terminal, UsersRound, Workflow } from "lucide-react";
import {
  Badge,
  Button,
  DisplayStatusPill,
  HarnessBadge,
  IconButton,
  UsageMeter,
} from "../../design-system/kit";
import { Eyebrow } from "../cockpit";
import { teamFixture } from "../display-fixtures";

/**
 * The Agent Team view (ported from kit-views3.jsx AgentTeamView): orchestrator
 * card + connected worker cards (role · status · task/worktree · harness · ctx
 * ring). DISPLAY-ONLY over a provisional fixture — the AgentTeam projection is
 * daemon-gated (flagged); Open-terminals/Integrate/Pause/Grant are disabled,
 * not faked (§11.6).
 */
export function AgentTeamView() {
  const t = teamFixture;
  return (
    <div aria-label="Agent Team" style={{ height: "100%", overflowY: "auto", background: "var(--surface-canvas)" }}>
      <div
        style={{
          padding: "14px 16px",
          borderBottom: "1px solid var(--border-subtle)",
          display: "flex",
          alignItems: "center",
          gap: 10,
        }}
      >
        <span aria-hidden="true" style={{ color: "var(--teal-ink)", display: "inline-flex" }}>
          <UsersRound size={18} />
        </span>
        <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)" }}>{t.name}</h1>
        <Badge tone="teal">{t.pack}</Badge>
        <Badge mono style={{ color: "var(--text-faint)" }}>
          display fixture — AgentTeam projection pending
        </Badge>
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          <span title="Team terminals arrive with the daemon terminal contract (6.3d/e)">
            <Button variant="secondary" size="sm" icon={<Terminal size={14} />} disabled>
              Open terminals
            </Button>
          </span>
          <span title="Integrate is a Gateway mutation (intent seam — daemon-gated)">
            <Button variant="ghost" size="sm" icon={<GitMerge size={14} />} disabled>
              Integrate
            </Button>
          </span>
          <span title="Pause is a team mutation (intent seam — daemon-gated)">
            <Button variant="ghost" size="sm" icon={<Pause size={14} />} disabled>
              Pause team
            </Button>
          </span>
        </div>
      </div>

      <div style={{ padding: 16, maxWidth: 760 }}>
        <Eyebrow style={{ marginBottom: 8 }}>Orchestrator</Eyebrow>
        <div
          style={{
            border: "1px solid var(--teal-line)",
            background: "var(--teal-surface)",
            borderRadius: "var(--r-3)",
            padding: "12px 13px",
            marginBottom: 6,
            display: "flex",
            alignItems: "center",
            gap: 11,
          }}
        >
          <span
            aria-hidden="true"
            style={{
              width: 30,
              height: 30,
              flex: "none",
              borderRadius: "var(--r-2)",
              background: "var(--teal-solid)",
              color: "var(--teal-on-solid)",
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <Workflow size={16} />
          </span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ font: "var(--fw-semibold) var(--fs-body) var(--font-sans)" }}>{t.lead.role}</span>
              <DisplayStatusPill status={t.lead.status} size="xs" />
            </div>
            <div style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)", marginTop: 3 }}>
              {t.lead.task}
            </div>
          </div>
          <HarnessBadge harness={t.lead.harness} />
          <UsageMeter variant="ring" size="sm" value={t.lead.ctx} max={100} label="ctx" />
        </div>

        {/* connector */}
        <div aria-hidden="true" style={{ height: 14, borderLeft: "1.5px solid var(--teal-line)", marginLeft: 24 }} />

        <Eyebrow style={{ marginBottom: 8 }}>Workers · {t.workers.length}</Eyebrow>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {t.workers.map((w) => (
            <div
              key={w.id}
              style={{
                position: "relative",
                border: "1px solid var(--border-default)",
                background: "var(--surface-card)",
                borderRadius: "var(--r-3)",
                padding: "11px 12px",
                display: "flex",
                alignItems: "center",
                gap: 11,
                marginLeft: 24,
              }}
            >
              <span
                aria-hidden="true"
                style={{ position: "absolute", left: -24, top: "50%", width: 24, height: 1, background: "var(--teal-line)" }}
              />
              <span
                aria-hidden="true"
                style={{
                  width: 28,
                  height: 28,
                  flex: "none",
                  borderRadius: "var(--r-2)",
                  background: "var(--surface-active)",
                  color: "var(--text-secondary)",
                  display: "inline-flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <Bot size={15} />
              </span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ font: "var(--fw-medium) var(--fs-body) var(--font-sans)" }}>{w.role}</span>
                  <DisplayStatusPill status={w.status} size="xs" beacon={w.status === "waiting-perm"} />
                </div>
                <div style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)", marginTop: 3 }}>
                  {w.task} · {w.wt}
                </div>
              </div>
              <HarnessBadge harness={w.harness} showLabel={false} />
              <UsageMeter variant="ring" size="sm" value={w.ctx} max={100} label="ctx" />
              {w.status === "waiting-perm" ? (
                <span title="Grant is a Gateway mutation (intent seam — daemon-gated)">
                  <Button variant="secondary" size="sm" disabled>
                    Grant
                  </Button>
                </span>
              ) : (
                <IconButton label={`Open ${w.role} terminal`} size="sm" disabled>
                  <Terminal size={15} />
                </IconButton>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
