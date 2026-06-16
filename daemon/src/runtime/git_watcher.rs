//! The git-watcher interval loop (§7.2 / ARCHITECTURE.md:340 "git watcher").
//!
//! A long-running async task that ticks on a config-injected interval and refreshes the §7.2
//! worktree-status live-read cache: it enumerates `proj_worktree` (a read-only WAL connection — never
//! the writer, forbidden #3) and asks the single write-actor to `refresh_worktree_status` for each
//! known worktree (the git2 read + the non-event `proj_worktree` cache UPDATE run on the writer). The
//! drainer/reaper precedent: it **survives a failed pass** (logged, the loop continues) and stops
//! cleanly on the shutdown signal. The deterministic refresh unit is `EventStore::refresh_worktree_status`;
//! this is the runtime spawn that drives it (1.6b's "ship the unit, wire the runtime" pattern).

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::eventstore::{open_read_only, EventStoreError};
use crate::runtime::WriteHandle;

/// One worktree's refresh inputs, read from `proj_worktree`.
struct WatchedWorktree {
    worktree_id: String,
    path: String,
    base: Option<String>,
}

/// Enumerate `proj_worktree` for the refresh loop (a read-only WAL conn — NEVER the writer, forbidden
/// #3). Propagates a read error (a transient open / prepare / query failure) so the loop can LOG it +
/// survive (the drainer precedent logs at the loop). A blocking rusqlite read, so the caller runs it
/// via `spawn_blocking` (off the async worker). A per-row decode error drops that row (best-effort).
fn list_worktrees(db_path: &Path) -> Result<Vec<WatchedWorktree>, EventStoreError> {
    let conn = open_read_only(db_path)?;
    // read errors map to `Write` (the codebase's reader-typing convention — `read_all`/
    // `first_event_of_type`; a dedicated `Read` variant is a deferred repo-wide cleanup).
    let mut stmt = conn
        .prepare("SELECT worktree_id, path, base_branch FROM proj_worktree")
        .map_err(EventStoreError::Write)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(WatchedWorktree {
                worktree_id: r.get(0)?,
                path: r.get(1)?,
                base: r.get(2)?,
            })
        })
        .map_err(EventStoreError::Write)?;
    Ok(rows.filter_map(Result::ok).collect())
}

/// Spawn the git-watcher loop: every `interval`, refresh each `proj_worktree` row's §7.2 live-read
/// cache through the write-actor. A failed pass (a gone actor, a transient read) is logged + survived;
/// the `shutdown` watch breaking to `true` stops it cleanly. Returns the task's [`JoinHandle`].
pub fn spawn_git_watcher(
    handle: WriteHandle,
    db_path: PathBuf,
    interval: Duration,
    mut shutdown: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // skip (don't burst) ticks missed while a slow pass ran — a backlog must not flood the writer.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                // shutdown takes priority over a ready tick (deterministic, prompt stop).
                biased;
                _ = shutdown.changed() => break,
                _ = ticker.tick() => {
                    // enumerate OFF the async worker (a blocking rusqlite read via spawn_blocking),
                    // then refresh each worktree through the single writer. A read error OR a panic in
                    // the read is logged + folds to an empty list → the loop survives (liveness; the
                    // drainer precedent logs failures rather than swallowing them).
                    let db = db_path.clone();
                    let worktrees = match tokio::task::spawn_blocking(move || list_worktrees(&db)).await {
                        Ok(Ok(wts)) => wts,
                        Ok(Err(e)) => {
                            eprintln!("nexusopsd: git-watcher proj_worktree read pass failed: {e}");
                            Vec::new()
                        }
                        Err(e) => {
                            eprintln!("nexusopsd: git-watcher enumeration task panicked: {e}");
                            Vec::new()
                        }
                    };
                    for wt in worktrees {
                        if let Err(e) = handle
                            .refresh_worktree_status(wt.worktree_id, wt.path, wt.base)
                            .await
                        {
                            // a failed refresh is logged + survived — the loop keeps ticking (liveness).
                            eprintln!("nexusopsd: worktree status refresh pass failed: {e}");
                        }
                    }
                }
            }
        }
    })
}
