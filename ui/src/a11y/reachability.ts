// The §9 reachability classifier (§11.6 a11y merge-gate): every actionable control
// in a view must be keyboard-reachable, and every visible tabpanel focusable. Pure
// DOM logic — throws a plain Error on the first violation (no vitest import: a
// non-test module pulling in the test framework would be a smell). The whole-Shell
// sweep calls it directly (an uncaught throw fails the test); the classifier units
// drive it with `.toThrow()` / `.not.toThrow()`.

const INTERACTIVE_SELECTOR =
  'button, a[href], [role="button"], [role="link"], input, select, summary';

const describeEl = (el: HTMLElement): string => {
  const role = el.getAttribute("role");
  return `<${el.tagName.toLowerCase()}${role ? ` role="${role}"` : ""}>`;
};

/**
 * A `role="tab"` at tabIndex=-1 is reachable IFF it's a roving member: its
 * `role="tablist"` ancestor has EXACTLY ONE tabIndex=0 tab. This whitelists APG
 * roving tabindex AND pins the one-tabstop invariant — a tablist with zero or
 * multiple tabstops is a real violation, not a roving widget.
 */
function isRovingTabMember(el: HTMLElement): boolean {
  if (el.getAttribute("role") !== "tab") return false;
  const tablist = el.closest('[role="tablist"]');
  if (!tablist) return false;
  const tabs = [...tablist.querySelectorAll<HTMLElement>('[role="tab"]')];
  const tabstops = tabs.filter((t) => t.tabIndex === 0);
  return tabstops.length === 1;
}

/**
 * Audit one rendered view. `el.tabIndex` reflects the effective tab order — 0 for
 * natively-focusable elements or an explicit tabindex>=0, and -1 if removed from
 * the order OR a non-focusable element (e.g. a div[role="button"] with no
 * tabindex). So one `>= 0` check catches both "tabindex=-1 on an actionable" and
 * "role=button on a non-focusable element"; roving tab members are the one
 * sanctioned exception. Visible tabpanels (`:not([hidden])`) must be focusable.
 * Non-vacuous: a view must have ≥1 interactive control.
 */
export function auditFocusable(container: HTMLElement): void {
  const interactive = [
    ...container.querySelectorAll<HTMLElement>(INTERACTIVE_SELECTOR),
  ];
  if (interactive.length === 0) {
    throw new Error("reachability: view has no interactive controls (vacuous audit)");
  }
  for (const el of interactive) {
    if (el.tabIndex >= 0) continue;
    if (isRovingTabMember(el)) continue;
    throw new Error(
      `reachability: unreachable control ${describeEl(el)} (tabIndex=${el.tabIndex})`,
    );
  }
  for (const panel of container.querySelectorAll<HTMLElement>(
    '[role="tabpanel"]:not([hidden])',
  )) {
    if (panel.tabIndex < 0) {
      throw new Error(
        `reachability: visible tabpanel ${describeEl(panel)} is not keyboard-focusable`,
      );
    }
  }
}
