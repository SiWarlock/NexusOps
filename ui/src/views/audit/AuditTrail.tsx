import { useState, type ReactNode } from "react";
import {
  Brain,
  CircleDot,
  Dot,
  Download,
  FolderGit2,
  GitCommitHorizontal,
  GitPullRequest,
  ScrollText,
  ShieldCheck,
  Workflow,
} from "lucide-react";
import type { AuditEventRow } from "../../contracts/index";
import { Badge, Button } from "../../design-system/kit";

// Filter chips over the REAL event_type namespace (the prototype's
// auditFilters, keyed to this repo's audit vocabulary).
const FILTERS: { label: string; namespaces: string[] | null }[] = [
  { label: "All", namespaces: null },
  { label: "Approvals", namespaces: ["approval", "action"] },
  { label: "Git", namespaces: ["git"] },
  { label: "Sessions", namespaces: ["session"] },
  { label: "Projects", namespaces: ["project"] },
];

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
      return <GitPullRequest size={13} />;
    case "workflow":
      return <Workflow size={13} />;
    case "project":
      return <FolderGit2 size={13} />;
    default:
      return <Dot size={13} />;
  }
}

const ACTOR_LABEL: Record<string, string> = {
  user: "You",
  action_gateway: "Gateway",
  session_adapter: "Adapter",
  project_brain: "Brain",
  system: "System",
};

function AuditRow({ e, last }: { e: AuditEventRow; last: boolean }) {
  const actor = ACTOR_LABEL[e.actor_type] ?? e.actor_type;
  return (
    <div style={{ display: "flex", gap: 12, position: "relative" }} data-item-id={`AuditEvent:${e.event_id}`}>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", flex: "none", width: 26 }}>
        <span
          aria-hidden="true"
          style={{
            width: 26,
            height: 26,
            borderRadius: 999,
            background: "var(--surface-card)",
            border: "1px solid var(--border-default)",
            color: e.actor_type === "project_brain" ? "var(--brain-ink)" : "var(--text-muted)",
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 1,
          }}
        >
          {eventIcon(e.event_type)}
        </span>
        {!last ? (
          <span style={{ flex: 1, width: 1, background: "var(--border-subtle)", minHeight: 14 }} />
        ) : null}
      </div>
      <div style={{ flex: 1, minWidth: 0, paddingBottom: 16 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
          <span style={{ font: "var(--fw-medium) var(--fs-body) var(--font-sans)", color: "var(--text-primary)" }}>
            {e.summary ?? e.event_type}
          </span>
          <Badge mono style={{ color: "var(--text-faint)" }}>
            {e.event_type}
          </Badge>
        </div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            marginTop: 3,
            font: "var(--fs-meta) var(--font-mono)",
            color: "var(--text-faint)",
          }}
        >
          <span title="event sequence (timestamps land with the daemon enrichment)">#{e.seq}</span>
          <span>·</span>
          <span style={{ color: e.actor_type === "user" ? "var(--accent-ink)" : "var(--text-muted)" }}>
            {actor}
          </span>
        </div>
      </div>
    </div>
  );
}

/**
 * The Audit Trail view (ported from kit-views4.jsx AuditTimeline): the
 * append-only event timeline over the REAL AuditTrail projection — sticky
 * header (title · count badge · project/all scope toggle · Export), filter
 * chips by event namespace, icon-bubble rows joined by connectors. seq renders
 * where the prototype shows clock time (timestamps are a flagged projection
 * enrichment). Export is an FS mutation — disabled, not faked (§11.6).
 */
