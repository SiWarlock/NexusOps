// The §9 reachability classifier (§11.6 a11y merge-gate): every actionable control
// in a view must be keyboard-reachable, and every visible tabpanel focusable. Pure
// DOM logic — throws a plain Error on the first violation (no vitest import: a
// non-test module pulling in the test framework would be a smell). The whole-Shell
// sweep calls it directly (an uncaught throw fails the test); the classifier units
// drive it with `.toThrow()` / `.not.toThrow()`.

const INTERACTIVE_SELECTOR =
  'button, a[href], [role="button"], [role="link"], [role="option"], input, select, summary';

const describeEl = (el: HTMLElement): string => {
  const role = el.getAttribute("role");
  return `<${el.tagName.toLowerCase()}${role ? ` role="${role}"` : ""}>`;
};

// The roving composites this audit understands: a member role inside its container
// role (APG roving tabindex). tab→tablist (Settings), option→listbox (ProjectSwitcher).
const ROVING_CONTAINER: Record<string, string> = {
  tab: "tablist",
  option: "listbox",
};

/**
 * A roving member (a `role="tab"`/`"option"`) at tabIndex=-1 is reachable IFF its
 * container (`role="tablist"`/`"listbox"`) has EXACTLY ONE tabIndex=0 member. This
 * whitelists APG roving tabindex AND pins the one-tabstop invariant — a container
 * with zero or multiple tabstops is a real violation, not a roving widget.
 */
function isRovingMember(el: HTMLElement): boolean {
  const role = el.getAttribute("role");
  const containerRole = role ? ROVING_CONTAINER[role] : undefined;
  if (!containerRole) return false;
  const container = el.closest(`[role="${containerRole}"]`);
  if (!container) return false;
  const members = [...container.querySelectorAll<HTMLElement>(`[role="${role}"]`)];
  const tabstops = members.filter((m) => m.tabIndex === 0);
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
    if (isRovingMember(el)) continue;
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
