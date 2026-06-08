// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { ProjectGraph } from "./ProjectGraph";
import { buildProjectGraph } from "./model";
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
});
