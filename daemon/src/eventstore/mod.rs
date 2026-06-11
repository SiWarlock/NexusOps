//! Single-writer WAL event log (§7/§7.1/§15/§16/§17).
//!
//! [`EventStore`] owns the one write connection (the single audited writer,
//! forbidden-pattern #3 / §15); all other access goes through a read-only WAL
//! connection ([`open_read_only`]). Append assigns the canonical monotonic `seq`,
//! the `event_id` (via the injected [`IdGen`]), and `recorded_at` (via the
//! injected [`Clock`]) — so a fixed input replays a byte-identical log (§14).
//! The redaction GATE lands in the L3 sub-feature; this layer is the spine.

mod migrations;
mod outbox;
pub mod redaction;
mod schema;

use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use serde::de::DeserializeOwned;
use serde::Serialize;

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{
    EventEnvelope, RedactionStatus, Sensitivity, SourceType, Visibility,
};
use nexusops_shared::events::{AuditIntegrityViolation, SensitiveOutputRedacted};

use nexusops_shared::ids::{
    ActionRequestId, AgentTeamId, EventId, ProjectId, SessionId, WorkspaceId,
};
pub use redaction::{PrefixRedactor, QuarantineSignal, RedactionOutcome, Redactor};
// migration DDL constants (re-exported so the projection-migration test + the
// migrations registry reference one canonical source).
pub use schema::{MIGRATION_1_EVENTS, MIGRATION_2_REDACTION, MIGRATION_3_PROJECTIONS};
// outbox delivery surface (the drainer + its destinations, §12/§17).
pub use outbox::{DeliveryOutcome, Destination, DrainSummary, JsonlMirror, DRAIN_BATCH_LIMIT};

use crate::clock::Clock;
use crate::idgen::IdGen;
use crate::locks::{self, FencingToken, Lease, LeaseError, LeaseKind, OwnerId, ResourceId};

pub use migrations::SUPPORTED_USER_VERSION;

/// the highest `event_version` this binary can fully reconstruct (§17). Shared
/// with `projections` (the degraded-skip path treats a newer version as unfoldable).
pub(crate) const MAX_SUPPORTED_EVENT_VERSION: i64 = 1;

/// Explicit "unknown seq" marker for a quarantined row whose `seq` column itself is unreadable
/// (1.6a-L1 cq cleanup — replaces the implicit `-1` sentinel). Used ONLY for the genuinely-
/// unreadable-seq case; a readable seq is always preserved verbatim in the quarantine record,
/// so this never flows into version/seq logic as a silent sentinel.
pub(crate) const UNKNOWN_SEQ: i64 = -1;

/// Typed event-store failures — fail-closed (§15/§17): a write that cannot
/// durably + validly persist returns an error, never a silent success.
#[derive(Debug, thiserror::Error)]
pub enum EventStoreError {
    #[error("event-store write failed: {0}")]
    Write(rusqlite::Error),
    #[error("event-store open failed: {0}")]
    Open(rusqlite::Error),
    #[error("duplicate idempotency_key")]
    DuplicateIdempotencyKey,
    #[error("refused to persist a non-redacted event (§15 redaction-before-persist)")]
    RedactionRequired,
    #[error("io error: {0}")]
    Io(std::io::Error),
    #[error("migration failed: {0}")]
    Migration(String),
    /// A migration failed AND the pre-migration `.bak` could not be restored (§16). Distinct
    /// from `Migration` (a clean rollback) so the caller never confuses "rolled back safely"
    /// with "rollback ALSO failed" — and so the restore failure is never silently swallowed
    /// (1.1 L3). Carries `from` to render the §16 "update failed, rolled back to vN" UX.
    #[error(
        "migration failed ({migration_error}) and the rollback to v{from} could not be restored: {source}"
    )]
    RestoreFailed {
        from: i64,
        /// the original migration failure — preserved so a `RestoreFailed` still tells the
        /// operator WHAT failed, not only that the rollback also failed.
        migration_error: String,
        source: Box<EventStoreError>,
    },
    #[error("db user_version {db} is newer than supported {supported}")]
    DbNewerThanSupported { db: i64, supported: i64 },
    #[error("event reconstruction failed: {0}")]
    Reconstruct(String),
}

/// What a caller provides to append an event; the store assigns `event_id`,
/// `seq`, and `recorded_at`.
pub struct AppendIntent {
    pub event_type: String,
    pub event_version: u32,
    pub occurred_at: String,
    pub workspace_id: WorkspaceId,
    pub actor_type: ActorType,
    pub actor_id: String,
    pub source_type: SourceType,
    pub source_id: String,
    pub correlation_id: String,
    pub sensitivity: Sensitivity,
    pub payload_json: String,
    pub schema_version: String,
    pub idempotency_key: Option<String>,
    // --- 1.2 identity/edge fields (the 1.1-carry-forward AppendIntent extension) ---
    // Typed `Option`/newtypes (Q4), not bare strings — these are events-table columns
    // AND the source the projectors DERIVE `object_refs` + read-models from (§2.2).
    pub project_id: Option<ProjectId>,
    pub session_id: Option<SessionId>,
    pub agent_team_id: Option<AgentTeamId>,
    /// §7.1 visibility; `None` persists the DDL default `project`.
    pub visibility: Option<Visibility>,
    // --- 2.1b gateway FK columns (the events table has held these since MIGRATION_1; the Action
    // Gateway is the first emitter to populate them so its `ActionExecution*` family is correlatable
    // to the action/approval, §6.2/§7.1). Typed `Option`/newtypes; `None` persists NULL. ---
    /// the action this event belongs to (the Gateway sets `correlation_id == action_request_id` too)
    pub action_request_id: Option<ActionRequestId>,
    /// the approval this event belongs to (`appr_` id; String — the envelope column is untyped)
    pub approval_id: Option<String>,
    /// the immediate prior event in the causal chain (§7.1; typed as the frozen `EventId`)
    pub causation_id: Option<EventId>,
}

