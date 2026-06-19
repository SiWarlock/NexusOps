// @vitest-environment jsdom
//
// ui-064 Layer 1 — the PR reviews-list component. Renders a ReviewRow as a verdict card:
// reviewer + a ReviewState badge (glyph + LABEL — never color alone, forbidden #5) + body +
// submitted_at. All 5 ReviewState values render a non-blank verdict (no unknown→blank); the
// empty-list state is distinct from a populated list.
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { ReviewsList } from "./ReviewsList";
import type { ReviewRow } from "../../contracts/index";

// ReviewState's string-union type comes from the ReviewRow `state` field (the generated enum is a
// value-only export; the row field delegates to it).
type ReviewState = ReviewRow["state"];

afterEach(cleanup);

const review = (over: Partial<ReviewRow> = {}): ReviewRow => ({
  review_id: 9001,
  pr_number: 101,
  project_id: "project_fixture_1",
  repo_id: "repo_fixture_1",
  reviewer: "alice",
  state: "approved",
  submitted_at: "2026-06-16T09:10:00Z",
  body: "LGTM — solid.",
  ...over,
});

describe("ReviewsList (ui-064)", () => {
  it("reviews_list_renders_review_row", () => {
    // [§11.2/forbidden #5] one review → reviewer + ReviewState badge LABEL + body + submitted_at.
    render(<ReviewsList reviews={[review()]} />);
    expect(screen.getByText("alice")).toBeTruthy();
    expect(screen.getByText("LGTM — solid.")).toBeTruthy();
    expect(screen.getByText("Approved")).toBeTruthy(); // the badge's text channel (never color alone)
    expect(screen.getByText(/2026-06-16/)).toBeTruthy(); // submitted_at surfaced
  });

  it("reviews_list_renders_all_review_states", () => {
    // [forbidden #5] every frozen ReviewState renders a distinct, non-blank verdict label.
    const states: ReviewState[] = [
      "approved",
      "changes_requested",
      "commented",
      "dismissed",
      "pending",
    ];
    render(
      <ReviewsList
        reviews={states.map((state, i) => review({ review_id: i, state, reviewer: `r${i}` }))}
      />,
    );
    for (const label of [
      "Approved",
      "Changes requested",
      "Commented",
      "Dismissed",
      "Pending",
    ]) {
      expect(screen.getByText(label)).toBeTruthy();
    }
  });

  it("reviews_list_empty_distinct_from_populated", () => {
    // [§11.2] an empty list renders an explicit empty-state (distinct from a populated list).
    const { rerender } = render(<ReviewsList reviews={[]} />);
    expect(screen.getByTestId("reviews-empty")).toBeTruthy();
    rerender(<ReviewsList reviews={[review()]} />);
    expect(screen.queryByTestId("reviews-empty")).toBeNull();
  });
});
