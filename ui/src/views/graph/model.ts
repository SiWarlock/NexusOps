// The Project Graph model (§11.6 / §4.2): a pure derivation of one project's
// graph — the project root + a node per session + per PR scoped to it, with a
// `contains` edge root→each child. Edges are DERIVED from the projections'
// project_id linkage, never invented state (§4.2 / forbidden #2). Node attention
// + labels come from the single descriptor source (LESSONS §5/§6/§8) — never
// re-declared at a call site; session/PR nodes route through the L1 mappers.
import type {
  ProjectActivityRow,
  SessionRow,
  PullRequestRow,
} from "../../contracts/index";
import {
  triageBucket,
  type AttentionRank,
  type TriageBucket,
} from "../../status/attention";
import { describeStatus } from "../../status/descriptors";
import {
  toSessionItems,
  toPrItems,
  type ProjectionItem,
} from "../../projections/items";

/** GraphNode kinds (§11.7 vocab — MVP subset; team/task/workflow deferred). */
export type GraphNodeType = "project" | "session" | "pull_request";

export interface GraphNode {
  /** Globally-unique node identity + DOM locator: `${type}:${rawId}` (LESSONS §8). */
  id: string;
  type: GraphNodeType;
  /** Status-machine for the StatusPill (machine,status) path; undefined for the root. */
  machine?: string;
  label: string;
  /** Frozen-enum status string; undefined for the root (projects have no status machine). */
  status?: string;
  /** Humanized status label (descriptor table, single source); undefined for the root. */
  statusLabel?: string;
  attentionRank: AttentionRank;
  /** Human attention descriptor (the §5.2 triage bucket) for the accessible name. */
  attentionLabel: TriageBucket;
}

export interface GraphEdge {
  kind: "contains";
  /** Parent node id. */
  from: string;
  /** Child node id. */
  to: string;
}

export interface ProjectGraphModel {
  projectId: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
}

const PROJECT_ROOT_RANK: AttentionRank = 0;

// One describeStatus call yields BOTH the attention rank and the humanized status
// label — the single source; the view never re-derives either.
function itemToNode(type: GraphNodeType, item: ProjectionItem): GraphNode {
  const descriptor = describeStatus(item.machine, item.status);
  return {
    id: `${type}:${item.id}`,
    type,
    machine: item.machine,
    label: item.label,
    status: item.status,
    statusLabel: descriptor.label,
    attentionRank: descriptor.attentionRank,
    attentionLabel: triageBucket(descriptor.attentionRank),
  };
}

export function buildProjectGraph(input: {
  projectId: string;
  projects: ProjectActivityRow[];
  sessions: SessionRow[];
  pullRequests: PullRequestRow[];
}): ProjectGraphModel {
  const { projectId } = input;
  const project = input.projects.find((p) => p.project_id === projectId);

  const rootId = `project:${projectId}`;
  const root: GraphNode = {
    id: rootId,
    type: "project",
    label: project?.name ?? projectId,
    attentionRank: PROJECT_ROOT_RANK,
    attentionLabel: triageBucket(PROJECT_ROOT_RANK),
  };

  // Scope to this project FIRST, then route through the L1 mappers (no inline re-map — §8).
  const sessionNodes = toSessionItems(
    input.sessions.filter((s) => s.project_id === projectId),
  ).map((it) => itemToNode("session", it));

  const prNodes = toPrItems(
    input.pullRequests.filter((pr) => pr.project_id === projectId),
  ).map((it) => itemToNode("pull_request", it));

  const children = [...sessionNodes, ...prNodes];
  const edges: GraphEdge[] = children.map((c) => ({
    kind: "contains",
    from: rootId,
    to: c.id,
  }));

  return { projectId, nodes: [root, ...children], edges };
}