export function AuditTrail({
  events,
  projectId,
  projectName,
}: {
  events: AuditEventRow[];
  projectId: string | null;
  projectName: string;
}) {
  const [filter, setFilter] = useState("All");
  const [scope, setScope] = useState<"project" | "all">("project");
  const active = FILTERS.find((f) => f.label === filter) ?? FILTERS[0]!;
  const rows = events
    .filter((e) => scope === "all" || !projectId || e.project_id === projectId)
    .filter(
      (e) =>
        active.namespaces === null ||
        active.namespaces.includes(e.event_type.split(".")[0] ?? ""),
    )
    .toSorted((a, b) => b.seq - a.seq);

  return (
    <div
      aria-label="Audit Trail"
      style={{ height: "100%", overflowY: "auto", background: "var(--surface-canvas)" }}
    >
      <div
        style={{
          position: "sticky",
          top: 0,
          zIndex: 5,
          padding: "14px 16px 10px",
          background: "var(--surface-canvas)",
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
          <span aria-hidden="true" style={{ color: "var(--slate-ink)", display: "inline-flex" }}>
            <ScrollText size={18} />
          </span>
          <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)" }}>
            Audit trail
          </h1>
          <Badge mono style={{ color: "var(--text-muted)" }}>
            append-only · {rows.length} events
          </Badge>
          <div style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
            <div
              role="group"
              aria-label="Audit scope"
              style={{
                display: "flex",
                borderRadius: "var(--r-1)",
                overflow: "hidden",
                border: "1px solid var(--border-default)",
              }}
            >
              <button
                type="button"
                aria-pressed={scope === "project"}
                onClick={() => setScope("project")}
                style={{
                  padding: "4px 9px",
                  border: "none",
                  cursor: "pointer",
                  font: "var(--fw-medium) var(--fs-meta) var(--font-sans)",
                  background: scope === "project" ? "var(--accent-surface)" : "transparent",
                  color: scope === "project" ? "var(--accent-ink)" : "var(--text-muted)",
                }}
              >
                {projectName.length > 18 ? `${projectName.slice(0, 18)}…` : projectName}
              </button>
              <button
                type="button"
                aria-pressed={scope === "all"}
                onClick={() => setScope("all")}
                style={{
                  padding: "4px 9px",
                  border: "none",
                  borderLeft: "1px solid var(--border-default)",
                  cursor: "pointer",
                  font: "var(--fw-medium) var(--fs-meta) var(--font-sans)",
                  background: scope === "all" ? "var(--accent-surface)" : "transparent",
                  color: scope === "all" ? "var(--accent-ink)" : "var(--text-muted)",
                }}
              >
                All projects
              </button>
            </div>
            <span title="NDJSON export arrives with the daemon export contract">
              <Button variant="ghost" size="sm" icon={<Download size={14} />} disabled>
                Export
              </Button>
            </span>
          </div>
        </div>
        <div style={{ display: "flex", gap: 6 }} role="group" aria-label="Audit filters">
          {FILTERS.map((f) => (
            <button
              key={f.label}
              type="button"
              aria-pressed={filter === f.label}
              onClick={() => setFilter(f.label)}
              style={{
                padding: "4px 10px",
                borderRadius: 999,
                cursor: "pointer",
                border: `1px solid ${filter === f.label ? "var(--accent-line)" : "var(--border-default)"}`,
                background: filter === f.label ? "var(--accent-surface)" : "transparent",
                color: filter === f.label ? "var(--accent-ink)" : "var(--text-muted)",
                font: "var(--fw-medium) var(--fs-meta) var(--font-sans)",
              }}
            >
              {f.label}
            </button>
          ))}
        </div>
      </div>
      <div style={{ padding: "8px 16px 24px", maxWidth: 760 }} data-testid="audit-timeline-view">
        {rows.length === 0 ? (
          <div
            style={{
              font: "var(--fs-label) var(--font-sans)",
              color: "var(--text-muted)",
              padding: "20px 4px",
              display: "flex",
              alignItems: "center",
              gap: 8,
            }}
          >
            <ScrollText size={15} aria-hidden="true" style={{ color: "var(--text-faint)" }} />
            No audit events for this scope yet.
          </div>
        ) : (
          rows.map((e, i) => <AuditRow key={e.event_id} e={e} last={i === rows.length - 1} />)
        )}
      </div>
    </div>
  );
}
