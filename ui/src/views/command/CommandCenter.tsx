import type { CSSProperties, ReactNode } from "react";
import {
  ArrowUpDown,
  Brain,
  CheckCheck,
  ChevronLeft,
  CircleCheck,
  Dot,
  FolderGit2,
  GitCommitHorizontal,
  GitPullRequest,
  Plus,
  RefreshCw,
  ShieldQuestion,
  Workflow,
} from "lucide-react";
import { Badge, Button, HarnessBadge, RiskBadge } from "../../design-system/kit";
import { StatusPill } from "../../status/StatusPill";
import { describeStatus } from "../../status/descriptors";
import { humanizeActorLabel } from "../audit/actor-label";
import type {
  ApprovalQueueRow,
  AuditEventRow,
  SessionRow,
  UsageRow,
} from "../../contracts/index";
import {
  approvalDisplayFixture,
  sessionDisplayFixture,
  type RiskLevel,
} from "../../shell/display-meta";
import { toSessionItems } from "../../projections/items";
import { groupForCommandCenter } from "./group";
import { SessionRowCard } from "./SessionRowCard";

const sectionPad: CSSProperties = { padding: "14px 16px" };

function Eyebrow({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return (
    <div
      style={{
        font: "var(--fw-semibold) var(--fs-micro)/1 var(--font-sans)",
        letterSpacing: "var(--tracking-caps)",
        textTransform: "uppercase",
        color: "var(--text-faint)",
        ...style,
      }}
    >
      {children}
    </div>
  );
}

/**
 * The attention card (ported from kit-views.jsx AttentionCard): a bordered,
 * surface-tinted card for a session waiting on the human / failed, with the
 * status pill, harness glyph, title, current line, and the action pair.
 * Approve/Retry are Gateway mutations — DISABLED until the Gateway overlay +
 * intent seam land (wire-or-disable §11.6; flagged, not faked). Open is live.
 */
function AttentionCard({
  session,
  usage: _usage,
  onOpen,
}: {
  session: SessionRow;
  usage: UsageRow[];
  onOpen: (s: SessionRow) => void;
}) {
  const descriptor = describeStatus("Session", session.status);
  // waiting-on-human (rank 5) gets the attention treatment; failed/blocked (4)
  // the danger treatment — same split as the prototype's waiting check.
  const waiting = descriptor.attentionRank === 5 || session.status === "waiting_on_permission" || session.status === "changes_ready";
  const display = sessionDisplayFixture[session.session_id];
  return (
    <div
      data-item-id={`Session:${session.session_id}`}
      style={{
        position: "relative",
        border: `1px solid ${waiting ? "var(--attention-line)" : "var(--danger-line)"}`,
        background: waiting ? "var(--attention-surface)" : "var(--danger-surface)",
        borderRadius: "var(--r-3)",
        padding: "11px 12px 12px",
        overflow: "hidden",
      }}
    >
      <span
        aria-hidden="true"
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          bottom: 0,
          width: 3,
          background: waiting ? "var(--attention-solid)" : "var(--danger-solid)",
        }}
      />
      <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 8 }}>
        <StatusPill machine="Session" status={session.status} />
        {display?.harness ? (
          <span style={{ marginLeft: "auto" }}>
            <HarnessBadge harness={display.harness} showLabel={false} />
          </span>
        ) : null}
      </div>
      <div style={{ font: "var(--fw-medium) var(--fs-body)/1.3 var(--font-sans)", marginBottom: 6 }}>
        {session.display_name ?? session.session_id}
      </div>
      <div
        style={{
          font: "var(--fs-meta) var(--font-mono)",
          color: "var(--text-muted)",
          marginBottom: 11,
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
        }}
      >
        {display?.current ?? descriptor.label}
      </div>
      <div style={{ display: "flex", gap: 6 }}>
        {waiting ? (
          <>
            <span title="Approve flows arrive with the Gateway overlay (intent seam — daemon-gated)">
              <Button
              variant="secondary"
              size="sm"
              disabled
            >
              Review &amp; approve
            </Button>
            </span>
            <Button variant="ghost" size="sm" onClick={() => onOpen(session)}>
              Open
            </Button>
          </>
        ) : (
          <>
            <span title="Retry is a Gateway mutation (intent seam — daemon-gated)">
              <Button
                variant="secondary"
                size="sm"
                icon={<RefreshCw size={14} />}
                disabled
              >
                Retry checks
              </Button>
            </span>
            <Button variant="ghost" size="sm" onClick={() => onOpen(session)}>
              Open log
            </Button>
          </>
        )}
      </div>
    </div>
  );
}

