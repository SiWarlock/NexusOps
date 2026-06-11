# Data Model — Local Persistence Design (ROUGH DRAFT)

> **Status:** Brain 1 / arch-draft rough draft. Written for adversarial finalization by `/arch-finalize` (Brain 2). NOT binding.
> **Date:** 2026-06-06
> **Scope:** The concrete local persistence design for the NexusOps daemon — the SQLite schema, source-of-truth rules, status state machines, shared-ID strategy, the four desktop-addendum objects that close the Shared Object Model gap, migrations/recovery/retention, and the open data questions.
> **Relationship to existing docs:** This artifact makes persistence *concrete*. It does **not** restate the object catalog. It references:
> - `docs/architecture/SHARED_OBJECT_MODEL.md` (SOM) — ~30 objects with fields/lifecycles (anchors `§4`–`§37`), the 4 canonical chains (`§35`), open questions (`§37`).
> - `docs/architecture/EVENT_MODEL_AND_AUDIT_TRAIL.md` (EM) — envelope (`§6`), actor (`§7`), source (`§8`), sensitivity (`§9`), taxonomy (`§10`), storage sketch (`§12`), projections (`§13`), audit (`§14`), failure modes (`§23`), retention (`§21`), versioning (`§22`).
> - `docs/architecture/PROJECT_BRAIN_INTERFACE.md` (PBI) — 22 shared IDs (`§3`), boundary (`§1`), safety (`§8`).
> - `docs/architecture/DESKTOP_FIRST_RUNTIME.md` (DFR) — required new objects (`§7`), trust boundary (`§5`).
> - `docs/domains/ACTION_GATEWAY.md` (AG) — ActorRef (`§9.7`), ResourceRef (`§9.8`), MVP action types (`§28.2`).
> - `docs/ux/UX_INFORMATION_ARCHITECTURE.md` (UX) — status enums (`§8.1`–`§8.8`).
> - `docs/planning/DECISIONS.md` — ADR-001 … ADR-011 (locked).
>
> **Tag legend:** `[LOCKED]` locked decision · `[PROPOSED]` proposed recommendation · `[OPEN]` open question · `[MVP-SIMP]` MVP simplification · `[DEFERRED]` deferred work · `[RESEARCH]` research required.

---

## 1. Storage overview

`[LOCKED — ADR-003]` One local SQLite database (WAL journal mode) owned by the **daemon** (ADR-002 detached long-lived process). The daemon is the **single writer** to this DB. All mutation flows through the Action Gateway executor running *inside* the daemon (`[LOCKED — ADR-004]`); agents, the Brain, the Tauri UI, and any future RemoteClient submit **intents** over UDS JSON-RPC and never open the DB for writing.

```text
~/Library/Application Support/NexusOps/            [PROPOSED — macOS-only MVP, ADR-001]
  nexusops.db            SQLite (WAL) — the single daemon-owned store
  nexusops.db-wal        WAL sidecar
  nexusops.db-shm        shared-memory index
  artifacts/             content-addressed large artifacts (path+hash refs)
    <project_id>/<content_hash[0:2]>/<content_hash>.<ext>
  transcripts/           symlinks/refs only — raw transcripts stay at source
  events.jsonl           [PROPOSED] optional append-only JSONL mirror (see §10.4)
  daemon.pid             pidlock single-instance guard [LOCKED — ADR-008]
```