/// A read result that degrades instead of crashing on a malformed row (§17).
pub enum DegradableEvent {
    Ok(Box<EventEnvelope>),
    /// `event_version` newer than this binary understands
    Degraded {
        seq: i64,
        reason: String,
    },
    /// payload / row could not be reconstructed (corrupt) — diverted, not crashed
    Quarantined {
        seq: i64,
        reason: String,
    },
}

impl DegradableEvent {
    pub fn is_degraded(&self) -> bool {
        matches!(self, Self::Degraded { .. })
    }
    pub fn is_quarantined(&self) -> bool {
        matches!(self, Self::Quarantined { .. })
    }
}

/// The single-writer WAL event store.
pub struct EventStore {
    conn: Connection,
    idgen: Box<dyn IdGen>,
    clock: Box<dyn Clock>,
    redactor: Box<dyn Redactor>,
}

impl EventStore {
    /// Open (creating if needed) with WAL pragmas + forward-only migrations.
    /// Refuses a db newer than this binary understands (§16). The `redactor`
    /// gates every payload before persist (§15).
    pub fn open(
        path: &Path,
        idgen: Box<dyn IdGen>,
        clock: Box<dyn Clock>,
        redactor: Box<dyn Redactor>,
    ) -> Result<Self, EventStoreError> {
        let mut conn = Connection::open(path).map_err(EventStoreError::Write)?;
        schema::apply_pragmas(&conn).map_err(EventStoreError::Write)?;
        migrations::refuses_db_newer_than_supported(&conn)?;
        migrations::run(path, &mut conn)?;
        // §7.2 startup catch-up: fold any events the projections lag behind (a db with
        // pre-existing events, or a crash mid-write) so the read models are eventually
        // consistent with the log on every start. A fresh/current db is a no-op.
        crate::projections::catch_up_replay(&mut conn).map_err(projection_to_store_err)?;
        // §17 outbox crash recovery: a row claimed `in_flight` but not confirmed
        // before a crash is reset to `pending` → re-deliverable (at-least-once).
        outbox::reset_in_flight(&conn)?;
        Ok(Self {
            conn,
            idgen,
            clock,
            redactor,
        })
    }

    /// Drain one outbox pass for `dest` (§12/§17): deliver its due rows, classify
    /// retryable/terminal, back off, dead-letter after the bounded budget. The unit
    /// the daemon's drainer Tokio task calls on an interval — the **spawn** is wired
    /// by the 1.6 bootstrap (where the Tokio runtime lives); 1.3 ships the unit only.
    pub fn drain_once(
        &mut self,
        clock: &dyn Clock,
        dest: &dyn Destination,
    ) -> Result<DrainSummary, EventStoreError> {
        outbox::drain_once(&self.conn, clock, dest)
    }

    /// Full projection rebuild (§7.2): truncate every derived table + reset offsets,
    /// then replay the entire log. Raw `events` are never touched. The MVP debug/CLI
    /// rebuild path; a deterministic re-derivation of every read model from the spine.
    pub fn rebuild_projections(&mut self) -> Result<(), EventStoreError> {
        crate::projections::rebuild(&mut self.conn).map_err(projection_to_store_err)
    }

    /// Append one event. Assigns `seq` (canonical monotonic order), `event_id`, `recorded_at`.
    /// Fails closed on a write/constraint failure (§15/§17). A thin wrapper that opens its own
    /// IMMEDIATE txn over [`append_in_txn`] — the gateway pipeline shares that same primitive WITHIN
    /// its own txn so a `{durable-row write + authoritative event}` commits atomically (2.1b).
    pub fn append(&mut self, i: AppendIntent) -> Result<EventId, EventStoreError> {
        // seq assignment + INSERT in ONE IMMEDIATE transaction so the canonical `seq` (§7.1) is
        // atomic at the DB level. Disjoint field borrows (vs `&self`) so the txn can co-exist with
        // the redactor/idgen/clock refs `append_in_txn` needs.
        let Self {
            conn,
            idgen,
            clock,
            redactor,
        } = self;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(EventStoreError::Write)?;
        let id = append_in_txn(&tx, &i, redactor.as_ref(), idgen.as_ref(), clock.as_ref())?;
        tx.commit().map_err(EventStoreError::Write)?;
        Ok(id)
    }

