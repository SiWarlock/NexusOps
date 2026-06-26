// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { ProjectGraph, parentLabels } from "./ProjectGraph";
import { buildProjectGraph, type GraphNode, type ProjectGraphModel } from "./model";
import { sessionPageFixture } from "../../projections/fixtures/proj_session";
import { pullRequestFixture } from "../../projections/fixtures/proj_pull_request";
import { projectActivityFixture } from "../../projections/fixtures/proj_project_activity";

afterEach(cleanup);

const base = {
  projects: projectActivityFixture.rows,
  sessions: sessionPageFixture.rows,
  pullRequests: pullRequestFixture.rows,
};
const renderGraph = (projectId: string) =>
  render(<ProjectGraph projectId={projectId} {...base} />);
const toList = () => fireEvent.click(screen.getByRole("button", { name: /list/i }));

describe("ProjectGraph view", () => {
  it("defaults_to_graph_and_toggles_to_list", () => {
    renderGraph("project_fixture_1");
    // Graph|List toggle, Graph shown first (§11.6) — no table yet
    expect(screen.getByTestId("graph-canvas")).toBeTruthy();
    expect(screen.queryByTestId("graph-table")).toBeNull();
    // activating List switches to the equivalent table fallback
    toList();
    expect(screen.getByTestId("graph-table")).toBeTruthy();
  });

  it("list_fallback_has_a_row_per_node", () => {
    const g = buildProjectGraph({ projectId: "project_fixture_1", ...base });
    renderGraph("project_fixture_1");
    toList();
    const rows = screen
      .getByTestId("graph-table")
      .querySelectorAll("tbody tr[data-item-id]");
    // functional equivalence: exactly one row per graph node (OBS-6)
    expect(rows).toHaveLength(g.nodes.length);
  });

  it("list_fallback_represents_every_edge", () => {
    const g = buildProjectGraph({ projectId: "project_fixture_1", ...base });
    const rootLabel = g.nodes.find((n) => n.type === "project")?.label;
    renderGraph("project_fixture_1");
    toList();
    const table = screen.getByTestId("graph-table");
    // every edge represented: each child row names its parent (the root label)
    for (const e of g.edges) {
      const cell = table.querySelector(
        `[data-item-id="${e.to}"] .graph-table__parent`,
      );
      expect(cell?.textContent).toBe(rootLabel);
    }
    // the root row has no parent
    const rootParent = table.querySelector(
      '[data-item-id="project:project_fixture_1"] .graph-table__parent',
    );
    expect(rootParent?.textContent).toBe("—");
  });

  it("node_accessible_name_includes_type_status_attention", () => {
    renderGraph("project_fixture_1");
    toList();
    const row = screen
      .getByTestId("graph-table")
      .querySelector('[data-item-id="session:session_fixture_2"]');
    // §11.6: the node's accessible name carries type + status + attention, and is
    // HUMAN-readable — the humanized descriptor label, not the raw enum / rank int.
    const name = row?.getAttribute("aria-label") ?? "";
    expect(name).toContain("session"); // type
    expect(name).toContain("Waiting on permission"); // humanized status (describeStatus)
    expect(name).toContain("needs-attention"); // human attention descriptor (triage bucket)
    expect(name).not.toMatch(/attention[:\s]*4\b/i); // not the bare rank int
    // and the status is visibly rendered (StatusPill), not aria-only
    expect(
      row?.querySelector('[data-status="waiting_on_permission"]'),
    ).not.toBeNull();
    expect(row?.querySelector("[data-level]")).not.toBeNull();
  });

  it("renders_only_projection_nodes", () => {
    const g = buildProjectGraph({ projectId: "project_fixture_1", ...base });
    renderGraph("project_fixture_1");
    toList();
    const ids = [
      ...screen
        .getByTestId("graph-table")
        .querySelectorAll("tbody tr[data-item-id]"),
    ]
      .map((r) => r.getAttribute("data-item-id"))
      .toSorted();
    // rendered node set === model node set — no invented nodes (forbidden #2)
    expect(ids).toEqual(g.nodes.map((n) => n.id).toSorted());
  });

  it("empty_graph_shows_explicit_empty_state", () => {
    // project_fixture_3 is root-only (activity-free) → explicit empty state
    renderGraph("project_fixture_3");
    expect(screen.getByTestId("graph-empty")).toBeTruthy();
  });

  it("graph_roots_at_active_project", () => {
    // rooting at a NON-first project (billing = project_fixture_2) roots the graph
    // THERE — not a hardcoded projects[0] (resolves the 6.3b Q3 graph project-source).
    renderGraph("project_fixture_2");
    toList();
    const table = screen.getByTestId("graph-table");
    expect(table.querySelector('[data-item-id="project:project_fixture_2"]')).not.toBeNull();
    expect(table.querySelector('[data-item-id="project:project_fixture_1"]')).toBeNull();
  });

  it("graph_zero_or_no_active_shows_guard", () => {
    // no active project (empty projectId) / no projects → the explicit no-projects
    // guard, NOT an empty-pid root (resolves the 6.3b zero-projects gap); distinct
    // from the per-project empty state.
    render(<ProjectGraph projectId="" projects={[]} sessions={[]} pullRequests={[]} />);
    expect(screen.getByTestId("graph-no-project")).toBeTruthy();
    expect(screen.queryByTestId("graph-canvas")).toBeNull();
    expect(screen.queryByTestId("graph-empty")).toBeNull();
  });
});

