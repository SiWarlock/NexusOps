// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { ProjectSwitcher } from "./ProjectSwitcher";
import { ActiveProjectProvider } from "./active-project";

afterEach(cleanup);

const projects = [
  { project_id: "project_fixture_1", name: "auth-service" },
  { project_id: "project_fixture_2", name: "billing" },
];

function renderSwitcher(activeProjectId: string | null, setActiveProject = () => {}) {
  return render(
    <ActiveProjectProvider value={{ activeProjectId, setActiveProject }}>
      <ProjectSwitcher projects={projects} counts={{}} />
    </ActiveProjectProvider>,
  );
}

describe("ProjectSwitcher (single-select)", () => {
  it("selecting_a_project_sets_active", () => {
    const setActiveProject = vi.fn();
    renderSwitcher(null, setActiveProject);
    // clicking billing selects it (completes the previously-inert switcher)
    fireEvent.click(screen.getByRole("button", { name: /billing/i }));
    expect(setActiveProject).toHaveBeenCalledWith("project_fixture_2");
  });

  it("active_project_marked_never_color_alone", () => {
    renderSwitcher("project_fixture_2");
    // names stay visible + keyboard-reachable (don't regress the name-visible tests)
    expect(screen.getByText("auth-service")).toBeTruthy();
    expect(screen.getByText("billing")).toBeTruthy();
    // the active project: a glyph+label indicator + aria-pressed (NOT color alone, §11.6)
    const active = screen.getByRole("button", { name: /billing/i });
    expect(active.getAttribute("aria-pressed")).toBe("true");
    // the indicator is a dedicated glyph+label element (a text channel, not color)
    expect(active.querySelector(".project-switcher__active")?.textContent).toBe("✓ Active");
    // the inactive project is not pressed
    expect(
      screen.getByRole("button", { name: /auth-service/i }).getAttribute("aria-pressed"),
    ).toBe("false");
  });
});
