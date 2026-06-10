import { useState, type ReactNode } from "react";
import {
  Brain,
  ChevronDown,
  ChevronUp,
  CircleDot,
  Dot,
  FolderGit2,
  GitCommitHorizontal,
  GitPullRequest,
  ScrollText,
  ShieldCheck,
  Workflow,
} from "lucide-react";
import type { AuditEventRow } from "../contracts/index";
import type { ConnectionState } from "../connection/state";
import { ConnectionIndicator } from "../connection/ConnectionIndicator";
import { deriveActivityFeed } from "./derive";

// event_type prefix → dock icon (the prototype's DOCK_ICON, keyed off the real
// audit event-type namespace).
function eventIcon(eventType: string): ReactNode {
  const ns = eventType.split(".")[0];
  switch (ns) {
    case "approval":
    case "action":
      return <ShieldCheck size={13} />;
    case "git":
      return <GitCommitHorizontal size={13} />;
    case "session":
      return <CircleDot size={13} />;
    case "brain":
      return <Brain size={13} />;
    case "pr":
    case "pull_request":
      return <GitPullRequest size={13} />;
    case "workflow":
      return <Workflow size={13} />;
    case "project":
      return <FolderGit2 size={13} />;
    default:
      return <Dot size={13} />;
  }
}

// actor_type → display name ("user" reads as You — the accent actor).
const ACTOR_LABEL: Record<string, string> = {
  user: "You",
  action_gateway: "Gateway",
  session_adapter: "Adapter",
  project_brain: "Brain",
  system: "System",
};

/**
 * Bottom event/activity dock (ported from kit-shell.jsx EventDock): a status
 * strip (always visible — Activity toggle, the live daemon ConnectionIndicator,
 * the latest event, the event count) that expands into the project-scoped event
 * timeline bound to the AuditTrail projection, with a "Full audit" jump to the
 * Audit Trail view. NOTE: the projection carries seq but no timestamps yet —
 * the right column renders `#seq` (real data; timestamps are a flagged daemon
 * projection-enrichment).
 */
export function EventDock({
  events,
  connection,
  projectId,
  projectName,
  onOpenAudit,
}: {
  events: AuditEventRow[];
  connection: ConnectionState;
  projectId?: string | null;
  projectName?: string;
  onOpenAudit: () => void;
}) {
  const [open, setOpen] = useState(false);
  const feed = deriveActivityFeed(events, { projectId: projectId ?? undefined });
  const latest = feed[0];

  return (
    <div className="event-dock" data-open={open}>
      {/* status strip (always visible) */}
      <button
        type="button"
        className="event-dock__toggle"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          height: "var(--shell-statusbar-h)",
          flex: "none",
          padding: "0 12px",
          border: "none",
          background: "transparent",
          cursor: "pointer",
          textAlign: "left",
          width: "100%",
        }}
      >
        <span aria-hidden="true" style={{ display: "inline-flex", color: "var(--text-faint)" }}>
          {open ? <ChevronDown size={14} /> : <ChevronUp size={14} />}
        </span>
        <span
          style={{
            font: "var(--fw-semibold) var(--fs-micro) var(--font-sans)",
            letterSpacing: "var(--tracking-caps)",
            textTransform: "uppercase",
            color: "var(--text-muted)",
          }}
        >
          Activity
        </span>
        <ConnectionIndicator state={connection} />
        {latest && !open ? (
          <span
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              minWidth: 0,
              font: "var(--fs-meta) var(--font-mono)",
              color: "var(--text-muted)",
            }}
          >
            <span style={{ color: "var(--text-faint)" }}>·</span>
            <span aria-hidden="true" style={{ display: "inline-flex", color: "var(--text-faint)" }}>
              {eventIcon(latest.event_type)}
            </span>
            <span
              style={{
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                maxWidth: 420,
                color: "var(--text-secondary)",
              }}
            >
              {latest.summary ?? latest.event_type}
            </span>
          </span>
        ) : null}
        <span style={{ marginLeft: "auto", font: "10px var(--font-mono)", color: "var(--text-faint)" }}>
          {feed.length} events
        </span>
      </button>
      {/* expanded timeline */}
      {open ? (
        <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "0 12px 6px" }}>
            <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-faint)" }}>
              {projectName}
            </span>
            <button
              type="button"
              onClick={onOpenAudit}
              style={{
                marginLeft: "auto",
                display: "inline-flex",
                alignItems: "center",
                gap: 5,
                padding: "3px 8px",
                borderRadius: "var(--r-1)",
                border: "1px solid var(--border-default)",
                background: "transparent",
                cursor: "pointer",
                font: "var(--fs-meta) var(--font-sans)",
                color: "var(--text-secondary)",
              }}
            >
              <ScrollText size={13} aria-hidden="true" /> Full audit
            </button>
          </div>
          <ul
            data-testid="activity-timeline"
            style={{
              flex: 1,
              overflowY: "auto",
              padding: "0 12px 10px",
              margin: 0,
              listStyle: "none",
            }}
          >
            {feed.length === 0 ? (
              <li style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)", padding: "8px 2px" }}>
                No recent activity for this project.
              </li>
            ) : (
              feed.map((e) => (
                <li
                  key={e.event_id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 9,
                    padding: "5px 0",
                    borderBottom: "1px solid var(--border-subtle)",
                  }}
                >
                  <span
                    aria-hidden="true"
                    style={{
                      display: "inline-flex",
                      color:
                        e.actor_type === "project_brain"
                          ? "var(--brain-ink)"
                          : "var(--text-faint)",
                    }}
                  >
                    {eventIcon(e.event_type)}
                  </span>
                  <span
                    style={{
                      flex: 1,
                      minWidth: 0,
                      font: "var(--fs-meta)/1.4 var(--font-sans)",
                      color: "var(--text-secondary)",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    <span data-event-type={e.event_type}>{e.event_type}</span>
                    {e.summary ? <> — {e.summary}</> : null}
                  </span>
                  <span
                    style={{
                      font: "var(--fs-micro) var(--font-mono)",
                      color: e.actor_type === "user" ? "var(--accent-ink)" : "var(--text-faint)",
                    }}
                  >
                    {ACTOR_LABEL[e.actor_type] ?? e.actor_type}
                  </span>
                  <span
                    title="event sequence (timestamps land with the daemon enrichment)"
                    style={{
                      font: "var(--fs-micro) var(--font-mono)",
                      color: "var(--text-faint)",
                      width: 56,
                      textAlign: "right",
                    }}
                  >
                    #{e.seq}
                  </span>
                </li>
              ))
            )}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
