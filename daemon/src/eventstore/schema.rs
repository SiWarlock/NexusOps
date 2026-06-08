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

/// Migration 3 (1.2 projections) — `object_refs` (§2.2), `projection_offsets`
/// (§2.4), and the 10 MVP projection tables (§2.3; ProjectGraph is node+edge, so
/// 11 physical `proj_*` tables). Forward-only over a 1.1-era (v2) db (which already
/// holds `events`), backed up before the raise (§16). Status columns are plain TEXT
/// at the DDL level; the projectors fail-closed-bind them to the frozen §5.1 enums
/// before write. `proj_pull_request` + `proj_agent_team` shapes are authored here
/// (the §2.3 SQL sketch predates the R-5/R-6 reconciliation → arch-note at Step 9).
pub const MIGRATION_3_PROJECTIONS: &str = "\
-- normalized event→object edges (§2.2): the ProjectGraph backbone + per-object timelines
CREATE TABLE object_refs (
  event_id    TEXT NOT NULL REFERENCES events(event_id),
  object_type TEXT NOT NULL,
  object_id   TEXT NOT NULL,
  PRIMARY KEY (event_id, object_type, object_id)
);
CREATE INDEX ix_object_refs_obj ON object_refs(object_type, object_id);

-- per-projector cursor (§2.4): last_seq advances in the SAME txn as the rows it writes
CREATE TABLE projection_offsets (
  projection_name   TEXT PRIMARY KEY,
  last_event_id     TEXT,
  last_seq          INTEGER NOT NULL DEFAULT 0,
  last_processed_at TEXT,
  state             TEXT NOT NULL DEFAULT 'healthy',  -- healthy|rebuilding|degraded
  schema_version    INTEGER NOT NULL DEFAULT 1
);

-- 1. ProjectActivity (§2.3) — sidebar/Command-Center counters
CREATE TABLE proj_project_activity (
  project_id         TEXT PRIMARY KEY,
  active_sessions    INTEGER NOT NULL DEFAULT 0,
  waiting_sessions   INTEGER NOT NULL DEFAULT 0,
  failed_sessions    INTEGER NOT NULL DEFAULT 0,
  idle_sessions      INTEGER NOT NULL DEFAULT 0,
  completed_sessions INTEGER NOT NULL DEFAULT 0,
  active_teams       INTEGER NOT NULL DEFAULT 0,
  open_prs           INTEGER NOT NULL DEFAULT 0,
  blocked_tasks      INTEGER NOT NULL DEFAULT 0,
  updated_at_seq     INTEGER NOT NULL
);

-- 2. Session (§2.3; §5.1 Session) — derived current state of every session
CREATE TABLE proj_session (
  session_id           TEXT PRIMARY KEY,
  project_id           TEXT NOT NULL,
  agent_team_id        TEXT,
  display_name         TEXT,
  harness              TEXT,
  model                TEXT,
  execution_profile_id TEXT,
  worktree_id          TEXT,
  branch_name          TEXT,
  linked_task_id       TEXT,
  linked_plan_task_id  TEXT,
  linked_pr_id         TEXT,
  workflow_command_id  TEXT,
  status               TEXT NOT NULL,  -- §5.1 Session (17)
  context_usage_pct    REAL,
  token_usage_json     TEXT,
  cost_estimate        REAL,
  pending_approvals    INTEGER NOT NULL DEFAULT 0,
  last_heartbeat_at    TEXT,
  started_at           TEXT,
  completed_at         TEXT,
  updated_at_seq       INTEGER NOT NULL
);
CREATE INDEX ix_proj_session_project ON proj_session(project_id, status);

-- 3. ApprovalQueue (§2.3; §5.1 Approval) — Human Input Queue read model
CREATE TABLE proj_approval_queue (
  approval_id       TEXT PRIMARY KEY,
  action_request_id TEXT NOT NULL,
  project_id        TEXT,
  session_id        TEXT,
  agent_team_id     TEXT,
  risk_level        INTEGER NOT NULL,
  status            TEXT NOT NULL,   -- §5.1 Approval (10)
  requester_type    TEXT NOT NULL,
  requester_id      TEXT NOT NULL,
  preview_summary   TEXT,
  requested_at      TEXT NOT NULL,
  expires_at        TEXT,
  sort_key          TEXT,
  updated_at_seq    INTEGER NOT NULL
);
CREATE INDEX ix_approval_queue_open ON proj_approval_queue(status, risk_level, requested_at);

-- 4. Worktree (§2.3; §5.1 Worktree two-axis) — git-truth fields refreshed live via git2 (Phase 5)
CREATE TABLE proj_worktree (
  worktree_id      TEXT PRIMARY KEY,
  project_id       TEXT NOT NULL,
  repo_id          TEXT NOT NULL,
  path             TEXT NOT NULL,
  branch_name      TEXT,
  base_branch      TEXT,
  owner_session_id TEXT,
  owner_team_id    TEXT,
  linked_task_id   TEXT,
  status           TEXT NOT NULL,
  dirty_state      TEXT,
  ahead_count      INTEGER,
  behind_count     INTEGER,
  last_commit_sha  TEXT,
  pr_status        TEXT,
  git_checked_at   TEXT,
  updated_at_seq   INTEGER NOT NULL
);

-- 5. PlanProgress (§2.3; §5.1 Task) — plan/task tree with links
CREATE TABLE proj_plan_progress (
  plan_task_id           TEXT PRIMARY KEY,
  implementation_plan_id TEXT NOT NULL,
  project_id             TEXT NOT NULL,
  phase                  TEXT,
  title                  TEXT,
  status                 TEXT NOT NULL,  -- §5.1 Task (17)
  linked_session_ids_json TEXT,
  linked_pr_ids_json     TEXT,
  linked_ticket_ids_json TEXT,
  architecture_anchor    TEXT,
  updated_at_seq         INTEGER NOT NULL
);

