//! P4.0b-2 L1b (CAT-1) — the per-session `decision_sink` **registry**: the live-interception
//! transport's pending-decision table (the genuinely-new concurrency surface; lead call 1 + the
//! MANDATORY dedicated `decision_sink` security pass). Deterministic, channel-driven — NO live agent.
//!
//! The registry pairs with the L1 `resolve_verdict` wait (`harness/claude/decision.rs`): the live
//! `intercept` transport `register`s a pending decision keyed by the adjudication `action_request_id`
//! (tagged with the daemon session it belongs to) and hands the `oneshot::Receiver` to
//! `resolve_verdict`; the `approve`/`deny` path `resolve`s it (fires the §15 #5 audit-gated terminal
//! → the verdict); the supervisor `cancel_session`s a dead session's pendings (DROP → Deny, fast).
//!
//! These tests pin the two L1 `decision_sink` security carry-forwards the dedicated pass must verify:
//!   (a) `cancel_session` DROPS the sender on session cancel/death → `resolve_verdict` sees Err → Deny
//!       (NEVER hangs to the 5-min timeout);
//!   (b) a decision arriving AFTER the waiter cleaned up (timeout) does NOT re-deliver (no re-arm).
//! plus exactly-once + first-terminal-wins (the oneshot single-send) + unknown-id benign no-op.

use std::time::Duration;

use nexusops_shared::status::ActionRequest as ActionRequestStatus;
use nexusopsd::decisions::DecisionRegistry;
use nexusopsd::harness::claude::decision::resolve_verdict;
use nexusopsd::harness::MutationVerdict;

/// the locked §6.2 wall-clock approval-wait (call 1). The carry-forward (a) tests assert a DROP
/// resolves FAST (Err, not this timeout) — so a hung registry would change Deny-now into Deny-in-5min.
const FIVE_MIN: Duration = Duration::from_secs(300);

/// an upper bound on a FAST (drop-resolved) verdict — `cancel_session`'s DROP wakes the receiver
/// immediately, so a fast wait completes in microseconds. A regression that LEAKED the sender (kept
/// it alive) would instead block on `FIVE_MIN`; this bound makes carry-forward (a)'s "never hangs to
/// the timeout" an EXPLICIT assertion (fails fast on regression instead of hanging the suite 5 min).
/// Generous (10 s) so CI scheduling jitter can't flake it — only a true leak would exceed it.
const FAST: Duration = Duration::from_secs(10);

/// Resolve a verdict that MUST come fast (the drop path) — fails the test (rather than hanging the
/// suite for 5 min) if a regression made `resolve_verdict` actually wait on the `FIVE_MIN` timer.
async fn fast_verdict(
    rx: tokio::sync::oneshot::Receiver<nexusopsd::harness::claude::decision::DecisionSignal>,
) -> MutationVerdict {
    tokio::time::timeout(FAST, resolve_verdict(rx, FIVE_MIN))
        .await
        .expect(
            "a dropped/resolved decision must resolve FAST — never hang to the 5-min wall-clock",
        )
}

// ---- L1b #1 — register → resolve delivers the audit-gated verdict, exactly once ------------------

#[tokio::test]
async fn test_register_resolve_delivers_verdict() {
    // spec(§15 call 1) — the live transport registers a pending decision; the approve path `resolve`s
    // it with the §6.2 terminal status; the per-call `resolve_verdict` wait then yields that verdict
    // (Approved → Allow / Denied → Deny). `resolve` REMOVES the entry (pending_count→0) so a later
    // resolve no-ops — the full first-terminal-wins property is `test_first_terminal_wins_*` (#4).
    let reg = DecisionRegistry::new();
    let rx = reg.register("act_1".to_string(), "sess_a".to_string());
    assert_eq!(reg.pending_count(), 1, "the decision is registered");

    reg.resolve("act_1", ActionRequestStatus::Approved);
    assert_eq!(
        reg.pending_count(),
        0,
        "resolve removes the entry (exactly-once)"
    );

    let verdict = resolve_verdict(rx, FIVE_MIN).await;
    assert!(
        matches!(verdict, MutationVerdict::Allow),
        "an Approved terminal resolves the wait to Allow"
    );

    // a Denied terminal resolves to Deny (the only allow-terminals are Approved/PolicyDecided).
    let rx = reg.register("act_2".to_string(), "sess_a".to_string());
    reg.resolve("act_2", ActionRequestStatus::Denied);
    assert!(matches!(
        resolve_verdict(rx, FIVE_MIN).await,
        MutationVerdict::Deny { .. }
    ));
}

