//! Brief 055 — P4.0b-2-F2: the intercept-wait permit-class split (§6.4/§10, fail-safe).
//!
//! The live `intercept` handler's 5-min approval-wait holds an accept-pool permit for the whole wait;
//! enough concurrent waits exhaust `MAX_CONNECTIONS` → the UI approve/deny connection is REFUSED →
//! pending intercepts time out → fail-closed Deny → degraded availability (circular starvation). Fix
//! (Option A — reserved sub-bound): a 2nd `Semaphore(wait_cap = MAX − RESERVED)`; a waiting intercept
//! holds BOTH its general permit AND a wait-class permit, so at most `wait_cap` general permits are
//! held by waits → `RESERVED` general permits ALWAYS stay free for UI/reads. Wait-class exhaustion →
//! fail-closed Deny (fail-SAFE, NEVER a bypass).
//!
//! Tested deterministically WITHOUT a concurrent UDS harness: the `InterceptWaitClass` semaphore
//! arithmetic (the no-starvation guarantee + the bound + the drain) + the pure
//! `intercept_verdict_with_wait_class` decision (acquire-only-on-AwaitingApproval, deny-on-exhaustion
//! before the wait, release-on-terminal via the OwnedSemaphorePermit RAII). The live handler delegates
//! to that fn (reachability = Step 7.5 /wired).

use nexusopsd::harness::claude::intercept::InterceptOutcome;
use nexusopsd::harness::MutationVerdict;
use nexusopsd::ipc::intercept_verdict_with_wait_class;
use nexusopsd::runtime::InterceptWaitClass;

// ---- 055 #1 — the wait class reserves general headroom (the no-starvation core, §6.4/§10) ---------

#[test]
fn test_wait_class_reserves_general_headroom() {
    // spec(§6.4/§10) — `wait_cap = MAX − RESERVED`; a waiting intercept holds one wait permit (AND, in
    // production, one general permit). Saturating the wait class holds at most `wait_cap` general
    // permits → `RESERVED` general permits ALWAYS stay free for UI approve/deny / reads. Pin the cap
    // arithmetic (the guarantee) + the bound.
    let max = 64;
    let reserved = 16;
    let wc = InterceptWaitClass::new(max, reserved);
    assert_eq!(wc.wait_cap(), max - reserved, "wait_cap = MAX − RESERVED");
    assert_eq!(wc.available_permits(), max - reserved);
    // park up to wait_cap → all succeed.
    let mut parked = Vec::new();
    for i in 0..wc.wait_cap() {
        let p = wc
            .try_park()
            .unwrap_or_else(|| panic!("park {i} (< wait_cap) must succeed"));
        parked.push(p);
    }
    // the wait class is now full; the next park fails (fail-closed at the handler).
    assert!(
        wc.try_park().is_none(),
        "the wait class is bounded at wait_cap"
    );
    // the GUARANTEE: at most wait_cap general permits are held by waits → MAX − wait_cap == RESERVED
    // general permits are never held by waits (UI/reads keep guaranteed headroom).
    assert_eq!(
        max - wc.wait_cap(),
        reserved,
        "RESERVED general headroom is never held by waits"
    );
    assert_eq!(parked.len(), wc.wait_cap());
}

// ---- 055 #3 — the wait class drains: releasing a parked permit frees a slot --------------------------

#[test]
fn test_wait_class_drains_on_release() {
    // spec(§6.4/§10) — once a parked intercept resolves (its permit drops), the wait class drains so
    // subsequent intercepts can park again. No leak / no permanent self-DoS.
    let wc = InterceptWaitClass::new(4, 1); // wait_cap = 3
    let p0 = wc.try_park().unwrap();
    let _p1 = wc.try_park().unwrap();
    let _p2 = wc.try_park().unwrap();
    assert!(wc.try_park().is_none(), "full at wait_cap=3");
    drop(p0); // a parked intercept resolved → its permit releases
    assert!(
        wc.try_park().is_some(),
        "a freed slot can be re-parked (drain)"
    );
}

// ---- 055 #2 — wait-class exhaustion → fail-closed Deny, the wait is NEVER entered (fail-SAFE) -----

