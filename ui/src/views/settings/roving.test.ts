import { describe, it, expect } from "vitest";
import { nextTabIndex } from "./roving";

// 5-tab tablist (Settings has five sections); roving is widget-agnostic so the
// count is a parameter (slice 5's ProjectSwitcher dropdown reuses this).
const COUNT = 5;

describe("nextTabIndex (roving tabindex)", () => {
  it("roving_arrow_right_wraps", () => {
    // ArrowRight advances and wraps past the end (APG Tabs horizontal roving).
    expect(nextTabIndex(1, COUNT, "ArrowRight")).toBe(2);
    expect(nextTabIndex(COUNT - 1, COUNT, "ArrowRight")).toBe(0);
  });

  it("roving_arrow_left_wraps", () => {
    // ArrowLeft retreats and wraps past the start.
    expect(nextTabIndex(3, COUNT, "ArrowLeft")).toBe(2);
    expect(nextTabIndex(0, COUNT, "ArrowLeft")).toBe(COUNT - 1);
  });

  it("roving_home_end", () => {
    // Home jumps to the first tab, End to the last (APG Tabs).
    expect(nextTabIndex(2, COUNT, "Home")).toBe(0);
    expect(nextTabIndex(2, COUNT, "End")).toBe(COUNT - 1);
    // Home on the first / End on the last is the same index (caller no-ops on it).
    expect(nextTabIndex(0, COUNT, "Home")).toBe(0);
    expect(nextTabIndex(COUNT - 1, COUNT, "End")).toBe(COUNT - 1);
  });

  it("roving_other_key_is_unchanged", () => {
    // A non-roving key returns currentIndex unchanged — the sentinel the caller
    // reads as "no roving move" (so it doesn't preventDefault / move focus).
    expect(nextTabIndex(2, COUNT, "Enter")).toBe(2);
    expect(nextTabIndex(2, COUNT, " ")).toBe(2);
    expect(nextTabIndex(2, COUNT, "a")).toBe(2);
  });
});