-- 6. PullRequest (§7.2; §5.1 PullRequest) — GitHub-authoritative synced cache.
--    Shape authored at 1.2 (no §2.3 sketch existed) → reconcile into DATA_MODEL §2.3.
CREATE TABLE proj_pull_request (
  pr_id          TEXT PRIMARY KEY,
  project_id     TEXT,
  repo_id        TEXT,
  pr_number      INTEGER,
  title          TEXT,
  status         TEXT NOT NULL,   -- §5.1 PullRequest (11)
  head_branch    TEXT,
  base_branch    TEXT,
  pr_checked_at  TEXT,
  updated_at_seq INTEGER NOT NULL
);

-- 7. ProjectGraph (§2.3) — nodes + edges read model; built from object_refs (§2.2)
CREATE TABLE proj_graph_node (
  node_id    TEXT NOT NULL,
  node_type  TEXT NOT NULL,
  project_id TEXT,
  label      TEXT,
  status     TEXT,
  attrs_json TEXT,
  PRIMARY KEY (node_type, node_id)
);
CREATE TABLE proj_graph_edge (
  src_type   TEXT NOT NULL, src_id TEXT NOT NULL,
  dst_type   TEXT NOT NULL, dst_id TEXT NOT NULL,
  edge_type  TEXT NOT NULL,
  project_id TEXT,
  PRIMARY KEY (src_type, src_id, dst_type, dst_id, edge_type)
);

-- 8. AgentTeam (R-6; §5.1 AgentTeam) — shape authored at 1.2 → reconcile into §2.3.
CREATE TABLE proj_agent_team (
  agent_team_id  TEXT PRIMARY KEY,
  project_id     TEXT,
  display_name   TEXT,
  status         TEXT NOT NULL,   -- §5.1 AgentTeam (9)
  updated_at_seq INTEGER NOT NULL
);

-- 9. AuditTrail (§2.3/§14) — human-readable ordered timeline (rendered, redaction-safe rows)
CREATE TABLE proj_audit_trail (
  event_id     TEXT PRIMARY KEY REFERENCES events(event_id),
  seq          INTEGER NOT NULL,
  project_id   TEXT,
  occurred_at  TEXT NOT NULL,
  scope_json   TEXT,
  headline     TEXT NOT NULL,
  actor_label  TEXT,
  outcome      TEXT,
  sensitivity  TEXT NOT NULL
);
CREATE INDEX ix_audit_scope ON proj_audit_trail(project_id, seq);

-- 10. UsageLedger (§2.3/§18) — tokens/context/cost rollups
CREATE TABLE proj_usage_ledger (
  ledger_id            TEXT PRIMARY KEY,
  project_id           TEXT,
  session_id           TEXT,
  execution_profile_id TEXT,
  model                TEXT,
  bucket_day           TEXT,
  tokens_in            INTEGER,
  tokens_out           INTEGER,
  context_pct_max      REAL,
  cost_estimate        REAL,
  metric_quality       TEXT,
  updated_at_seq       INTEGER NOT NULL
);
";

/// Migration 4 (1.3 outbox) — the transactional-outbox table (DATA_MODEL §2.5). Rows
/// are written in the event-commit txn (the writer); an async drainer delivers them
/// at-least-once with backoff. `status` ∈ pending|in_flight|delivered|failed|dead;
/// `next_attempt_at` (NULL = due now) drives `ix_outbox_due`. `out_` ULID is a
/// daemon-internal id (NOT one of the 22 frozen `shared/` IDs — the outbox is the
/// daemon's own delivery mechanism, not a UI/Brain contract surface).
pub const MIGRATION_4_OUTBOX: &str = "\
CREATE TABLE outbox (
  outbox_id       TEXT PRIMARY KEY,
  destination     TEXT NOT NULL,
  event_id        TEXT NOT NULL REFERENCES events(event_id),
  payload_json    TEXT NOT NULL,
  status          TEXT NOT NULL DEFAULT 'pending',
  retry_count     INTEGER NOT NULL DEFAULT 0,
  next_attempt_at TEXT,
  last_error      TEXT,
  created_at      TEXT NOT NULL
);
CREATE INDEX ix_outbox_due ON outbox(status, next_attempt_at);
";

/// Migration 5 (1.4 leases) — the cross-restart lease table (ADR-008 / §7.2). ONE row
/// per `(resource_id, lease_kind)`; `fencing_token` is a MONOTONIC high-water mark that
/// only ever increments and survives restart (persisted in the row, never an in-memory
/// counter — see §17 / safety rule #6). The holder fields (owner_id/acquired_at/
/// heartbeat_at/expires_at) are NULL when the slot is free; `release` + the reaper NULL
/// them but KEEP the token, so the next acquire still increments (no token reuse). A live
/// lease = owner present AND `expires_at > now`; authority is tied to a live lease, not a
/// merely-unsuperseded token (human-ruled Option B). `ix_leases_expiry` drives the reaper's
/// expired-lease scan (§12). **Daemon-internal** — NOT a `shared/` contract (the UI/Brain
/// never read `leases`), analogous to the 1.3 outbox: no CONTRACT_VERSION bump.
pub const MIGRATION_5_LEASES: &str = "\
CREATE TABLE leases (
  resource_id   TEXT NOT NULL,
  lease_kind    TEXT NOT NULL,
  owner_id      TEXT,
  fencing_token INTEGER NOT NULL DEFAULT 0,
  acquired_at   TEXT,
  heartbeat_at  TEXT,
  expires_at    TEXT,
  PRIMARY KEY (resource_id, lease_kind)
);
CREATE INDEX ix_leases_expiry ON leases(expires_at);
";
