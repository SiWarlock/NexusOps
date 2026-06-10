//! The outbox-drainer + lease-reaper interval loops (§12 / §7.2 / §17).
//!
//! Two long-running async tasks tick on a config-injected interval and ask the **single
//! write-actor** (via its [`WriteHandle`]) to `drain_once` (deliver due outbox rows, bounded by
//! `DRAIN_BATCH_LIMIT`) and `reap_leases` (free expired leases). Each loop **survives a failed
//! pass** — one error is logged and the loop continues — and stops cleanly on the shutdown
//! signal. The deterministic units (`drain_once`/`reap_once`) landed in 1.3/1.4; this is the
//! 1.6b spawn that wires them onto the runtime.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::eventstore::Destination;
use crate::runtime::WriteHandle;

/// Spawn the outbox-drainer loop: every `interval`, drain one bounded pass for `dest` through the
/// write-actor. A failed pass (e.g. the actor is gone) is logged and the loop continues; the
/// `shutdown` watch breaking to `true` stops it cleanly. Returns the task's [`JoinHandle`].
pub fn spawn_drainer(
    handle: WriteHandle,
    dest: Arc<dyn Destination>,
    interval: Duration,
    mut shutdown: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // skip (don't burst) ticks missed while a slow pass ran — a backlog must not flood the
        // single writer with back-to-back drain commands.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                // shutdown takes priority over a ready tick (deterministic, prompt stop).
                biased;
                _ = shutdown.changed() => break,
                _ = ticker.tick() => match handle.drain_once(dest.clone()).await {
                    Ok(summary) => {
                        if summary.delivered > 0 || summary.dead > 0 {
                            eprintln!(
                                "nexusopsd: outbox drained {} (retried {}, dead {})",
                                summary.delivered, summary.retried, summary.dead
                            );
                        }
                    }
                    // a failed pass is logged + survived — the loop keeps ticking (liveness).
                    Err(e) => eprintln!("nexusopsd: outbox drain pass failed: {e}"),
                },
            }
        }
    })
}

/// Spawn the lease-reaper loop: every `interval`, free expired leases through the write-actor.
/// A failed pass is logged + survived; the `shutdown` watch stops it cleanly.
pub fn spawn_reaper(
    handle: WriteHandle,
    interval: Duration,
    mut shutdown: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => break,
                _ = ticker.tick() => match handle.reap_leases().await {
                    Ok(reaped) if !reaped.is_empty() => {
                        eprintln!("nexusopsd: reaped {} expired lease(s)", reaped.len());
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("nexusopsd: lease reap pass failed: {e}"),
                },
            }
        }
    })
}
