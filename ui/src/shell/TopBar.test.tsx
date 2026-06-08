// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { TopBar } from "./TopBar";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";

afterEach(cleanup);

// onOpenSettings is REQUIRED (the Settings nav trigger is load-bearing) — passed
// explicitly per test, not defaulted, so a forgotten handler fails honestly.
const renderTopBar = (onOpenSettings: () => void) =>
  render(
    <TopBar
      projects={projectActivityFixture.rows}
      counts={{}}
      onOpenSettings={onOpenSettings}
    />,
  );

describe("TopBar", () => {
  it("topbar_settings_opens_settings", () => {
    // the §11.2 nav reconcile: the TopBar Settings control is the Settings entry
    const onOpenSettings = vi.fn();
    renderTopBar(onOpenSettings);
    fireEvent.click(screen.getByRole("button", { name: /settings/i }));
    expect(onOpenSettings).toHaveBeenCalledTimes(1);
  });

  it("icon_only_topbar_controls_have_accessible_names", () => {
    renderTopBar(() => {});
    // the glyph-only back/forward controls expose accessible names (§11.7) — the
    // kit Button's props are closed, so the name is a visually-hidden child label.
    expect(screen.getByRole("button", { name: /back/i })).toBeTruthy();
    expect(screen.getByRole("button", { name: /forward/i })).toBeTruthy();
  });
});
