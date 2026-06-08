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

use nexusops_shared::ids::{
    ActionRequestId, AgentTeamId, EventId, ProjectId, SessionId, WorkspaceId,
};
pub use redaction::{PrefixRedactor, RedactionOutcome, Redactor};
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

    /// Append one event. Assigns `seq` (canonical monotonic order), `event_id`,
    /// `recorded_at`. Fails closed on a write/constraint failure (§15/§17).
    pub fn append(&mut self, i: AppendIntent) -> Result<EventId, EventStoreError> {
        // fail-closed: serialize the enum audit fields up front — an enum that
        // can't render to its wire string is an error, never an empty column.
        let actor = enum_wire(&i.actor_type)?;
        let source = enum_wire(&i.source_type)?;
        let sensitivity = enum_wire(&i.sensitivity)?;
        // `None` → the DDL default `project` (the column is NOT NULL); an explicit
        // value persists verbatim. Fail-closed if a Visibility can't render.
        let visibility = match &i.visibility {
            Some(v) => enum_wire(v)?,
            None => "project".to_string(),
        };

        // §15 redaction-before-persist GATE: route the payload through the Redactor
        // and REFUSE to persist anything not `redacted` (fail-closed).
        let outcome = self.redactor.redact(&i.payload_json);
        if outcome.status != RedactionStatus::Redacted {
            return Err(EventStoreError::RedactionRequired);
        }
        let redaction_status = enum_wire(&outcome.status)?;

        let event_id = self.idgen.new_event_id();
        let recorded_at = self.clock.now_rfc3339();

        // seq assignment + INSERT in ONE IMMEDIATE transaction so the canonical
        // `seq` (§7.1) is atomic at the DB level, not merely by the &mut writer.
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(EventStoreError::Write)?;
        let seq: i64 = tx
            .query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM events", [], |r| {
                r.get(0)
            })
            .map_err(EventStoreError::Write)?;
        let res = tx.execute(
            "INSERT INTO events \
             (event_id, seq, event_type, event_version, occurred_at, recorded_at, \
              workspace_id, project_id, actor_type, actor_id, source_type, source_id, \
              correlation_id, session_id, agent_team_id, idempotency_key, sensitivity, \
              visibility, payload_json, schema_version, redaction_status, redaction_engine_version) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
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
                // §7 in-band fan-out: fold the just-appended event into the read
                // models WITHIN this txn, on the already-redacted payload (the §15
                // gate ran above). A reader never sees an event whose projections
                // haven't applied; a projector failure degrades (never aborts) the
                // raw event (§7.2). Reading the row back (vs reusing the in-memory
                // intent) means append + rebuild fold byte-identical envelopes.
                let env = read_envelope_by_seq(&tx, seq)?;
                crate::projections::apply_all(&tx, &env).map_err(projection_to_store_err)?;
                // §7 transactional-outbox: delivery intents join the SAME txn, after
                // the projections, on the already-redacted event (§15 sync sink). A
                // failure here rolls the whole append back (recorded-iff-intended).
                outbox::write_for_event(&tx, &env, self.idgen.as_ref(), self.clock.as_ref())?;
                tx.commit().map_err(EventStoreError::Write)?;
                Ok(event_id)
            }
            // tx drops → rollback on either error path (fail-closed; nothing persists)
            Err(e) if is_idempotency_violation(&e) => Err(EventStoreError::DuplicateIdempotencyKey),
            Err(e) => Err(EventStoreError::Write(e)),
        }
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

    /// All events, degrading (unknown version) or quarantining (corrupt) bad rows
    /// instead of crashing (§17).
    pub fn read_all_degradable(&self) -> Result<Vec<DegradableEvent>, EventStoreError> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT seq, event_version, payload_json, {COLS} FROM events ORDER BY seq"
            ))
            .map_err(EventStoreError::Write)?;
        let rows = stmt
            .query_map([], |row| {
                // a bad/unreadable leading column quarantines THAT row — it must
                // not abort the whole replay (§17 resilience). An unreadable `seq` is
                // handled like its sibling columns below (quarantine-on-unreadable, not a
                // silent `.unwrap_or(-1)`): -1 is then an EXPLICIT "unknown seq" marker on
                // a quarantined row, never a sentinel that flows into version/event logic.
                let seq: i64 = match row.get(0) {
                    Ok(s) => s,
                    Err(_) => {
                        return Ok(DegradableEvent::Quarantined {
                            seq: -1,
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
                // columns 3.. are the COLS block (offset by the 3 leading cols)
                match row_to_envelope_offset(row, 3) {
                    Ok(env) => Ok(DegradableEvent::Ok(Box::new(env))),
                    // §15: the reason must NOT echo (possibly sensitive) row content.
                    Err(_) => Ok(DegradableEvent::Quarantined {
                        seq,
                        reason: "event reconstruction failed".to_string(),
                    }),
                }
            })
            .map_err(EventStoreError::Write)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(EventStoreError::Write)?);
        }
        Ok(out)
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

/// Reconstruct the canonical envelope of the row at `seq`, inside `tx` — so the
/// in-band projection fold sees exactly what a later rebuild reconstructs (§7.2
/// rebuild-equivalence). `object_refs` is empty here (projectors derive it).
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

/// Reconstruct envelopes for events with `seq > after_seq`, in canonical order — the
/// input the projection catch-up replay / rebuild folds (`projections::*`). Reads on
/// the given connection (typically the replay txn). `object_refs` is empty here;
/// projectors derive it (so a rebuild reproduces it deterministically, §2.2/§7.2).
pub(crate) fn read_events_after(
    conn: &Connection,
    after_seq: i64,
) -> Result<Vec<EventEnvelope>, EventStoreError> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLS} FROM events WHERE seq > ?1 ORDER BY seq"
        ))
        .map_err(EventStoreError::Write)?;
    let rows = stmt
        .query_map([after_seq], |row| Ok(row_to_envelope(row)))
        .map_err(EventStoreError::Write)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(EventStoreError::Write)??);
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
