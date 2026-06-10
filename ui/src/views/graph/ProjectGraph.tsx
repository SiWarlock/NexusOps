import { useEffect, useRef, useState, type ReactNode } from "react";
import { Filter, Maximize, Workflow } from "lucide-react";
import type {
  ProjectActivityRow,
  SessionRow,
  PullRequestRow,
  UsageRow,
} from "../../contracts/index";
import { Badge, Button, GraphNode as KitGraphNode } from "../../design-system/kit";
import { StatusPill, kitKindFor } from "../../status/StatusPill";
import { AttentionMarker } from "../../status/AttentionMarker";
import {
  sessionDisplayFixture,
  projectDisplayFixture,
  contextForSession,
} from "../../shell/display-meta";
import { buildProjectGraph, type GraphNode } from "./model";

type GraphView = "graph" | "list";

interface ProjectGraphProps {
  projectId: string;
  projects: ProjectActivityRow[];
  sessions: SessionRow[];
  pullRequests: PullRequestRow[];
  usage?: UsageRow[];
  /** Node activation opens the Inspector drawer (overlay) when provided. */
  onInspect?: (node: GraphNode) => void;
}

// §11.6: a node's accessible name carries type + status + attention — HUMAN
// readable (the humanized descriptor labels carried by the model, never the raw
// enum / rank int). Built declaratively from the node; nothing re-derived here.
function nodeAccessibleName(node: GraphNode): string {
  return [node.type, node.label, node.statusLabel, `attention: ${node.attentionLabel}`]
    .filter(Boolean)
    .join(" · ");
}

// Deterministic layered layout (the prototype's hand-placed geometry, derived):
// column 0 = the project root, column 1 = sessions, column 2 = PRs; rows pitch
// down each column; the root centers against the tallest column.
const COL_X: Record<string, number> = { project: 40, session: 290, pull_request: 560 };
const ROW_Y0 = 60;
const ROW_PITCH = 115;
const NODE_CENTER_X = 84; // kit GraphNode ~168px wide — edge anchors at center/edge
const NODE_CENTER_Y = 26;

interface PlacedNode {
  node: GraphNode;
  x: number;
  y: number;
}

function layout(nodes: GraphNode[]): PlacedNode[] {
  const sessions = nodes.filter((n) => n.type === "session");
  const prs = nodes.filter((n) => n.type === "pull_request");
  const root = nodes.find((n) => n.type === "project");
  const maxRows = Math.max(sessions.length, prs.length, 1);
  const placed: PlacedNode[] = [];
  if (root) {
    placed.push({
      node: root,
      x: COL_X.project!,
      y: ROW_Y0 + ((maxRows - 1) * ROW_PITCH) / 2,
    });
  }
  sessions.forEach((n, i) =>
    placed.push({ node: n, x: COL_X.session!, y: ROW_Y0 + i * ROW_PITCH }),
  );
  prs.forEach((n, i) =>
    placed.push({ node: n, x: COL_X.pull_request!, y: ROW_Y0 + i * ROW_PITCH }),
  );
  return placed;
}

// Model node type → kit GraphNode kind.
const KIT_KIND: Record<string, "project" | "session" | "pr"> = {
  project: "project",
  session: "session",
  pull_request: "pr",
};

/**
 * Focusability patch for the kit GraphNode: it hardcodes `role="button"` with
 * closed props (no tabIndex/aria pass-through), which would leave a clickable
 * node keyboard-unreachable (§11.6 MUST / the reachability audit). The wrapper
 * sets tabIndex + the accessible name at the DOM seam (Lesson §6 family) and
 * adds Enter/Space activation, making selection genuinely keyboard-operable.
 * (Upstream kit improvement flagged: expose tabIndex/aria-label on GraphNode.)
 */
function FocusableNode({
  label,
  onActivate,
  children,
}: {
  label: string;
  onActivate: () => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const el = ref.current?.querySelector<HTMLElement>('[role="button"]');
    if (el) {
      el.tabIndex = 0;
      el.setAttribute("aria-label", label);
    }
  });
  return (
    <div
      ref={ref}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onActivate();
        }
      }}
    >
      {children}
    </div>
  );
}