**Single-writer enforcement (concrete):**
- `[LOCKED — ADR-003]` SQLite opened with `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`.
- `[LOCKED — ADR-008]` `daemon.pid` pidlock guarantees one daemon instance; combined with WAL single-writer this is the durable concurrency guarantee. OS advisory/flock locks are **rejected** (don't survive restart).
- `[PROPOSED]` One long-lived `rusqlite::Connection` (or a tiny serialized write-actor) owns all writes; readers (projection queries serving the UI) may use additional read-only connections against WAL.

**Large artifacts** `[LOCKED — ADR-003]`: never stored as BLOBs. Stored on disk under `artifacts/`, content-addressed by `content_hash` (sha256), and referenced from events/rows via `path + content_hash`. See SOM `§28` (Artifact) and EM `§12.2`. Raw harness transcripts (`~/.claude/projects/.../<id>.jsonl`, `~/.codex/sessions/...`) are **referenced in place**, never copied into the DB (ADR-006); only path + hash + redaction summary are persisted.

**Project Brain keeps its OWN store** `[LOCKED — ADR-005 / PBI §1]`: the Brain is a stdio MCP sidecar that owns its index/embeddings/evidence store. It is **NOT a second writer** to `nexusops.db`. It *consumes* platform events (via the events→MCP-notifications adapter) and *proposes/queries* through the gateway. It cites platform objects by **shared ID** (§5). Any Brain-originated mutation becomes a gateway `ActionRequest`, executed by the daemon, which is the only thing that writes the DB. `[PROPOSED]` The UI never writes the DB either — it is a reattaching gateway client (ADR-002).

---

## 2. Core tables (column sketches)

> Types are SQLite affinities. `TEXT` IDs are prefixed ULIDs (§5). `*_json` columns hold validated JSON text (SQLite has no native JSON type; `json_valid()` CHECK constraints `[PROPOSED]`). Timestamps are RFC3339 UTC `TEXT` for portability/inspectability, with a parallel monotonic `seq INTEGER` for ordering (see events table). `?` denotes nullable.

### 2.1 `events` — the append-only spine `[LOCKED — ADR-003, EM §6/§12]`

```sql
CREATE TABLE events (
  event_id            TEXT PRIMARY KEY,        -- 'evt_' + ULID  (EM §6)
  seq                 INTEGER NOT NULL,         -- AUTOINCREMENT-style monotonic; canonical total order [PROPOSED]
  event_type          TEXT NOT NULL,           -- PascalCase, EM §10 taxonomy (16 categories)
  event_version       INTEGER NOT NULL,        -- per-type schema version (EM §22)
  occurred_at         TEXT NOT NULL,           -- when the fact happened (EM §6.1)
  recorded_at         TEXT NOT NULL,           -- when the daemon recorded it (clock-skew handling, EM §23)
  workspace_id        TEXT NOT NULL,           -- 'ws_' + ULID
  project_id          TEXT,                    -- 'proj_' + ULID, nullable (workspace/app-lifecycle events)
  actor_type          TEXT NOT NULL,           -- §7 extended actor enum (adds 'remote_client')
  actor_id            TEXT NOT NULL,
  source_type         TEXT NOT NULL,           -- EM §8 source model
  source_id           TEXT NOT NULL,
  correlation_id      TEXT NOT NULL,           -- groups one workflow (EM §4.8) — mandatory
  causation_id        TEXT,                    -- immediate prior event_id
  action_request_id   TEXT,                    -- FK -> action_requests.action_request_id (nullable)
  approval_id         TEXT,                    -- FK -> approvals.approval_id (nullable)
  session_id          TEXT,                    -- promoted hot-path scope (EM §6.2) [PROPOSED]
  agent_team_id       TEXT,                    -- promoted hot-path scope (EM §6.2) [PROPOSED]
  workflow_run_id     TEXT,                    -- promoted hot-path scope (EM §6.2) [PROPOSED]
  idempotency_key     TEXT,                    -- dedup (EM §23, AG §16.1)
  sensitivity         TEXT NOT NULL,           -- EM §9: public|internal|confidential|secret|restricted
  visibility          TEXT NOT NULL DEFAULT 'project',  -- user|project|workspace|system
  redaction_status    TEXT NOT NULL,           -- §15: 'unredacted'|'redacted'; writer fail-closes — NEVER persists 'unredacted' (1.1)
  redaction_engine_version TEXT,               -- which Redactor processed the payload (prefix vs 1.7 entropy); NULL pre-redaction
  payload_json        TEXT NOT NULL,           -- type-specific (CHECK json_valid) — NO large artifacts, refs only; redacted-before-INSERT (§15)
  payload_hash        TEXT,                    -- sha256(payload_json), reserved/used for dedup + future chain
  previous_event_hash TEXT,                    -- RESERVED — hash chain post-MVP [DEFERRED — ADR-003]
  schema_version      TEXT,                    -- 'event-envelope-v1' (provenance, EM §6)
  app_version         TEXT
);
CREATE UNIQUE INDEX ux_events_seq            ON events(seq);
CREATE INDEX        ix_events_project_seq    ON events(project_id, seq);
CREATE INDEX        ix_events_correlation    ON events(correlation_id, seq);
CREATE INDEX        ix_events_type_seq       ON events(event_type, seq);
CREATE INDEX        ix_events_session        ON events(session_id, seq);
CREATE UNIQUE INDEX ux_events_idempotency    ON events(idempotency_key) WHERE idempotency_key IS NOT NULL;
-- object_refs (EM §6) are NOT inline columns; they are normalized (§2.10) for graph queries.
```

`[PROPOSED]` `seq` is the canonical ordering key (not `occurred_at` — clocks skew, EM §23). `previous_event_hash` and the *use* of `payload_hash` for tamper-evidence are reserved columns wired but inert in MVP (`[DEFERRED — ADR-003]`; EM §24 lists the hash chain as a non-MVP hard requirement). FTS5 (§2.11) indexes a redaction-safe text projection of events for the audit search box.

### 2.2 `object_refs` — normalized event→object edges `[PROPOSED]`

The EM `§6` envelope carries `object_refs[]`. Persisting them inline as JSON makes graph/timeline queries (per-session, per-worktree, per-PR audit views, EM `§14.2`) slow. Normalize:

```sql
CREATE TABLE object_refs (
  event_id    TEXT NOT NULL REFERENCES events(event_id),
  object_type TEXT NOT NULL,   -- mirrors AG §9.8 ResourceRef.type + Brain object types
  object_id   TEXT NOT NULL,
  PRIMARY KEY (event_id, object_type, object_id)
);
CREATE INDEX ix_object_refs_obj ON object_refs(object_type, object_id);
```
This table is the backbone of the **ProjectGraph** projection and every per-object timeline. It is itself a projection (rebuildable from `events.payload_json`), but written in the same txn as the event for read consistency `[PROPOSED]`.

### 2.3 Projection tables — the 10 MVP projections `[LOCKED — EM §13.1; reconciled to ARCHITECTURE.md §7]`

The raw `events` log is for correctness/audit; the UI reads **projections** (EM `§4.6`, `§13`). All **10** are rebuildable from `events` (EM `§13.2`) and tracked by `projection_offsets` (§2.4). **[RECONCILED 2026-06-07 — §7]** the binding set adds **PullRequest** (§7.2) and **AgentTeam** (R-6) to the 8 sketched below: ProjectActivity, Session, ApprovalQueue, Worktree, **PullRequest**, PlanProgress, ProjectGraph, **AgentTeam**, AuditTrail, UsageLedger. Sketches below are read-model shapes, not normalized truth.

```sql
-- 1. ProjectActivity (EM §13.1) — one row per project, the sidebar/Command-Center counters
CREATE TABLE proj_project_activity (
  project_id        TEXT PRIMARY KEY,
  active_sessions   INTEGER NOT NULL DEFAULT 0,
  waiting_sessions  INTEGER NOT NULL DEFAULT 0,   -- waiting_on_permission|human|external
  failed_sessions   INTEGER NOT NULL DEFAULT 0,
  idle_sessions     INTEGER NOT NULL DEFAULT 0,
  completed_sessions INTEGER NOT NULL DEFAULT 0,
  active_teams      INTEGER NOT NULL DEFAULT 0,
  open_prs          INTEGER NOT NULL DEFAULT 0,
  blocked_tasks     INTEGER NOT NULL DEFAULT 0,
  updated_at_seq    INTEGER NOT NULL
);

-- 2. Session (EM §13.1; SOM §12) — derived current state of every session
CREATE TABLE proj_session (
  session_id          TEXT PRIMARY KEY,
  project_id          TEXT NOT NULL,
  agent_team_id       TEXT,
  display_name        TEXT,
  harness             TEXT,     -- 'claude_code' | 'codex'
  model               TEXT,
  execution_profile_id TEXT,
  worktree_id         TEXT,
  branch_name         TEXT,
  linked_task_id      TEXT,
  linked_plan_task_id TEXT,
  linked_pr_id        TEXT,
  workflow_command_id TEXT,
  status              TEXT NOT NULL,  -- §4.1 Session state machine (16 states)
  context_usage_pct   REAL,           -- nullable; Codex does NOT expose context-window % (ADR-006) [OPEN]
  token_usage_json    TEXT,
  cost_estimate       REAL,
  pending_approvals   INTEGER NOT NULL DEFAULT 0,
  last_heartbeat_at   TEXT,
  started_at          TEXT,
  completed_at        TEXT,
  updated_at_seq      INTEGER NOT NULL
);
CREATE INDEX ix_proj_session_project ON proj_session(project_id, status);

-- 3. ApprovalQueue (EM §13.1; AG §8) — the Human Input Queue read model
CREATE TABLE proj_approval_queue (
  approval_id        TEXT PRIMARY KEY,
  action_request_id  TEXT NOT NULL,
  project_id         TEXT,
  session_id         TEXT,
  agent_team_id      TEXT,
  risk_level         INTEGER NOT NULL,   -- AG §7 risk 0–4
  status             TEXT NOT NULL,      -- §4.7 Approval state machine
  requester_type     TEXT NOT NULL,      -- user|project_brain|agent_session|workflow_pack|system_policy|remote_client
  requester_id       TEXT NOT NULL,
  preview_summary    TEXT,               -- redacted preview, AG §13
  requested_at       TEXT NOT NULL,
  expires_at         TEXT,
  sort_key           TEXT,               -- composite risk DESC, age — UX §8.7 ordering
  updated_at_seq     INTEGER NOT NULL
);
CREATE INDEX ix_approval_queue_open ON proj_approval_queue(status, risk_level, requested_at);

-- 4. Worktree (EM §13.1; SOM §7) — derived; git-truth fields refreshed live via git2 (§3)
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
  status           TEXT NOT NULL,   -- §4.3 Worktree state machine — DERIVED (event hint + live git2 read)
  dirty_state      TEXT,            -- live-read cache, authoritative source = git2 (§3)
  ahead_count      INTEGER,
  behind_count     INTEGER,
  last_commit_sha  TEXT,
  pr_status        TEXT,
  git_checked_at   TEXT,            -- staleness marker for live-read cache
  updated_at_seq   INTEGER NOT NULL
);

-- 5. PlanProgress (EM §13.1; SOM §10/§11) — plan/task tree with links
CREATE TABLE proj_plan_progress (
  plan_task_id        TEXT PRIMARY KEY,
  implementation_plan_id TEXT NOT NULL,
  project_id          TEXT NOT NULL,
  phase               TEXT,
  title               TEXT,
  status              TEXT NOT NULL,  -- §4.2 Task state machine
  linked_session_ids_json TEXT,
  linked_pr_ids_json  TEXT,
  linked_ticket_ids_json  TEXT,
  architecture_anchor TEXT,
  updated_at_seq      INTEGER NOT NULL
);

-- 6. ProjectGraph (EM §13.1) — nodes+edges read model; built from object_refs (§2.2)
CREATE TABLE proj_graph_node (
  node_id     TEXT NOT NULL,   -- = object_id
  node_type   TEXT NOT NULL,
  project_id  TEXT NOT NULL,
  label       TEXT,
  status      TEXT,
  attrs_json  TEXT,
  PRIMARY KEY (node_type, node_id)
);
CREATE TABLE proj_graph_edge (
  src_type TEXT NOT NULL, src_id TEXT NOT NULL,
  dst_type TEXT NOT NULL, dst_id TEXT NOT NULL,
  edge_type TEXT NOT NULL,   -- 'owns'|'links'|'produces'|'derived_from' (SOM §35 chains)
  project_id TEXT NOT NULL,
  PRIMARY KEY (src_type, src_id, dst_type, dst_id, edge_type)
);

-- 7. AuditTrail (EM §13.1, §14) — human-readable ordered timeline (rendered rows)
CREATE TABLE proj_audit_trail (
  event_id      TEXT PRIMARY KEY REFERENCES events(event_id),
  seq           INTEGER NOT NULL,
  project_id    TEXT,
  occurred_at   TEXT NOT NULL,
  scope_json    TEXT,         -- {workspace,project,session,team,task,worktree,pr} for §14.2 scoped views
  headline      TEXT NOT NULL,-- "Project Brain proposed starting Phase 2.3 backend team."
  actor_label   TEXT,
  outcome       TEXT,         -- requested|approved|denied|succeeded|failed|rolled_back
  sensitivity   TEXT NOT NULL
);
CREATE INDEX ix_audit_scope ON proj_audit_trail(project_id, seq);

-- 8. UsageLedger (EM §13.1, §18) — tokens/context/cost rollups
CREATE TABLE proj_usage_ledger (
  ledger_id           TEXT PRIMARY KEY,  -- composite e.g. day|project|session|model|profile
  project_id          TEXT,
  session_id          TEXT,
  execution_profile_id TEXT,
  model               TEXT,
  bucket_day          TEXT,              -- YYYY-MM-DD rollup
  tokens_in           INTEGER,
  tokens_out          INTEGER,
  context_pct_max     REAL,
  cost_estimate       REAL,
  metric_quality      TEXT,              -- exact|estimated|unavailable (EM §10.15)
  updated_at_seq      INTEGER NOT NULL
);
```

`[MVP-SIMP]` MVP ships exactly these 8 (UX-critical: ProjectActivity, Session, ApprovalQueue, ProjectGraph, AuditTrail per EM `§24`; the other 3 round out the chains in SOM `§35`). Workflow-pack/cc-crew progress projections and richer usage breakdowns are `[DEFERRED]` to P1 (EM `§25`).

**[IMPLEMENTED 1.2]** All **10** projection tables created (migration 3), incl. `proj_pull_request` + `proj_agent_team` (no DDL sketch existed above — authored minimal, status-bound to the frozen §5.1 PullRequest/AgentTeam enums; a fuller sketch reconciles when their projectors land in P7/P9). Projector **bodies**: 4 Phase-1-feedable (Session, ProjectGraph/object_refs, AuditTrail+FTS, ProjectActivity) folded **in-band** in the event-commit txn; the other 6 re-homed to their producing phases (ARCHITECTURE Appendix A + `daemon/CLAUDE.md` cross-doc). `(P1.2, brief 004)`

### 2.4 `projection_offsets` `[LOCKED — EM §12/§13.2]`

```sql
CREATE TABLE projection_offsets (
  projection_name   TEXT PRIMARY KEY,   -- 'session' | 'approval_queue' | ...
  last_event_id     TEXT,
  last_seq          INTEGER NOT NULL DEFAULT 0,  -- ordering key actually used for resume
  last_processed_at TEXT,
  state             TEXT NOT NULL DEFAULT 'healthy',  -- healthy|rebuilding|degraded (EM §13.2/§23)
  schema_version    INTEGER NOT NULL DEFAULT 1        -- bumped to force a rebuild on projector code change
);
```
Each projector advances `last_seq` in the **same transaction** as the rows it writes (`[PROPOSED]` — atomic apply, so a crash never leaves an offset ahead of applied rows; EM `§13.2` "safe after app crash"). On startup the daemon replays `events WHERE seq > last_seq` per projection.

### 2.5 `outbox` — reliable side-effects `[LOCKED — ADR-003, EM §12]`

```sql
CREATE TABLE outbox (
  outbox_id       TEXT PRIMARY KEY,   -- 'out_' + ULID
  destination     TEXT NOT NULL,      -- 'brain_mcp' | 'github' | 'linear' | 'notifier' | 'jsonl_mirror' | future:'remote_relay'
  event_id        TEXT NOT NULL REFERENCES events(event_id),
  payload_json    TEXT NOT NULL,      -- redacted per-destination payload
  status          TEXT NOT NULL DEFAULT 'pending',  -- pending|in_flight|delivered|failed|dead
  retry_count     INTEGER NOT NULL DEFAULT 0,
  next_attempt_at TEXT,
  last_error      TEXT,
  created_at      TEXT NOT NULL
);
CREATE INDEX ix_outbox_due ON outbox(status, next_attempt_at);
```
`[LOCKED — ADR-003]` event + projection + outbox writes commit in **one transaction** (the transactional-outbox pattern: a fact is never recorded without its delivery intents, and never delivered without being recorded). The Brain-notification adapter (ADR-005), GitHub/Linear syncers (ADR-007), the notifier, and the optional JSONL mirror (§10.4) all drain the outbox with backoff. Remote-relay delivery is a future destination `[DEFERRED — DFR §6]`.

**[IMPLEMENTED 1.3]** `outbox` created (migration 4); rows written in the event-commit txn (recorded-iff-intended); the **§15 *sync* sink** — per-destination payload derives from the already-redacted event (filter-only). At-least-once drainer (`drain_once`: backoff + retryable/terminal + bounded dead-letter; `reset_in_flight` on `open`; idempotent consumers). `out_` id is **daemon-internal** (not a frozen contract ID); `jsonl_mirror` is the one Phase-1 real destination; the drainer Tokio spawn is 1.6-wired; the brain_mcp/github/linear/notifier destination adapters re-home to P8/P7/P10. `(P1.3, brief 005)`

### 2.6 `leases` — distributed locks `[LOCKED — ADR-008]`

```sql
CREATE TABLE leases (
  resource_id    TEXT NOT NULL,      -- e.g. 'worktree:wt_..' | 'branch:proj_..:agent/x' | 'action:idemkey' (opaque key)
  lease_kind     TEXT NOT NULL,      -- MVP seeds 'resource_mutation'; closed kind-enum deferred to the Phase-2 gateway; future:'shared' (branch co-ownership, §8)
  owner_id       TEXT,               -- NULL when free/released; session_id / team_id / executor_id holding the lease
  fencing_token  INTEGER NOT NULL,   -- monotonic per-(resource_id,lease_kind) HIGH-WATER mark, MANDATORY — persisted; survives restart
  acquired_at    TEXT,               -- NULL when free
  heartbeat_at   TEXT,               -- NULL when free
  expires_at     TEXT,               -- NULL when free
  PRIMARY KEY (resource_id, lease_kind)
);
CREATE INDEX ix_leases_expiry ON leases(expires_at);
```
`[LOCKED — ADR-008]` SQLite LEASE table + mandatory monotonic fencing tokens + pidlock single-instance. An OS flock for the *lease itself* is rejected (a lease must survive restart; flock doesn't) — but the *pidlock* (single-instance) correctly IS a std advisory file lock (it SHOULD release on process death). An executor presents its lease's `fencing_token` to the gateway, which rejects any write not from a live lease holder — protection against a paused-then-resumed stale session clobbering a resource another session now owns. Expired leases are reclaimable; reclamation mints a *new* token, invalidating the old holder.

**[IMPLEMENTED 1.4]** `leases` created (migration 5; `SUPPORTED_USER_VERSION` 4→5). **PK = `(resource_id, lease_kind)`** (composite — a resource can hold distinct lease kinds; resolves the latent exclusive-vs-shared single-PK conflict the original sketch implied). `fencing_token` = a **persisted monotonic high-water mark per `(resource_id, lease_kind)`** (new→1, reclaim→+1, minted under `BEGIN IMMEDIATE`); `release`/`reap_once` NULL the holder fields (`owner_id`/`acquired_at`/`heartbeat_at`/`expires_at`) but **keep** the token → monotonicity survives restart. **Authority = a LIVE lease (Option B, human-ratified):** `validate_held` = owner-match ∧ `token == fencing_token` ∧ `expires_at > now`; "stale" = expired **OR** superseded → `fencing_conflict` (safety rule #6, never auto-resolved). `pidlock` = std advisory `File::try_lock` (OS-fd held → auto-released on death → immune to PID reuse). Reaper `reap_once` is the deterministic unit; the Tokio spawn is 1.6-wired. **Daemon-internal** — no `shared/` surface, no CONTRACT_VERSION bump (stays 0.8.0). `(P1.4, brief 006; LESSON §6)`

### 2.7 `artifacts` — large-content references `[LOCKED — ADR-003, EM §12.2, SOM §28]`

```sql
CREATE TABLE artifacts (
  artifact_id        TEXT PRIMARY KEY,   -- 'art_' + ULID
  project_id         TEXT,
  type               TEXT NOT NULL,      -- transcript|diff|patch|test_log|file_snapshot|scrollback|embedding_bundle|episode_card
  path               TEXT NOT NULL,      -- absolute path on local FS (artifacts/ or in-place harness transcript)
  content_hash       TEXT NOT NULL,      -- sha256 (content-addressed; in-place transcripts hashed at capture)
  size               INTEGER,
  redaction_status   TEXT NOT NULL DEFAULT 'unredacted', -- unredacted|redacted|summarized|quarantined (EM §23)
  sensitivity        TEXT NOT NULL DEFAULT 'restricted', -- transcripts default restricted (EM §9)
  summary            TEXT,               -- redaction-safe summary surfaced in UI/Brain
  producer_session_id TEXT,
  is_in_place        INTEGER NOT NULL DEFAULT 0,  -- 1 = harness-owned file (~/.claude, ~/.codex), refs only (ADR-006)
  created_at         TEXT NOT NULL
);
CREATE INDEX ix_artifacts_hash ON artifacts(content_hash);
CREATE INDEX ix_artifacts_producer ON artifacts(producer_session_id);
```
`[LOCKED — ADR-006]` harness transcripts are referenced in place (`is_in_place=1`), hardened to `0600` for Codex rollout files; never copied as BLOBs. `[PROPOSED]` content-addressing lets identical diffs/snapshots dedupe across sessions.

### 2.8 Registry / config tables — durable domain objects (NOT event-derived)

These are the authoritative rows the daemon mutates directly (each mutation still emits an event for audit, but the **row is the source of truth**, not a projection — see §3). They correspond to SOM objects whose identity/config the user owns rather than a derived state.

```sql
-- projects (SOM §5)
CREATE TABLE projects (
  project_id   TEXT PRIMARY KEY,   -- 'proj_' + ULID
  workspace_id TEXT NOT NULL,
  name         TEXT NOT NULL,
  root_path    TEXT NOT NULL,
  policy_json  TEXT,               -- per-project policy/privacy (transcript ingestion consent, PBI §8)
  brain_enabled INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL,
  archived_at  TEXT
);

-- repositories (SOM §6) — identity/config durable; live git status read via git2 (§3)
CREATE TABLE repositories (
  repo_id        TEXT PRIMARY KEY,  -- 'repo_' + ULID
  project_id     TEXT NOT NULL REFERENCES projects(project_id),
  remote_url     TEXT,
  provider       TEXT,              -- github|...
  owner          TEXT, name TEXT,
  local_path     TEXT NOT NULL,
  default_branch TEXT,
  created_at     TEXT NOT NULL
  -- NO current_head_sha / git_status here: those are git-derived (§3), cached in proj_worktree/live reads
);

-- execution_profiles (SOM §15) — local account/runtime config; secrets in keychain, NOT here (ADR-007/011)
CREATE TABLE execution_profiles (
  execution_profile_id TEXT PRIMARY KEY,  -- 'exec_' + ULID
  workspace_id  TEXT NOT NULL,
  provider      TEXT NOT NULL,     -- anthropic|openai
  harness       TEXT NOT NULL,     -- claude_code|codex
  model         TEXT,
  account_alias TEXT,
  keychain_ref  TEXT,              -- pointer into OS keychain (keyring crate), NEVER the secret
  usage_policy_json TEXT,
  status        TEXT NOT NULL DEFAULT 'available',  -- §4.8 ExecutionProfile state machine
  created_at    TEXT NOT NULL
);

-- workflow_instances (SOM §17) — detected/personalized pack state per project
CREATE TABLE workflow_instances (
  workflow_instance_id TEXT PRIMARY KEY, -- 'wfi_' + ULID
  project_id    TEXT NOT NULL REFERENCES projects(project_id),
  workflow_pack_id TEXT NOT NULL,        -- e.g. 'cc-crew'
  status        TEXT NOT NULL,           -- §4.5 WorkflowInstance state machine
  manifest_hash TEXT,                    -- drift detection (EM §10.3)
  personalization_run_id TEXT,
  created_at    TEXT NOT NULL
);

-- integration_connections (SOM §33) — GitHub/Linear connection rows; tokens in keychain (ADR-007/011)
CREATE TABLE integration_connections (
  connection_id TEXT PRIMARY KEY,  -- 'conn_' + ULID
  workspace_id  TEXT NOT NULL,
  provider      TEXT NOT NULL,     -- github|linear
  account_label TEXT,
  keychain_ref  TEXT,              -- pointer only; refresh metadata for Linear 24h refresh (ADR-007)
  scopes_json   TEXT,
  status        TEXT NOT NULL DEFAULT 'connected',
  connected_at  TEXT, expires_at TEXT
);

-- plan_tasks (SOM §11) — durable parsed plan rows (truth = parsed plan file; status churns via events → see §3 / §8)
CREATE TABLE plan_tasks (
  plan_task_id  TEXT PRIMARY KEY,  -- 'plan_task_' + ULID (or stable hash of plan+anchor)
  implementation_plan_id TEXT NOT NULL,
  project_id    TEXT NOT NULL,
  phase         TEXT, title TEXT,
  architecture_anchor TEXT,
  source        TEXT,              -- implementation_plan|linear|github
  source_ref    TEXT,
  created_at    TEXT NOT NULL
  -- status is event-derived in proj_plan_progress; this table is the durable identity/spine
);

-- command_registry (SOM §19 WorkflowCommand) — discovered commands per workflow instance
CREATE TABLE command_registry (
  workflow_command_id TEXT PRIMARY KEY, -- 'wfc_' + ULID
  workflow_instance_id TEXT NOT NULL REFERENCES workflow_instances(workflow_instance_id),
  project_id    TEXT NOT NULL,
  name          TEXT NOT NULL,    -- '/team-start', '/tdd', ...
  input_schema_json TEXT,         -- nullable; older formats have none (SOM §37 Q9) [OPEN]
  discovered_at TEXT NOT NULL
);
```

### 2.9 `action_requests` & `approvals` — gateway durable rows `[LOCKED — ADR-004, AG §8]`

The gateway's lifecycle is event-driven, but the *current* request/approval row is a durable record the executor reads (idempotency, lease ownership, fencing token). These straddle registry + projection; treated as **durable rows kept consistent by their own events**.

```sql
CREATE TABLE action_requests (
  action_request_id TEXT PRIMARY KEY,  -- 'act_' + ULID
  project_id      TEXT,
  action_type     TEXT NOT NULL,       -- AG §28.2 MVP action types
  requester_type  TEXT NOT NULL,       -- AG §9.7 ActorRef + 'remote_client' (§6, §7)
  requester_id    TEXT NOT NULL,
  resource_refs_json TEXT NOT NULL,    -- AG §9.8 ResourceRef[]
  inputs_json     TEXT,
  risk_level      INTEGER NOT NULL,    -- AG §7 risk 0–4
  idempotency_key TEXT,                -- AG §16.1
  fencing_token   INTEGER,             -- the lease token this execution must present (§2.6)
  status          TEXT NOT NULL,       -- ActionRequest (R-5, §5.1; frozen 15): submitted|previewed|policy_decided|awaiting_approval|approved|denied|queued|executing|succeeded|failed|partially_succeeded|rolled_back|rollback_failed|cancelled|expired
  preview_json    TEXT,                -- AG §13
  created_at      TEXT NOT NULL
);
CREATE UNIQUE INDEX ux_action_idem ON action_requests(idempotency_key) WHERE idempotency_key IS NOT NULL;

CREATE TABLE approvals (
  approval_id     TEXT PRIMARY KEY,    -- 'appr_' + ULID
  action_request_id TEXT NOT NULL REFERENCES action_requests(action_request_id),
  status          TEXT NOT NULL,       -- §4.7 Approval state machine
  required_approver TEXT,              -- AG §9.7: 'current_user'|'project_owner'|ActorRef
  decided_by      TEXT,
  decided_at      TEXT,
  expires_at      TEXT,
  created_at      TEXT NOT NULL
);
```

**[IMPLEMENTED — the 2.1c MIGRATION_8 plan dimension (`b9e00a1`); `SUPPORTED_USER_VERSION` 7→8]** the O-3 bundled-plan addition — a thin `action_plans` metadata table + the `plan_id` FK + the plan-level-approval generalization:

```sql
CREATE TABLE action_plans (
  plan_id         TEXT PRIMARY KEY,    -- 'aplan_' + ULID
  project_id      TEXT,
  requester_type  TEXT NOT NULL,       -- §6.2 RequesterType (plan submitter)
  requester_id    TEXT NOT NULL,
  title           TEXT NOT NULL,
  overall_risk    INTEGER NOT NULL,    -- §6.2 RiskLevel 0-4
  approval_mode   TEXT NOT NULL,       -- §6.2 ApprovalMode (approve_all|step_by_step|mixed|blocked)
  created_at      TEXT NOT NULL
);
ALTER TABLE action_requests ADD COLUMN plan_id TEXT;  -- nullable FK → action_plans (single action = NULL)
-- approvals + proj_approval_queue GENERALIZED (table-rebuild): action_request_id → NULLABLE, + plan_id —
-- so a plan-level approve-all approval (scope=Plan, plan_id set, action_request_id NULL) persists,
-- matching the frozen §6.2 Approval.action_request_id: Option (the 2.1b NOT NULL was a single-action shortcut).
-- proj_approval_queue is a projection → DROP+CREATE with the new shape + reset its offset (re-fold).
```

The plan = grouping-over-`action_requests` (single action = `plan_id` NULL); `submit_action_plan` is ONE atomic gateway txn (whole-plan fail-closed). An uncatalogued-action_type step rejects the whole plan (2.2 #11).

### 2.10 (reserved) — see §6 for the 4 new desktop objects (`devices`, `remote_clients`, `local_runners`, `event_projections`).

### 2.11 FTS5 search `[LOCKED — ADR-003]`

```sql
CREATE VIRTUAL TABLE events_fts USING fts5(
  headline, actor_label, project_id UNINDEXED, event_id UNINDEXED,
  content='proj_audit_trail', content_rowid='rowid'
);
```
`[PROPOSED]` FTS5 indexes the **redaction-safe audit projection**, not raw `payload_json` (avoids the event log becoming a secret-searchable dump; EM `§4.5`/`§9`). Powers the Command Center / audit search box.

**[IMPLEMENTED 1.2 — deviation recorded]** 1.1 shipped a standalone `fts_events(event_id UNINDEXED, body)` scaffold (not the contentless `events_fts content='proj_audit_trail'` above); the 1.2 AuditTrail projector **populates `fts_events`** with the redaction-safe headline. The redaction-safety intent holds (indexes the rendered audit text, never `payload_json`). The contentless `content=`-linked form is a deferred refinement (no functional gap for MVP search). `(P1.2, brief 004 Q3)`

---

## 3. Source-of-truth rule

Every piece of state has exactly **one** authoritative source. The daemon must never treat a projection as truth when a live source exists. `[PROPOSED]` classification:

| State class | Source of truth | Where it lives | Rebuild / refresh |
|---|---|---|---|
| **Event-derived (projections)** | the `events` log | `proj_*` tables (§2.3), `object_refs` (§2.2) | fully rebuildable from `events` (EM §13.2); offsets in `projection_offsets` |
| **Durable registry** | the registry row itself | `projects`, `repositories`, `execution_profiles`, `workflow_instances`, `integration_connections`, `plan_tasks`, `command_registry` (§2.8) | NOT rebuildable from events; backed up with the DB. Mutations still emit audit events, but the row is canonical. |
| **Gateway state** | `action_requests` / `approvals` rows, kept consistent by their events | §2.9 | row is durable; events are the audit/replay trail |
| **Locks** | `leases` table | §2.6 | authoritative; expired leases reclaimable with new fencing token |
| **Git/filesystem-derived** | the actual git repo / working tree | read **live via git2-rs** (ADR-007 reads); cached in `proj_worktree.dirty_state/ahead/behind/last_commit_sha` with `git_checked_at` staleness | never trust the cache for a mutation decision — re-read git2 before acting; mutations use git CLID (ADR-007) |
| **Harness-derived** | the harness's own transcript/thread (`~/.claude/projects/...jsonl`, `~/.codex/sessions/...`, `codex app-server` thread state) | referenced via `artifacts` (`is_in_place=1`, §2.7); machine state from SDK/app-server streams (ADR-006) | **NEVER scrape PTY for machine state** (ADR-006); replay via `claude --resume` / `codex thread/resume` (ADR-010) |
| **Secrets** | OS keychain | `keyring` crate (ADR-007/011); only `keychain_ref` pointers in DB | never in `events`/rows (EM §18) |
| **Brain memory/index** | Project Brain's OWN store | sidecar, not `nexusops.db` (§1, PBI §1) | Brain re-indexes from events + code/docs |

**Concrete consequences:**
- `[LOCKED — ADR-006/ADR-009]` Worktree `status` (UX §8.3) is *not* stored as truth — it is **derived** at read time from a live git2 read combined with event hints (last `WorktreeDirtyStateChanged`). PTY/scrollback is for human display only (ADR-009); machine state never comes from it.
- `[PROPOSED]` Session `context_usage_pct` is harness-derived and may be `NULL` for Codex (ADR-006 gap: no context-window % exposed). The UI must render "unknown", not "0%".
- `[LOCKED — ADR-005]` The Brain never writes any of these; it reads events and cites by shared ID.

---

## 4. Status state machines `[SUPERSEDED — see ARCHITECTURE.md §5.1 (LOCKED — R-4..R-9; 10 machines)]`

> **SUPERSEDED (2026-06-07, Phase-0-exit `/arch-finalize`).** This rough-draft section enumerated **8** machines; the binding, reconciled contract is **`ARCHITECTURE.md` §5.1 — 10 machines**: the 8 below **+ ActionRequest** (R-5 split: approval-decision axis vs execution axis) **+ AgentTeam** (R-6 promoted to first-class). The §5.1 enums are frozen in `shared/` (0.5; **ExecutionProfile** held → 0.5b). Where the values below differ, **§5.1 wins**; kept for provenance.

Canonical enum values below come from SOM object lifecycles and UX `§8.1`–`§8.8` (the two must stay in sync; finalize should reconcile any drift). `[PROPOSED]` Each is stored as a `TEXT status` on its projection/registry row; transitions are driven by events and validated by the projector (illegal transitions emit a degraded-state marker, EM `§22`/`§23`, not a silent overwrite).

### 4.1 Session `[LOCKED — SOM §12, UX §8.1]` (event-derived → `proj_session.status`)
`creating → starting → active`; runtime sub-states `thinking | running_command | editing_files | running_tests`; waiting sub-states `waiting_on_permission | waiting_on_human_input | waiting_on_external_service`; `idle → stale`; terminal `failed | completed | archived | killed`. Driven by `SessionStarted`, `SessionStatusChanged`, `SessionHeartbeatReceived` (heartbeat from `statusLine` for Claude / `push status` for Codex, ADR-006), `SessionWaitingOn*`, `SessionFailed/Completed/Killed`. `stale` is derived by the daemon when `now - last_heartbeat_at > threshold` (no event needed — a *time-derived* transition `[PROPOSED]`).

### 4.2 Task / PlanTask `[LOCKED — SOM §9/§11, UX §8.2]` (event-derived → `proj_plan_progress.status`)
`unassigned → queued → assigned → in_progress`; `blocked | needs_clarification`; `changes_ready → pr_opened → needs_review → requested_changes`; terminal `merged | closed | abandoned`. **[RESOLVED — R-8, §5.1]** Task and PlanTask are **one `tasks` table** with `kind ∈ {plan_task, external_task}` over a **superset** machine (kind-scoped subsets render per view) — not two objects (ADR-012).

### 4.3 Worktree `[LOCKED — SOM §7, UX §8.3]` (**DERIVED**, not stored as truth — §3)
`creating | clean | dirty | untracked_files | conflicts | behind_base | ahead_of_base | pr_open | merged | prunable | locked | deleted`. Computed from live git2 read + event hints; `proj_worktree.status` is a cache stamped with `git_checked_at`.

### 4.4 PullRequest `[LOCKED — SOM §29, UX §8.4]` (event-derived; remote truth = GitHub via octocrab/webhooks, ADR-007)
`draft | open | checks_pending | checks_failing | needs_review | changes_requested | approved | mergeable | conflict | merged | closed`. Driven by `PullRequest*` events sourced from the GitHub syncer; **remote authority is GitHub** — local row is a synced cache.

### 4.5 WorkflowInstance `[LOCKED — SOM §17, UX §8.5]` (durable row `workflow_instances.status`, churned by events)
`not_detected | pack_available | needs_personalization | personalization_in_progress | generated_review_required | active | ready_for_team_run | degraded | drift_detected | upgrade_available | archived | detached`. _(R-7: `ready_for_team_mode`→`ready_for_team_run`; team-run-in-progress is tracked on AgentTeam, not the instance.)_

### 4.6 ProjectBrain `[LOCKED — SOM §23, UX §8.6]` (reported by sidecar via MCP notifications → events; Brain owns the underlying index, §1)
`not_configured | indexing | ready | partial_index | stale | graph_degraded | transcript_ingestion_off | transcript_ingestion_active | reindex_required | error`. The daemon stores the *last reported* status; the Brain's store is authoritative for its own index.

### 4.7 Approval `[LOCKED — AG §8, UX §8.7; SPLIT per R-5 — see §5.1]` (event-derived → `proj_approval_queue.status` + durable `approvals.status`)
**[RECONCILED — R-5, §5.1]** This draft conflated approval + execution into one machine; **R-5 split them into two**: **Approval** = the decision axis `{requested, previewed, awaiting_approval, approved, denied, edited, auto_approved_by_policy, expired, cancelled, escalated}` (10), and the execution states (`queued → executing → succeeded | failed | partially_succeeded | rolled_back | rollback_failed`) moved to the **new ActionRequest** machine (15, §5.1). The authoritative `Action*`/`Approval*` events are emitted **only** by the gateway (ADR-004, EM §16).

### 4.8 ExecutionProfile `[LOCKED — SOM §15, UX §8.8]` (durable row `execution_profiles.status`)
`available | active | in_use | rate_limited | auth_expired | misconfigured | disabled | unknown`. `rate_limited`/`auth_expired` set by adapter telemetry + keychain self-test (ADR-007/011).

---

## 5. Shared ID strategy

`[LOCKED — PBI §3]` Adopt the **22 shared IDs** from PBI §3 as the cross-product contract between the platform and Project Brain: `workspace_id, project_id, repo_id, worktree_id, branch_name, commit_sha, session_id, agent_team_id, execution_profile_id, workflow_pack_id, workflow_instance_id, workflow_command_id, implementation_plan_id, plan_task_id, architecture_anchor, linear_issue_id, github_issue_number, pr_number, action_request_id, event_id, artifact_id, evidence_item_id`. Every event `object_ref`, every Brain evidence chip, and every gateway `ResourceRef` (AG §9.8) uses these exact IDs.

**ID format** `[LOCKED — R-1, §5.2; frozen 0.5 in `shared/src/ids.rs` as newtypes]`: **prefixed ULIDs** for platform-minted IDs — `<prefix>_<ULID>` (e.g. `sess_01JZ...`, `evt_01JZ...`, `wt_01JZ...`). Rationale:
- ULIDs are lexicographically sortable (Crockford base32) → `seq`-aligned event ordering and natural sidebar ordering without extra columns.
- Type-prefixes make IDs self-describing in logs, audit headlines, and Brain citations (matches EM §6 examples `evt_`, `proj_`, `sess_`, `wt_`).
- Monotonic-ish creation time embedded → cheap "recent" queries.
- `[PROPOSED]` Use `ulid` crate; prefixes registered in a single `id_kind` table/const so the gateway and Brain agree.

**External IDs are NOT re-minted**: `branch_name`, `commit_sha`, `github_issue_number`, `pr_number`, `linear_issue_id` are carried as their native external values (a `pr_number` is `84`, not a ULID) — they're foreign keys into provider systems. `architecture_anchor` is a stable doc anchor string, not a ULID.

**Harness → `session_id` mapping** `[LOCKED — ADR-006; SOM §37 Q5; ADR-006 Codex gap]`:
- **Claude Code (settable id):** the platform mints `sess_<ULID>` and passes it as the session id to the Agent SDK; transcript at `~/.claude/projects/.../<id>.jsonl` is keyed by it. One-to-one, clean.
- **Codex (NO settable id):** `thread/start{cwd}` returns a Codex-chosen `thread_id`; the platform still mints its own `sess_<ULID>` as the canonical `session_id` and stores the mapping. Re-association after restart uses `thread/list?cwd=` keyed on **`(cwd, returned thread_id)`** (ADR-006). `[PROPOSED]` mapping table:

```sql
CREATE TABLE harness_session_map (
  session_id    TEXT PRIMARY KEY REFERENCES proj_session(session_id),  -- platform canonical 'sess_'+ULID
  harness       TEXT NOT NULL,            -- claude_code|codex
  harness_native_id TEXT,                 -- claude: = session_id (settable); codex: returned thread_id
  cwd           TEXT,                     -- codex re-assoc key (cwd + native_id)
  rollout_path  TEXT,                     -- codex ~/.codex/sessions ref (hardened 0600, ADR-006)
  updated_at    TEXT NOT NULL
);
CREATE INDEX ix_hsm_cwd ON harness_session_map(harness, cwd);
```
This is the concrete answer to SOM §37 Q5 (canonical ID when wrapping an existing terminal session): **the platform's `sess_` ULID is always canonical; the harness-native id is a mapped attribute.**

---

## 6. Closing the gap: the 4 desktop-addendum objects `[PROPOSED]`

DFR §7 requires Device, RemoteClient, LocalRunner, EventProjection, but SOM defines only Terminal (`§13`). These are **proposed** here so the schema is build-ready and the future iOS companion (EM §2.2/§19, DFR §6) is not blocked. All four are durable registry rows except EventProjection (which is the persisted catalog around §2.3/§2.4). `[MVP-SIMP]` Only `local_runners` and `event_projections` are exercised in MVP; `devices`/`remote_clients` are created-but-dormant scaffolding for the iOS stretch (`[DEFERRED]`).

### 6.1 Device `[PROPOSED]`
**Purpose:** a registered physical/logical machine that may observe or request control — the desktop host itself, and (future) a paired iPhone. The trust-boundary anchor (DFR §5).
**Key fields:** `device_id` (`dev_`+ULID), `device_kind` (`desktop_host` | `ios_companion` `[DEFERRED]`), `display_name`, `device_public_key` (for future pairing, EM §20), `platform`, `registered_at`, `revoked_at?`, `last_seen_at`.
**Lifecycle:** `registered → active → revoked`. The desktop host is auto-registered on first daemon start. Emits `RemoteDeviceRegistered/Revoked` (EM §10.16/§20).
```sql
CREATE TABLE devices (
  device_id     TEXT PRIMARY KEY,
  device_kind   TEXT NOT NULL,        -- desktop_host | ios_companion
  display_name  TEXT,
  device_public_key TEXT,
  platform      TEXT,
  status        TEXT NOT NULL DEFAULT 'registered',
  registered_at TEXT NOT NULL, revoked_at TEXT, last_seen_at TEXT
);
```

### 6.2 RemoteClient `[PROPOSED]`
**Purpose:** an authenticated *remote* connection/session on a non-host Device that submits redacted-projection reads and **remote ActionRequests** through the gateway (DFR §6 hard boundary: never a remote shell). One Device may have multiple RemoteClient sessions over time.
**Key fields:** `remote_client_id` (`rc_`+ULID), `device_id` (FK → devices), `capability_scope_json` (redaction policy + allowed action risk ceiling, EM §20), `network_path` (`relay` | `tunnel` — DFR §6, EM §19), `paired_at`, `last_active_at`, `revoked_at?`.
**Lifecycle:** `pairing → paired/active → revoked`.
**Mapping to gateway/event (the key contract):** when a RemoteClient submits an action, it becomes `action_requests.requester_type = 'remote_client'` and `requester_id = remote_client_id`; the resulting events carry **`actor_type = 'remote_client'`**, `actor_id = remote_client_id`. This **extends** the AG §9.7 ActorRef union and EM §7 actor enum with `remote_client` (**[RESOLVED 2026-06-07 — R-2, §7.1]** EM §7's legacy `remote_device` was swept forward to `remote_client`; the 10-value audit enum is frozen in `shared/src/actor.rs`). All remote mutations therefore flow `RemoteClient → gateway → daemon executor`, never touching the DB/PTY directly (DFR §5/§6, locked by ADR-004).
```sql
CREATE TABLE remote_clients (
  remote_client_id TEXT PRIMARY KEY,
  device_id     TEXT NOT NULL REFERENCES devices(device_id),
  capability_scope_json TEXT,
  network_path  TEXT,                 -- relay | tunnel
  status        TEXT NOT NULL DEFAULT 'pairing',
  paired_at     TEXT, last_active_at TEXT, revoked_at TEXT
);
```

### 6.3 LocalRunner `[PROPOSED]`
**Purpose:** the daemon-owned execution surface that owns PTYs, launches harness processes, runs git/worktree ops, and hosts the executor adapters (DFR §4, ADR-002/009). It is the "local runner" lane in the process model; sessions/terminals are *bound to* a runner. MVP has exactly one (the daemon itself), but modeling it explicitly lets the gateway attribute execution and lets fencing/leases name an owner.
**Key fields:** `local_runner_id` (`lr_`+ULID), `device_id` (FK → devices, the desktop host), `pid`, `started_at`, `status`, `capabilities_json` (pty/git/harness adapters present).
**Lifecycle:** `starting → running → degraded → stopped`. Emits `DesktopDaemonStarted/Stopped` (EM §10.1). On daemon restart a new `local_runner_id` is minted; survival/resume (ADR-010) re-binds sessions to it.
```sql
CREATE TABLE local_runners (
  local_runner_id TEXT PRIMARY KEY,
  device_id     TEXT NOT NULL REFERENCES devices(device_id),
  pid           INTEGER,
  status        TEXT NOT NULL DEFAULT 'starting',
  capabilities_json TEXT,
  started_at    TEXT NOT NULL, stopped_at TEXT
);
```
`[PROPOSED]` add `local_runner_id` to `proj_session` (nullable) so a session records which runner instance executed it — useful for the resume path (ADR-010) and audit.

### 6.4 EventProjection `[PROPOSED]`
**Purpose:** the first-class catalog/metadata object for each projection (EM §4.6, the SOM gap noted in the prompt). It is the registry *describing* the 10 projections (§2.3) plus future ones, including for redacted **remote** projections the iOS companion would consume (EM §19/§20). `projection_offsets` (§2.4) is its per-instance progress; this is its definition/metadata.
**Key fields:** `event_projection_id` (`eprj_`+ULID — frozen 0.5, de-collided from `proj_`), `name` (matches `projection_offsets.projection_name`), `projector_version`, `redaction_policy_id?` (for remote variants), `target` (`local_ui` | `remote_companion` `[DEFERRED]`), `state` (`healthy` | `rebuilding` | `degraded`), `last_seq`.
**Lifecycle:** `defined → building → healthy ↔ rebuilding/degraded`. Bumping `projector_version` triggers a rebuild (§7).
```sql
CREATE TABLE event_projections (
  event_projection_id TEXT PRIMARY KEY,
  name          TEXT NOT NULL UNIQUE,   -- joins projection_offsets.projection_name
  projector_version INTEGER NOT NULL,
  redaction_policy_id TEXT,
  target        TEXT NOT NULL DEFAULT 'local_ui',  -- local_ui | remote_companion
  state         TEXT NOT NULL DEFAULT 'defined',
  last_seq      INTEGER NOT NULL DEFAULT 0
);
```

---

## 7. Migrations, rebuild/recovery, retention, fallback

### 7.1 Migrations `[LOCKED — ADR-003]`
SQLite `PRAGMA user_version` drives forward-only, ordered migrations applied by the daemon on startup **inside a transaction** before serving any client. `[PROPOSED]` migrations live as embedded ordered SQL (e.g. `refinery`/`rusqlite_migration`); `user_version` = the highest applied migration index. Rules (EM §22): additive columns are minor; field-meaning changes require a new **event_version** (raw events are never mutated, EM §22). A migration that changes a projector's shape bumps `event_projections.projector_version` and forces a rebuild (§7.2) rather than back-filling raw events.

### 7.2 Projection rebuild & crash recovery `[LOCKED — EM §13.2/§23]`
- **Startup replay:** for each projection, replay `events WHERE seq > projection_offsets.last_seq`, advancing the offset in the same txn as applied rows (§2.4) → crash-safe (an interrupted apply rolls back together).
- **Full rebuild:** `TRUNCATE proj_*; set last_seq=0; replay all events`. Safe because raw `events` are the source of truth and untouched (EM §13.2: "projection corruption must not corrupt raw events").
- **Degraded handling:** on a projector error or unknown `event_version`, mark `projection_offsets.state='degraded'` + emit a visible degraded marker (EM §22/§23), **skip** the bad event, continue — never crash, never corrupt raw events.
- **Recovery on daemon restart (ADR-010):** rebuild projections from `events`; reconcile live state by re-reading git2 (worktrees, §3), pinging the Brain sidecar (ADR-005), and resuming harnesses (`claude --resume` / `codex thread/resume`, or serialized-scrollback replay + relaunch, ADR-010). Reclaim/expire `leases` and mint new fencing tokens (§2.6).

### 7.3 Retention `[LOCKED — EM §21]`
- Audit-critical events (EM §14.1) kept **indefinitely** unless the user deletes the project/workspace.
- High-volume terminal-output `artifacts` references: **configurable** retention (default short); raw transcripts referenced only under project policy/consent (EM §21.1, PBI §8). Brain `episode_card` artifacts may outlive raw-transcript retention if policy allows.
- Deletion is explicit and scoped (EM §21.3); deleting audit events warns about provenance loss (EM §21.3). `[MVP-SIMP]` MVP: keep everything except terminal scrollback refs; rich retention policies are P1 (EM §25).

### 7.4 SQLite + optional JSONL-mirror fallback `[OPEN — EM §27 / PROPOSED resolution]`
EM §27 asks: SQLite-only, or SQLite + append-only JSONL mirror? `[PROPOSED]` SQLite is the **canonical** store; ship an **opt-in append-only `events.jsonl` mirror** driven off the `outbox` (`destination='jsonl_mirror'`, §2.5) for: (a) debug/export (EM §21.2 ships JSONL export anyway), (b) belt-and-suspenders durability if the DB is corrupted. The JSONL mirror is **never read on the hot path** and is never a second writer — it's a derived, append-only export. Defer making it authoritative `[DEFERRED]`.

---

## 8. Open data questions → `docs/planning/OPEN_QUESTIONS.md`

These are unresolved data-modeling decisions for the adversarial finalize pass; record/track in the sibling `docs/planning/OPEN_QUESTIONS.md` (this planning chain). All `[OPEN]`, cross-referenced to SOM §37.

1. **Task vs PlanTask subtype** (SOM §37 Q2): **[RESOLVED — R-8, §5.1]** one `tasks` table, `kind ∈ {plan_task, external_task}`, **superset** state machine (ADR-012); Plan View renders the plan-task subset, external tasks render the GitHub/Linear subset. _(Draft used `plan_tasks` (§2.8) + a shared §4.2 machine — superseded.)_
2. **Worktree ↔ Repo cardinality / multi-repo** (SOM §37 Q1): `proj_worktree.repo_id` is currently 1:1 → one repo. Multi-repo projects / virtual worktrees would need a join table (`worktree_repos`). Mark schema extension point.
3. **Branch co-ownership** (SOM §37 Q3): `leases.lease_kind='shared'` is reserved (§2.6) but the semantics for two sessions on one branch are undefined. Resolve before any multi-agent-on-one-branch flow.
4. **AgentTeam PR reconciliation** (SOM §37 Q4): one PR vs many PRs per team — affects whether `proj_worktree`/`linked_pr_id` is 1:1 with a team or fans out. Schema currently assumes per-worktree PR.
5. **Actor enum unification** (this artifact, §6.2): **[RESOLVED — R-2, §7.1; frozen in `shared/src/actor.rs`]** canonical audit actor = **`remote_client`** (EM §7 `remote_device` swept forward; request-time `requester_type` aliases map per Appendix A). The 10-value audit enum is frozen.
6. **Hash-chain activation** (EM §27): `payload_hash`/`previous_event_hash` reserved (§2.1) — decide if/when to turn on tamper-evidence (post-MVP per ADR-003).
7. **ID format** (this artifact, §5): **[RESOLVED — R-1, §5.2; frozen 0.5]** prefixed-ULID adopted + frozen in `shared/src/ids.rs` (16 minted prefixes + 6 external natives); UUIDv7 not taken.
8. Plus the harness-state-capture reliability and EpisodeCard generation-timing questions (SOM §37 Q6/Q7) that bound what the `artifacts`/`proj_session` tables can safely persist.
