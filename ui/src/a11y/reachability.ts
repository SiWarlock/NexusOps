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

// The text content of `el`, EXCLUDING aria-hidden subtrees — an aria-hidden glyph
// is out of the accessibility tree so it never contributes a name, but a `.sr-only`
// child IS in the tree (visually hidden, not aria-hidden → it counts). Not trimmed
// here; the caller trims.
function accessibleText(el: Element): string {
  let text = "";
  for (const node of el.childNodes) {
    if (node.nodeType === Node.TEXT_NODE) {
      text += node.textContent ?? "";
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      const child = node as Element;
      if (child.getAttribute("aria-hidden") === "true") continue;
      text += accessibleText(child);
    }
  }
  return text;
}

/**
 * The accessible name of an interactive control, by a pragmatic subset of the
 * WAI-ARIA accname algorithm (NOT the full recursive spec — YAGNI; mirrors this
 * audit's flat-container assumption): `aria-label` → `aria-labelledby` target
 * text → visible text from non-`aria-hidden` descendants (INCL `.sr-only`, the
 * Lesson §6 pattern) → `title` → (form controls) an associated `<label>`. Returns
 * `""` when none of these yields a name.
 */
function getAccessibleName(el: HTMLElement): string {
  const ariaLabel = el.getAttribute("aria-label");
  if (ariaLabel?.trim()) return ariaLabel.trim();

  const labelledby = el.getAttribute("aria-labelledby");
  if (labelledby) {
    // Resolve each referenced element through accessibleText (NOT raw textContent)
    // so a target's own aria-hidden subtrees are excluded — consistent with the
    // direct-descendant rule below.
    const text = labelledby
      .split(/\s+/)
      .map((id) => {
        const ref = el.ownerDocument.getElementById(id);
        return ref ? accessibleText(ref) : "";
      })
      .join(" ")
      .trim();
    if (text) return text;
  }

  const visible = accessibleText(el).trim();
  if (visible) return visible;

  const title = el.getAttribute("title");
  if (title?.trim()) return title.trim();

  // Form controls (input/select/…) expose their associated <label>s via `.labels`
  // (explicit `for=` or an implicit wrap); other elements (e.g. <a>) lack it.
  const labels = (el as Partial<HTMLInputElement>).labels;
  if (labels && labels.length > 0) {
    const text = [...labels].map((l) => l.textContent ?? "").join(" ").trim();
    if (text) return text;
  }

  return "";
}

/**
 * Audit one rendered view for accessible-NAME coverage (§11.6): every interactive
 * control (the same `INTERACTIVE_SELECTOR`) must compute a non-empty accessible
 * name — a control can be keyboard-reachable yet a screen-reader dead end. Only
 * roving members at tabIndex=-1 are skipped (`auditFocusable`'s `isRovingMember`
 * exception); the one-tabstop member itself (tabIndex=0) is STILL name-checked, so
 * the group is guaranteed to carry a name somewhere. Throws on the first nameless
 * control. The fix for a caught control is a visually-hidden `.sr-only` child
 * (Lesson §6), never a wrapper `aria-label`.
 *
 * No non-vacuous (zero-controls) guard here on purpose: this audit is always paired
 * with `auditFocusable` in the whole-Shell sweep (`auditView`), which fires its
 * empty-view check first — duplicating it would be redundant.
 */
export function auditAccessibleNames(container: HTMLElement): void {
  for (const el of container.querySelectorAll<HTMLElement>(INTERACTIVE_SELECTOR)) {
    if (el.tabIndex < 0 && isRovingMember(el)) continue;
    if (getAccessibleName(el)) continue;
    throw new Error(
      `accessible-name: control ${describeEl(el)} has no accessible name`,
    );
  }
}
