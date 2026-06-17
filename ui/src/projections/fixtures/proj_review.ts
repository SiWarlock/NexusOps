// Fixture data for the Review projection (D5b-1). §14 test/dev infrastructure.
// A separate get_projection("Review") page joined to the PR fixture client-side on
// pr_number (reviewsByPr) — the L2 PR Review Workspace surface. States span the
// frozen ReviewState value enum; pr_numbers reference the proj_pull_request fixture.
import type { ReviewProjectionPage } from "../../contracts/index";

export const reviewFixture: ReviewProjectionPage = {
  projection: "Review",
  rows: [
    {
      review_id: 9001,
      pr_number: 101,
      project_id: "project_fixture_1",
      repo_id: "repo_fixture_1",
      reviewer: "alice",
      state: "approved",
      submitted_at: "2026-06-16T09:10:00Z",
      body: "LGTM — device flow looks solid.",
    },
    {
      review_id: 9002,
      pr_number: 101,
      project_id: "project_fixture_1",
      repo_id: "repo_fixture_1",
      reviewer: "bob",
      state: "changes_requested",
      submitted_at: "2026-06-16T09:14:00Z",
      body: "Please add a refresh-token test.",
    },
    {
      review_id: 9003,
      pr_number: 201,
      project_id: "project_fixture_2",
      repo_id: "repo_fixture_2",
      reviewer: "carol",
      state: "commented",
      submitted_at: "2026-06-16T08:58:00Z",
      body: null,
    },
  ],
  cursor: null,
};
