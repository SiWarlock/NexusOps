import { HarnessBadge, MetaChip, ProfileBadge, UsageMeter } from "../../design-system/kit";
import { AttentionMarker } from "../../status/AttentionMarker";
import { StatusPill } from "../../status/StatusPill";
import { describeStatus } from "../../status/descriptors";
import type { SessionRow, UsageRow } from "../../contracts/index";
import { contextForSession, sessionDisplayFixture } from "../../shell/display-meta";

/**
 * The Command Center session row — a faithful port of the kit SessionRow
 * anatomy (objects/SessionRow.jsx: attention rail · status pill + title + ctx
 * ring · ownership chips · current/activity line), rebuilt from kit primitives
 * so that:
 *  - status binds through the DESCRIPTOR (machine,status) wrappers (Lesson §6),
 *    never the kit's own status→ATTN re-derivation;
 *  - a session whose adapter reports no context renders "ctx unknown" — NEVER a
 *    fabricated ring value (forbidden #4); accuracy rides the real
 *    metric_quality.
 * Ownership chips come from the display side-map (fixture until the daemon
 * enriches the Session projection — flagged); ctx comes from the REAL Usage
 * projection.
 */
export function SessionRowCard({
  session,
  usage,
  onOpen,
  dim = false,
}: {
  session: SessionRow;
  usage: UsageRow[];
  onOpen: (s: SessionRow) => void;
  dim?: boolean;
}) {
  const descriptor = describeStatus("Session", session.status);
  const display = sessionDisplayFixture[session.session_id];
  const ctx = contextForSession(usage, session.session_id);

  return (
    <button
      type="button"
      data-item-id={`Session:${session.session_id}`}
      onClick={() => onOpen(session)}
      style={{
        display: "flex",
        alignItems: "stretch",
        gap: 0,
        width: "100%",
        textAlign: "left",
        background: "transparent",
        border: "none",
        borderRadius: "var(--r-2)",
        cursor: "pointer",
        overflow: "hidden",
        padding: 0,
        opacity: dim ? 0.85 : 1,
      }}
    >
      <AttentionMarker rank={descriptor.attentionRank} variant="rail" />
      <span
        style={{
          flex: 1,
          minWidth: 0,
          padding: "8px 11px",
          display: "flex",
          flexDirection: "column",
          gap: 6,
        }}
      >
        {/* line 1: status + title + context */}
        <span style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
          <StatusPill machine="Session" status={session.status} size="xs" />
          <span
            style={{
              font: "var(--fw-medium) var(--fs-body)/1.2 var(--font-sans)",
              color: "var(--text-primary)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              minWidth: 0,
              flex: 1,
            }}
          >
            {session.title ?? session.session_id}
          </span>
          {ctx ? (
            ctx.pct === null || ctx.accuracy === "unavailable" ? (
              // §9.1: no context metadata → an explicit "unknown", never a number.
              <span
                data-testid="ctx-unknown"
                style={{ font: "10px var(--font-mono)", color: "var(--text-faint)", flex: "none" }}
              >
                ctx unknown
              </span>
            ) : (
              <UsageMeter
                variant="ring"
                size="sm"
                value={ctx.pct}
                max={100}
                label="ctx"
                accuracy={ctx.accuracy}
              />
            )
          ) : null}
        </span>
        {/* line 2: ownership chips (display side-map — fixture until daemon enrichment) */}
        <span style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
          {display?.harness ? (
            <HarnessBadge harness={display.harness} />
          ) : null}
          {display?.profile ? (
            <ProfileBadge name={display.profile} provider={display.provider} />
          ) : null}
          {display?.task ? (
            <MetaChip tone={display.task.tone ?? "linear"} mono={false}>
              {display.task.id}
            </MetaChip>
          ) : null}
          {display?.branch ? <MetaChip tone="branch">{display.branch}</MetaChip> : null}
          {display?.worktree ? (
            <MetaChip tone="worktree">{display.worktree}</MetaChip>
          ) : null}
          {display?.pr ? <MetaChip tone="pr">{display.pr}</MetaChip> : null}
        </span>
        {/* line 3: current activity + last activity */}
        {display?.current || display?.activity ? (
          <span
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              font: "var(--fw-regular) var(--fs-meta)/1.3 var(--font-mono)",
              color: "var(--text-muted)",
              minWidth: 0,
            }}
          >
            {display.current ? (
              <span
                style={{
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                  minWidth: 0,
                }}
              >
                {display.current}
              </span>
            ) : null}
            {display.activity ? (
              <span style={{ marginLeft: "auto", flex: "none", color: "var(--text-faint)" }}>
                {display.activity}
              </span>
            ) : null}
          </span>
        ) : null}
      </span>
    </button>
  );
}
