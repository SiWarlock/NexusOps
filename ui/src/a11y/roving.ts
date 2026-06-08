// Pure roving-tabindex key mapping for WAI-ARIA composite widgets (APG). Shared
// a11y primitive (co-located with reachability.ts) — consumed by the Settings
// tablist (horizontal) and the ProjectSwitcher listbox (vertical). Widget-agnostic:
// the caller passes the current index + member count, so this computes the target
// index only; the caller owns focus + activation.

/** Tablist = horizontal (Left/Right); listbox/menu = vertical (Up/Down). */
export type RovingOrientation = "horizontal" | "vertical";

const NEXT_KEY: Record<RovingOrientation, string> = {
  horizontal: "ArrowRight",
  vertical: "ArrowDown",
};
const PREV_KEY: Record<RovingOrientation, string> = {
  horizontal: "ArrowLeft",
  vertical: "ArrowUp",
};

/**
 * Whether a key drives roving for the given orientation (defaults horizontal, so
 * existing tablist callers are unchanged). Home/End apply to both axes.
 */
export function isRovingKey(
  key: string,
  orientation: RovingOrientation = "horizontal",
): boolean {
  return (
    key === NEXT_KEY[orientation] ||
    key === PREV_KEY[orientation] ||
    key === "Home" ||
    key === "End"
  );
}

/**
 * Map a key to the next focus index for the orientation (default horizontal). The
 * directional arrows wrap; Home/End jump to the ends; any non-roving key (incl.
 * the OTHER axis's arrows) returns `currentIndex` unchanged — the sentinel the
 * caller reads as "no roving move" (so it doesn't preventDefault / move focus).
 */
export function nextTabIndex(
  currentIndex: number,
  count: number,
  key: string,
  orientation: RovingOrientation = "horizontal",
): number {
  if (key === NEXT_KEY[orientation]) return (currentIndex + 1) % count;
  if (key === PREV_KEY[orientation]) return (currentIndex - 1 + count) % count;
  if (key === "Home") return 0;
  if (key === "End") return count - 1;
  return currentIndex;
}
