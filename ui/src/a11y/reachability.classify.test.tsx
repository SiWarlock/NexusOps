// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { auditFocusable, auditAccessibleNames } from "./reachability";

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

  it("audit_passes_roving_option_in_listbox", () => {
    // the roving classifier generalizes to listboxes: a one-tabstop listbox (one
    // option tabIndex=0, the rest -1 role=option) passes — options are arrow-
    // reachable roving members, not violations (slice 5 / the ProjectSwitcher).
    const { container } = render(
      <div role="listbox" aria-label="l">
        <div role="option" tabIndex={0}>A</div>
        <div role="option" tabIndex={-1}>B</div>
        <div role="option" tabIndex={-1}>C</div>
      </div>,
    );
    expect(() => auditFocusable(container)).not.toThrow();
  });

  it("audit_fails_option_not_in_one_tabstop_listbox", () => {
    // a role=option at -1 with no listbox ancestor is unreachable
    const orphan = render(
      <div>
        <div role="option" tabIndex={-1}>x</div>
      </div>,
    );
    expect(() => auditFocusable(orphan.container)).toThrow(/unreachable control/);
    // a listbox with MULTIPLE tabstops violates the one-tabstop invariant
    const multi = render(
      <div role="listbox" aria-label="l">
        <div role="option" tabIndex={0}>A</div>
        <div role="option" tabIndex={0}>B</div>
        <div role="option" tabIndex={-1}>C</div>
      </div>,
    );
    expect(() => auditFocusable(multi.container)).toThrow(/unreachable control/);
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

// Test-first the §11.6 accessible-NAME net (045): every interactive control must
// compute a non-empty accessible name (a control can be keyboard-reachable yet a
// screen-reader dead end). The classifier is a pragmatic subset of WAI-ARIA
// accname — aria-label / aria-labelledby / non-aria-hidden visible text (incl.
// .sr-only, Lesson §6) / title / an associated <label>. It throws on the first
// nameless control, mirroring auditFocusable.

describe("auditAccessibleNames classification", () => {
  it("accessible_names_throws_on_nameless_control", () => {
    // spec(§11.6): an icon-only button with no name is a screen-reader dead end.
    // Assert the SPECIFIC violation (not just "throws") so a future regression
    // can't masquerade as a pass.
    const { container } = render(
      <div>
        <button type="button">
          <svg aria-hidden="true" />
        </button>
      </div>,
    );
    expect(() => auditAccessibleNames(container)).toThrow(/no accessible name/);
  });

  it.each([
    ["aria_label", <button type="button" aria-label="Settings" key="a"><span aria-hidden="true">⚙</span></button>],
    [
      "aria_labelledby",
      <span key="b">
        <span id="an-lbl">Project graph</span>
        <button type="button" aria-labelledby="an-lbl" />
      </span>,
    ],
    ["visible_text", <button type="button" key="c">Save</button>],
    ["title", <button type="button" title="Switch project" key="d"><span aria-hidden="true">⇄</span></button>],
    ["input_label", <label key="e">Filter<input type="text" /></label>],
  ])("accessible_names_accepts_%s", (_source, node) => {
    // spec(§11.6): each name source yields a non-empty accessible name → passes.
    const { container } = render(<div>{node}</div>);
    expect(() => auditAccessibleNames(container)).not.toThrow();
  });

  it("accessible_names_accepts_sr_only_child", () => {
    // spec(§11.6) / Lesson §6: the name comes from a visually-hidden child INSIDE
    // the control (.sr-only is NOT aria-hidden → a11y-visible), never a wrapper
    // aria-label; the decorative glyph is aria-hidden (excluded).
    const { container } = render(
      <div>
        <button type="button">
          <span className="sr-only">Close tab</span>
          <span aria-hidden="true">×</span>
        </button>
      </div>,
    );
    expect(() => auditAccessibleNames(container)).not.toThrow();
  });

  it("accessible_names_excludes_aria_hidden_in_labelledby_target", () => {
    // spec(§11.6): the aria-labelledby target's name is itself computed with its
    // aria-hidden subtrees excluded (consistent with the direct-descendant rule) —
    // a labelledby pointing at an all-aria-hidden element yields no name → throws.
    const { container } = render(
      <div>
        <span id="an-hidden-lbl">
          <span aria-hidden="true">Hidden</span>
        </span>
        <button type="button" aria-labelledby="an-hidden-lbl" />
      </div>,
    );
    expect(() => auditAccessibleNames(container)).toThrow(/no accessible name/);
  });

  it("accessible_names_excludes_aria_hidden_text", () => {
    // spec(§11.6): text inside an aria-hidden subtree is out of the a11y tree →
    // it does NOT count as a name (the glyph is hidden; the name must come from a
    // non-hidden source). The only text here is aria-hidden → nameless → throws.
    const { container } = render(
      <div>
        <button type="button">
          <span aria-hidden="true">Decorative</span>
        </button>
      </div>,
    );
    expect(() => auditAccessibleNames(container)).toThrow(/no accessible name/);
  });

  it("accessible_names_skips_roving_member_at_-1", () => {
    // spec(§11.6): a roving member (role="tab"/"option") at tabIndex=-1 in a
    // one-tabstop container is skipped — the one tabstop carries the group name
    // (symmetric with isRovingMember in auditFocusable). The -1 member here is
    // deliberately nameless to prove the skip; the tabstop is named.
    const { container } = render(
      <div role="tablist" aria-label="views">
        <button type="button" role="tab" tabIndex={0} aria-label="Overview" />
        <button type="button" role="tab" tabIndex={-1}>
          <svg aria-hidden="true" />
        </button>
      </div>,
    );
    expect(() => auditAccessibleNames(container)).not.toThrow();
  });
});
