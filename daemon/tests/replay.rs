//! Phase 1.6c L1 — degradable catch-up replay (§17, USER-RULED Option C). RED first.
//!
//! The startup replay path (`catch_up_replay` inside `EventStore::open`) must **degrade
//! instead of crash** on a corrupt / unknown-version / unredacted event row: a corrupt or
//! unredacted row is QUARANTINED (preserved + recorded, the offset marked degraded, replay
//! continues, `open` returns Ok); an unknown-`event_version` row is DEGRADED (offset marked
//! degraded, continue — a newer binary folds it; not an integrity violation). The raw `events`
//! spine is NEVER mutated (§17/§7.2). L2 adds the loud audit-integrity event for quarantines.

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType, Visibility};
use nexusops_shared::events::{AuditIntegrityViolation, SessionStarted};
use nexusops_shared::ids::{EventId, ProjectId, SessionId, WorkspaceId};
use nexusops_shared::status::Session;
use nexusopsd::bootstrap::{cold_start, BootstrapConfig, DaemonContext, DB_FILENAME};
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{
    AppendIntent, DegradableEvent, EventStore, EventStoreError, PrefixRedactor,
};
use nexusopsd::idgen::UlidGen;
use rusqlite::params;
use std::path::{Path, PathBuf};

fn db_path() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("nexusops.db");
    (tmp, p)
}

fn open(p: &Path) -> Result<EventStore, EventStoreError> {
    EventStore::open(
        p,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-10T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
}

/// A fresh app-support base dir under a tempdir (cold_start creates `<base>/nexusops.db`).
fn bootstrap_base() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("Application Support").join("NexusOps");
    (tmp, base)
}

/// Cold-start the daemon over `base` (§14 injected seams; the L2 audit-integrity emission runs
/// from this caller after open, per Q2).
fn cold(base: &Path) -> Result<DaemonContext, nexusopsd::bootstrap::BootstrapError> {
    cold_start(BootstrapConfig {
        base_dir: base.to_path_buf(),
        idgen: Box::new(UlidGen),
        clock: Box::new(FixedClock::new("2026-06-10T00:00:00Z")),
        redactor: Box::new(PrefixRedactor),
    })
}

/// Count `AuditIntegrityViolation` events in the log (read degradably — a corrupt row in the
/// log would abort the strict `read_all`).
fn audit_integrity_event_count(store: &EventStore) -> usize {
    store
        .read_all_degradable()
        .unwrap()
        .into_iter()
        .filter(|d| {
            matches!(d, DegradableEvent::Ok(e) if e.event_type == AuditIntegrityViolation::EVENT_TYPE)
        })
        .count()
}

/// Raw-insert a row at `seq` (simulating an event row past the projection offset that replay
/// must fold). `actor_type` / `event_version` / `redaction_status` are the knobs a test turns
/// to make the row corrupt (bad actor_type → reconstruction-fail → Quarantined), degraded
/// (version > MAX_SUPPORTED → Degraded), or unredacted (→ Quarantined, §15 replay-side defense).
/// `payload_json` is `{}` (valid JSON — the events CHECK(json_valid) forbids storing non-JSON,
/// so realistic at-replay quarantines come from a bad typed column, not the payload string).
fn raw_insert(p: &Path, seq: i64, actor_type: &str, event_version: i64, redaction_status: &str) {
    let conn = rusqlite::Connection::open(p).unwrap();
    conn.execute(
        "INSERT INTO events (event_id, seq, event_type, event_version, occurred_at, recorded_at, \
         workspace_id, actor_type, actor_id, source_type, source_id, correlation_id, sensitivity, \
         payload_json, schema_version, redaction_status) \
         VALUES (?1,?2,'SessionStarted',?3,'2026-06-10T00:00:00Z','2026-06-10T00:00:00Z', \
         'ws_00000000000000000000000000',?4,'system','local_daemon','system','corr-1','internal', \
         '{}','event-envelope-v1',?5)",
        params![
            EventId::new().as_str(),
            seq,
            event_version,
            actor_type,
            redaction_status
        ],
    )
    .unwrap();
}

/// The next free `seq` after the current max — the slot a "past the projection offset" raw row takes.
/// Computed (not hard-coded) so adding a cold-start registration event (P5.3a's seeded default profile
/// took the count from 2 → 3) does not shift a literal seq out from under these tests.
fn next_seq(p: &Path) -> i64 {
    let conn = rusqlite::Connection::open(p).unwrap();
    conn.query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM events", [], |r| {
        r.get(0)
    })
    .unwrap()
}

