import { describe, it, expect } from "vitest";
import { nextTabIndex, isRovingKey } from "./roving";

// nextTabIndex re-homed from views/settings/roving.ts to the shared a11y layer
// (slice 5) — a PURE MOVE, no behavior change. These are the slice-2 tests,
// re-pointed; they pin that the helper is byte-identical after the move.
const COUNT = 5;

describe("nextTabIndex (roving tabindex — shared a11y primitive)", () => {
  it("roving_arrow_right_wraps", () => {
    expect(nextTabIndex(1, COUNT, "ArrowRight")).toBe(2);
    expect(nextTabIndex(COUNT - 1, COUNT, "ArrowRight")).toBe(0);
  });

  it("roving_arrow_left_wraps", () => {
    expect(nextTabIndex(3, COUNT, "ArrowLeft")).toBe(2);
    expect(nextTabIndex(0, COUNT, "ArrowLeft")).toBe(COUNT - 1);
  });

  it("roving_home_end", () => {
    expect(nextTabIndex(2, COUNT, "Home")).toBe(0);
    expect(nextTabIndex(2, COUNT, "End")).toBe(COUNT - 1);
    expect(nextTabIndex(0, COUNT, "Home")).toBe(0);
    expect(nextTabIndex(COUNT - 1, COUNT, "End")).toBe(COUNT - 1);
  });

  it("roving_other_key_is_unchanged", () => {
    expect(nextTabIndex(2, COUNT, "Enter")).toBe(2);
    expect(nextTabIndex(2, COUNT, " ")).toBe(2);
    expect(nextTabIndex(2, COUNT, "a")).toBe(2);
  });

  it("roving_vertical_orientation_uses_down_up", () => {
    // vertical (listbox): ArrowDown→next, ArrowUp→prev (wrapping); Home/End jump.
    expect(nextTabIndex(1, COUNT, "ArrowDown", "vertical")).toBe(2);
    expect(nextTabIndex(COUNT - 1, COUNT, "ArrowDown", "vertical")).toBe(0);
    expect(nextTabIndex(3, COUNT, "ArrowUp", "vertical")).toBe(2);
    expect(nextTabIndex(0, COUNT, "ArrowUp", "vertical")).toBe(COUNT - 1);
    expect(nextTabIndex(2, COUNT, "Home", "vertical")).toBe(0);
    expect(nextTabIndex(2, COUNT, "End", "vertical")).toBe(COUNT - 1);
    // each orientation ignores the OTHER axis's arrows (no cross-axis roving)
    expect(nextTabIndex(1, COUNT, "ArrowRight", "vertical")).toBe(1);
    expect(nextTabIndex(1, COUNT, "ArrowDown", "horizontal")).toBe(1);
    expect(isRovingKey("ArrowDown", "vertical")).toBe(true);
    expect(isRovingKey("ArrowRight", "vertical")).toBe(false);
    expect(isRovingKey("ArrowDown", "horizontal")).toBe(false);
  });

  it("roving_single_member_stays_put", () => {
    // a one-item list (e.g. a single-project listbox open): every roving key
    // resolves to index 0 — no out-of-range / NaN from the modulo.
    expect(nextTabIndex(0, 1, "ArrowDown", "vertical")).toBe(0);
    expect(nextTabIndex(0, 1, "ArrowUp", "vertical")).toBe(0);
    expect(nextTabIndex(0, 1, "End", "vertical")).toBe(0);
    expect(nextTabIndex(0, 1, "ArrowRight")).toBe(0);
  });
});
