// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { ProjectSwitcher } from "./ProjectSwitcher";
import { ActiveProjectProvider } from "./active-project";
import type { ProjectActivityRow } from "../contracts/index";

afterEach(cleanup);

const projects = [
  { project_id: "project_fixture_1", name: "auth-service" },
  { project_id: "project_fixture_2", name: "billing" },
];

function renderSwitcher(
  activeProjectId: string | null,
  setActiveProject: (id: string) => void = () => {},
  projectList: ProjectActivityRow[] = projects,
) {
  return render(
    <ActiveProjectProvider value={{ activeProjectId, setActiveProject }}>
      <ProjectSwitcher projects={projectList} counts={{}} />
    </ActiveProjectProvider>,
  );
}

// L1 — popover shell (trigger + listbox structure + click-select-close + zero-disabled).
describe("ProjectSwitcher dropdown — popover shell (L1)", () => {
  it("trigger_shows_active_project_and_toggles", () => {
    renderSwitcher("project_fixture_2");
    const trigger = screen.getByRole("button", { name: /billing/i });
    // a button-triggered listbox popover (WAI-ARIA): closed by default
    expect(trigger.getAttribute("aria-haspopup")).toBe("listbox");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByRole("listbox")).toBeNull();
    // clicking opens the popover
    fireEvent.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("listbox")).toBeTruthy();
  });

  it("listbox_lists_projects_with_selected", () => {
    renderSwitcher("project_fixture_2");
    fireEvent.click(screen.getByRole("button", { name: /billing/i }));
    const options = screen.getAllByRole("option");
    // one option per project; the active one is aria-selected + ✓ glyph+label
    expect(options).toHaveLength(projects.length);
    const active = options.find((o) => o.getAttribute("aria-selected") === "true");
    expect(active?.getAttribute("data-project-id")).toBe("project_fixture_2");
    expect(active?.querySelector(".project-switcher__active")?.textContent).toBe(
      "✓ Active",
    );
    // the inactive option is not selected (never color alone — aria-selected, not hue)
    const inactive = options.find((o) => o.getAttribute("data-project-id") === "project_fixture_1");
    expect(inactive?.getAttribute("aria-selected")).toBe("false");
  });

  it("option_click_selects_and_closes", () => {
    const setActiveProject = vi.fn();
    renderSwitcher("project_fixture_1", setActiveProject);
    fireEvent.click(screen.getByRole("button", { name: /auth-service/i }));
    fireEvent.click(screen.getByRole("option", { name: /billing/i }));
    // selection behavior preserved (setActiveProject) + the popover closes
    expect(setActiveProject).toHaveBeenCalledWith("project_fixture_2");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("zero_projects_trigger_disabled", () => {
    renderSwitcher(null, () => {}, []);
    // wire-or-disable (§11.6): no projects → a disabled trigger, never a dead click
    const trigger = screen.getByRole("button");
    expect(trigger).toHaveProperty("disabled", true);
    expect(trigger.textContent).toContain("No project");
  });
});
