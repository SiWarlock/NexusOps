// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { auditFocusable } from "./reachability";

afterEach(cleanup);

// Test-first the refined §9 classifier: it throws on a genuinely-unreachable
// control + a non-focusable visible tabpanel, and PASSES a roving tab member.

describe("auditFocusable classification", () => {
  it("audit_passes_roving_tab_at_-1", () => {
    // a one-tabstop tablist: the selected tab is tabIndex=0, the rest -1 roving
    // members — arrow-reachable, NOT violations (§11.6 / APG roving tabindex).
    const { container } = render(
      <div role="tablist" aria-label="t">
        <button type="button" role="tab" tabIndex={0}>A</button>
        <button type="button" role="tab" tabIndex={-1}>B</button>
        <button type="button" role="tab" tabIndex={-1}>C</button>
      </div>,
    );
    expect(() => auditFocusable(container)).not.toThrow();
  });

  it("audit_fails_nontab_unreachable", () => {
    // (a) a plain actionable at tabIndex=-1 with no roving context → unreachable.
    const orphan = render(
      <div>
        <button type="button" tabIndex={-1}>orphan</button>
      </div>,
    );
    // assert the SPECIFIC violation (not just "throws") so a future regression
    // throwing the vacuous-audit error instead can't masquerade as a pass.
    expect(() => auditFocusable(orphan.container)).toThrow(/unreachable control/);
    // (b) a tablist with MULTIPLE tabstops → a -1 tab there is not whitelisted
    // (the one-tabstop invariant is violated).
    const multi = render(
      <div role="tablist" aria-label="t">
        <button type="button" role="tab" tabIndex={0}>A</button>
        <button type="button" role="tab" tabIndex={0}>B</button>
        <button type="button" role="tab" tabIndex={-1}>C</button>
      </div>,
    );
    expect(() => auditFocusable(multi.container)).toThrow(/unreachable control/);
    // (c) a tablist with ZERO tabstops → no way in at all.
    const zero = render(
      <div role="tablist" aria-label="t">
        <button type="button" role="tab" tabIndex={-1}>A</button>
        <button type="button" role="tab" tabIndex={-1}>B</button>
      </div>,
    );
    expect(() => auditFocusable(zero.container)).toThrow(/unreachable control/);
  });

  it("audit_covers_visible_tabpanel", () => {
    // a visible tabpanel that isn't focusable → a violation (panels are now
    // audited). A valid button keeps the interactive-count check non-vacuous.
    const fail = render(
      <div>
        <button type="button">ok</button>
        <div role="tabpanel">visible, not focusable</div>
      </div>,
    );
    expect(() => auditFocusable(fail.container)).toThrow(/tabpanel/);
    // a hidden tabpanel is excluded (it's out of the a11y tree).
    const hidden = render(
      <div>
        <button type="button">ok</button>
        <div role="tabpanel" hidden>hidden</div>
      </div>,
    );
    expect(() => auditFocusable(hidden.container)).not.toThrow();
    // a visible tabIndex=0 tabpanel passes.
    const pass = render(
      <div>
        <button type="button">ok</button>
        <div role="tabpanel" tabIndex={0}>focusable</div>
      </div>,
    );
    expect(() => auditFocusable(pass.container)).not.toThrow();
  });
});
