// Fixture data for the PullRequest projection. §14 test/dev infrastructure.
// pr merged is terminal → excluded from openPRs counts. Reconciled to the frozen
// 11-field PullRequestRow (ui-061): pr_id = the `{repo_id}#{pr_number}` composite PK,
// pr_number is a NUMBER (the work-order str→number drift), + the rich D5 mergeable/checks.
import type {
  ProjectionDelta,
  PullRequestProjectionPage,
} from "../../contracts/index";

export const pullRequestFixture: PullRequestProjectionPage = {
  projection: "PullRequest",
  rows: [
    {
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
    },
    {
      pr_id: "repo_fixture_1#102",
      project_id: "project_fixture_1",
      repo_id: "repo_fixture_1",
      pr_number: 102,
      title: "Rotate signing keys",
      status: "merged",
      head_branch: "chore/deps",
      base_branch: "main",
      pr_checked_at: "2026-06-16T08:40:00Z",
      mergeable: null,
      checks_summary: "merged",
    },
    {
      pr_id: "repo_fixture_2#201",
      project_id: "project_fixture_2",
      repo_id: "repo_fixture_2",
      pr_number: 201,
      title: "Refund webhook retries",
      status: "checks_failing",
      head_branch: "fix/flaky-integration",
      base_branch: "main",
      pr_checked_at: "2026-06-16T08:54:00Z",
      mergeable: false,
      checks_summary: "1/3 checks failing",
    },
  ],
  cursor: null,
};

// A daemon-shaped `row:None` NUDGE for the PullRequest subscribe stream (ui-063). The daemon emits an
// id-nudge keyed by pr_id (deltas_for_event, on PullRequestSynced), NOT the row — so this carries NO
// `row`. The live PR list consumes it via refetch-on-nudge (re-read get_projection), never a row-apply
// reducer (which would no-op on the absent row — LESSON §29).
export const pullRequestDeltaFixture: ProjectionDelta = {
  projection: "PullRequest",
  kind: "upsert",
  id: "repo_fixture_1#101",
};