    /// Run a Gateway transaction (2.1b): open ONE IMMEDIATE txn, hand the closure a [`GatewayTxn`]
    /// (its registry-row writes via `.tx()`, its authoritative events via `.append()` — the SAME
    /// §15 gate + seq + in-band projection fold + outbox as [`EventStore::append`]), then commit on
    /// `Ok` / roll back on `Err` (the tx drops). This makes `{action_requests/approvals row +
    /// ActionExecution* event}` atomic per transition — fail-closed (INV-SEC-1, §15/§17). The conn
    /// stays private; only the Action Gateway (the sole mutator, forbidden #2) drives this.
    pub fn gateway_txn<T, E>(&mut self, f: impl FnOnce(&GatewayTxn) -> Result<T, E>) -> Result<T, E>
    where
        E: From<EventStoreError>,
    {
        let Self {
            conn,
            idgen,
            clock,
            redactor,
        } = self;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| E::from(EventStoreError::Write(e)))?;
        let gtx = GatewayTxn {
            tx: &tx,
            redactor: redactor.as_ref(),
            idgen: idgen.as_ref(),
            clock: clock.as_ref(),
        };
        let result = f(&gtx)?;
        tx.commit()
            .map_err(|e| E::from(EventStoreError::Write(e)))?;
        Ok(result)
    }

    /// All events in canonical `seq` order. Strict — a malformed row is an error
    /// (use [`EventStore::read_all_degradable`] for resilient reads).
    pub fn read_all(&self) -> Result<Vec<EventEnvelope>, EventStoreError> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {COLS} FROM events ORDER BY seq"))
            .map_err(EventStoreError::Write)?;
        let rows = stmt
            .query_map([], |row| Ok(row_to_envelope(row)))
            .map_err(EventStoreError::Write)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(EventStoreError::Write)??);
        }
        Ok(out)
    }

    /// All events, degrading (unknown version) or quarantining (corrupt / unredacted) bad rows
    /// instead of crashing (§17). The per-row classification is shared with the replay-scoped
    /// [`read_events_after_degradable`] via [`classify_degradable_row`].
    pub fn read_all_degradable(&self) -> Result<Vec<DegradableEvent>, EventStoreError> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT seq, event_version, payload_json, {COLS} FROM events ORDER BY seq"
            ))
            .map_err(EventStoreError::Write)?;
        let rows = stmt
            .query_map([], classify_degradable_row)
            .map_err(EventStoreError::Write)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(EventStoreError::Write)?);
        }
        Ok(out)
    }

    /// The earliest event of `event_type` in canonical `seq` order, if any. The
    /// register-if-absent read (§5.3 — e.g. bootstrap reuses an existing `DeviceRegistered`'s
    /// id rather than minting a second host). Strict — a malformed row is an error.
    pub fn first_event_of_type(
        &self,
        event_type: &str,
    ) -> Result<Option<EventEnvelope>, EventStoreError> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {COLS} FROM events WHERE event_type = ?1 ORDER BY seq LIMIT 1"
            ))
            .map_err(EventStoreError::Write)?;
        let mut rows = stmt
            .query_map([event_type], |row| Ok(row_to_envelope(row)))
            .map_err(EventStoreError::Write)?;
        match rows.next() {
            None => Ok(None),
            Some(r) => Ok(Some(r.map_err(EventStoreError::Write)??)),
        }
    }

    /// Emit a loud [`AuditIntegrityViolation`] event for each quarantined row not yet recorded in
    /// the event stream (§17 Option C — the consumer-visible gap record). Called by the CALLER
    /// after [`EventStore::open`] (Q2 — replay itself stays append-free); the production caller is
    /// `bootstrap::cold_start`. **Idempotent across restarts AND re-detection:** only
    /// `audit_emitted=0` quarantine rows are emitted, each append carries an `audit-integrity-{seq}`
    /// idempotency_key (so a crash between append + mark, or a re-detected seq, can never
    /// double-emit), and `audit_emitted` is set after. The event flows through the normal `append`
    /// path (the §15 redaction gate + projector fold both run). Returns the count processed.
    pub fn emit_quarantine_audit_events(&mut self) -> Result<usize, EventStoreError> {
        let pending: Vec<(i64, String)> = {
            let mut stmt = self
                .conn
                .prepare("SELECT seq, reason FROM quarantine WHERE audit_emitted = 0 ORDER BY seq")
                .map_err(EventStoreError::Write)?;
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                .map_err(EventStoreError::Write)?;
            let mut v = Vec::new();
            for r in rows {
                v.push(r.map_err(EventStoreError::Write)?);
            }
            v
        };

        // counts rows PROCESSED — a freshly-appended event AND an already-emitted no-op (a
        // re-detected seq whose idempotency_key already exists) both count; the caller treats
        // "processed" as "the integrity record is now in the stream," not "newly written."
        let mut processed = 0usize;
        for (seq, reason) in pending {
            // the reason is the structural quarantine descriptor (content-free, §15); the seq is
            // the affected canonical order key. NOT the corrupt payload.
            let payload = serde_json::to_string(&AuditIntegrityViolation { seq, reason })
                .map_err(|e| EventStoreError::Reconstruct(e.to_string()))?;
            let intent = AppendIntent {
                event_type: AuditIntegrityViolation::EVENT_TYPE.to_string(),
                event_version: 1,
                occurred_at: self.clock.now_rfc3339(),
                workspace_id: WorkspaceId::system(),
                actor_type: ActorType::System,
                actor_id: "system".to_string(),
                source_type: SourceType::LocalDaemon,
                source_id: "system".to_string(),
                // correlates to the quarantined row (its own integrity record); a distinct value
                // from the dedup key below — correlation groups events, it is not a dedup token.
                correlation_id: format!("quarantine-{seq}"),
                sensitivity: Sensitivity::Internal,
                payload_json: payload,
                schema_version: "event-envelope-v1".to_string(),
                // exactly-once dedup token (the events UNIQUE index enforces it across restarts).
                idempotency_key: Some(format!("audit-integrity-{seq}")),
                project_id: None,
                session_id: None,
                agent_team_id: None,
                visibility: Some(Visibility::System),
                // the §17 integrity record is a System event, not a gateway action — no FK edges.
                action_request_id: None,
                approval_id: None,
                causation_id: None,
            };
            // crash-safe dedup: a duplicate (re-detected) emission hits the UNIQUE idempotency_key
            // → DuplicateIdempotencyKey, which we treat as already-emitted (still mark the flag).
            match self.append(intent) {
                Ok(_) | Err(EventStoreError::DuplicateIdempotencyKey) => {
                    self.conn
                        .execute(
                            "UPDATE quarantine SET audit_emitted = 1 WHERE seq = ?1",
                            rusqlite::params![seq],
                        )
                        .map_err(EventStoreError::Write)?;
                    processed += 1;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(processed)
    }

    // ---- lease locks + fencing (§7.2/§17, safety rule #6) -------------------
    //
    // `locks/` is a persistence-core sibling; `EventStore` (the single connection
    // owner) drives it over `self.conn` — exactly the `drain_once`/`rebuild_projections`
    // precedent, so no second writable connection is opened (Forbidden #3 / LESSON §3).
    // The production caller is the **Phase-2 Action Gateway** (acquire → re-read after
    // lock → validate the token at execute → `fencing_conflict` on a stale token); 1.4
    // ships the primitive, the gateway wires it in Phase 2.

    /// Acquire `(resource_id, lease_kind)` for `owner_id` with a `ttl_secs` lease — minting a
    /// strictly-monotonic fencing token. Refuses (typed `Held`) a live lease held by another.
    pub fn acquire_lease(
        &mut self,
        resource_id: &ResourceId,
        lease_kind: &LeaseKind,
        owner_id: &OwnerId,
        ttl_secs: i64,
        clock: &dyn Clock,
    ) -> Result<Lease, LeaseError> {
        locks::acquire(
            &mut self.conn,
            resource_id,
            lease_kind,
            owner_id,
            ttl_secs,
            clock,
        )
    }

    /// Renew an owned lease (extend the TTL; token unchanged). Owner-and-token-and-expiry guarded.
    pub fn renew_lease(
        &self,
        lease: &Lease,
        ttl_secs: i64,
        clock: &dyn Clock,
    ) -> Result<Lease, LeaseError> {
        locks::renew(&self.conn, lease, ttl_secs, clock)
    }

    /// Release an owned lease — free the slot, preserve the fencing high-water mark.
    pub fn release_lease(&self, lease: &Lease) -> Result<(), LeaseError> {
        locks::release(&self.conn, lease)
    }

    /// The authority oracle (safety rule #6, Option B): `true` iff `owner_id` holds the lease
    /// with `token` AND it is still live. The Phase-2 gateway's stale-token check (§17).
    pub fn validate_lease_held(
        &self,
        resource_id: &ResourceId,
        lease_kind: &LeaseKind,
        owner_id: &OwnerId,
        token: FencingToken,
        clock: &dyn Clock,
    ) -> Result<bool, LeaseError> {
        locks::validate_held(
            &self.conn,
            resource_id,
            lease_kind,
            owner_id,
            token,
            &clock.now_rfc3339(),
        )
    }

    /// Reap expired leases (§12): free every lease past `expires_at` (holder fields NULL,
    /// token kept) + return the reclaimed set. The deterministic unit; the Tokio interval
    /// spawn is the 1.6 bootstrap (joins the outbox drainer spawn).
    pub fn reap_leases(
        &mut self,
        clock: &dyn Clock,
    ) -> Result<Vec<(ResourceId, LeaseKind)>, LeaseError> {
        locks::reap_once(&mut self.conn, clock)
    }

    /// The applied schema version (§16). Typed `Result<u32, _>` — a real read error is
    /// `Err`, never a silent `-1` sentinel the version-compat code might branch on (1.1 L2).
    pub fn user_version(&self) -> Result<u32, EventStoreError> {
        let v = migrations::current_user_version(&self.conn)?;
        // a user_version outside u32 is a corrupt/hostile schema-version pragma — a
        // migration-domain integrity error, NOT an event-reconstruction failure.
        u32::try_from(v).map_err(|e| {
            EventStoreError::Migration(format!("user_version {v} is not a valid u32: {e}"))
        })
    }

    /// Read an integer PRAGMA from the WRITER's connection (the per-connection
    /// pragmas — synchronous/foreign_keys/busy_timeout — live here, ADR-003).
    pub fn pragma_i64(&self, name: &str) -> rusqlite::Result<i64> {
        self.conn.pragma_query_value(None, name, |r| r.get(0))
    }

    /// Read a text PRAGMA from the writer's connection (e.g. `journal_mode`).
    pub fn pragma_string(&self, name: &str) -> rusqlite::Result<String> {
        self.conn.pragma_query_value(None, name, |r| r.get(0))
    }

    /// §16 version floor — refuse a db newer than this binary understands.
    pub fn refuses_db_newer_than_supported(path: &Path) -> Result<(), EventStoreError> {
        let conn = open_read_only(path)?;
        migrations::refuses_db_newer_than_supported(&conn)
    }

    /// expose the backup/restore helpers for tests + the bootstrap path (§16)
    pub fn backup_db(path: &Path, from: i64) -> Result<(), EventStoreError> {
        migrations::backup_db(path, from)
    }
    pub fn restore_db(path: &Path, from: i64) -> Result<(), EventStoreError> {
        migrations::restore_db(path, from)
    }
}

/// Open a read-only WAL connection (every non-writer path, §15 single-writer).
pub fn open_read_only(path: &Path) -> Result<Connection, EventStoreError> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(EventStoreError::Open)?;
    conn.execute_batch("PRAGMA query_only=ON; PRAGMA busy_timeout=5000;")
        .map_err(EventStoreError::Write)?;
    Ok(conn)
}

// ---- row mapping ------------------------------------------------------------

const COLS: &str = "event_id, seq, event_type, event_version, occurred_at, recorded_at, \
workspace_id, project_id, actor_type, actor_id, source_type, source_id, correlation_id, \
causation_id, action_request_id, approval_id, session_id, agent_team_id, workflow_run_id, \
idempotency_key, sensitivity, visibility, payload_hash, previous_event_hash, schema_version, \
payload_json, redaction_status, redaction_engine_version";

fn row_to_envelope(row: &rusqlite::Row) -> Result<EventEnvelope, EventStoreError> {
    row_to_envelope_offset(row, 0)
}

/// A Gateway-scoped handle to one open write transaction (2.1b). Bundles the txn + the redaction/
/// id/clock injectables so the Action Gateway composes `{durable-row write + authoritative event}`
/// in ONE atomic txn. `.append()` is the SAME §15-gated append path as [`EventStore::append`];
/// `.tx()` is for the gateway's OWN registry tables (`action_requests`/`approvals`) — events go
/// ONLY through `.append()` (forbidden #2/#3: the single audited event-emit path).
pub struct GatewayTxn<'a> {
    tx: &'a rusqlite::Transaction<'a>,
    redactor: &'a dyn Redactor,
    idgen: &'a dyn IdGen,
    clock: &'a dyn Clock,
}

