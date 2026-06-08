//! Events-table DDL + WAL pragmas (DATA_MODEL §2.1, ADR-003 / §18).
//!
//! The `events` table is the append-only spine. `redaction_status` +
//! `redaction_engine_version` columns are added by the L3 redaction migration
//! (user_version 2), not here. `payload_json` carries a `json_valid` CHECK
//! (defense-in-depth; the writer fails closed on a violation, §15/§17). FTS5 is
//! scaffolding only in 1.1 (populated with the AuditTrail projection in 1.2).

use rusqlite::Connection;

/// Per-connection pragmas (ADR-003): WAL + NORMAL sync + FK on + 5s busy timeout.
/// `fullfsync` left OFF (§18 caveat / OQ-DATA-SPIKE-3). Applied before any txn.
pub fn apply_pragmas(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;\
         PRAGMA synchronous=NORMAL;\
         PRAGMA foreign_keys=ON;\
         PRAGMA busy_timeout=5000;",
    )
}

/// Migration 1 — the `events` table (DATA_MODEL §2.1) + its 6 indexes + the FTS5
/// scaffolding. `redaction_status` arrives in migration 2 (L3).
pub const MIGRATION_1_EVENTS: &str = "\
CREATE TABLE events (
  event_id            TEXT PRIMARY KEY,
  seq                 INTEGER NOT NULL,
  event_type          TEXT NOT NULL,
  event_version       INTEGER NOT NULL,
  occurred_at         TEXT NOT NULL,
  recorded_at         TEXT NOT NULL,
  workspace_id        TEXT NOT NULL,
  project_id          TEXT,
  actor_type          TEXT NOT NULL,
  actor_id            TEXT NOT NULL,
  source_type         TEXT NOT NULL,
  source_id           TEXT NOT NULL,
  correlation_id      TEXT NOT NULL,
  causation_id        TEXT,
  action_request_id   TEXT,
  approval_id         TEXT,
  session_id          TEXT,
  agent_team_id       TEXT,
  workflow_run_id     TEXT,
  idempotency_key     TEXT,
  sensitivity         TEXT NOT NULL,
  visibility          TEXT NOT NULL DEFAULT 'project',
  payload_json        TEXT NOT NULL CHECK(json_valid(payload_json)),
  payload_hash        TEXT,
  previous_event_hash TEXT,
  schema_version      TEXT,
  app_version         TEXT  -- provenance; populated at daemon bootstrap (Phase 1.6), NULL until then
);
CREATE UNIQUE INDEX ux_events_seq         ON events(seq);
CREATE INDEX        ix_events_project_seq ON events(project_id, seq);
CREATE INDEX        ix_events_correlation ON events(correlation_id, seq);
CREATE INDEX        ix_events_type_seq    ON events(event_type, seq);
CREATE INDEX        ix_events_session     ON events(session_id, seq);
CREATE UNIQUE INDEX ux_events_idempotency ON events(idempotency_key) WHERE idempotency_key IS NOT NULL;
CREATE VIRTUAL TABLE fts_events USING fts5(event_id UNINDEXED, body);
";

/// Migration 2 (L3 redaction) — the §15 redaction columns. The backfill DEFAULT
/// is **`unredacted`** (NOT `redacted`): pre-gate (L2-era) rows never passed the
/// Redactor, so honest provenance labels them unredacted for a future read-path
/// sweep — never falsely 'redacted'. New rows set `redacted` explicitly via the
/// append gate. `redaction_engine_version` records which Redactor masked them.
pub const MIGRATION_2_REDACTION: &str = "\
ALTER TABLE events ADD COLUMN redaction_status TEXT NOT NULL DEFAULT 'unredacted';
ALTER TABLE events ADD COLUMN redaction_engine_version TEXT;
";