// ---- L1b #2 — carry-forward (a): cancel_session DROPS the pendings → Deny (fast, not a timeout) ---

#[tokio::test]
async fn test_cancel_session_drops_pending_fails_closed() {
    // spec(§15 call 1 / carry-forward a) — on session cancel/death the supervisor `cancel_session`s
    // the dead session's pendings: each sender is DROPPED → its `resolve_verdict` resolves to Err →
    // Deny IMMEDIATELY (never the 5-min timeout). ONLY the named session is swept; others remain.
    let reg = DecisionRegistry::new();
    let rx_a1 = reg.register("act_a1".to_string(), "sess_a".to_string());
    let rx_a2 = reg.register("act_a2".to_string(), "sess_a".to_string());
    let rx_b = reg.register("act_b".to_string(), "sess_b".to_string());

    let cancelled = reg.cancel_session("sess_a");
    assert_eq!(cancelled, 2, "both of sess_a's pendings were cancelled");
    assert_eq!(reg.pending_count(), 1, "sess_b's pending is untouched");

    // sess_a's waits fail closed → Deny, resolved BY THE DROP (not the 5-min wall-clock). `fast_verdict`
    // makes "fast" an explicit assertion — a leaked sender would hang to FIVE_MIN, tripping the FAST bound.
    assert!(matches!(
        fast_verdict(rx_a1).await,
        MutationVerdict::Deny { .. }
    ));
    assert!(matches!(
        fast_verdict(rx_a2).await,
        MutationVerdict::Deny { .. }
    ));
    // sess_b is unaffected — it still resolves normally.
    reg.resolve("act_b", ActionRequestStatus::Approved);
    assert!(matches!(
        resolve_verdict(rx_b, FIVE_MIN).await,
        MutationVerdict::Allow
    ));
}

// ---- L1b #3 — carry-forward (b): a late decision after the waiter cleaned up does NOT re-deliver --

#[tokio::test]
async fn test_resolve_after_waiter_cleanup_no_redeliver() {
    // spec(§15 call 1 / carry-forward b) — when `resolve_verdict` times out, the waiter `remove`s its
    // own entry (cleanup). A human decision arriving AFTER that must NOT re-deliver / re-arm any wait
    // — `resolve` finds nothing → a benign no-op. The orphaned receiver fails closed → Deny.
    let reg = DecisionRegistry::new();
    let rx = reg.register("act_1".to_string(), "sess_a".to_string());

    // the waiter's post-timeout cleanup.
    reg.remove("act_1");
    assert_eq!(reg.pending_count(), 0, "the waiter removed its own entry");

    // a LATE decision — must be a no-op (no re-arm, no phantom, no panic).
    reg.resolve("act_1", ActionRequestStatus::Approved);
    assert_eq!(
        reg.pending_count(),
        0,
        "a late resolve does NOT re-register"
    );

    // the orphaned receiver resolves to Deny (its sender was dropped by `remove`) — fail-closed.
    assert!(matches!(
        resolve_verdict(rx, FIVE_MIN).await,
        MutationVerdict::Deny { .. }
    ));
}

// ---- L1b #4 — first-terminal-wins + an unknown-id resolve/cancel is a benign no-op ----------------

#[tokio::test]
async fn test_first_terminal_wins_and_unknown_resolve_noop() {
    // spec(§15 call 1) — first-terminal-wins: the first `resolve` fires + removes; a second `resolve`
    // for the same id finds nothing → ignored, so the wait yields the FIRST verdict (Approved→Allow),
    // never the later Denied. And `resolve`/`cancel_session` of an UNKNOWN id is a benign no-op (the
    // approve/deny path fires `resolve` UNCONDITIONALLY — a non-adjudication approve must not panic).
    let reg = DecisionRegistry::new();
    let rx = reg.register("act_1".to_string(), "sess_a".to_string());

    reg.resolve("act_1", ActionRequestStatus::Approved); // first terminal — fires + removes
    reg.resolve("act_1", ActionRequestStatus::Denied); // second — no-op (already removed)
    assert!(
        matches!(resolve_verdict(rx, FIVE_MIN).await, MutationVerdict::Allow),
        "the FIRST terminal (Approved → Allow) wins; the later Denied is ignored"
    );

    // unknown-id paths are benign no-ops (the approve/deny handler resolves every ack unconditionally).
    reg.resolve("act_unknown", ActionRequestStatus::Approved);
    assert_eq!(
        reg.cancel_session("sess_unknown"),
        0,
        "cancelling an unknown session cancels nothing"
    );
    assert_eq!(reg.pending_count(), 0);
}
