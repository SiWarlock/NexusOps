import { describe, it, expect } from "vitest";
import { deriveProjectSwitcherCounts, deriveActivityFeed } from "./derive";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";
import { sessionPageFixture } from "../projections/fixtures/proj_session";
import { pullRequestFixture } from "../projections/fixtures/proj_pull_request";
import {
  approvalQueueFixture,
  makeApprovalRow,
} from "../projections/fixtures/proj_approval_queue";
import { auditTrailFixture } from "../projections/fixtures/proj_audit_trail";

const input = {
  projects: projectActivityFixture.rows,
  sessions: sessionPageFixture.rows,
  pullRequests: pullRequestFixture.rows,
  approvals: approvalQueueFixture.rows,
};

describe("deriveProjectSwitcherCounts", () => {
  it("derive_project_switcher_counts_from_projections", () => {
    const counts = deriveProjectSwitcherCounts(input);
    // project_fixture_1: active + waiting_on_permission active (completed excluded);
    // 1 open PR (merged excluded); waitingOnYou = 1 pending approval + 1 waiting_on_permission session.
    expect(counts["project_fixture_1"]).toEqual({
      activeSessions: 2,
      openPRs: 1,
      waitingOnYou: 2,
    });
    // project_fixture_2: changes_ready + waiting_on_human_input active; 1 open PR
    // (checks_failing); waitingOnYou = 1 (the waiting_on_human_input session;
    // approval_fixture_2 is decided/approved so it does not count).
    expect(counts["project_fixture_2"]).toEqual({
      activeSessions: 2,
      openPRs: 1,
      waitingOnYou: 1,
    });
  });

  it("derive_counts_exclude_a_null_project_approval_from_every_per_project_count", () => {
    // The frozen ApprovalQueueRow made project_id optional. A plan-level / workspace pending approval
    // (project_id null) is cross-project → it must NOT be attributed to any single project's
    // waitingOnYou (the === pid filter excludes null by construction). Adding one leaves every count
    // unchanged vs the baseline. (A global "waiting" bucket for these is a future product call.)
    const baseline = deriveProjectSwitcherCounts(input);
    const withNullProject = deriveProjectSwitcherCounts({
      ...input,
      approvals: [
        ...approvalQueueFixture.rows,
        makeApprovalRow({
          approval_id: "appr_plan_level",
          project_id: null,
          plan_id: "plan_1",
          status: "awaiting_approval",
        }),
      ],
    });
    expect(withNullProject).toEqual(baseline);
  });

  it("derive_counts_empty_projection_is_zeroed_not_absent", () => {
    const counts = deriveProjectSwitcherCounts(input);
    // project_fixture_3 has no sessions/PRs/approvals — explicit zeros, present, not undefined.
    expect(counts["project_fixture_3"]).toEqual({
      activeSessions: 0,
      openPRs: 0,
      waitingOnYou: 0,
    });
    // every project in the projection gets an entry (derived per project, not per activity row).
    expect(Object.keys(counts).toSorted()).toEqual(
      input.projects.map((p) => p.project_id).toSorted(),
    );
  });
});

describe("deriveActivityFeed", () => {
  it("derive_activity_feed_scoped_ordered_limited", () => {
    const events = auditTrailFixture.rows;

    // project-scoped: only the selected project's events
    const scoped = deriveActivityFeed(events, {
      projectId: "project_fixture_1",
    });
    expect(scoped.every((e) => e.project_id === "project_fixture_1")).toBe(true);
    expect(scoped).toHaveLength(3);

    // ordering: by seq descending (most-recent first; §7.1 canonical order).
    // Full ordering assertion (mutation-resistant; no index access).
    expect(scoped.map((e) => e.seq)).toEqual([30, 25, 10]);

    // dock limit: the N most-recent
    const limited = deriveActivityFeed(events, {
      projectId: "project_fixture_1",
      limit: 2,
    });
    expect(limited.map((e) => e.seq)).toEqual([30, 25]);

    // unscoped: all events pass through (5 < default limit), pinning the no-filter path
    expect(deriveActivityFeed(events)).toHaveLength(events.length);
  });
});
