// Fixture data for the PullRequest projection. §14 test/dev infrastructure.
// pr merged is terminal → excluded from openPRs counts.
import type { PullRequestProjectionPage } from "../../contracts/index";

export const pullRequestFixture: PullRequestProjectionPage = {
  projection: "PullRequest",
  rows: [
    {
      pr_number: "101",
      project_id: "project_fixture_1",
      status: "open",
      title: "Add OAuth device flow",
    },
    {
      pr_number: "102",
      project_id: "project_fixture_1",
      status: "merged",
      title: "Rotate signing keys",
    },
    {
      pr_number: "201",
      project_id: "project_fixture_2",
      status: "checks_failing",
      title: "Refund webhook retries",
    },
  ],
  cursor: null,
};