impl GatewayTxn<'_> {
    /// Append an authoritative event through the §15 gate + seq + in-band projections + outbox,
    /// WITHIN this gateway txn (fail-closed: a redaction/write failure aborts the whole txn → the
    /// caller's row writes roll back too).
    pub fn append(&self, i: &AppendIntent) -> Result<EventId, EventStoreError> {
        // §14 fault-injection (2.4) — armed ONLY in the daemon's own test targets (the `fault-injection`
        // feature; compiled out of release). When `TerminalEventWrite` is armed, the next terminal-event
        // append fails with an injected write error, exercising the §17 fail-closed path deterministically.
        #[cfg(feature = "fault-injection")]
        {
            use nexusops_shared::events::{
                ActionFailed, ActionPartiallySucceeded, ActionSucceeded,
            };
            let is_terminal = i.event_type == ActionSucceeded::EVENT_TYPE
                || i.event_type == ActionFailed::EVENT_TYPE
                || i.event_type == ActionPartiallySucceeded::EVENT_TYPE;
            if is_terminal && crate::fault::take_if(crate::fault::FaultPoint::TerminalEventWrite) {
                // a synthetic SQLITE_IOERR — the realistic shape of an audit-write failure (disk I/O),
                // wrapped exactly as a real write error so the fail-closed path is identical.
                return Err(EventStoreError::Write(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some("injected §14 audit-write fault (TerminalEventWrite)".to_string()),
                )));
            }
        }
        append_in_txn(self.tx, i, self.redactor, self.idgen, self.clock)
    }

    /// The open transaction — for the gateway's OWN registry-row writes (`action_requests`/
    /// `approvals`). NEVER write `events` through this directly — use [`GatewayTxn::append`].
    pub fn tx(&self) -> &rusqlite::Transaction<'_> {
        self.tx
    }

    /// `now` as an RFC3339 `Z` string (the injected clock — deterministic in tests).
    pub fn now_rfc3339(&self) -> String {
        self.clock.now_rfc3339()
    }

    /// Run the §15 Redactor over a **registry-row** payload (e.g. `action_requests.inputs_json`)
    /// before it persists at rest. The Gateway's proposers (agents / Brain / UI) are UNTRUSTED, so
    /// a secret slipped into the open `inputs` blob must be masked the same way an event payload is
    /// (rule #3 *same redactor gates persist*; rule #4 *secrets never in rows*). Fail-closed: a
    /// payload the Redactor cannot bring to `redacted` (quarantine / unredactable) is REFUSED — a
    /// row is not an event, so there is no divert path; it never persists unredacted.
    pub fn redact_row(&self, payload: &str) -> Result<String, EventStoreError> {
        let outcome = self.redactor.redact(payload);
        if outcome.status != RedactionStatus::Redacted {
            return Err(EventStoreError::RedactionRequired);
        }
        Ok(outcome.payload_json)
    }
}