/** A Human-Input-Queue rail card (ported QueueItem): risk badge + actor + ask. */
function QueueItem({
  risk,
  who,
  text,
}: {
  risk?: RiskLevel;
  who?: string;
  text: string;
}) {
  return (
    <button
      type="button"
      disabled
      title="Approval resolution arrives with the Gateway overlay (intent seam — daemon-gated)"
      style={{
        textAlign: "left",
        cursor: "default",
        border: "1px solid var(--attention-line)",
        background: "var(--attention-surface)",
        borderRadius: "var(--r-2)",
        padding: "9px 10px",
        display: "flex",
        flexDirection: "column",
        gap: 7,
      }}
    >
      <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
        {risk ? <RiskBadge level={risk} /> : null}
        {who ? (
          <span
            style={{
              marginLeft: "auto",
              font: "var(--fs-micro) var(--font-mono)",
              color: "var(--text-muted)",
            }}
          >
            {who}
          </span>
        ) : null}
      </span>
      <span style={{ font: "var(--fw-medium) var(--fs-label)/1.35 var(--font-sans)", color: "var(--text-primary)" }}>
        {text}
      </span>
    </button>
  );
}

// audit event_type namespace → rail icon (ported EVENT_ICON).
function eventIcon(eventType: string): ReactNode {
  const ns = eventType.split(".")[0];
  switch (ns) {
    case "approval":
    case "action":
      return <ShieldQuestion size={13} />;
    case "git":
      return <GitCommitHorizontal size={13} />;
    case "brain":
      return <Brain size={13} />;
    case "pr":
      return <GitPullRequest size={13} />;
    case "workflow":
      return <Workflow size={13} />;
    case "session":
      return <CircleCheck size={13} />;
    case "project":
      return <FolderGit2 size={13} />;
    default:
      return <Dot size={13} />;
  }
}

function EventLine({ e }: { e: AuditEventRow }) {
  const tone =
    e.actor_label === "project_brain"
      ? "var(--brain-ink)"
      : e.event_type.startsWith("action") || e.event_type.startsWith("approval")
        ? "var(--attention-ink)"
        : "var(--text-faint)";
  return (
    <div style={{ display: "flex", gap: 8, padding: "5px 0", alignItems: "flex-start" }}>
      <span aria-hidden="true" style={{ marginTop: 1, display: "inline-flex", color: tone }}>
        {eventIcon(e.event_type)}
      </span>
      <div style={{ minWidth: 0 }}>
        <div style={{ font: "var(--fs-meta)/1.4 var(--font-sans)", color: "var(--text-secondary)" }}>
          {e.headline}
        </div>
        <div style={{ font: "var(--fs-micro) var(--font-mono)", color: "var(--text-faint)", marginTop: 1 }}>
          #{e.seq} · {humanizeActorLabel(e.actor_label)}
        </div>
      </div>
    </div>
  );
}

/**
 * The right rail (ported CommandRail): the GLOBAL Human Input Queue (pending
 * approvals + waiting sessions — triage is cross-cutting, Lesson §13), the
 * Capacity meters, and the recent-events feed. Queue risk/actor lines come from
 * the approval display side-map (fixture until the daemon enriches the
 * ApprovalQueue projection — flagged). Capacity renders the REAL credit pool as
 * its meter; spend/tokens/sessions render as stat lines because the daemon has
 * no configured limits yet (a meter without a real max would fabricate one).
 */
