//! P4.0b-2 L1 — the live INV-SEC-1 interception **decision-wait** (the genuinely-new, security-
//! critical concurrency surface; lead call 1 + the MANDATORY dedicated `decision_sink` security pass).
//!
//! Deterministic, FakeHarness/driven-channel — NO live agent. Pins the verdict-resolution race:
//! the human decision vs. the 5-min wall-clock timeout vs. session cancel/death, **fail-closed**
//! (every non-Allow terminal → Deny; LOCKED, not a knob), exactly-once, first-terminal-wins. The
//! live hook transport + the real launch are L2 (behavior-pinned + security-reviewed, not unit-tested).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use nexusops_shared::status::ActionRequest as ActionRequestStatus;
use nexusopsd::harness::claude::decision::{deliver_once, resolve_verdict, DecisionSignal};
use nexusopsd::harness::{MutationDecisionSink, MutationVerdict};
use tokio::sync::oneshot;

const FIVE_MIN: Duration = Duration::from_secs(300);

/// a counting FnOnce sink — proves the decision_sink fires EXACTLY once.
fn counting_sink(
    seen: Arc<AtomicU32>,
) -> (MutationDecisionSink, Arc<std::sync::Mutex<Option<bool>>>) {
    let allowed = Arc::new(std::sync::Mutex::new(None));
    let a = allowed.clone();
    let sink: MutationDecisionSink = Box::new(move |v: MutationVerdict| {
        seen.fetch_add(1, Ordering::SeqCst);
        *a.lock().unwrap() = Some(matches!(v, MutationVerdict::Allow));
    });
    (sink, allowed)
}

// ---- L1 #1 — exactly-once + first-terminal-wins (the human approves) -------------------------------

#[tokio::test]
async fn test_decision_sink_exactly_once_first_terminal_wins() {
    // spec(§15 call 1) — an Approved decision resolves to Allow; the FnOnce sink fires EXACTLY once;
    // a second resolution cannot occur (the oneshot is single-send → first-terminal-wins structurally).
    let (tx, rx) = oneshot::channel::<DecisionSignal>();
    tx.send(DecisionSignal::Resolved(ActionRequestStatus::Approved))
        .expect("send the decision");
    let verdict = resolve_verdict(rx, FIVE_MIN).await;
    assert!(
        matches!(verdict, MutationVerdict::Allow),
        "Approved → Allow"
    );

    let seen = Arc::new(AtomicU32::new(0));
    let (sink, allowed) = counting_sink(seen.clone());
    deliver_once(sink, verdict);
    assert_eq!(
        seen.load(Ordering::SeqCst),
        1,
        "the decision_sink fires exactly once"
    );
    assert_eq!(*allowed.lock().unwrap(), Some(true), "the sink saw Allow");
}

// ---- L1 #1b — a Denied/other terminal fails closed -------------------------------------------------

#[tokio::test]
async fn test_denied_terminal_fails_closed() {
    // spec(§15 call 1) — only Approved/PolicyDecided open the gate; a Denied terminal → Deny.
    for status in [ActionRequestStatus::Denied, ActionRequestStatus::Expired] {
        let (tx, rx) = oneshot::channel::<DecisionSignal>();
        tx.send(DecisionSignal::Resolved(status)).unwrap();
        let verdict = resolve_verdict(rx, FIVE_MIN).await;
        assert!(
            matches!(verdict, MutationVerdict::Deny { .. }),
            "{status:?} is not an allow-terminal → Deny (fail-closed)"
        );
    }
    // a risk-0 auto-allow terminal (PolicyDecided) DOES open the gate.
    let (tx, rx) = oneshot::channel::<DecisionSignal>();
    tx.send(DecisionSignal::Resolved(ActionRequestStatus::PolicyDecided))
        .unwrap();
    assert!(matches!(
        resolve_verdict(rx, FIVE_MIN).await,
        MutationVerdict::Allow
    ));
}

// ---- L1 #2 — the 5-min wall-clock timeout fails closed --------------------------------------------

#[tokio::test(start_paused = true)]
async fn test_approval_wait_timeout_fails_closed() {
    // spec(§6.2 d.1 / call 1) — the decision never arrives; the 5-min wall-clock elapses (paused
    // tokio time auto-advances to the timer) → Deny (fail-closed-LOCKED). The sender is held so the
    // channel doesn't drop — the TIMEOUT is what fires.
    let (_tx_held, rx) = oneshot::channel::<DecisionSignal>();
    let verdict = resolve_verdict(rx, FIVE_MIN).await;
    assert!(
        matches!(verdict, MutationVerdict::Deny { .. }),
        "a 5-min timeout with no decision → Deny (fail-closed)"
    );
}

// ---- L1 #3 — cancel / session-death mid-wait fails closed -----------------------------------------

#[tokio::test]
async fn test_approval_wait_cancel_and_death_fail_closed() {
    // spec(§15 call 1) — an explicit Cancelled signal → Deny.
    let (tx, rx) = oneshot::channel::<DecisionSignal>();
    tx.send(DecisionSignal::Cancelled).unwrap();
    assert!(
        matches!(
            resolve_verdict(rx, FIVE_MIN).await,
            MutationVerdict::Deny { .. }
        ),
        "an explicit cancel → Deny"
    );

    // session DEATH = the sender is dropped without sending (the per-session binding tore down) → the
    // receiver resolves to Err → Deny (fail-closed; never an allow from a dropped channel).
    let (tx, rx) = oneshot::channel::<DecisionSignal>();
    drop(tx);
    assert!(
        matches!(
            resolve_verdict(rx, FIVE_MIN).await,
            MutationVerdict::Deny { .. }
        ),
        "a dropped decision channel (session death) → Deny (fail-closed)"
    );
}
