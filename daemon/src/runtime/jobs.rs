//! Phase 4.3 — the deterministic background maintenance jobs (§10 / §17).
//!
//! Two long-running async tasks tick on a config-injected interval (the [`spawn_drainer`] /
//! [`spawn_reaper`] family, LESSON §9):
//! - the **WAL checkpointer** ([`spawn_wal_checkpointer`]) asks the **single write-actor** to run a
//!   bounded `wal_checkpoint` each interval — bounding the SQLite WAL WITHOUT a rogue writable
//!   connection (forbidden #3). PASSIVE is the production mode (non-blocking, won't contend with the
//!   read-only WAL readers).
//! - the **session-staleness poller** ([`spawn_staleness_poller`]) recomputes per-session staleness
//!   over the **read-only WAL** each interval. Staleness is a **live-read recompute** ([`is_stale`]
//!   over `last_heartbeat_at` age) — computed/rebuilt, **never event-folded/replayed** (the §5.1
//!   `Session::Stale` is time-derived; the §7.2 worktree-status live-read precedent) — so it never
//!   back-pressures the writer.
//!
//! [`spawn_drainer`]: super::spawn_drainer
//! [`spawn_reaper`]: super::spawn_reaper

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rusqlite::Connection;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use nexusops_shared::ids::SessionId;
use nexusops_shared::status::Session;

use crate::clock::Clock;
use crate::eventstore::{open_read_only, WalCheckpointMode};
use crate::runtime::WriteHandle;

/// True iff `last_heartbeat_at` is older than `threshold_secs` relative to `now` (§5.1 stale-by-age,
/// §17). Pure + total (the LESSON §9/§36 pure-classifier family):
/// - a **missing** heartbeat (`None`) is NOT stale — absence ≠ stale (no false-stale on a session
///   that has not yet emitted a heartbeat);
/// - an **unparseable** heartbeat (or `now`) is NOT stale — the age can't be proven, so never assert;
/// - the boundary is **strict** (`age > threshold`): an exactly-`threshold`-old heartbeat is NOT
///   stale yet.
///
/// Timestamps are RFC3339 (the daemon-`Clock` form). A future-dated heartbeat (clock skew) → a
/// negative age → not stale.
pub fn is_stale(last_heartbeat_at: Option<&str>, now: &str, threshold_secs: i64) -> bool {
    let Some(hb) = last_heartbeat_at else {
        return false; // absence ≠ stale (brief Q4)
    };
    let rfc3339 = time::format_description::well_known::Rfc3339;
    let (Ok(hb_t), Ok(now_t)) = (
        time::OffsetDateTime::parse(hb, &rfc3339),
        time::OffsetDateTime::parse(now, &rfc3339),
    ) else {
        return false; // can't compute age → never assert stale
    };
    (now_t - hb_t) > time::Duration::seconds(threshold_secs)
}

/// Recompute the set of stale sessions over `proj_session` from the (read-only) `conn` — the
/// staleness poller's deterministic `*_once` unit. A LIVE recompute (never persisted, never
/// event-folded): only **non-terminal** sessions are candidates (a terminal `failed`/`completed`/…
/// is never "stale"); a row whose `session_id` or `status` fails to parse is skipped (reject-unknown,
/// the recovery-scan precedent), never fabricated.
pub fn recompute_staleness_once(
    conn: &Connection,
    now: &str,
    threshold_secs: i64,
) -> rusqlite::Result<Vec<SessionId>> {
    let mut stmt =
        conn.prepare("SELECT session_id, status, last_heartbeat_at FROM proj_session")?;
    let rows: Vec<(String, String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut stale = Vec::new();
    for (sid, status_wire, last_hb) in rows {
        let Ok(session_id) = SessionId::parse(&sid) else {
            continue; // unparseable id → skip (never fabricate)
        };
        let Ok(status) = serde_json::from_value::<Session>(serde_json::Value::String(status_wire))
        else {
            continue; // reject-unknown status → skip
        };
        if status.is_terminal() {
            continue; // a terminal session is never "stale"
        }
        if is_stale(last_hb.as_deref(), now, threshold_secs) {
            stale.push(session_id);
        }
    }
    Ok(stale)
}

/// Spawn the WAL-checkpointer loop: every `interval`, run a bounded `wal_checkpoint(mode)` through
/// the write-actor (bounding the SQLite WAL — forbidden #3: the checkpoint rides the single writer,
/// never a rogue connection). A failed pass (the actor is gone) is logged + survived; the `shutdown`
/// watch breaking to `true` stops it cleanly. Mirrors [`spawn_drainer`](super::spawn_drainer).
pub fn spawn_wal_checkpointer(
    handle: WriteHandle,
    mode: WalCheckpointMode,
    interval: Duration,
    mut shutdown: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // skip (don't burst) ticks missed while a slow pass ran — a backlog must not flood the
        // single writer with back-to-back checkpoint commands.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased; // shutdown takes priority over a ready tick (deterministic, prompt stop).
                _ = shutdown.changed() => break,
                _ = ticker.tick() => match handle.wal_checkpoint(mode).await {
                    Ok(_) => {} // routine + quiet — a successful checkpoint is unremarkable.
                    // a failed pass is logged + survived — the loop keeps ticking (liveness).
                    Err(e) => eprintln!("nexusopsd: WAL checkpoint pass failed: {e}"),
                },
            }
        }
    })
}

/// Spawn the session-staleness poller loop: every `interval`, recompute staleness over the
/// **read-only WAL** (a fresh read snapshot per tick; never the write-actor, so it can NEVER
/// back-pressure the writer, forbidden #3). `clock` supplies `now` (injectable, §14). The
/// `open_read_only` + recompute is synchronous SQLite, so it runs **off the async runtime** via
/// `spawn_blocking` (LESSON §9 — a blocking SQLite call must not stall a tokio worker). A failed pass
/// (the db momentarily unavailable) is logged + survived; `shutdown` stops it cleanly.
pub fn spawn_staleness_poller(
    db_path: PathBuf,
    clock: Arc<dyn Clock>,
    threshold_secs: i64,
    interval: Duration,
    mut shutdown: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // log only when the stale COUNT changes (the drainer's log-on-change discipline): a
        // non-terminal `Stale` session would otherwise re-log identically on every tick.
        let mut last_count: Option<usize> = None;
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => break,
                _ = ticker.tick() => {
                    let db = db_path.clone();
                    let now = clock.now_rfc3339();
                    // the SQLite open + read is blocking → off the async runtime (LESSON §9). The
                    // closure returns a structural error string (no row data — §15-safe to log).
                    let pass = tokio::task::spawn_blocking(move || -> Result<usize, String> {
                        let conn = open_read_only(&db)
                            .map_err(|e| format!("read-only WAL open failed: {e}"))?;
                        recompute_staleness_once(&conn, &now, threshold_secs)
                            .map(|stale| stale.len())
                            .map_err(|e| format!("recompute failed: {e}"))
                    })
                    .await;
                    match pass {
                        Ok(Ok(count)) => {
                            if last_count != Some(count) {
                                if count > 0 {
                                    eprintln!("nexusopsd: {count} session(s) stale (heartbeat age > {threshold_secs}s)");
                                }
                                last_count = Some(count);
                            }
                        }
                        // a failed pass is logged + survived — the loop keeps ticking (liveness).
                        Ok(Err(msg)) => eprintln!("nexusopsd: staleness poller pass failed: {msg}"),
                        Err(join) => eprintln!("nexusopsd: staleness poller task failed: {join}"),
                    }
                }
            }
        }
    })
}
