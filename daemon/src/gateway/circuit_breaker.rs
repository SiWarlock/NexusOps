//! P4.0b-2c (CAT-1-adjacent, §17/§15 #5) — the daemon-wide **audit-backbone circuit-breaker**.
//!
//! The single audited mutator must NOT operate when it can no longer audit. The per-action audit-write
//! fault is already handled (deny + the C2 per-incident durable alarm, `integrity.rs`); this is the
//! **SYSTEMIC** layer — when audit-write faults are *persistent* (N consecutive, no intervening clean
//! commit) or *clearly unrecoverable* (disk-full / DB-corruption / IO), the breaker **trips** and the
//! daemon **quiesce-and-refuses** every mutation (RULED B, lead 2026-06-13): the latch makes every
//! subsequent mutation fail-closed-deny WITHOUT attempting an audit-write, while reads stay live.
//!
//! This module is PURE state (a thread-safe consecutive-fault counter + a latched trip flag) plus the
//! recoverable-vs-unrecoverable classification of an [`EventStoreError`]. It raises the durable
//! systemic [`IntegrityAlarm`] exactly once on the trip, ATOMICALLY with the latch. The production
//! gate + feed live at the write-actor (`run_actor`) — the single chokepoint every Gateway mutation
//! funnels through (forbidden #2/#3); the breaker is daemon-internal (no `shared/` surface).

use std::sync::{Arc, Mutex, PoisonError};

use rusqlite::ErrorCode;

use crate::eventstore::EventStoreError;
use crate::gateway::GatewayError;
use crate::integrity::{IntegrityAlarm, IntegrityIncident};

/// The default N for the consecutive-fault trip (Q2) — rides out a brief transient cluster (each fault
/// still per-action-denies + alarms) but trips fast on a sustained fault.
pub const DEFAULT_AUDIT_BREAKER_THRESHOLD: u32 = 5;

/// The classified outcome of one Gateway audit-write attempt, as the breaker sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    /// the audit event durably committed — the backbone is proven healthy (resets the consecutive run).
    Success,
    /// a TRANSIENT audit-write fault (SQLITE_BUSY / lock-timeout / an unclassified write error) — counts
    /// toward N but does NOT fast-trip (the per-action deny already protected the invariant on it).
    RecoverableFault,
    /// a clearly-UNRECOVERABLE audit-write fault (disk-full / DB-corruption / IO failure) — fast-trips
    /// on the FIRST occurrence (well below N): the audit backbone is not coming back without operator
    /// intervention, so the daemon must stop mutating immediately.
    UnrecoverableFault,
}

/// What an `observe`/`observe_gateway_result` call did to the breaker (the caller acts on `Tripped`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerAction {
    /// a clean audit-commit cleared the consecutive run to 0.
    Reset,
    /// a recoverable fault was counted (still < N — not tripped).
    Counted,
    /// THIS observation tripped the breaker (the latch flipped; the systemic alarm was raised once).
    Tripped,
    /// the breaker was ALREADY tripped (latched) — this observation is ignored (never auto-resets).
    AlreadyTripped,
    /// a non-audit-write outcome (a pre-audit reject: policy-deny / serialize / not-found …) — a
    /// no-op: it neither proves the backbone healthy (no commit) nor is it an audit fault, so it must
    /// NOT reset the run (else spammed denials could mask a concurrent systemic audit fault).
    Unaffected,
}

/// Classify a fail-closed audit-write [`EventStoreError`] as recoverable (transient) or unrecoverable
/// (Q3). The unrecoverable allow-list is the clearly-terminal SQLite primary codes — disk-full,
/// DB-corruption, IO failure, not-a-database; everything else (SQLITE_BUSY/LOCKED, and any
/// unclassified write error) is RECOVERABLE — conservative, so a single unclassified transient never
/// false-latches the daemon (a sustained run still trips at N).
pub fn classify_audit_fault(err: &EventStoreError) -> AuditOutcome {
    match err {
        EventStoreError::Write(rusqlite::Error::SqliteFailure(e, _)) => match e.code {
            ErrorCode::DiskFull
            | ErrorCode::DatabaseCorrupt
            | ErrorCode::SystemIoFailure
            | ErrorCode::NotADatabase => AuditOutcome::UnrecoverableFault,
            _ => AuditOutcome::RecoverableFault,
        },
        _ => AuditOutcome::RecoverableFault,
    }
}

