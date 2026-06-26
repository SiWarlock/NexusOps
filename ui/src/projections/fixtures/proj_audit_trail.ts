// Fixture data for the AuditTrail projection (the event timeline). §14 test/dev
// infrastructure. seq is the canonical event order (§7.1); the tile sorts by it
// descending. The 8-field shape matches the frozen daemon AuditEventRow (W2-audit
// 0.49.0): `headline` (the redaction-safe render), `actor_label` (the snake_case
// ActorType wire value, nullable), `occurred_at` (RFC3339 UTC), `sensitivity`.
import type { AuditTrailPage } from "../../contracts/index";

export const auditTrailFixture: AuditTrailPage = {
  projection: "AuditTrail",
  rows: [
    {
      event_id: "event_fixture_1",
      seq: 10,
      project_id: "project_fixture_1",
      occurred_at: "2026-06-26T09:10:00Z",
      event_type: "session.started",
      headline: "Started session on auth-service",
      actor_label: "user",
      sensitivity: "internal",
    },
    {
      event_id: "event_fixture_2",
      seq: 20,
      project_id: "project_fixture_2",
      occurred_at: "2026-06-26T09:20:00Z",
      event_type: "action.approved",
      headline: "Approved github.create_pr",
      actor_label: "action_gateway",
      sensitivity: "internal",
    },
    {
      event_id: "event_fixture_3",
      seq: 30,
      project_id: "project_fixture_1",
      occurred_at: "2026-06-26T09:30:00Z",
      event_type: "project.rescanned",
      headline: "Rescanned auth-service worktree",
      actor_label: "session_adapter",
      sensitivity: "internal",
    },
    {
      event_id: "event_fixture_4",
      seq: 25,
      project_id: "project_fixture_1",
      occurred_at: "2026-06-26T09:25:00Z",
      event_type: "git.committed",
      headline: "Committed on feature branch",
      actor_label: "user",
      sensitivity: "internal",
    },
    {
      event_id: "event_fixture_5",
      seq: 5,
      project_id: "project_fixture_3",
      occurred_at: "2026-06-26T09:05:00Z",
      event_type: "project.registered",
      headline: "Registered docs-site",
      actor_label: "system",
      sensitivity: "internal",
    },
  ],
  cursor: null,
};