/// Append one event into the caller's transaction: the §15 redaction GATE (strictly BEFORE the
/// INSERT) → quarantine-divert / reject-unredacted → `seq` assignment → INSERT → the 1.2 in-band
/// projection fold → the 1.3 outbox write. NO commit (the caller owns the txn boundary). Extracted
/// from `append` (2.1b) so the Gateway shares the EXACT same gate + fold within its own atomic txn
/// — the ordering (gate-before-INSERT, fold + outbox after) is preserved byte-for-byte.
pub(crate) fn append_in_txn(
    tx: &rusqlite::Transaction,
    i: &AppendIntent,
    redactor: &dyn Redactor,
    idgen: &dyn IdGen,
    clock: &dyn Clock,
) -> Result<EventId, EventStoreError> {
    // fail-closed: serialize the enum audit fields up front — an enum that can't render to its wire
    // string is an error, never an empty column.
    let actor = enum_wire(&i.actor_type)?;
    let source = enum_wire(&i.source_type)?;
    let sensitivity = enum_wire(&i.sensitivity)?;
    let visibility = match &i.visibility {
        Some(v) => enum_wire(v)?,
        None => "project".to_string(),
    };

    // §15 redaction-before-persist GATE: route the payload through the Redactor and REFUSE to
    // persist anything not `redacted` (fail-closed) — strictly BEFORE the INSERT below.
    let outcome = redactor.redact(&i.payload_json);
    if let Some(q) = outcome.quarantine {
        // never divert a divert (we can't redact our own audit record) — fail closed.
        if i.event_type == SensitiveOutputRedacted::EVENT_TYPE {
            return Err(EventStoreError::RedactionRequired);
        }
        return divert_in_txn(tx, i, q, redactor, idgen, clock);
    }
    if outcome.status != RedactionStatus::Redacted {
        return Err(EventStoreError::RedactionRequired);
    }
    let redaction_status = enum_wire(&outcome.status)?;

    let event_id = idgen.new_event_id();
    let recorded_at = clock.now_rfc3339();

    let seq: i64 = tx
        .query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM events", [], |r| {
            r.get(0)
        })
        .map_err(EventStoreError::Write)?;
    let res = tx.execute(
        "INSERT INTO events \
         (event_id, seq, event_type, event_version, occurred_at, recorded_at, \
          workspace_id, project_id, actor_type, actor_id, source_type, source_id, \
          correlation_id, causation_id, action_request_id, approval_id, session_id, \
          agent_team_id, idempotency_key, sensitivity, visibility, payload_json, \
          schema_version, redaction_status, redaction_engine_version) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)",
        rusqlite::params![
            event_id.as_str(),
            seq,
            i.event_type,
            i.event_version,
            i.occurred_at,
            recorded_at,
            i.workspace_id.as_str(),
            i.project_id.as_ref().map(|x| x.as_str()),
            actor,
            i.actor_id,
            source,
            i.source_id,
            i.correlation_id,
            i.causation_id.as_ref().map(|x| x.as_str()),
            i.action_request_id.as_ref().map(|x| x.as_str()),
            i.approval_id,
            i.session_id.as_ref().map(|x| x.as_str()),
            i.agent_team_id.as_ref().map(|x| x.as_str()),
            i.idempotency_key,
            sensitivity,
            visibility,
            outcome.payload_json, // the REDACTED payload (§15)
            i.schema_version,
            redaction_status,
            outcome.engine_version,
        ],
    );
    match res {
        Ok(_) => {
            // §7 in-band fan-out: fold the just-appended event into the read models WITHIN this txn,
            // on the already-redacted payload (the §15 gate ran above). Read the row back (vs the
            // in-memory intent) so append + rebuild fold byte-identical envelopes.
            let env = read_envelope_by_seq(tx, seq)?;
            crate::projections::apply_all(tx, &env).map_err(projection_to_store_err)?;
            // §7 transactional-outbox: delivery intents join the SAME txn, after the projections, on
            // the already-redacted event (§15 sync sink). A failure here rolls the append back.
            outbox::write_for_event(tx, &env, idgen, clock)?;
            Ok(event_id)
        }
        // the caller's tx drops → rollback on either error path (fail-closed; nothing persists)
        Err(e) if is_idempotency_violation(&e) => Err(EventStoreError::DuplicateIdempotencyKey),
        Err(e) => Err(EventStoreError::Write(e)),
    }
}

