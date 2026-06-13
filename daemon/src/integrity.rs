//! P4.0b-2 C1b/C2 (CAT-1, call-2; §17/§15 #5) — the independent durable **integrity alarm**.
//!
//! **The user-ruled best-practice audit-fault surface.** When the single audited mutator cannot
//! AUDIT — an audit-event write FAULTS — the §17 integrity signal must NOT travel the broken
//! audit-write path. This sink is that independent durable channel: an append-only, fsync'd incident
//! record, written WITHOUT the DB. The fail-closed Deny (the agent's tool is refused) is the
//! load-bearing control; this alarm makes the integrity fault **loud + durable + UI-markable** even
//! when the DB write path is down.
//!
//! C1b lands the SINK (binding-safe — no live agent, no DB handle); C2 WIRES it into
//! `route_intercept`'s call-2 path (fire on an audit-write fault, paired with the structured
//! `GatewayDenyKind::AuditWriteFailed` deny). The **systemic-failure circuit-breaker / fail-stop**
//! (N-consecutive / unrecoverable audit-backbone failure → the daemon halts) is a SEPARATE daemon-wide
//! slice (part 3) — this sink is the per-incident durable record it (and the per-action deny) build on.

use std::path::PathBuf;

use serde::Serialize;

/// The class of integrity incident (§17). Extensible; the call-2 case is the audit-write fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityKind {
    /// §15 #5 — an audit-required event could not be durably committed (the audited mutator cannot
    /// audit). Raised on the agent-mutation adjudication's audit-write fault (call-2), paired with a
    /// fail-closed Deny carrying `GatewayDenyKind::AuditWriteFailed`.
    AuditWriteFailed,
}

/// One integrity incident — the structured record raised on the independent durable channel. The
/// `detail` is **content-free by CONSTRUCTION** (§15): it is built only via the factory ctors below
/// from the STRUCTURAL `action_type`, so a tool payload / secret can NEVER reach the alarm file (which
/// is an UN-REDACTED persist sink — the Redactor doesn't gate it). The field is private to enforce
/// this structurally, not by a doc comment the C2 caller might forget.
#[derive(Debug, Clone, Serialize)]
pub struct IntegrityIncident {
    pub kind: IntegrityKind,
    detail: String,
}

impl IntegrityIncident {
    /// The §15 #5 audit-write-fault incident — built from the STRUCTURAL `action_type` ONLY (a catalog
    /// key like `agent.bash`, never the tool inputs). Raised when an agent-mutation adjudication's
    /// audit event could not durably commit; paired with a fail-closed Deny carrying
    /// `GatewayDenyKind::AuditWriteFailed` (the C2 call-2 wiring).
    pub fn audit_write_failed(action_type: &str) -> Self {
        Self {
            kind: IntegrityKind::AuditWriteFailed,
            detail: format!("audit event could not durably commit for action_type '{action_type}'"),
        }
    }
}

/// The independent durable integrity-alarm sink (call-2). An injectable, object-safe seam (the
/// `TelemetryEventSink` / `TerminalEventSink` precedent) so the production binds the durable file sink
/// and tests assert via a recording double. **MUST NOT depend on the DB audit-write path** — that is
/// the broken thing it reports; a sink that used it would be silent in exactly the case it exists for.
pub trait IntegrityAlarm: Send + Sync {
    fn raise(&self, incident: IntegrityIncident);
}

/// The production sink — appends each incident as one JSON line to an append-only file (fsync'd) in
/// the daemon app-support dir, **independent of the DB** (it holds only a path). Best-effort on its
/// OWN I/O error: we are ALREADY degraded (the audit path faulted), and the load-bearing control is
/// the fail-closed Deny — a lost alarm line must never panic the deny path, so an append error is
/// logged to stderr, not propagated.
pub struct FileIntegrityAlarm {
    path: PathBuf,
}

impl FileIntegrityAlarm {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Append one incident as a JSON line, fsync'd. Fallible — the caller ([`raise`](Self::raise))
    /// degrades to a stderr log on error (never panics the deny path). Opens a fresh handle per
    /// incident (open→append→fsync→close): independence-over-performance — a per-incident alarm, NOT a
    /// hot path (one incident per faulted adjudication, which already paid a Gateway round-trip).
    fn append(&self, incident: &IntegrityIncident) -> std::io::Result<()> {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        let line = serde_json::to_string(incident).map_err(std::io::Error::other)?;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            // 0600 owner-only (§15 / safety rule #11 ethos) — the incident log is daemon-private safety
            // state; never group/world-readable, even though `detail` is content-free by construction.
            .mode(0o600)
            .open(&self.path)?;
        writeln!(f, "{line}")?;
        // fsync — the record is DURABLE before we return (independent of the DB's WAL); this is the
        // "durable" half of the user-ruled best-practice surface.
        f.sync_all()
    }
}

impl IntegrityAlarm for FileIntegrityAlarm {
    fn raise(&self, incident: IntegrityIncident) {
        if let Err(e) = self.append(&incident) {
            // ALREADY degraded — log loudly to stderr but never panic the (fail-closed) deny path.
            eprintln!(
                "nexusopsd: INTEGRITY ALARM could not be durably recorded ({:?}: {}): {e}",
                incident.kind, incident.detail
            );
        }
    }
}

/// A recording [`IntegrityAlarm`] test double — collects raised incidents so C2's wiring tests can
/// assert `route_intercept` fired the alarm on an audit-write fault. **`test-support`-gated (P4.0b-2
/// L3)** — a test double that SILENTLY SWALLOWS incidents must never be bound in production by
/// accident (excluded from `cargo build`/release; production binds [`FileIntegrityAlarm`]).
#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
pub struct RecordingIntegrityAlarm {
    incidents: std::sync::Mutex<Vec<IntegrityIncident>>,
}

#[cfg(any(test, feature = "test-support"))]
impl RecordingIntegrityAlarm {
    pub fn new() -> Self {
        Self::default()
    }

    /// A snapshot of the incidents raised so far.
    pub fn incidents(&self) -> Vec<IntegrityIncident> {
        self.incidents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl IntegrityAlarm for RecordingIntegrityAlarm {
    fn raise(&self, incident: IntegrityIncident) {
        self.incidents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(incident);
    }
}