function CommandRail({
  approvals,
  waiting,
  usage,
  events,
  activeSessionCount,
}: {
  approvals: ApprovalQueueRow[];
  waiting: SessionRow[];
  usage: UsageRow[];
  events: AuditEventRow[];
  activeSessionCount: number;
}) {
  const queueCount = approvals.length + waiting.length;
  const spend = usage.reduce((s, u) => s + (u.cost_estimate ?? 0), 0);
  const tokens = usage.reduce((s, u) => s + (u.tokens_in ?? 0) + (u.tokens_out ?? 0), 0);
  const anyEstimated = usage.some((u) => u.metric_quality !== "exact");
  return (
    <aside
      aria-label="Command rail"
      style={{
        borderLeft: "1px solid var(--border-default)",
        background: "var(--surface-panel)",
        overflowY: "auto",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div style={{ ...sectionPad, borderBottom: "1px solid var(--border-subtle)" }}>
        <Eyebrow style={{ marginBottom: 10 }}>
          Human input queue{" "}
          {queueCount > 0 ? <span style={{ color: "var(--attention-ink)" }}>· {queueCount}</span> : null}
        </Eyebrow>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }} data-testid="hiq-rail">
          {queueCount === 0 ? (
            <div
              style={{
                font: "var(--fs-meta) var(--font-sans)",
                color: "var(--text-faint)",
                padding: "6px 2px",
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              <CheckCheck size={14} aria-hidden="true" style={{ color: "var(--success-ink)" }} />
              Queue clear — nothing waiting on you.
            </div>
          ) : (
            <>
              {approvals.map((a) => {
                const meta = approvalDisplayFixture[a.approval_id];
                return (
                  <QueueItem
                    key={a.approval_id}
                    risk={meta?.risk}
                    who={meta?.who}
                    text={a.preview_summary ?? a.approval_id}
                  />
                );
              })}
              {waiting.map((s) => {
                const display = sessionDisplayFixture[s.session_id];
                return (
                  <QueueItem
                    key={s.session_id}
                    risk={s.status === "waiting_on_permission" ? "medium" : undefined}
                    who={display?.profile}
                    text={display?.current ?? s.display_name ?? s.session_id}
                  />
                );
              })}
            </>
          )}
        </div>
      </div>
      <div style={{ ...sectionPad, borderBottom: "1px solid var(--border-subtle)" }}>
        <Eyebrow style={{ marginBottom: 10 }}>Capacity</Eyebrow>
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {/* Credit-pool meter HONESTLY OMITTED (W2-usage) — the daemon has no remaining-balance
              source. Spend/token/runtime LIMITS are daemon config not yet exposed — stat
              lines, not meters (a meter needs a real max; flagged). */}
          <div style={{ display: "flex", flexDirection: "column", gap: 5, font: "var(--fs-meta) var(--font-mono)", color: "var(--text-muted)" }}>
            <span style={{ display: "flex", justifyContent: "space-between" }}>
              <span>Spend today</span>
              <span style={{ color: "var(--text-secondary)" }}>
                {anyEstimated ? "≈ " : ""}${spend.toFixed(2)}
              </span>
            </span>
            <span style={{ display: "flex", justifyContent: "space-between" }}>
              <span>Tokens</span>
              <span style={{ color: "var(--text-secondary)" }}>
                {Math.round(tokens / 1000)}k
              </span>
            </span>
            <span style={{ display: "flex", justifyContent: "space-between" }}>
              <span>Active sessions</span>
              <span style={{ color: "var(--text-secondary)" }}>{activeSessionCount}</span>
            </span>
          </div>
        </div>
      </div>
      <div style={{ ...sectionPad }}>
        <Eyebrow style={{ marginBottom: 10 }}>Recent events</Eyebrow>
        <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
          {events.slice(0, 6).map((e) => (
            <EventLine key={e.event_id} e={e} />
          ))}
        </div>
      </div>
    </aside>
  );
}

/**
 * Command Center (ported from kit-views.jsx CommandCenter): the project-scoped
 * triage cockpit — header (Projects ← · project title · session count · sort ·
 * New session), the NEEDS-MY-ATTENTION card grid, WORKING NOW / RECENTLY
 * SETTLED session rows, and the global right rail (HIQ · Capacity · Recent
 * events). Grouping routes through groupForCommandCenter (the 6.2a single
 * source — changes_ready keeps its prominence by ordering first in the
 * attention section). New session / Sort are daemon-gated mutations/options —
 * disabled, not faked (§11.6).
 */
export function CommandCenter({
  sessions,
  approvals,
  waiting,
  usage,
  events,
  projectName,
  onOpenSession,
  onOpenProjects,
}: {
  /** Sessions scoped to the ACTIVE project (the prototype's project cockpit). */
  sessions: SessionRow[];
  /** GLOBAL pending approvals (rail HIQ — triage stays cross-cutting). */
  approvals: ApprovalQueueRow[];
  /** GLOBAL waiting sessions (rail HIQ). */
  waiting: SessionRow[];
  usage: UsageRow[];
  events: AuditEventRow[];
  projectName: string;
  onOpenSession: (s: SessionRow) => void;
  onOpenProjects: () => void;
}) {
  // Group via the single grouping source (items mapper + groupForCommandCenter);
  // map ids back to rows for the rich cards.
  const groups = groupForCommandCenter(toSessionItems(sessions));
  const byId = new Map(sessions.map((s) => [s.session_id, s]));
  const rows = (ids: { id: string }[]) =>
    ids.flatMap((i) => {
      const row = byId.get(i.id);
      return row ? [row] : [];
    });
  const attention = rows([...groups.changesReady, ...groups.needsAttention]);
  const working = rows(groups.working);
  const settled = rows(groups.settled);
  const activeCount = sessions.length - settled.length;

  return (
    <div
      aria-label="Command Center"
      className="command-center"
      style={{
        background: "var(--surface-canvas)",
        overflow: "hidden",
        display: "grid",
        gridTemplateColumns: "1fr 300px",
        height: "100%",
      }}
    >
      <div style={{ overflowY: "auto", position: "relative" }}>
        <div
          style={{
            ...sectionPad,
            display: "flex",
            alignItems: "center",
            gap: 10,
            borderBottom: "1px solid var(--border-subtle)",
          }}
        >
          <button
            type="button"
            onClick={onOpenProjects}
            title="All projects"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              padding: "3px 8px 3px 6px",
              borderRadius: "var(--r-1)",
              border: "1px solid var(--border-default)",
              background: "var(--surface-card)",
              cursor: "pointer",
              font: "var(--fs-meta) var(--font-sans)",
              color: "var(--text-muted)",
            }}
          >
            <ChevronLeft size={13} aria-hidden="true" /> Projects
          </button>
          <h1
            style={{
              margin: 0,
              font: "var(--fw-semibold) var(--fs-h2)/1.1 var(--font-sans)",
              letterSpacing: "var(--tracking-tight)",
            }}
          >
            {projectName}
          </h1>
          <Badge tone="neutral" mono>
            {sessions.length} sessions
          </Badge>
          <div style={{ marginLeft: "auto", display: "flex", gap: 7 }}>
            <span title="Attention is the only sort order in this build">
              <Button variant="ghost" size="sm" icon={<ArrowUpDown size={14} />} disabled>
                Sort: attention
              </Button>
            </span>
            <span title="Session dispatch arrives with the intent seam (daemon-gated)">
              <Button variant="primary" size="sm" icon={<Plus size={14} />} disabled>
                New session
              </Button>
            </span>
          </div>
        </div>

        {/* Needs my attention */}
        <section style={sectionPad} aria-label="Needs my attention" data-group="needsAttention">
          <Eyebrow
            style={{
              marginBottom: 10,
              color: attention.length ? "var(--attention-ink)" : "var(--text-faint)",
            }}
          >
            ● Needs my attention{attention.length ? ` · ${attention.length}` : ""}
          </Eyebrow>
          {attention.length === 0 ? (
            <div
              data-testid="empty-needsAttention"
              style={{
                font: "var(--fs-label) var(--font-sans)",
                color: "var(--text-muted)",
                display: "flex",
                alignItems: "center",
                gap: 7,
                padding: "6px 0",
              }}
            >
              <CheckCheck size={15} aria-hidden="true" style={{ color: "var(--success-ink)" }} />
              All clear. No session is waiting on you.
            </div>
          ) : (
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
              {attention.map((s) => (
                <AttentionCard key={s.session_id} session={s} usage={usage} onOpen={onOpenSession} />
              ))}
            </div>
          )}
        </section>

        {/* Working */}
        <section style={sectionPad} aria-label="Working now" data-group="working">
          <Eyebrow style={{ marginBottom: 8 }}>▶ Working now</Eyebrow>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {working.length === 0 ? (
              <span
                data-testid="empty-working"
                style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)" }}
              >
                Nothing running right now.
              </span>
            ) : (
              working.map((s) => (
                <SessionRowCard key={s.session_id} session={s} usage={usage} onOpen={onOpenSession} />
              ))
            )}
          </div>
        </section>

        {/* Settled */}
        <section style={sectionPad} aria-label="Recently settled" data-group="settled">
          <Eyebrow style={{ marginBottom: 8 }}>Recently settled</Eyebrow>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {settled.length === 0 ? (
              <span
                data-testid="empty-settled"
                style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)" }}
              >
                Nothing settled yet.
              </span>
            ) : (
              settled.map((s) => (
                <SessionRowCard key={s.session_id} session={s} usage={usage} onOpen={onOpenSession} dim />
              ))
            )}
          </div>
        </section>
      </div>

      <CommandRail
        approvals={approvals}
        waiting={waiting}
        usage={usage}
        events={events}
        activeSessionCount={activeCount}
      />
    </div>
  );
}
