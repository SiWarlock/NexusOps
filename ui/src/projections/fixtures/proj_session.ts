// Fixture projection data for the Session projection.
//
// §14-sanctioned test/dev infrastructure — NOT dead production code. Every status
// value here is a member of the frozen §5.1 Session enum; the mock validates
// these through the boundary before returning, and mock.test.ts re-pins each
// row against the generated enum so fixtures can never drift from the contract.
import type {
  ProjectionDelta,
  SessionProjectionPage,
} from "../../contracts/index";

export const sessionPageFixture: SessionProjectionPage = {
  projection: "Session",
  rows: [
    {
      session_id: "session_fixture_1",
      status: "active",
      display_name: "Refactor auth module",
      project_id: "project_fixture_1",
      resume_mode: "resumed", // O-2: brought back live after restart
      replayed_event_count: 0,
      recovered_at: "2026-06-16T09:00:00Z",
    },
    {
      session_id: "session_fixture_2",
      status: "waiting_on_permission",
      display_name: "Add rate limiting",
      project_id: "project_fixture_1",
    },
    {
      session_id: "session_fixture_3",
      status: "changes_ready",
      display_name: "Fix flaky integration test",
      project_id: "project_fixture_2",
      resume_mode: "replayed", // O-2: relaunched from the event log after restart
      replayed_event_count: 128,
      recovered_at: "2026-06-16T08:30:00Z",
    },
    {
      // terminal session — excluded from activeSessions counts (non-terminal filter).
      session_id: "session_fixture_4",
      status: "completed",
      display_name: "Bump dependencies",
      project_id: "project_fixture_1",
    },
    {
      // waiting_on_human_input — exercises the second WAITING_SESSION state.
      session_id: "session_fixture_5",
      status: "waiting_on_human_input",
      display_name: "Confirm migration plan",
      project_id: "project_fixture_2",
    },
  ],
  cursor: null,
};

// A sample upsert delta the MockGatewayPort streams (the parse-don't-trust dogfood). The live Session
// subscribe now consumes a delta as a `row:None` NUDGE → coalesced refetch (ui-062, LESSON §29), so this
// row-bearing delta drives a re-read (the row content is ignored). A status-CHANGING live re-render is
// exercised against a dedicated fake gateway in Shell.subscribe.test.tsx.
export const sessionDeltaFixture: ProjectionDelta = {
  projection: "Session",
  kind: "upsert",
  row: {
    session_id: "session_fixture_2",
    status: "waiting_on_permission",
    display_name: "Add rate limiting",
    project_id: "project_fixture_1",
  },
};
