// @vitest-environment jsdom
//
// ui-064/068/069 — the read-only PR Review Workspace panel. For a selected PullRequestRow it renders the
// header (number/title/branches/status) + mergeability/checks + the reviews-list + the real D6 diff-stats
// (ui-068) + the read-only D7 PR code-diff (ui-069, passed down by DiffReview as `prDiff`), with ALL
// mutations + Brain controls rendered DISABLED (a future cat-1 arc + the deferred Brain sibling).
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { PrWorkspace, type PrDiffState } from "./PrWorkspace";
import type { DiffResult, PullRequestRow, ReviewRow } from "../../contracts/index";

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

// A ready PR code-diff with a line unique to it (so the render assertion can't false-match a fixture).
const PR_DIFF: DiffResult = {
  hunks: [
    {
      header: "@@ -1,2 +1,3 @@",
      old_start: 1,
      old_lines: 2,
      new_start: 1,
      new_lines: 3,
      lines: [
        { kind: "context", content: "export function review() {\n" },
        { kind: "added", content: "  const dry = dryRun()\n" },
      ],
    },
  ],
};
const NO_LINK: PrDiffState = { kind: "no-link" };

const isDisabled = (name: RegExp) =>
  (screen.getByRole("button", { name }) as HTMLButtonElement).disabled;

const noop = () => {};

// PrWorkspace is pure-display: `prDiff` is REQUIRED (no silent default — the discriminated-union
// discipline). Tests that don't exercise the code-diff pass the honest no-link state.
const renderWs = (
  opts: { pr?: PullRequestRow; reviews?: ReviewRow[]; prDiff?: PrDiffState } = {},
) =>
  render(
    <PrWorkspace
      pr={opts.pr ?? pr()}
      reviews={opts.reviews ?? []}
      onBack={noop}
      prDiff={opts.prDiff ?? NO_LINK}
    />,
  );