/// The recorded quarantine seqs (daemon-internal `quarantine` table — read directly).
fn quarantine_seqs(p: &Path) -> Vec<i64> {
    let conn = rusqlite::Connection::open(p).unwrap();
    let mut stmt = conn
        .prepare("SELECT seq FROM quarantine ORDER BY seq")
        .unwrap();
    let rows = stmt.query_map([], |r| r.get::<_, i64>(0)).unwrap();
    rows.map(|r| r.unwrap()).collect()
}

/// How many projectors are flagged `degraded` (projection_offsets.state).
fn degraded_projector_count(p: &Path) -> i64 {
    let conn = rusqlite::Connection::open(p).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM projection_offsets WHERE state = 'degraded'",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

/// The recorded quarantine `reason` for `seq` (the structural descriptor that becomes the
/// L2 audit-integrity event's reason — must never echo row content, §15).
fn quarantine_reason(p: &Path, seq: i64) -> Option<String> {
    let conn = rusqlite::Connection::open(p).unwrap();
    conn.query_row(
        "SELECT reason FROM quarantine WHERE seq = ?1",
        params![seq],
        |r| r.get(0),
    )
    .ok()
}

/// The quarantine row's `audit_emitted` flag (L2 dedup; re-detection must PRESERVE it).
fn quarantine_audit_emitted(p: &Path, seq: i64) -> i64 {
    let conn = rusqlite::Connection::open(p).unwrap();
    conn.query_row(
        "SELECT audit_emitted FROM quarantine WHERE seq = ?1",
        params![seq],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn test_corrupt_row_open_recovers_not_aborts() {
    // THE load-bearing pin (§17 / Option C): a corrupt event row at open → daemon STARTS
    // (open returns Ok, not abort), the row is quarantine-RECORDED (the gap is not silent),
    // and the raw `events` row is UNTOUCHED (the spine is never corrupted).
    let (_tmp, p) = db_path();
    drop(open(&p).expect("fresh open seeds offsets at 0")); // seed offsets so seq 1 > offset

    // a reconstruction-failing row (bad actor_type enum) — valid JSON payload, redacted, so the
    // quarantine is attributable to corruption, not the §15 unredacted path.
    raw_insert(&p, 1, "definitely_not_an_actor", 1, "redacted");

    let store = open(&p).expect("a corrupt row degrades — open RECOVERS, does NOT abort");

    assert_eq!(
        quarantine_seqs(&p),
        vec![1],
        "the corrupt row is quarantine-recorded at its REAL seq (never a silent drop, never -1)"
    );
    // raw spine untouched: the corrupt row is preserved exactly (forensics), not mutated/deleted.
    let actor: String = store
        .read_all_degradable()
        .unwrap()
        .iter()
        .find_map(|d| match d {
            nexusopsd::eventstore::DegradableEvent::Quarantined { seq, .. } if *seq == 1 => {
                Some("quarantined".to_string())
            }
            _ => None,
        })
        .expect("seq 1 reads back as quarantined (raw row preserved, classified on read)");
    assert_eq!(actor, "quarantined");
}

#[test]
fn test_unknown_event_version_degrades_continues() {
    // §17 "unknown version → store raw + degraded marker, don't crash". A version this binary
    // can't fold is DEGRADED (offset marked degraded, replay continues, open Ok) — NOT a
    // corruption quarantine (a newer binary would fold it; no integrity violation, no quarantine).
    let (_tmp, p) = db_path();
    drop(open(&p).expect("fresh open"));

    raw_insert(&p, 1, "user", 999, "redacted"); // event_version 999 > MAX_SUPPORTED

    open(&p).expect("an unknown-version row degrades — open continues, does NOT abort");

    assert!(
        degraded_projector_count(&p) > 0,
        "an unknown-version row marks the projector(s) degraded (§17 degraded marker)"
    );
    assert!(
        quarantine_seqs(&p).is_empty(),
        "an unknown-version row is DEGRADED, not quarantined — no integrity violation recorded"
    );
}

#[test]
fn test_unredacted_row_quarantine_skipped_on_replay() {
    // §15 replay-side defense-in-depth: a row carrying redaction_status='unredacted' (non-
    // producible via the 1.1 write gate) is QUARANTINE-SKIPPED on replay — never folded into a
    // projection (a projection must never surface an unredacted payload). The reason echoes NO
    // row content (§15).
    let (_tmp, p) = db_path();
    drop(open(&p).expect("fresh open"));

    raw_insert(&p, 1, "user", 1, "unredacted"); // otherwise-valid, but unredacted

    open(&p).expect("an unredacted row is quarantine-skipped — open recovers");

    assert_eq!(
        quarantine_seqs(&p),
        vec![1],
        "the unredacted row is quarantined (skipped, recorded), not folded"
    );
    // not folded into the audit trail (a projection must never surface an unredacted event)
    let conn = rusqlite::Connection::open(&p).unwrap();
    let audit_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM proj_audit_trail", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        audit_rows, 0,
        "the unredacted row was NOT folded into proj_audit_trail (§15)"
    );
}

#[test]
fn test_healthy_log_replays_unchanged() {
    // No regression: a clean log replays through the degradable path exactly like the strict
    // path did — the event folds, NO quarantine record, NO spurious degraded marker.
    let (_tmp, p) = db_path();
    let session_id = SessionId::new();
    {
        let mut store = open(&p).expect("open");
        let payload = serde_json::to_string(&SessionStarted {
            status: Session::Active,
            harness: None,
            model: None,
            display_name: None,
            execution_profile_id: None,
        })
        .unwrap();
        store
            .append(AppendIntent {
                event_type: "SessionStarted".to_string(),
                event_version: 1,
                occurred_at: "2026-06-10T00:00:00Z".to_string(),
                workspace_id: WorkspaceId::new(),
                actor_type: ActorType::User,
                actor_id: "u_1".to_string(),
                source_type: SourceType::DesktopUi,
                source_id: "src_1".to_string(),
                correlation_id: "corr-1".to_string(),
                sensitivity: Sensitivity::Internal,
                payload_json: payload,
                schema_version: "event-envelope-v1".to_string(),
                idempotency_key: None,
                project_id: Some(ProjectId::new()),
                session_id: Some(session_id.clone()),
                agent_team_id: None,
                visibility: Some(Visibility::Project),
                action_request_id: None,
                approval_id: None,
                causation_id: None,
            })
            .expect("append a healthy SessionStarted");
        // rebuild over the degradable path — must reproduce the read model with no quarantine.
        store
            .rebuild_projections()
            .expect("rebuild via degradable path");
    }

    assert!(
        quarantine_seqs(&p).is_empty(),
        "a healthy log produces NO quarantine records"
    );
    assert_eq!(
        degraded_projector_count(&p),
        0,
        "a healthy log degrades no projector"
    );
    let conn = rusqlite::Connection::open(&p).unwrap();
    let session_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM proj_session WHERE session_id = ?1",
            params![session_id.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        session_rows, 1,
        "the healthy SessionStarted folded into proj_session (degradable path == strict path)"
    );
}

#[test]
fn test_corrupt_row_quarantine_reason_is_content_free() {
    // ADD 1 (§15): the quarantine `reason` (which becomes the persisted L2 audit-integrity
    // event's reason) must be a STRUCTURAL descriptor — never echo the offending column bytes
    // (a raw serde error like "unknown variant `<value>`" would leak a column value into the
    // audit trail). The redactor on append is the backstop; reason-is-structural is primary.
    let (_tmp, p) = db_path();
    drop(open(&p).expect("fresh open"));
    let sensitive = "zz_sensitive_actor_value_zz";
    raw_insert(&p, 1, sensitive, 1, "redacted"); // bad actor_type carrying a distinctive value
    open(&p).expect("corrupt row recovers");

    let reason = quarantine_reason(&p, 1).expect("seq 1 quarantined");
    assert!(
        !reason.contains(sensitive),
        "quarantine reason must NOT echo the offending column value (§15): {reason:?}"
    );
}

#[test]
fn test_quarantine_record_idempotent_on_reread() {
    // ADD 2: re-detection — catch_up RE-READS a trailing corrupt row whose offset never advanced
    // past it (a plain `open` appends nothing, so the offset stays before seq 1). The second
    // detection must NOT duplicate the quarantine row NOR reset `audit_emitted` — the insert is
    // ON CONFLICT(seq) DO NOTHING (preserve), never REPLACE (which would re-arm a re-emit).
    let (_tmp, p) = db_path();
    drop(open(&p).expect("fresh open"));
    raw_insert(&p, 1, "definitely_not_an_actor", 1, "redacted"); // trailing corrupt row
    drop(open(&p).expect("open #1 quarantines seq 1"));

    // simulate L2 having emitted the audit-integrity event for seq 1 (audit_emitted := 1)
    {
        let conn = rusqlite::Connection::open(&p).unwrap();
        conn.execute("UPDATE quarantine SET audit_emitted = 1 WHERE seq = 1", [])
            .unwrap();
    }

    drop(
        open(&p)
            .expect("open #2 RE-READS the trailing corrupt row (offset never advanced past it)"),
    );

    assert_eq!(
        quarantine_seqs(&p),
        vec![1],
        "re-detection does NOT duplicate the quarantine row"
    );
    assert_eq!(
        quarantine_audit_emitted(&p, 1),
        1,
        "re-detection PRESERVES audit_emitted (ON CONFLICT DO NOTHING, never REPLACE → no re-emit)"
    );
}

// ===================== L2 — the audit-integrity event (loud record) ==========

#[test]
fn test_quarantine_is_not_silent() {
    // L2 — Option C vs the rejected silent-skip: a quarantined row produces BOTH a quarantine
    // record AND a loud, consumer-visible AuditIntegrityViolation event in proj_audit_trail.
    // Emission is from the caller (cold_start) after open (Q2), via the write-actor append path.
    let (_tmp, base) = bootstrap_base();
    let db = base.join(DB_FILENAME);
    drop(cold(&base).expect("cold_start #1 creates the DB + registers identity (offset advances)"));

    // a corrupt row past the offset → catch_up re-reads it on the next start
    let corrupt_seq = next_seq(&db);
    raw_insert(&db, corrupt_seq, "definitely_not_an_actor", 1, "redacted");
    let ctx = cold(&base).expect("cold_start #2 quarantines the corrupt row + emits the AIV event");

    // recorded (the gap is not silent)
    assert_eq!(
        quarantine_seqs(&db),
        vec![corrupt_seq],
        "the corrupt row is quarantine-recorded"
    );
    assert_eq!(
        quarantine_audit_emitted(&db, corrupt_seq),
        1,
        "the audit-integrity event was emitted (audit_emitted set → deduped next start)"
    );
    // loud: exactly one AuditIntegrityViolation event, consumer-visible in the audit trail
    assert_eq!(
        audit_integrity_event_count(&ctx.store),
        1,
        "exactly one audit-integrity event for the quarantined row"
    );
    let conn = rusqlite::Connection::open(&db).unwrap();
    let audit: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM proj_audit_trail WHERE headline = 'Audit-integrity violation'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        audit, 1,
        "the audit-integrity event is consumer-visible in proj_audit_trail (loud, not silent)"
    );
}

#[test]
fn test_quarantine_idempotent_across_restart() {
    // ADD 2 — exactly ONE AIV per seq under genuine re-detection: a full rebuild re-reads ALL
    // rows (offsets reset) and re-detects the corrupt seq, but ON CONFLICT(seq) DO NOTHING
    // preserves audit_emitted, so a subsequent emit is a no-op (no duplicate audit-integrity event).
    let (_tmp, base) = bootstrap_base();
    let db = base.join(DB_FILENAME);
    drop(cold(&base).expect("cold_start #1"));
    let corrupt_seq = next_seq(&db);
    raw_insert(&db, corrupt_seq, "definitely_not_an_actor", 1, "redacted");
    let mut ctx = cold(&base).expect("cold_start #2: quarantine + emit AIV");
    assert_eq!(
        audit_integrity_event_count(&ctx.store),
        1,
        "one AIV after first emit"
    );

    // genuine re-detection: rebuild re-reads the corrupt seq → record_quarantine ON CONFLICT DO NOTHING.
    ctx.store
        .rebuild_projections()
        .expect("rebuild re-detects the corrupt row (offsets reset → re-read all)");
    // emit again — audit_emitted preserved → no second event.
    ctx.store
        .emit_quarantine_audit_events()
        .expect("emit is a no-op for already-emitted seqs");

    assert_eq!(
        audit_integrity_event_count(&ctx.store),
        1,
        "exactly ONE audit-integrity event across re-detection (rebuild + re-emit) — idempotent"
    );
}
