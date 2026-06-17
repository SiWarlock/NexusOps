// PR ↔ reviews client-side join (§11.2). The Review projection is a SEPARATE
// get_projection("Review") page; the L2 PR Review Workspace joins each PR to its
// reviews on `pr_number` here (pure, no daemon dep — reads two already-validated
// projection pages). Exposed-ahead for L2 (no production consumer yet — the
// ui-059 L1 precedent: ship the pure core before the L2 component).
import type { ReviewRow } from "../contracts/index";

/**
 * Group reviews by their `pr_number` (the PR they belong to).
 *
 * A review with a null/absent `pr_number` cannot attach to a PR row → it is
 * DROPPED (not bucketed) — the workspace shows attached reviews only. Insertion
 * order is preserved within each PR's list.
 */
export function reviewsByPr(reviews: ReviewRow[]): Map<number, ReviewRow[]> {
  const byPr = new Map<number, ReviewRow[]>();
  for (const review of reviews) {
    // null/absent pr_number → unattachable → dropped (see reviews_by_pr test).
    if (review.pr_number == null) continue;
    const list = byPr.get(review.pr_number);
    if (list) list.push(review);
    else byPr.set(review.pr_number, [review]);
  }
  return byPr;
}
