//! P4.0b-2 C1b (CAT-1, call-2) — the independent durable **integrity alarm** (the user-ruled
//! best-practice audit-fault surface). When the single audited mutator cannot AUDIT (an audit-event
//! write faults), the §17/§15 #5 integrity signal must travel a channel that does NOT depend on the
//! broken audit-write path — an append-only, fsync'd incident sink. The fail-closed Deny is the
//! load-bearing control; this alarm makes the integrity fault loud + durable + UI-markable.
//!
//! Binding-safe (no live agent, no DB): this is the SINK; C2 WIRES it into route_intercept's call-2
//! path (fire on an audit-write fault). The systemic-failure circuit-breaker/fail-stop is a separate
//! daemon-wide slice (part 3).

use nexusopsd::integrity::{
    FileIntegrityAlarm, IntegrityAlarm, IntegrityIncident, IntegrityKind, RecordingIntegrityAlarm,
};

// ---- C1b #1 — the production sink appends each incident DURABLY (append-only, independent of DB) --

#[test]
fn test_file_alarm_appends_each_incident_durably() {
    // spec(§17/§15 #5) — the durable independent sink APPENDS each incident as one JSON line (never
    // overwrites — an earlier incident must survive a later one) to its own file, built ONLY from the
    // structural action_type (content-free by construction — no payload/secret, §15). No DB handle.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("integrity-incidents.jsonl");
    let alarm = FileIntegrityAlarm::new(path.clone());

    alarm.raise(IntegrityIncident::audit_write_failed("agent.bash"));
    alarm.raise(IntegrityIncident::audit_write_failed("agent.file_edit"));

    // re-read from a FRESH open AFTER dropping the alarm — proves the fsync'd records are durable on
    // disk (an in-process read could pass even if `sync_all` were removed; this re-open cannot).
    drop(alarm);
    let contents = std::fs::read_to_string(&path).expect("the incident file exists + is readable");
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "each incident is APPENDED (the first survives the second)"
    );
    // each line is a structured record carrying the kind + the structural (content-free) detail.
    for (line, needle) in lines.iter().zip(["agent.bash", "agent.file_edit"]) {
        let v: serde_json::Value =
            serde_json::from_str(line).expect("each line is one JSON object");
        assert_eq!(
            v["kind"], "audit_write_failed",
            "the structured incident kind"
        );
        assert!(
            v["detail"].as_str().unwrap().contains(needle),
            "the structural detail records the action_type (content-free — no payload/secret)"
        );
    }

    // the incident file is 0600 (owner-only) — daemon-private safety state (§15 / rule #11 ethos).
    use std::os::unix::fs::PermissionsExt as _;
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "the incident log is owner-only (not group/world-readable)"
    );
}

// ---- C1b #2 — the trait is object-safe + a recording double collects (the C2 wiring seam) ---------

#[test]
fn test_integrity_alarm_object_safe_recording_double() {
    // spec(§17) — the alarm is an injectable, object-safe seam (the TelemetryEventSink/
    // TerminalEventSink precedent) so the production binds the file sink + C2's wiring tests assert
    // route_intercept fired the alarm via a recording double.
    let recorder = RecordingIntegrityAlarm::new();
    let alarm: &dyn IntegrityAlarm = &recorder;
    alarm.raise(IntegrityIncident::audit_write_failed("agent.bash"));

    let seen = recorder.incidents();
    assert_eq!(
        seen.len(),
        1,
        "the recording double collects the raised incident"
    );
    assert_eq!(seen[0].kind, IntegrityKind::AuditWriteFailed);
}
