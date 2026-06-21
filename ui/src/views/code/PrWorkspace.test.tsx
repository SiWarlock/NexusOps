// @vitest-environment jsdom
//
// ui-064 Layer 2 — the read-only PR Review Workspace panel. For a selected PullRequestRow it renders the
// header (number/title/branches/status) + mergeability/checks (from the frozen row) + the reviews-list +
// honest D6/D7 daemon-gap placeholders, with ALL mutations + Brain controls rendered DISABLED (a future
// cat-1 arc + the deferred Brain sibling; wire-or-disable, never a dead click).
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { PrWorkspace } from "./PrWorkspace";
import type { PullRequestRow, ReviewRow } from "../../contracts/index";

afterEach(cleanup);

const pr = (over: Partial<PullRequestRow> = {}): PullRequestRow => ({
  pr_id: "repo_fixture_1#101",
  project_id: "project_fixture_1",
  repo_id: "repo_fixture_1",
  pr_number: 101,
  title: "Add OAuth device flow",
  status: "open",
  head_branch: "agent/auth-refactor",
  base_branch: "main",
  pr_checked_at: "2026-06-16T09:00:00Z",
  mergeable: true,
  checks_summary: "3/3 checks passing",
  ...over,
});

const isDisabled = (name: RegExp) =>
  (screen.getByRole("button", { name }) as HTMLButtonElement).disabled;

const noop = () => {};

