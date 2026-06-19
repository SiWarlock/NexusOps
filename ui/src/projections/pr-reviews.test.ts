import { describe, it, expect } from "vitest";
import { reviewsByPr } from "./pr-reviews";
import {
  PullRequestProjectionPage,
  ReviewProjectionPage,
  type ReviewRow,
} from "../contracts/index";
import { pullRequestFixture } from "./fixtures/proj_pull_request";
import { reviewFixture } from "./fixtures/proj_review";

describe("reviewsByPr (§11.2 PR↔reviews join)", () => {
  it("reviews_by_pr_groups_by_pr_number", () => {
    // spec(§11.2) — reviews group by pr_number, insertion order preserved per PR.
    const reviews: ReviewRow[] = [
      { review_id: 1, pr_number: 101, state: "approved" },
      { review_id: 2, pr_number: 101, state: "changes_requested" },
      { review_id: 3, pr_number: 202, state: "commented" },
      // null pr_number → unattachable → DROPPED (the default — drop, don't bucket).
      { review_id: 4, pr_number: null, state: "pending" },
    ];
    const byPr = reviewsByPr(reviews);
    expect(byPr.get(101)?.map((r) => r.review_id)).toEqual([1, 2]);
    expect(byPr.get(202)?.map((r) => r.review_id)).toEqual([3]);
    // the null-pr_number review is dropped: 2 buckets, 3 grouped (4 input − 1 dropped).
    expect(byPr.size).toBe(2);
    expect([...byPr.values()].flat().length).toBe(3);
  });

  it("reviews_by_pr_empty_to_empty", () => {
    // boundary: no reviews → no buckets.
    expect(reviewsByPr([]).size).toBe(0);
  });

  it("fixtures_parse_and_join_on_pr_number", () => {
    // spec(§14) — the 11-field PR fixture + the new review fixture parse clean against the frozen
    // shadows, and every grouped review pr_number exists in the PR fixture (the L2 join is valid).
    expect(PullRequestProjectionPage.safeParse(pullRequestFixture).success).toBe(true);
    expect(ReviewProjectionPage.safeParse(reviewFixture).success).toBe(true);
    const byPr = reviewsByPr(reviewFixture.rows);
    const prNumbers = new Set(pullRequestFixture.rows.map((p) => p.pr_number));
    for (const n of byPr.keys()) {
      expect(prNumbers.has(n), `review pr_number ${n} joins a PR fixture row`).toBe(true);
    }
  });
});