/// §15 quarantine-divert WITHIN the caller's txn: build a content-free `SensitiveOutputRedacted`
/// (preserves routing/identity envelope fields for forensics, drops the payload) and append it via
/// [`append_in_txn`] (whose recursion guard stops a divert-of-a-divert). The original payload never
/// touches `events`.
fn divert_in_txn(
    tx: &rusqlite::Transaction,
    original: &AppendIntent,
    q: QuarantineSignal,
    redactor: &dyn Redactor,
    idgen: &dyn IdGen,
    clock: &dyn Clock,
) -> Result<EventId, EventStoreError> {
    let payload = serde_json::to_string(&SensitiveOutputRedacted {
        original_event_type: original.event_type.clone(),
        reason: q.reason,
        detector: q.detector,
    })
    .map_err(|e| EventStoreError::Reconstruct(e.to_string()))?;
    let sor = AppendIntent {
        event_type: SensitiveOutputRedacted::EVENT_TYPE.to_string(),
        event_version: 1,
        payload_json: payload,
        occurred_at: original.occurred_at.clone(),
        workspace_id: original.workspace_id.clone(),
        actor_type: original.actor_type,
        actor_id: original.actor_id.clone(),
        source_type: original.source_type,
        source_id: original.source_id.clone(),
        correlation_id: original.correlation_id.clone(),
        // content-free record → classify by its OWN sensitivity (Internal), NOT the diverted
        // event's (which may be Restricted/Secret) — inheriting would over-classify a payload-free
        // row (mirrors the 1.6c AuditIntegrityViolation, which is Internal).
        sensitivity: Sensitivity::Internal,
        schema_version: original.schema_version.clone(),
        // namespace the dedup key so a retry dedups to the ONE SOR without colliding with an
        // unrelated event that legitimately holds the original key. `None` stays `None`.
        idempotency_key: original
            .idempotency_key
            .as_ref()
            .map(|k| format!("divert-{k}")),
        project_id: original.project_id.clone(),
        session_id: original.session_id.clone(),
        agent_team_id: original.agent_team_id.clone(),
        visibility: original.visibility,
        // preserve the diverted event's gateway FKs for forensics — only its PAYLOAD is dropped.
        action_request_id: original.action_request_id.clone(),
        approval_id: original.approval_id.clone(),
        causation_id: original.causation_id.clone(),
    };
    append_in_txn(tx, &sor, redactor, idgen, clock)
}