describe("PrWorkspace (ui-064 Layer 2)", () => {
  it("pr_workspace_renders_header_and_mergeability", () => {
    // [§11.2/§7.2] the selected PullRequestRow → header (number/title/branch/status) + mergeable/checks
    // from the frozen row (never color alone — a label, not just a hue).
    render(<PrWorkspace pr={pr()} reviews={[]} onBack={noop} />);
    expect(screen.getByText(/Add OAuth device flow/)).toBeTruthy();
    expect(screen.getByText(/#101/)).toBeTruthy();
    expect(screen.getByText(/agent\/auth-refactor/)).toBeTruthy();
    expect(screen.getByText(/Mergeable/i)).toBeTruthy(); // mergeable=true → a label
    expect(screen.getByText("3/3 checks passing")).toBeTruthy(); // checks_summary
  });

  it("pr_workspace_shows_reviews_for_selected_pr", () => {
    // [§11.2] the reviews-list is wired — the PR's reviews (reviewsByPr.get(pr_number)) render.
    const reviews: ReviewRow[] = [
      { review_id: 9001, pr_number: 101, state: "approved", reviewer: "alice", body: "LGTM" },
      { review_id: 9002, pr_number: 101, state: "changes_requested", reviewer: "bob", body: "nit" },
    ];
    render(<PrWorkspace pr={pr()} reviews={reviews} onBack={noop} />);
    expect(screen.getByText("alice")).toBeTruthy();
    expect(screen.getByText("bob")).toBeTruthy();
  });

  it("pr_workspace_d7_code_diff_is_honest_placeholder", () => {
    // [honest-degrade/§7.2] the D7 PR code-diff renders an honest "unavailable — needs daemon
    // get_pr_diff" affordance, NEVER get_diff-as-PR-diff (PrWorkspace has no gateway). (D6 diff-stats
    // are now CONSUMED — see the pr_card_* tests below.)
    render(<PrWorkspace pr={pr()} reviews={[]} onBack={noop} />);
    expect(screen.getByTestId("pr-diff-unavailable").textContent).toMatch(/get_pr_diff/);
  });

  it("pr_card_renders_real_diff_stats", () => {
    // [§11.2 D6] a row carrying the 4 D6 diff-stats renders the real +additions / −deletions / N files /
    // M commits (glyph + label) and RETIRES the old `pr-diffstats-unavailable` placeholder.
    render(
      <PrWorkspace
        pr={pr({ additions: 40, deletions: 7, changed_files: 3, commits: 2 })}
        reviews={[]}
        onBack={noop}
      />,
    );
    const text = screen.getByTestId("pr-diffstats").textContent ?? "";
    expect(text).toMatch(/\+\s*40/); // additions carry a + glyph
    expect(text).toMatch(/40\s*additions/);
    expect(text).toMatch(/7\s*deletions/);
    expect(text).toMatch(/3 files/);
    expect(text).toMatch(/2 commits/);
    // the old honest placeholder is gone (D6 consumed).
    expect(screen.queryByTestId("pr-diffstats-unavailable")).toBeNull();
    expect(screen.queryByTestId("pr-diffstats-empty")).toBeNull();
  });

  it("pr_card_diff_stats_null_safe", () => {
    // [LESSON §32 / forbidden #2] all four diff-stats null → an honest "unavailable for this PR" state,
    // NEVER a fabricated 0 / + / − glyph; the real-stats container is not rendered.
    render(
      <PrWorkspace
        pr={pr({ additions: null, deletions: null, changed_files: null, commits: null })}
        reviews={[]}
        onBack={noop}
      />,
    );
    const empty = screen.getByTestId("pr-diffstats-empty");
    const text = empty.textContent ?? "";
    expect(text).not.toMatch(/[+−]/); // no fabricated +/− glyph
    expect(text).not.toMatch(/\b0\b/); // no fabricated zero
    expect(screen.queryByTestId("pr-diffstats")).toBeNull();
    // the old placeholder testid is retired (replaced by the empty state).
    expect(screen.queryByTestId("pr-diffstats-unavailable")).toBeNull();
  });

  it("pr_card_diff_stats_partial_null", () => {
    // [LESSON §32 per-field null-safe] some fields present, some null → render the present ones, omit
    // the absent ones; NOT the all-null `pr-diffstats-empty` unavailable state.
    render(
      <PrWorkspace
        pr={pr({ additions: 5, deletions: null, changed_files: null, commits: null })}
        reviews={[]}
        onBack={noop}
      />,
    );
    const text = screen.getByTestId("pr-diffstats").textContent ?? "";
    expect(text).toMatch(/5\s*additions/); // the present field renders
    expect(text).not.toMatch(/deletions/); // the absent fields are omitted, not zero-filled
    expect(text).not.toMatch(/file/);
    expect(text).not.toMatch(/commit/);
    expect(screen.queryByTestId("pr-diffstats-empty")).toBeNull();
  });

  it("pr_card_diff_stats_never_color_alone", () => {
    // [§11 / forbidden #5] +/− deltas carry a TEXT label ("additions"/"deletions"), not color/hue alone.
    render(
      <PrWorkspace
        pr={pr({ additions: 40, deletions: 7, changed_files: 3, commits: 2 })}
        reviews={[]}
        onBack={noop}
      />,
    );
    const text = screen.getByTestId("pr-diffstats").textContent ?? "";
    expect(text).toMatch(/additions/i);
    expect(text).toMatch(/deletions/i);
  });

  it("pr_card_diff_stats_zero_is_real_not_unavailable", () => {
    // [LESSON §32 nullish-vs-falsy] a PRESENT 0 is a REAL stat, not hidden: the guard must be
    // `== null`/`??`, never `!x`/`x || …` (a `||` guard would hide a real 0). +0 renders; 1 file /
    // 1 commit render singular; the empty/unavailable state is NOT shown.
    render(
      <PrWorkspace
        pr={pr({ additions: 0, deletions: 40, changed_files: 1, commits: 1 })}
        reviews={[]}
        onBack={noop}
      />,
    );
    const text = screen.getByTestId("pr-diffstats").textContent ?? "";
    expect(text).toMatch(/\+\s*0\b/); // +0 rendered, NOT omitted
    expect(text).toMatch(/0\s*additions/);
    expect(text).toMatch(/40\s*deletions/);
    expect(text).toMatch(/1 file/);
    expect(text).toMatch(/1 commit/);
    expect(text).not.toMatch(/1 files/); // singular for 1
    expect(text).not.toMatch(/1 commits/);
    expect(screen.queryByTestId("pr-diffstats-empty")).toBeNull();
  });

  it("pr_workspace_mutations_and_brain_disabled", () => {
    // [a11y wire-or-disable / forbidden #6] the future cat-1 mutation arc + the deferred Brain sibling
    // render DISABLED with accessible names — present but non-interactive, never a dead click.
    render(<PrWorkspace pr={pr()} reviews={[]} onBack={() => {}} />);
    expect(isDisabled(/Merge/i)).toBe(true);
    expect(isDisabled(/Approve PR/i)).toBe(true);
    expect(isDisabled(/Request changes/i)).toBe(true);
    expect(isDisabled(/Ask Brain/i)).toBe(true);
    // the "← Worktree diff" deselect IS wired (onBack provided) — not a dead click.
    expect(isDisabled(/Worktree diff/i)).toBe(false);
  });
});