#[test]
fn test_wait_class_exhaustion_denies_fail_closed() {
    // spec(§6.4/§10, lead-ruled fail-safe) — when the wait class is full, an AwaitingApproval intercept
    // DENIES immediately: it does NOT park, does NOT enter the wait (`register_and_wait` is NEVER
    // called), does NOT bypass the gate. The verdict is Deny with an honest saturation reason.
    let wc = InterceptWaitClass::new(2, 1); // wait_cap = 1
    let _hold = wc.try_park().unwrap(); // saturate the class
    let verdict = intercept_verdict_with_wait_class(
        InterceptOutcome::AwaitingApproval {
            action_request_id: "act_x".to_string(),
        },
        &wc,
        |_id| panic!("the wait must NOT be entered when the wait class is exhausted"),
    );
    match verdict {
        MutationVerdict::Deny { reason } => assert_eq!(
            reason,
            nexusopsd::ipc::INTERCEPT_SATURATED_REASON,
            "the exhaustion Deny carries the published saturation reason (distinct from the timeout Deny)"
        ),
        MutationVerdict::Allow => panic!("exhaustion must NEVER Allow (fail-safe toward Deny)"),
    }
}

// ---- 055 #4 — the wait permit releases on every terminal (the wait was entered → permit returned) -

#[test]
fn test_wait_permit_released_after_wait() {
    // spec(§6.4/§10 / LESSONS §9 — pin permit-release) — when the wait IS entered (a permit acquired)
    // and the wait returns ANY terminal verdict, the wait-class permit is released (the OwnedSemaphore-
    // Permit RAII drops at the end of the arm). After a parked+resolved intercept the class is full
    // capacity again — no leak on the verdict/timeout/cancel/death/bridge-drop paths (all return a
    // verdict from the closure).
    let wc = InterceptWaitClass::new(2, 1); // wait_cap = 1
    assert_eq!(wc.available_permits(), 1);
    let verdict = intercept_verdict_with_wait_class(
        InterceptOutcome::AwaitingApproval {
            action_request_id: "act_y".to_string(),
        },
        &wc,
        // the wait was entered (a permit was acquired); it returns a terminal verdict (any path).
        |_id| MutationVerdict::Deny {
            reason: "decision bridge dropped — fail-closed".to_string(),
        },
    );
    assert!(matches!(verdict, MutationVerdict::Deny { .. }));
    assert_eq!(
        wc.available_permits(),
        1,
        "the wait-class permit is released after the wait returns (no leak)"
    );
}

// ---- 055 #5 — a Resolved (non-waiting) intercept NEVER touches the wait class (gate unchanged) ----

#[test]
fn test_resolved_intercept_never_touches_wait_class() {
    // spec(INV-SEC-1) — only `AwaitingApproval` consumes a wait-class permit. A risk-0 auto-allow (or
    // any immediately-`Resolved` verdict) returns its verdict WITHOUT acquiring a permit — the gate
    // semantics are unchanged + the class is scoped to the wait only.
    let wc = InterceptWaitClass::new(2, 1); // wait_cap = 1
    let before = wc.available_permits();
    let allow = intercept_verdict_with_wait_class(
        InterceptOutcome::Resolved(MutationVerdict::Allow),
        &wc,
        |_id| panic!("a Resolved intercept must NOT enter the wait"),
    );
    assert!(matches!(allow, MutationVerdict::Allow));
    let deny = intercept_verdict_with_wait_class(
        InterceptOutcome::Resolved(MutationVerdict::Deny {
            reason: "coverage gap".to_string(),
        }),
        &wc,
        |_id| panic!("a Resolved deny must NOT enter the wait"),
    );
    assert!(matches!(deny, MutationVerdict::Deny { .. }));
    assert_eq!(
        wc.available_permits(),
        before,
        "a Resolved intercept consumes NO wait-class permit"
    );
}

// ---- 055 #6 — the default RESERVED is a sane fraction of MAX -------------------------------------

#[test]
fn test_default_reserved_headroom() {
    // spec(§6.4/§10, Q2) — the production default reserves a quarter of the pool (RESERVED = MAX/4 =
    // 16 of 64 → wait_cap = 48): generous concurrent-approval headroom for the multi-agent goal while
    // the UI/reads never starve. Pinned via the production constructor.
    let wc = InterceptWaitClass::production_default();
    assert_eq!(wc.wait_cap(), 48, "wait_cap = 64 − 16 (RESERVED = MAX/4)");
}
