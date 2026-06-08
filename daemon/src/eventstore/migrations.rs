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
pub const SUPPORTED_USER_VERSION: i64 = 5;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(schema::MIGRATION_1_EVENTS),
        M::up(schema::MIGRATION_2_REDACTION),
        M::up(schema::MIGRATION_3_PROJECTIONS),
        M::up(schema::MIGRATION_4_OUTBOX),
        M::up(schema::MIGRATION_5_LEASES),
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
            // best-effort in-place restore; the caller drops the conn on the Err
            // return so the next open() reads the restored file.
            if backed_up {
                let _ = restore_db(path, from);
            }
            Err(EventStoreError::Migration(e.to_string()))
        }
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
