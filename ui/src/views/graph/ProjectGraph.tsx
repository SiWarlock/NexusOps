import { useState } from "react";
import type {
  ProjectActivityRow,
  SessionRow,
  PullRequestRow,
} from "../../contracts/index";
import { StatusPill } from "../../status/StatusPill";
import { AttentionMarker } from "../../status/AttentionMarker";
import { buildProjectGraph, type GraphNode } from "./model";

type GraphView = "graph" | "list";

interface ProjectGraphProps {
  projectId: string;
  projects: ProjectActivityRow[];
  sessions: SessionRow[];
  pullRequests: PullRequestRow[];
}

// §11.6: a node's accessible name carries type + status + attention — HUMAN
// readable (the humanized descriptor labels carried by the model, never the raw
// enum / rank int). Built declaratively from the node; nothing re-derived here.
function nodeAccessibleName(node: GraphNode): string {
  return [node.type, node.label, node.statusLabel, `attention: ${node.attentionLabel}`]
    .filter(Boolean)
    .join(" · ");
}

/**
 * The Project Graph view (§11.6): a per-project graph with a Graph | List
 * toggle. The visual Graph is shown first; the List/table fallback is the
 * functionally-equivalent (same nodes + edges) and designated keyboard/AT
 * surface (per-node graph keyboard-operability + the focus ring are the 6.4
 * a11y pass). Renders ONLY the model's nodes (no invented state — forbidden #2);
 * reads its projection data through props (the Shell's gateway boundary).
 */
export function ProjectGraph({
  projectId,
  projects,
  sessions,
  pullRequests,
}: ProjectGraphProps) {
  const [view, setView] = useState<GraphView>("graph");

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

  return (
    <div className="project-graph" aria-label="Project Graph">
      <div className="project-graph__toolbar" role="group" aria-label="Graph view">
        <button
          type="button"
          aria-pressed={view === "graph"}
          onClick={() => setView("graph")}
        >
          Graph
        </button>
        <button
          type="button"
          aria-pressed={view === "list"}
          onClick={() => setView("list")}
        >
          List
        </button>
      </div>

      {/* Root-only project: the root node still renders below (it IS the
          project) — this is an additive "no children yet" note, NOT an
          exclusive empty screen that hides the graph. */}
      {isEmpty ? (
        <p className="project-graph__empty" data-testid="graph-empty">
          No sessions or pull requests in this project yet.
        </p>
      ) : null}

      {view === "graph" ? (
        // Visual view is non-AT-primary (role=img): the List/table is the
        // equivalent keyboard/AT surface (§11.6 OR-clause).
        <div
          className="project-graph__canvas"
          data-testid="graph-canvas"
          role="img"
          aria-label="Project graph (visual) — switch to List for an accessible table"
        >
          {graph.nodes.map((node) => (
            <div
              key={node.id}
              className="project-graph__node"
              data-item-id={node.id}
              data-node-type={node.type}
            >
              <AttentionMarker rank={node.attentionRank} variant="dot" />
              <span className="project-graph__node-label">{node.label}</span>
              {node.machine && node.status ? (
                <StatusPill machine={node.machine} status={node.status} size="xs" />
              ) : null}
            </div>
          ))}
        </div>
      ) : (
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
      )}
    </div>
  );
}
