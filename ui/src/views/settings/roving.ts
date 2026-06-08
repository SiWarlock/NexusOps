// Pure roving-tabindex key mapping for WAI-ARIA composite widgets (APG Tabs).
// Widget-agnostic: the caller passes the current index + member count, so this is
// reused by the Settings tablist and (slice 5) the ProjectSwitcher dropdown. The
// caller owns focus + activation; this only computes the target index.

/** A horizontal tablist's roving keys. */
const ROVING_KEYS = new Set(["ArrowRight", "ArrowLeft", "Home", "End"]);

export function isRovingKey(key: string): boolean {
  return ROVING_KEYS.has(key);
}

/**
 * Map a key to the next focus index. Arrow keys wrap; Home/End jump to the ends.
 * Any non-roving key returns `currentIndex` unchanged — the sentinel the caller
 * reads as "no roving move" (so it doesn't preventDefault / move focus).
 */
export function nextTabIndex(
  currentIndex: number,
  count: number,
  key: string,
): number {
  switch (key) {
    case "ArrowRight":
      return (currentIndex + 1) % count;
    case "ArrowLeft":
      return (currentIndex - 1 + count) % count;
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return currentIndex;
  }
}
