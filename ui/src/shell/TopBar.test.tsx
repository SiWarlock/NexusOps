// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { type ComponentProps } from "react";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { TopBar } from "./TopBar";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";

afterEach(cleanup);

// onOpenSettings + the history props (onBack/onForward/canBack/canForward) are
// REQUIRED (load-bearing nav wiring) — supplied here with inert defaults so each
// test overrides only what it exercises and a forgotten handler fails honestly.
const renderTopBar = (overrides: Partial<ComponentProps<typeof TopBar>> = {}) =>
  render(
    <TopBar
      projects={projectActivityFixture.rows}
      counts={{}}
      onOpenSettings={() => {}}
      onBack={() => {}}
      onForward={() => {}}
      canBack={false}
      canForward={false}
      {...overrides}
    />,
  );

describe("TopBar", () => {
  it("topbar_settings_opens_settings", () => {
    // the §11.2 nav reconcile: the TopBar Settings control is the Settings entry
    const onOpenSettings = vi.fn();
    renderTopBar({ onOpenSettings });
    fireEvent.click(screen.getByRole("button", { name: /settings/i }));
    expect(onOpenSettings).toHaveBeenCalledTimes(1);
  });

  it("icon_only_topbar_controls_have_accessible_names", () => {
    renderTopBar();
    // the glyph-only back/forward controls expose accessible names (§11.7) — the
    // kit Button's props are closed, so the name is a visually-hidden child label.
    expect(screen.getByRole("button", { name: /back/i })).toBeTruthy();
    expect(screen.getByRole("button", { name: /forward/i })).toBeTruthy();
  });

  it("topbar_back_forward_disabled_at_boundaries", () => {
    // a named control is never a dead click — at a history boundary the direction
    // is `disabled` (AC + §11.6), enabled only when the move is available.
    const { unmount } = renderTopBar({ canBack: false, canForward: false });
    expect(screen.getByRole("button", { name: /back/i })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: /forward/i })).toHaveProperty(
      "disabled",
      true,
    );
    unmount();
    renderTopBar({ canBack: true, canForward: true });
    expect(screen.getByRole("button", { name: /back/i })).toHaveProperty("disabled", false);
    expect(screen.getByRole("button", { name: /forward/i })).toHaveProperty(
      "disabled",
      false,
    );
  });

  it("topbar_back_forward_invoke_handlers", () => {
    // the wiring is real: an enabled Back/Forward calls its handler.
    const onBack = vi.fn();
    const onForward = vi.fn();
    renderTopBar({ canBack: true, canForward: true, onBack, onForward });
    fireEvent.click(screen.getByRole("button", { name: /back/i }));
    expect(onBack).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: /forward/i }));
    expect(onForward).toHaveBeenCalledTimes(1);
  });
});