describe("PrWorkspace (ui-064 Layer 2)", () => {
  it("pr_workspace_renders_header_and_mergeability", () => {
    // [§11.2/§7.2] the selected PullRequestRow → header (number/title/branch/status) + mergeable/checks
    // from the frozen row (never color alone — a label, not just a hue).
    renderWs();
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
    renderWs({ reviews });
    expect(screen.getByText("alice")).toBeTruthy();
    expect(screen.getByText("bob")).toBeTruthy();
  });

  it("pr_workspace_renders_pr_code_diff", () => {
    // [§11.2 D7] a ready PrDiffState → the read-only PR code-diff renders the hunk lines (reused kit
    // DiffHunk) and the `pr-diff-unavailable` state is gone; NO per-hunk action bar (PR-per-hunk is a
    // future cat-1 — the worktree-scoped HunkGitActions is excluded; render read-only).
    renderWs({ prDiff: { kind: "ready", diff: PR_DIFF } });
    expect(screen.getByTestId("pr-diff")).toBeTruthy();
    expect(screen.getByText(/const dry = dryRun/)).toBeTruthy(); // a line unique to PR_DIFF
    expect(screen.queryByTestId("pr-diff-unavailable")).toBeNull();
    expect(screen.queryByTestId("pr-diff-no-link")).toBeNull();
    // read-only: no per-hunk git action buttons (HunkGitActions excluded).
    expect(
      screen.queryByRole("button", { name: /Stage hunk|Unstage hunk|Discard hunk/i }),
    ).toBeNull();
  });

  it("pr_workspace_renders_no_changes_when_diff_empty", () => {
    // [§11.7] a ready diff with zero hunks → an honest "no changes" state, not blank/faked.
    renderWs({ prDiff: { kind: "ready", diff: { hunks: [] } } });
    expect(screen.getByTestId("pr-diff-no-changes")).toBeTruthy();
    expect(screen.queryByTestId("pr-diff")).toBeNull();
  });

  it("pr_workspace_pr_diff_honest_unavailable_on_error", () => {
    // [§11.7 / forbidden #2] an error PrDiffState → an honest "PR diff unavailable" state with the
    // daemon code verbatim, NEVER a fabricated diff.
    renderWs({ prDiff: { kind: "error", code: "not_found" } });
    const el = screen.getByTestId("pr-diff-unavailable");
    expect(el.textContent).toMatch(/unavailable/i);
    expect(el.textContent).toMatch(/not_found/); // the daemon code, verbatim
    expect(screen.queryByTestId("pr-diff")).toBeNull(); // no fabricated diff
    expect(screen.queryByText(/const dry = dryRun/)).toBeNull();
  });

  it("pr_workspace_pr_diff_loading_shows_indicator", () => {
    // [§11.7] a loading PrDiffState → an honest loading indicator, not blank / stale / a placeholder.
    renderWs({ prDiff: { kind: "loading" } });
    expect(screen.getByTestId("pr-diff-loading")).toBeTruthy();
    expect(screen.queryByTestId("pr-diff")).toBeNull();
    expect(screen.queryByTestId("pr-diff-unavailable")).toBeNull();
  });

  it("pr_workspace_pr_diff_no_link_when_state_no_link", () => {
    // [§11.7] a no-link PrDiffState (null repo_id/pr_number upstream) → an honest "no repo link" state,
    // distinct from a daemon error, never a fabricated diff.
    renderWs({ prDiff: { kind: "no-link" } });
    expect(screen.getByTestId("pr-diff-no-link")).toBeTruthy();
    expect(screen.queryByTestId("pr-diff")).toBeNull();
    expect(screen.queryByTestId("pr-diff-unavailable")).toBeNull();
  });

  it("pr_card_renders_real_diff_stats", () => {
    // [§11.2 D6] a row carrying the 4 D6 diff-stats renders the real +additions / −deletions / N files /
    // M commits (glyph + label) and RETIRES the old `pr-diffstats-unavailable` placeholder.
    renderWs({ pr: pr({ additions: 40, deletions: 7, changed_files: 3, commits: 2 }) });
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
    renderWs({
      pr: pr({ additions: null, deletions: null, changed_files: null, commits: null }),
    });
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
    renderWs({
      pr: pr({ additions: 5, deletions: null, changed_files: null, commits: null }),
    });
    const text = screen.getByTestId("pr-diffstats").textContent ?? "";
    expect(text).toMatch(/5\s*additions/); // the present field renders
    expect(text).not.toMatch(/deletions/); // the absent fields are omitted, not zero-filled
    expect(text).not.toMatch(/file/);
    expect(text).not.toMatch(/commit/);
    expect(screen.queryByTestId("pr-diffstats-empty")).toBeNull();
  });

  it("pr_card_diff_stats_never_color_alone", () => {
    // [§11 / forbidden #5] +/− deltas carry a TEXT label ("additions"/"deletions"), not color/hue alone.
    renderWs({ pr: pr({ additions: 40, deletions: 7, changed_files: 3, commits: 2 }) });
    const text = screen.getByTestId("pr-diffstats").textContent ?? "";
    expect(text).toMatch(/additions/i);
    expect(text).toMatch(/deletions/i);
  });

  it("pr_card_diff_stats_zero_is_real_not_unavailable", () => {
    // [LESSON §32 nullish-vs-falsy] a PRESENT 0 is a REAL stat, not hidden: the guard must be
    // `== null`/`??`, never `!x`/`x || …` (a `||` guard would hide a real 0). +0 renders; 1 file /
    // 1 commit render singular; the empty/unavailable state is NOT shown.
    renderWs({ pr: pr({ additions: 0, deletions: 40, changed_files: 1, commits: 1 }) });
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
    renderWs();
    expect(isDisabled(/Merge/i)).toBe(true);
    expect(isDisabled(/Approve PR/i)).toBe(true);
    expect(isDisabled(/Request changes/i)).toBe(true);
    expect(isDisabled(/Ask Brain/i)).toBe(true);
    // the "← Worktree diff" deselect IS wired (onBack provided) — not a dead click.
    expect(isDisabled(/Worktree diff/i)).toBe(false);
  });
});
