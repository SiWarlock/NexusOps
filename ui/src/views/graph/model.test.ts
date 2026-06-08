import { describe, it, expect } from "vitest";
import { buildProjectGraph } from "./model";
import { sessionPageFixture } from "../../projections/fixtures/proj_session";
import { pullRequestFixture } from "../../projections/fixtures/proj_pull_request";
import { projectActivityFixture } from "../../projections/fixtures/proj_project_activity";

const base = {
  projects: projectActivityFixture.rows,
  sessions: sessionPageFixture.rows,
  pullRequests: pullRequestFixture.rows,
};

describe("buildProjectGraph", () => {
  it("builds_project_root_plus_session_and_pr_nodes", () => {
    const g = buildProjectGraph({ projectId: "project_fixture_1", ...base });
    // a project-root node (no status), labelled from the ProjectActivity row
    const root = g.nodes.find((n) => n.type === "project");
    expect(root).toMatchObject({
      id: "project:project_fixture_1",
      type: "project",
      label: "auth-service",
    });
    expect(root?.status).toBeUndefined();
    // root + its 3 sessions + its 2 PRs
    expect(g.nodes.map((n) => n.type).toSorted()).toEqual([
      "project",
      "pull_request",
      "pull_request",
      "session",
      "session",
      "session",
    ]);
    // every non-root node carries label + status + a numeric attentionRank
    for (const node of g.nodes.filter((n) => n.type !== "project")) {
      expect(node.label).toBeTruthy();
      expect(node.status).toBeTruthy();
      expect(typeof node.attentionRank).toBe("number");
    }
    // a concrete node pins the real fixture status flows through (not any string)
    expect(g.nodes.find((n) => n.id === "session:session_fixture_1")?.status).toBe(
      "active",
    );
    expect(g.nodes.find((n) => n.id === "pull_request:101")?.status).toBe("open");
  });

  it("scopes_nodes_to_the_given_project", () => {
    const g = buildProjectGraph({ projectId: "project_fixture_1", ...base });
    const ids = g.nodes.map((n) => n.id);
    // project_fixture_2's session + PR are excluded
    expect(ids).not.toContain("session:session_fixture_3");
    expect(ids).not.toContain("pull_request:201");
    // project_fixture_1's are present
    expect(ids).toContain("session:session_fixture_1");
    expect(ids).toContain("pull_request:101");
  });

  it("builds_contains_edges_root_to_each_child", () => {
    const g = buildProjectGraph({ projectId: "project_fixture_1", ...base });
    const children = g.nodes.filter((n) => n.type !== "project");
    // one contains edge root→each child; edge count === child count
    expect(g.edges).toHaveLength(children.length);
    for (const e of g.edges) {
      expect(e.kind).toBe("contains");
      expect(e.from).toBe("project:project_fixture_1");
    }
    // every child has exactly one incoming contains edge
    expect(g.edges.map((e) => e.to).toSorted()).toEqual(
      children.map((c) => c.id).toSorted(),
    );
  });

  it("node_attention_from_descriptor_table", () => {
    const g = buildProjectGraph({ projectId: "project_fixture_1", ...base });
    // waiting_on_permission → rank 4 via describeStatus (single source, not re-derived);
    // the same one call also carries the humanized status label + triage-bucket
    // attention label for the accessible name (never re-declared at a call site).
    const waiting = g.nodes.find((n) => n.id === "session:session_fixture_2");
    expect(waiting?.status).toBe("waiting_on_permission");
    expect(waiting?.attentionRank).toBe(4);
    expect(waiting?.statusLabel).toBe("Waiting on permission");
    expect(waiting?.attentionLabel).toBe("needs-attention");
    // the project root is rank 0 / settled, with no status label
    const root = g.nodes.find((n) => n.type === "project");
    expect(root?.attentionRank).toBe(0);
    expect(root?.statusLabel).toBeUndefined();
    expect(root?.attentionLabel).toBe("settled");
  });

  it("empty_project_has_root_only_no_edges", () => {
    // project_fixture_3 (docs-site) is activity-free
    const g = buildProjectGraph({ projectId: "project_fixture_3", ...base });
    expect(g.nodes).toHaveLength(1);
    expect(g.nodes[0]).toMatchObject({
      type: "project",
      id: "project:project_fixture_3",
      label: "docs-site",
    });
    expect(g.edges).toEqual([]);
  });
});