/// Reconstruct the canonical envelope of the row at `seq`, inside `tx` — so the in-band projection
/// fold sees exactly what a later rebuild reconstructs (§7.2 rebuild-equivalence). `object_refs` is
/// empty here (projectors derive it).
fn read_envelope_by_seq(
    tx: &rusqlite::Transaction,
    seq: i64,
) -> Result<EventEnvelope, EventStoreError> {
    match tx.query_row(
        &format!("SELECT {COLS} FROM events WHERE seq = ?1"),
        [seq],
        |row| Ok(row_to_envelope(row)),
    ) {
        Ok(inner) => inner,
        Err(e) => Err(EventStoreError::Write(e)),
    }
}

/// Reconstruct envelopes for events with `seq > after_seq`, in canonical order — the input the
/// projection catch-up replay / rebuild folds (`projections::*`). **Degradable** (§17): a corrupt
/// / unredacted row becomes a `Quarantined` and an unknown-version row a `Degraded` instead of
/// aborting the read, so one bad row never crashes `open` (1.6c superseded the prior strict reader;
/// `object_refs` is empty here — projectors derive it, so a rebuild reproduces it deterministically,
/// §2.2/§7.2). Shares per-row classification with [`EventStore::read_all_degradable`].
pub(crate) fn read_events_after_degradable(
    conn: &Connection,
    after_seq: i64,
) -> Result<Vec<DegradableEvent>, EventStoreError> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT seq, event_version, payload_json, {COLS} FROM events WHERE seq > ?1 ORDER BY seq"
        ))
        .map_err(EventStoreError::Write)?;
    let rows = stmt
        .query_map([after_seq], classify_degradable_row)
        .map_err(EventStoreError::Write)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(EventStoreError::Write)?);
    }
    Ok(out)
}

/// Map a projection failure that escaped a projection call onto the store's error.
/// From `apply_all` only a `Db` (infrastructure) error escapes — projector logic
/// failures degrade in-place; the offset-anomaly + decode arms are fail-closed
/// integrity errors (§15/§17).
fn projection_to_store_err(e: crate::projections::ProjectionError) -> EventStoreError {
    match e {
        crate::projections::ProjectionError::Db(err) => EventStoreError::Write(err),
        crate::projections::ProjectionError::Decode(s) => EventStoreError::Reconstruct(s),
        crate::projections::ProjectionError::OffsetAnomaly { projector, rows } => {
            EventStoreError::Reconstruct(format!(
                "projection offset anomaly: '{projector}' write touched {rows} rows (expected 1)"
            ))
        }
    }
}

/// Classify one row of a `SELECT seq, event_version, payload_json, {COLS} …` degradable read
/// (§17). A bad/unreadable leading column quarantines THAT row (never aborts the whole read);
/// an unknown `event_version` DEGRADES (a newer binary folds it — not an integrity violation);
/// a reconstruction failure or an `unredacted` row QUARANTINES. **§15: a `reason` is a STRUCTURAL
/// descriptor and must NEVER echo row content** — the serde reconstruction error (which would
/// quote the offending value) is deliberately discarded. An unreadable `seq` uses the explicit
/// [`UNKNOWN_SEQ`] marker, never an implicit `-1`.
fn classify_degradable_row(row: &rusqlite::Row) -> rusqlite::Result<DegradableEvent> {
    let seq: i64 = match row.get(0) {
        Ok(s) => s,
        Err(_) => {
            return Ok(DegradableEvent::Quarantined {
                seq: UNKNOWN_SEQ,
                reason: "unreadable seq".to_string(),
            })
        }
    };
    let event_version: i64 = match row.get(1) {
        Ok(v) => v,
        Err(_) => {
            return Ok(DegradableEvent::Quarantined {
                seq,
                reason: "unreadable event_version".to_string(),
            })
        }
    };
    let payload_json: String = match row.get(2) {
        Ok(v) => v,
        Err(_) => {
            return Ok(DegradableEvent::Quarantined {
                seq,
                reason: "unreadable payload_json".to_string(),
            })
        }
    };
    if event_version > MAX_SUPPORTED_EVENT_VERSION {
        // unknown version → DEGRADE (store raw + degraded marker, don't crash; a newer binary
        // folds it). NOT a quarantine — no integrity violation. The version number is not content.
        return Ok(DegradableEvent::Degraded {
            seq,
            reason: format!("unknown event_version {event_version}"),
        });
    }
    if serde_json::from_str::<serde_json::Value>(&payload_json).is_err() {
        return Ok(DegradableEvent::Quarantined {
            seq,
            reason: "corrupt payload_json".to_string(),
        });
    }
    // columns 3.. are the COLS block (offset by the 3 leading cols).
    match row_to_envelope_offset(row, 3) {
        // §15 replay-side defense-in-depth: a row that somehow carries `unredacted` (non-
        // producible via the write gate) is QUARANTINED, never folded — a projection must
        // never surface an unredacted payload. The reason carries no row content.
        Ok(env) if env.redaction_status == RedactionStatus::Unredacted => {
            Ok(DegradableEvent::Quarantined {
                seq,
                reason: "unredacted row (replay-side §15 defense)".to_string(),
            })
        }
        Ok(env) => Ok(DegradableEvent::Ok(Box::new(env))),
        // §15: the reason must NOT echo (possibly sensitive) row content — a structural
        // descriptor only, NOT the serde error (which would quote the offending value).
        Err(_) => Ok(DegradableEvent::Quarantined {
            seq,
            reason: "event reconstruction failed".to_string(),
        }),
    }
}