// ─── ui-076 (P6.6) — parentLabels memo: O(n²) per-row double-.find() → O(n) precomputed map ──────
// The `parentLabels(graph)` pure helper replaces the per-row scan. It builds, once, a complete
// childId→parentLabel resolution for EVERY node (root/missing-parent → "—"), read O(1) per row by the
// Contained-by column. The existing `list_fallback_represents_every_edge` render test stays the
// behavior + wiring pin end-to-end; these pin the helper directly (incl. the dangling-edge defensive
// "—" the rendered path can't reach — buildProjectGraph always includes the root). [[31]] (§18 bench guards it).
// a minimal GraphNode (the helper reads only id + label; type/rank are irrelevant to it).
const mk = (id: string, label: string): GraphNode => ({
  id,
  type: "session",
  label,
  attentionRank: 0,
  attentionLabel: "settled",
});

describe("ProjectGraph parentLabels memo (ui-076, §18 perf-hardening)", () => {
  it("parent_label_matches_for_all_nodes", () => {
    // spec(§18) — perf-hardening must not change output: the memo === the prior O(n²) parentLabelOf for
    // EVERY node, INCLUDING a child whose parent is a NON-root node (distinct parents — the case a
    // "map every child to the root" bug would silently pass under a star graph).
    const graph: ProjectGraphModel = {
      projectId: "p",
      nodes: [mk("project:p", "Root"), mk("session:a", "Alpha"), mk("pull_request:b", "Beta")],
      edges: [
        { kind: "contains", from: "project:p", to: "session:a" }, // a's parent = Root
        { kind: "contains", from: "session:a", to: "pull_request:b" }, // b's parent = Alpha (NON-root)
      ],
    };
    // the prior O(n²) reference logic this memo must preserve byte-for-byte (ProjectGraph.tsx:167-171).
    const oldParentLabelOf = (n: GraphNode): string => {
      const e = graph.edges.find((x) => x.to === n.id);
      if (!e) return "—";
      return graph.nodes.find((x) => x.id === e.from)?.label ?? "—";
    };
    const labels = parentLabels(graph);
    for (const n of graph.nodes) {
      expect(labels.get(n.id)).toBe(oldParentLabelOf(n));
    }
    expect(labels.get("pull_request:b")).toBe("Alpha"); // distinct-parent: resolves to Alpha, NOT the root
  });

  it("parent_label_root_node_renders_dash", () => {
    // spec(§11.6) — a node with no incoming edge (the project root) → "—".
    const graph: ProjectGraphModel = {
      projectId: "p",
      nodes: [mk("project:p", "Root"), mk("session:a", "Alpha")],
      edges: [{ kind: "contains", from: "project:p", to: "session:a" }],
    };
    expect(parentLabels(graph).get("project:p")).toBe("—");
  });

  it("parent_label_missing_parent_renders_dash", () => {
    // spec(§11.6) — an edge whose `from` node is ABSENT from nodes → "—" (the defensive ?? "—" guard,
    // preserved through the memo). Unreachable via buildProjectGraph (root always present) → pinned here.
    const graph: ProjectGraphModel = {
      projectId: "p",
      nodes: [mk("session:a", "Alpha")],
      edges: [{ kind: "contains", from: "ghost:absent", to: "session:a" }],
    };
    expect(parentLabels(graph).get("session:a")).toBe("—");
  });

  it("parent_label_duplicate_edges_first_wins", () => {
    // The memo preserves the prior `.find` FIRST-WINS semantics: when two edges share a `to`, the FIRST
    // incoming edge's parent is used. Moot for buildProjectGraph's unique-`to` contains edges, but pins
    // the JSDoc fidelity claim against a future build-loop refactor (e.g. an accidental last-wins).
    const graph: ProjectGraphModel = {
      projectId: "p",
      nodes: [mk("project:p", "Root"), mk("session:a", "Alpha"), mk("pull_request:c", "Child")],
      edges: [
        { kind: "contains", from: "project:p", to: "pull_request:c" }, // first → Root (wins)
        { kind: "contains", from: "session:a", to: "pull_request:c" }, // second → ignored
      ],
    };
    expect(parentLabels(graph).get("pull_request:c")).toBe("Root");
  });
});
