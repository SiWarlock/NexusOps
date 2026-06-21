//! Forward-only `user_version` migrations + backup/rollback + version-compat
//! floor (§16). Raw events are the irreplaceable spine — a migration that raises
//! `user_version` on a db that already holds data backs the file up first and
//! restores it on failure.

use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::eventstore::{schema, EventStoreError};

/// Highest migration index this binary understands. A db whose `user_version`
/// exceeds this was written by a newer binary → refuse-safe (§16).
pub const SUPPORTED_USER_VERSION: i64 = 16;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(schema::MIGRATION_1_EVENTS),
        M::up(schema::MIGRATION_2_REDACTION),
        M::up(schema::MIGRATION_3_PROJECTIONS),
        M::up(schema::MIGRATION_4_OUTBOX),
        M::up(schema::MIGRATION_5_LEASES),
        M::up(schema::MIGRATION_6_QUARANTINE),
        M::up(schema::MIGRATION_7_GATEWAY),
        M::up(schema::MIGRATION_8_PLANS),
        M::up(schema::MIGRATION_9_POLICY_DECISION),
        M::up(schema::MIGRATION_10_PROJECT_REGISTRY),
        M::up(schema::MIGRATION_11_INTEGRATION_CONNECTIONS),
        M::up(schema::MIGRATION_12_SESSION_RECOVERY),
        M::up(schema::MIGRATION_13_PULL_REQUEST_MERGEABLE_CHECKS),
        M::up(schema::MIGRATION_14_REVIEW),
        M::up(schema::MIGRATION_15_PULL_REQUEST_DIFF_STATS),
        M::up(schema::MIGRATION_16_EXECUTION_PROFILES),
    ])
}

pub fn current_user_version(conn: &Connection) -> Result<i64, EventStoreError> {
    conn.pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(EventStoreError::Write)
}

/// `nexusops.db` → `nexusops.db.bak-<from>` (pre-migration snapshot). Checkpoints
/// the WAL first so the copied main file holds all committed data (WAL-safe).
pub fn backup_db(path: &Path, from: i64) -> Result<(), EventStoreError> {
    {
        let conn = Connection::open(path).map_err(EventStoreError::Write)?;
        conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")
            .map_err(EventStoreError::Write)?;
    } // drop conn before copying
    std::fs::copy(path, backup_path(path, from)).map_err(EventStoreError::Io)?;
    Ok(())
}

/// restore `nexusops.db.bak-<from>` → `nexusops.db` (migration-failure rollback).
/// Removes any stale `-wal`/`-shm` sidecars so SQLite cannot replay post-backup
/// WAL frames over the restored main file.
pub fn restore_db(path: &Path, from: i64) -> Result<(), EventStoreError> {
    std::fs::copy(backup_path(path, from), path).map_err(EventStoreError::Io)?;
    for ext in ["-wal", "-shm"] {
        let mut side = path.as_os_str().to_owned();
        side.push(ext);
        let _ = std::fs::remove_file(std::path::PathBuf::from(side)); // best-effort
    }
    Ok(())
}

fn backup_path(path: &Path, from: i64) -> std::path::PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(format!(".bak-{from}"));
    std::path::PathBuf::from(p)
}

/// Run forward-only migrations to latest. Before raising a db that already holds
/// data (`from >= 1`, i.e. a real upgrade like 1→2), back the file up first and
/// restore it on failure — the raw event log is irreplaceable (§16). A fresh db
/// (`from == 0`) has nothing to protect.
pub fn run(path: &Path, conn: &mut Connection) -> Result<(), EventStoreError> {
    let from = current_user_version(conn)?;
    let backed_up = (1..SUPPORTED_USER_VERSION).contains(&from);
    if backed_up {
        backup_db(path, from)?;
    }
    match migrations().to_latest(conn) {
        Ok(()) => Ok(()),
        Err(e) => {
            let mig_err = e.to_string();
            if backed_up {
                // attempt the in-place restore; the caller drops the conn on the Err
                // return so the next open() reads the restored file. The result is
                // CLASSIFIED (never swallowed): a clean rollback renders "rolled back to
                // vN"; a failed restore surfaces a distinct RestoreFailed (1.1 L3 / §16).
                Err(on_migration_failure(restore_db(path, from), from, &mig_err))
            } else {
                // a fresh db (from == 0) had nothing to back up — nothing to restore.
                Err(EventStoreError::Migration(mig_err))
            }
        }
    }
}

/// Classify the outcome AFTER a migration failed on a backed-up db (§16). A clean rollback
/// keeps the failure as a `Migration` error but renders the "rolled back to vN" UX; a
/// rollback that ITSELF failed surfaces a DISTINCT `RestoreFailed` carrying `from` — the
/// restore failure is never the swallowed `let _ = restore_db(..)` of before (1.1 L3).
fn on_migration_failure(
    restore: Result<(), EventStoreError>,
    from: i64,
    mig_err: &str,
) -> EventStoreError {
    match restore {
        Ok(()) => EventStoreError::Migration(format!(
            "migration failed; rolled back to v{from}: {mig_err}"
        )),
        Err(source) => EventStoreError::RestoreFailed {
            from,
            // preserve WHY the migration failed — a RestoreFailed must not lose the
            // original cause that triggered the (then-failed) rollback.
            migration_error: mig_err.to_string(),
            source: Box::new(source),
        },
    }
}

/// §16 version floor: a db newer than this binary understands is refused.
pub fn refuses_db_newer_than_supported(conn: &Connection) -> Result<(), EventStoreError> {
    let v = current_user_version(conn)?;
    if v > SUPPORTED_USER_VERSION {
        Err(EventStoreError::DbNewerThanSupported {
            db: v,
            supported: SUPPORTED_USER_VERSION,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // a genuinely-internal helper the public API can't reach: the post-migration-failure
    // classifier that decides whether the rollback was clean (renderable "rolled back to
    // vN") or itself failed (a DISTINCT, never-swallowed RestoreFailed). 1.1 L3 / §16.
    use super::on_migration_failure;
    use crate::eventstore::EventStoreError;

    #[test]
    fn restore_success_maps_to_renderable_rolled_back() {
        // migration failed, .bak rollback SUCCEEDED → the migration is still what failed
        // (Migration family) but the message renders the §16 "rolled back to vN" UX.
        let err = on_migration_failure(Ok(()), 3, "M4 boom");
        match err {
            EventStoreError::Migration(msg) => assert!(
                msg.contains("rolled back to v3"),
                "successful rollback renders the §16 'rolled back to vN' UX: {msg}"
            ),
            other => panic!("expected Migration (clean rollback), got {other:?}"),
        }
    }

    #[test]
    fn restore_failure_is_typed_not_swallowed() {
        // migration failed AND the rollback could NOT run → a DISTINCT typed RestoreFailed
        // carrying `from` (1.1 L3), never the swallowed `let _ = restore_db(..)` of before.
        let src = EventStoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no .bak to restore",
        ));
        let err = on_migration_failure(Err(src), 2, "M3 boom");
        assert!(
            matches!(
                &err,
                EventStoreError::RestoreFailed { from: 2, migration_error, .. }
                    if migration_error.contains("M3 boom")
            ),
            "a failed restore is typed RestoreFailed, carrying `from` AND the original \
             migration error (never losing why it failed): {err:?}"
        );
    }
}