/**
 * The Project Graph view (ported from kit-views2.jsx ProjectGraph): the dotted
 * observability canvas — sticky header (title · project · node count · Filter ·
 * Fit), curved contains-edges, and kit GraphNode cards (kind chrome + status
 * ring + ctx meta) — with the Graph | List toggle. The List/table fallback is
 * the functionally-equivalent designated keyboard/AT surface (§11.6 OR-clause).
 * Renders ONLY the model's nodes (no invented state — forbidden #2); node
 * subtitles/ctx come from the display side-map + the real Usage projection.
 * Filter/Fit are layout options without backing state yet — disabled, not faked.
 * Node-click selection highlights; the inspector drawer arrives with overlays.
 */
export function ProjectGraph({
  projectId,
  projects,
  sessions,
  pullRequests,
  usage = [],
  onInspect,
}: ProjectGraphProps) {
  const [view, setView] = useState<GraphView>("graph");
  const [sel, setSel] = useState<string | null>(null);

  // No active project (empty projectId) / no projects → an explicit no-projects
  // guard, NOT an empty-pid root (6.3b zero-projects gap). Distinct from the
  // per-project empty state below (a selected project with no activity yet).
  if (!projectId) {
    return (
      <div className="project-graph" aria-label="Project Graph">
        <p className="project-graph__no-project" data-testid="graph-no-project">
          No project selected — add or select a project to see its graph.
        </p>
      </div>
    );
  }

  const graph = buildProjectGraph({ projectId, projects, sessions, pullRequests });
  const projectName =
    projects.find((p) => p.project_id === projectId)?.name ?? projectId;

  // The Contained-by column represents the edge set: each child names its parent.
  // O(n²) across the table (a scan per row) — fine for MVP node counts; when real
  // subscriptions + larger graphs land, memoize a childId→parent map (the graph-
  // render perf budget is tracked in MVP_TASKS Carry-forward).
  const parentLabelOf = (node: GraphNode): string => {
    const edge = graph.edges.find((e) => e.to === node.id);
    if (!edge) return "—";
    return graph.nodes.find((n) => n.id === edge.from)?.label ?? "—";
  };

  const isEmpty = graph.edges.length === 0;
  const placed = layout(graph.nodes);
  const posById = new Map(placed.map((p) => [p.node.id, p]));
  const canvasW = 820;
  const canvasH =
    ROW_Y0 + Math.max(...placed.map((p) => p.y), ROW_Y0) + ROW_PITCH;

  // Node decorations: session subtitle (harness · profile) + ctx meta from the
  // REAL usage projection ("ctx unknown" stays honest — §9.1 forbidden #4).
  const subtitleOf = (node: GraphNode): string | undefined => {
    if (node.type === "project") return projectDisplayFixture[projectId]?.repo;
    if (node.type === "session") {
      const d = sessionDisplayFixture[node.id.split(":")[1] ?? ""];
      return d?.harness;
    }
    return undefined;
  };
  const metaOf = (node: GraphNode): string[] | undefined => {
    if (node.type !== "session") return undefined;
    const rawId = node.id.split(":")[1] ?? "";
    const ctx = contextForSession(usage, rawId);
    if (!ctx) return undefined;
    return [ctx.pct === null ? "ctx unknown" : `${ctx.pct}% ctx`];
  };

  return (
    <div
      className="project-graph"
      aria-label="Project Graph"
      style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}
    >
      {/* sticky header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          padding: "12px 16px",
          flex: "none",
        }}
      >
        <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)" }}>
          Project Graph
        </h1>
        <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-faint)" }}>
          {projectName}
        </span>
        <Badge tone="neutral" mono>
          {graph.nodes.length} nodes
        </Badge>
        <div
          className="project-graph__toolbar"
          role="group"
          aria-label="Graph view"
          style={{ display: "flex", gap: 6, marginLeft: 12 }}
        >
          <button
            type="button"
            aria-pressed={view === "graph"}
            onClick={() => setView("graph")}
            className="graph-toggle"
          >
            Graph
          </button>
          <button
            type="button"
            aria-pressed={view === "list"}
            onClick={() => setView("list")}
            className="graph-toggle"
          >
            List
          </button>
        </div>
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          <span title="Graph filters arrive with the inspector slice">
            <Button variant="ghost" size="sm" icon={<Filter size={14} />} disabled>
              Filter
            </Button>
          </span>
          <span title="Fit-to-view arrives with the pan/zoom canvas">
            <Button variant="secondary" size="sm" icon={<Maximize size={14} />} disabled>
              Fit
            </Button>
          </span>
        </div>
      </div>

      {view === "graph" ? (
        <div
          style={{
            position: "relative",
            flex: 1,
            minHeight: 0,
            background: "var(--graph-canvas)",
            overflow: "auto",
            backgroundImage: "radial-gradient(var(--graph-grid) 1px, transparent 1px)",
            backgroundSize: "22px 22px",
          }}
        >
          {isEmpty ? (
            <div
              data-testid="graph-empty"
              style={{
                position: "absolute",
                inset: 0,
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                gap: 10,
                color: "var(--text-muted)",
              }}
            >
              <Workflow size={26} aria-hidden="true" style={{ color: "var(--text-faint)" }} />
              <div style={{ font: "var(--fw-medium) var(--fs-body) var(--font-sans)" }}>
                No observability graph for {projectName} yet
              </div>
              <div style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)" }}>
                Start a session or run a workflow to populate it.
              </div>
            </div>
          ) : null}
          {/* The visual canvas: nodes are keyboard-operable (FocusableNode);
              the List/table remains the equivalent structured AT surface
              (§11.6 OR-clause). */}
          <div
            data-testid="graph-canvas"
            role="group"
            aria-label="Project graph (visual) — the List view is the equivalent table"
            style={{ position: "relative", width: canvasW, height: canvasH }}
          >
            <svg
              style={{ position: "absolute", inset: 0, pointerEvents: "none" }}
              width={canvasW}
              height={canvasH}
              aria-hidden="true"
            >
              {graph.edges.map((e, i) => {
                const a = posById.get(e.from);
                const b = posById.get(e.to);
                if (!a || !b) return null;
                const x1 = a.x + NODE_CENTER_X;
                const y1 = a.y + NODE_CENTER_Y;
                const x2 = b.x;
                const y2 = b.y + NODE_CENTER_Y;
                const mx = (x1 + x2) / 2;
                return (
                  <path
                    key={i}
                    d={`M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`}
                    fill="none"
                    stroke="var(--graph-edge-active)"
                    strokeWidth="1.5"
                  />
                );
              })}
            </svg>
            {placed.map(({ node, x, y }) => (
              <div
                key={node.id}
                style={{ position: "absolute", left: x, top: y }}
                data-item-id={node.id}
                data-node-type={node.type}
              >
                <FocusableNode
                  label={nodeAccessibleName(node)}
                  onActivate={() => {
                    setSel(node.id);
                    onInspect?.(node);
                  }}
                >
                  <KitGraphNode
                    kind={KIT_KIND[node.type] ?? "project"}
                    title={node.label}
                    subtitle={subtitleOf(node)}
                    status={
                      node.machine && node.status
                        ? kitKindFor(node.machine, node.status)
                        : undefined
                    }
                    beacon={node.attentionRank === 5}
                    meta={metaOf(node)}
                    selected={sel === node.id}
                    onClick={() => {
                      setSel(node.id);
                      onInspect?.(node);
                    }}
                  />
                </FocusableNode>
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
          {isEmpty ? (
            <p
              className="project-graph__empty"
              data-testid="graph-empty"
              style={{ padding: "8px 16px", color: "var(--text-muted)", font: "var(--fs-meta) var(--font-sans)" }}
            >
              No sessions or pull requests in this project yet.
            </p>
          ) : null}
          <table className="project-graph__table" data-testid="graph-table">
            <thead>
              <tr>
                <th scope="col">Type</th>
                <th scope="col">Name</th>
                <th scope="col">Status</th>
                <th scope="col">Attention</th>
                <th scope="col">Contained by</th>
              </tr>
            </thead>
            <tbody>
              {graph.nodes.map((node) => (
                <tr
                  key={node.id}
                  data-item-id={node.id}
                  data-node-type={node.type}
                  aria-label={nodeAccessibleName(node)}
                >
                  <td>{node.type}</td>
                  <td className="project-graph__name">{node.label}</td>
                  <td>
                    {node.machine && node.status ? (
                      <StatusPill
                        machine={node.machine}
                        status={node.status}
                        size="xs"
                      />
                    ) : (
                      "—"
                    )}
                  </td>
                  <td>
                    <AttentionMarker rank={node.attentionRank} variant="dot" />
                  </td>
                  <td className="graph-table__parent">{parentLabelOf(node)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
