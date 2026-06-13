//! P4.0b-2-F2 (brief 055; §6.4/§10) — the intercept-wait permit class (the reserved sub-bound).
//!
//! The live `intercept` handler's 5-min approval-wait holds a general accept-pool permit for the whole
//! wait. Enough concurrent waits would exhaust the general cap and starve the UI `approve`/`deny` (and
//! reads) → pending intercepts time out → fail-closed Deny → degraded availability (circular
//! starvation). Fix (Option A — reserved sub-bound): a SECOND semaphore bounded at
//! `wait_cap = MAX_CONNECTIONS − RESERVED`. A waiting intercept holds BOTH its general permit AND a
//! wait-class permit; because `wait_cap < MAX_CONNECTIONS`, at most `wait_cap` general permits are ever
//! held by waits → `RESERVED` general permits ALWAYS stay free for non-wait connections (no
//! accept-time classification, no mid-connection permit release). Wait-class exhaustion → fail-closed
//! Deny (fail-SAFE, never a bypass).

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// The general concurrent-connection cap (§6.4 anti-DoS) — the canonical value (the accept loop's
/// general semaphore + the wait-class sub-bound both derive from it; `main.rs` passes it through).
pub const MAX_CONNECTIONS: usize = 64;

/// The reserved general-pool headroom the intercept-waits can NEVER hold — guaranteeing the UI
/// `approve`/`deny` + reads never starve (Q2: a quarter of the pool — 16 of 64; the UI's connections
/// are short-lived and cycle through the reserved slots).
pub const RESERVED_GENERAL: usize = MAX_CONNECTIONS / 4;

/// The intercept-wait permit class — a bounded sub-pool (`wait_cap = max − reserved`) the live
/// `intercept` handler parks in for the duration of an `AwaitingApproval` approval-wait. Cheaply
/// cloneable (shares one `Semaphore` via `Arc`) so the accept loop hands a clone to each connection.
#[derive(Clone)]
pub struct InterceptWaitClass {
    sem: Arc<Semaphore>,
    wait_cap: usize,
}

impl InterceptWaitClass {
    /// A wait class capped at `max − reserved`, with `reserved` clamped to `1..=max−1`. For any
    /// **`max ≥ 2`** (every real config — production is 64) this guarantees BOTH `wait_cap ≥ 1` AND
    /// `wait_cap < max` (so `reserved = max − wait_cap ≥ 1` general slots ALWAYS stay free — the
    /// no-starvation guarantee). `max ≤ 1` is a DEGENERATE config (no valid split exists — you can't
    /// have ≥1 reserved AND ≥1 wait slot out of ≤1 total); it clamps to `wait_cap = 1` (no headroom),
    /// but the daemon never runs with `max_connections ≤ 1`. The debug_assert guards the guarantee
    /// against a future edit that breaks it for a real config.
    pub fn new(max: usize, reserved: usize) -> Self {
        let reserved = reserved.clamp(1, max.saturating_sub(1).max(1));
        let wait_cap = max.saturating_sub(reserved).max(1);
        debug_assert!(
            wait_cap < max || max <= 1,
            "InterceptWaitClass: wait_cap ({wait_cap}) must be < max ({max}) so RESERVED general \
             headroom stays free (the no-starvation guarantee); only the degenerate max≤1 is exempt"
        );
        Self {
            sem: Arc::new(Semaphore::new(wait_cap)),
            wait_cap,
        }
    }

    /// The production wait class: `MAX_CONNECTIONS − RESERVED_GENERAL` (48 of 64). The accept loop
    /// derives the same value from the `max_connections` it is given; this is the canonical default
    /// the unit test pins.
    pub fn production_default() -> Self {
        Self::new(MAX_CONNECTIONS, RESERVED_GENERAL)
    }

    /// Try to PARK an intercept-wait (non-blocking). `Some(permit)` — a wait slot was acquired; hold it
    /// for the whole approval-wait, drop it on EVERY terminal (verdict / timeout / cancel / death /
    /// bridge-drop) to release the slot. `None` — the class is saturated → the caller fail-closed-denies
    /// (never parks, never bypasses).
    pub fn try_park(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.sem).try_acquire_owned().ok()
    }

    /// The wait-class capacity (`max − reserved`). The reserved general headroom is `max − wait_cap`.
    pub fn wait_cap(&self) -> usize {
        self.wait_cap
    }

    /// The currently-free wait slots (diagnostic / test — a leak check).
    pub fn available_permits(&self) -> usize {
        self.sem.available_permits()
    }
}