#[derive(Default)]
struct BreakerState {
    consecutive: u32,
    tripped: bool,
}

/// The daemon-wide audit-backbone breaker: a thread-safe consecutive-fault counter + a latched trip
/// flag, holding the durable [`IntegrityAlarm`] it raises (once) on the trip. Shared as an `Arc` (the
/// write-actor observes + gates; `main` keeps a handle so the §17 surface can read [`is_tripped`]).
pub struct AuditBackboneBreaker {
    alarm: Arc<dyn IntegrityAlarm>,
    threshold: u32,
    state: Mutex<BreakerState>,
}

impl AuditBackboneBreaker {
    /// A breaker with the [`DEFAULT_AUDIT_BREAKER_THRESHOLD`] N.
    pub fn new(alarm: Arc<dyn IntegrityAlarm>) -> Self {
        Self::with_threshold(alarm, DEFAULT_AUDIT_BREAKER_THRESHOLD)
    }

    /// A breaker with an explicit N (tunable; secondary to the Q1 mechanism).
    pub fn with_threshold(alarm: Arc<dyn IntegrityAlarm>, threshold: u32) -> Self {
        Self {
            alarm,
            // a 0 threshold would trip on the first recoverable fault — clamp to ≥1 (a real N).
            threshold: threshold.max(1),
            state: Mutex::new(BreakerState::default()),
        }
    }

    /// Observe one classified audit-write outcome + advance the breaker. Latched: once tripped, every
    /// call returns `AlreadyTripped` (never auto-resets — §17 never-auto-resolved). The systemic alarm
    /// is raised exactly once, on the trip, ATOMICALLY with the latch (still under the state lock — no
    /// observer sees `tripped=true` without the alarm having fired).
    pub fn observe(&self, outcome: AuditOutcome) -> BreakerAction {
        let mut guard = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if guard.tripped {
            return BreakerAction::AlreadyTripped;
        }
        let trip = match outcome {
            AuditOutcome::Success => {
                guard.consecutive = 0;
                return BreakerAction::Reset;
            }
            AuditOutcome::UnrecoverableFault => true,
            AuditOutcome::RecoverableFault => {
                // saturating: a pathological `with_threshold(_, u32::MAX)` + u32::MAX faults must never
                // panic (debug overflow) the safety path — it would trip long before saturating anyway.
                guard.consecutive = guard.consecutive.saturating_add(1);
                guard.consecutive >= self.threshold
            }
        };
        if trip {
            guard.tripped = true;
            self.alarm
                .raise(IntegrityIncident::audit_backbone_systemic_failure());
            BreakerAction::Tripped
        } else {
            BreakerAction::Counted
        }
    }

    /// Observe a Gateway pipeline `Result` (the daemon-wide feed). `Ok` ⟺ an event committed = the
    /// backbone is proven healthy → reset. `AuditWriteFailed` → the §17 fault (classified). EVERY OTHER
    /// `Err` (a pre-audit reject that never touched the audit-write path) → `Unaffected` (no-op, NOT a
    /// reset) — so spammed denials can't mask a concurrent systemic audit fault.
    pub fn observe_gateway_result<T>(&self, result: &Result<T, GatewayError>) -> BreakerAction {
        match result {
            Ok(_) => self.observe(AuditOutcome::Success),
            Err(GatewayError::AuditWriteFailed(e)) => self.observe(classify_audit_fault(e)),
            Err(_) => BreakerAction::Unaffected,
        }
    }

    /// Whether the breaker is latched-tripped (the daemon-side §17 safety-state the gate consults + a
    /// future §17/Phase-6 surface reads). Once true, stays true until an operator restart.
    pub fn is_tripped(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tripped
    }

    /// The current consecutive-fault count (diagnostic / test).
    pub fn consecutive(&self) -> u32 {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .consecutive
    }
}
