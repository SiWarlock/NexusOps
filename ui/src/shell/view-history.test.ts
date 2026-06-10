import { describe, it, expect } from "vitest";
import {
  initialViewHistory,
  viewHistoryReducer,
  currentView,
  canGoBack,
  canGoForward,
  type ViewHistoryState,
  type ViewName,
} from "./view-history";

// Thin action helpers — keep the assertions about the invariant, not the wire shape.
const nav = (s: ViewHistoryState, view: ViewName) =>
  viewHistoryReducer(s, { type: "navigate", view });
const back = (s: ViewHistoryState) => viewHistoryReducer(s, { type: "back" });
const forward = (s: ViewHistoryState) => viewHistoryReducer(s, { type: "forward" });

describe("viewHistoryReducer", () => {
  it("history_initial_is_command_no_back_no_forward", () => {
    // §11.2 default content view + a fresh history has no neighbors either side.
    expect(currentView(initialViewHistory)).toBe("command");
    expect(canGoBack(initialViewHistory)).toBe(false);
    expect(canGoForward(initialViewHistory)).toBe(false);
  });

  it("navigate_to_new_view_enables_back", () => {
    // Forward nav to a DIFFERENT view pushes a back-step; nothing ahead yet.
    const s = nav(initialViewHistory, "graph");
    expect(currentView(s)).toBe("graph");
    expect(canGoBack(s)).toBe(true);
    expect(canGoForward(s)).toBe(false);
  });

  it("navigate_to_current_view_is_noop", () => {
    // Collapse duplicate entries — re-selecting the active view is not a back-step
    // (Step-2.5 Q2). State is unchanged value-wise.
    const s = nav(initialViewHistory, "command");
    expect(s).toEqual(initialViewHistory);
  });

  it("back_then_forward_round_trips", () => {
    // back/forward move the cursor, never the stack.
    const g = nav(initialViewHistory, "graph");
    const b = back(g);
    expect(currentView(b)).toBe("command");
    expect(canGoForward(b)).toBe(true);
    expect(b.stack).toEqual(g.stack); // stack untouched by back()
    const f = forward(b);
    expect(currentView(f)).toBe("graph");
    expect(canGoBack(f)).toBe(true);
    expect(canGoForward(f)).toBe(false);
    expect(f.stack).toEqual(g.stack); // stack untouched by forward()
  });

  it("navigate_after_back_truncates_forward", () => {
    // Classic browser forward-discard: a new nav from a back position strands +
    // truncates the forward entry.
    const g = nav(initialViewHistory, "graph");
    const b = back(g); // at "command", "graph" stranded ahead
    const s = nav(b, "terminal");
    expect(currentView(s)).toBe("terminal");
    expect(canGoForward(s)).toBe(false);
    expect(s.stack).toEqual(["command", "terminal"]); // graph discarded
    expect(forward(s)).toEqual(s); // forward is a no-op now
  });

  it("navigate_to_current_view_mid_history_collapses_and_preserves_forward", () => {
    // Re-selecting the ALREADY-ACTIVE view from a back position is a pure no-op:
    // the cursor doesn't move AND the stranded forward entry survives (collapse is
    // not a navigation, so it must NOT truncate — distinct from forward-discard).
    const g = nav(initialViewHistory, "graph");
    const b = back(g); // at "command", "graph" stranded ahead, canForward true
    const s = nav(b, "command"); // click the already-active Command Center
    expect(s).toEqual(b); // cursor + stack unchanged
    expect(canGoForward(s)).toBe(true); // "graph" not discarded by a no-op
    expect(currentView(forward(s))).toBe("graph"); // forward still reaches it
  });

  it("back_at_start_and_forward_at_end_are_noops", () => {
    // Idempotent boundaries — never throw, never an out-of-bounds cursor.
    expect(back(initialViewHistory)).toEqual(initialViewHistory);
    const tip = nav(initialViewHistory, "graph");
    expect(forward(tip)).toEqual(tip);
  });

  it("settings_participates_in_history", () => {
    // Settings is a uniform history entry even though it's REACHED via the TopBar,
    // not the view-switch (§11.2 governs how it's reached, not whether it's in
    // history — Step-2.5 Q4).
    const s = nav(initialViewHistory, "settings");
    expect(currentView(s)).toBe("settings");
    expect(currentView(back(s))).toBe("command");
  });
});
