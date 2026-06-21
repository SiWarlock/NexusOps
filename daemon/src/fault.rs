//! §14 deterministic fault-injection checkpoints (2.4).
//!
//! **Compiled out of every production build.** The whole module is behind the `fault-injection`
//! feature, which is enabled ONLY for the daemon's own test/bench targets (the self-dev-dependency
//! in `Cargo.toml`) — `cargo build`/`cargo run`/`--release` do NOT pull dev-dependencies, so the
//! feature is off and this module (and every `#[cfg(feature = "fault-injection")]` consult site)
//! disappears. There is **no fault-injection surface in a production binary** — it cannot be armed.
//!
//! A test arms a one-shot/counted fault on its thread; the gateway/eventstore consult the matching
//! checkpoint and inject the failure deterministically — NO real SIGKILL/threads (a real kill is
//! flaky + un-assertable). This is the verification vehicle for the §17 fail-closed audit-write (L2)
//! + crash-reconcile (L5) safety behaviors.

use std::cell::Cell;

/// The named §14 checkpoints a test can arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultPoint {
    /// The next (counted) **terminal-event** append — `ActionSucceeded`, `ActionFailed`, OR
    /// `ActionPartiallySucceeded` (all three terminal outcome events are treated UNIFORMLY: any of
    /// them failing to write is the §15/§17 audit-write failure) — returns an injected
    /// `EventStoreError` instead of writing, modelling the fail-closed completion-txn path (L2).
    /// Count > 1 fails BOTH the success record (txn-B) AND the best-effort partial-success record
    /// (txn-C) — the audit-fully-broken case.
    TerminalEventWrite,
    /// the gateway aborts at execute START, BEFORE the executing-commit (txn-A) — models a crash after
    /// approve+queue but before execute began, leaving a `queued` orphan (L5 crash-reconcile). One-shot/counted.
    BeforeExecutingTxn,
    /// the gateway aborts AFTER the executing-commit (txn-A) + BEFORE the terminal txn — models a crash
    /// mid-execute, leaving an `executing` orphan (L5 crash-reconcile). One-shot/counted.
    BeforeTerminalTxn,
    /// (043 L3) the next (counted) `ActionRequested` OR `ActionApproved` append — the **adjudication
    /// audit-gate** events: a risk-0 allow verdict is gated on `ActionRequested`, a human-approved
    /// allow on `ActionApproved` (§15 #5 — an Allow is gated on the authoritative event being durably
    /// written FIRST). Returns an injected write error, exercising the "audit-write BEFORE verdict,
    /// fail-closed" path: the submit/approve txn rolls back → the intercept resolves to Deny, never Allow.
    AuditEventWrite,
    /// (P5.3a) the next event append inside a durable-registry register-mutator `gateway_txn` — models a
    /// fail-closed audit-write fault for the {canonical row + `ExecutionProfileRegistered`} dual-gate: the
    /// event append fails → the whole txn rolls back → NO row persists (LESSON 16; the registry row is
    /// durable ONLY if its audit trail is). One-shot/counted.
    RegistryEventWrite,
}

thread_local! {
    // (armed checkpoint, remaining fire count). Per-thread: in tests the gateway runs synchronously on
    // the arming thread; the feature is off in the production runtime/write-actor, so nothing arms it.
    static ARMED: Cell<Option<(FaultPoint, usize)>> = const { Cell::new(None) };
}

/// Arm a ONE-SHOT fault on the current thread (fires on the next matching checkpoint).
pub fn arm(fp: FaultPoint) {
    arm_n(fp, 1);
}

/// Arm a fault that fires on the next `count` matching checkpoints. `count == 0` disarms. Used to fail
/// BOTH the terminal-success AND the partial-success append (the audit-fully-broken fail-closed case).
pub fn arm_n(fp: FaultPoint, count: usize) {
    ARMED.with(|c| c.set(if count == 0 { None } else { Some((fp, count)) }));
}

/// If `fp` is armed with a remaining count, consume ONE fire (decrement; disarm at 0) and return
/// `true` (the caller injects its failure). A non-matching / disarmed checkpoint returns `false`.
pub fn take_if(fp: FaultPoint) -> bool {
    ARMED.with(|c| match c.get() {
        Some((armed, n)) if armed == fp && n > 0 => {
            c.set(if n - 1 == 0 {
                None
            } else {
                Some((armed, n - 1))
            });
            true
        }
        _ => false,
    })
}
