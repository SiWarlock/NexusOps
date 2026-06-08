// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import {
  defaultActiveProject,
  resolveActiveProject,
  filterByActiveProject,
  ActiveProjectProvider,
  useActiveProject,
} from "./active-project";

afterEach(cleanup);

const p1 = { project_id: "project_fixture_1", name: "auth-service" };
const p2 = { project_id: "project_fixture_2", name: "billing" };

function Reader() {
  const { activeProjectId } = useActiveProject();
  return <span data-testid="active">{activeProjectId ?? "none"}</span>;
}

describe("active-project model", () => {
  it("default_active_project_first_or_null", () => {
    // first project is the default scope; null only at zero-projects
    expect(defaultActiveProject([p1, p2])).toBe("project_fixture_1");
    expect(defaultActiveProject([])).toBeNull();
  });

  it("filter_by_active_project", () => {
    const rows = [
      { id: "a", project_id: "project_fixture_1" },
      { id: "b", project_id: "project_fixture_2" },
      { id: "c", project_id: "project_fixture_1" },
      { id: "d" }, // unassigned (no project_id)
    ];
    // active id → only that project's rows; an unassigned row is excluded (it
    // belongs to no project, so not in the active scope)
    expect(filterByActiveProject(rows, "project_fixture_1").map((r) => r.id)).toEqual(["a", "c"]);
    // null active (no scope) → all rows unchanged (the no-active treatment = unscoped)
    expect(filterByActiveProject(rows, null)).toEqual(rows);
  });

  it("resolve_active_project_guards_stale_id", () => {
    // a valid raw pick is kept; a stale pick (project removed) re-scopes to the
    // default (first); no projects → null
    expect(resolveActiveProject([p1, p2], "project_fixture_2")).toBe("project_fixture_2");
    expect(resolveActiveProject([p1], "project_fixture_2")).toBe("project_fixture_1");
    expect(resolveActiveProject([], "project_fixture_2")).toBeNull();
    // null raw → the default
    expect(resolveActiveProject([p1, p2], null)).toBe("project_fixture_1");
  });

  it("active_project_context_propagates", () => {
    render(
      <ActiveProjectProvider
        value={{ activeProjectId: "project_fixture_2", setActiveProject: () => {} }}
      >
        <Reader />
      </ActiveProjectProvider>,
    );
    // the provider exposes the active id to consumers (mirrors ReadOnlyProvider)
    expect(screen.getByTestId("active").textContent).toBe("project_fixture_2");
  });
});