/// Reconstruct an envelope from the `COLS` block starting at `base`.
fn row_to_envelope_offset(
    row: &rusqlite::Row,
    base: usize,
) -> Result<EventEnvelope, EventStoreError> {
    let g = |i: usize| {
        row.get::<_, String>(base + i)
            .map_err(EventStoreError::Write)
    };
    let go = |i: usize| {
        row.get::<_, Option<String>>(base + i)
            .map_err(EventStoreError::Write)
    };
    let event_version: i64 = row.get(base + 3).map_err(EventStoreError::Write)?;
    let seq: i64 = row.get(base + 1).map_err(EventStoreError::Write)?;

    Ok(EventEnvelope {
        event_id: EventId::parse(&g(0)?)
            .map_err(|e| EventStoreError::Reconstruct(e.to_string()))?,
        seq,
        event_type: g(2)?,
        event_version: u32::try_from(event_version)
            .map_err(|e| EventStoreError::Reconstruct(e.to_string()))?,
        occurred_at: g(4)?,
        recorded_at: g(5)?,
        workspace_id: WorkspaceId::parse(&g(6)?)
            .map_err(|e| EventStoreError::Reconstruct(e.to_string()))?,
        project_id: parse_opt_id(go(7)?, ProjectId::parse)?,
        actor_type: parse_enum(&g(8)?)?,
        actor_id: g(9)?,
        source_type: parse_enum(&g(10)?)?,
        source_id: g(11)?,
        correlation_id: g(12)?,
        causation_id: parse_opt_id(go(13)?, EventId::parse)?,
        action_request_id: parse_opt_id(go(14)?, ActionRequestId::parse)?,
        approval_id: go(15)?,
        session_id: parse_opt_id(go(16)?, SessionId::parse)?,
        agent_team_id: parse_opt_id(go(17)?, AgentTeamId::parse)?,
        workflow_run_id: go(18)?,
        idempotency_key: go(19)?,
        sensitivity: parse_enum::<Sensitivity>(&g(20)?)?,
        visibility: Some(parse_enum::<Visibility>(&g(21)?)?),
        object_refs: Vec::new(), // normalized object_refs table is 1.2
        payload_hash: go(22)?,
        previous_event_hash: go(23)?,
        schema_version: g(24)?,
        payload_json: g(25)?,
        redaction_status: parse_enum::<RedactionStatus>(&g(26)?)?,
        redaction_engine_version: go(27)?,
    })
}

fn parse_opt_id<T, E: std::fmt::Display>(
    v: Option<String>,
    parse: impl Fn(&str) -> Result<T, E>,
) -> Result<Option<T>, EventStoreError> {
    match v {
        None => Ok(None),
        Some(s) => parse(&s)
            .map(Some)
            .map_err(|e| EventStoreError::Reconstruct(e.to_string())),
    }
}

fn parse_enum<T: DeserializeOwned>(s: &str) -> Result<T, EventStoreError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| EventStoreError::Reconstruct(e.to_string()))
}

/// The snake_case wire string for a contract enum — fail-closed (an enum that
/// doesn't serialize to a JSON string is an error, never a silent empty column).
fn enum_wire<T: Serialize>(v: &T) -> Result<String, EventStoreError> {
    match serde_json::to_value(v).map_err(|e| EventStoreError::Reconstruct(e.to_string()))? {
        serde_json::Value::String(s) => Ok(s),
        other => Err(EventStoreError::Reconstruct(format!(
            "enum did not serialize to a string: {other}"
        ))),
    }
}

fn is_idempotency_violation(e: &rusqlite::Error) -> bool {
    if let rusqlite::Error::SqliteFailure(err, Some(msg)) = e {
        // extended code distinguishes UNIQUE from CHECK/NOT-NULL; the message
        // names the offending column (`events.idempotency_key`) — together these
        // pin the idempotency index without depending on a single fragile string.
        return err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
            && msg.contains("idempotency_key");
    }
    false
}

#[cfg(test)]
mod tests {
    // 1.6c test 7 (1.6a-L1 cq cleanup, re-homed): the unreadable-`seq` quarantine marker is a
    // NAMED const, not an implicit `-1` flowing into version/seq logic. A readable seq is always
    // preserved verbatim (pinned end-to-end by `tests/replay.rs::test_corrupt_row…` quarantining
    // at the REAL seq); this const is reserved ONLY for the genuinely-unreadable-`seq` case.
    #[test]
    fn unknown_seq_is_the_explicit_unreadable_marker() {
        assert_eq!(super::UNKNOWN_SEQ, -1);
    }
}
