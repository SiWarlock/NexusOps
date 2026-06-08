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
pub const SUPPORTED_USER_VERSION: i64 = 1;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(schema::MIGRATION_1_EVENTS)])
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

/// Run forward-only migrations to latest.
///
/// 1.1 has only the 0→1 migration (a fresh db has no irreplaceable data to
/// protect), so this runs `to_latest` directly. The **auto-backup-before-raise +
/// restore-on-failure** integrates here when migration 2 lands (L3) — wrapping
/// this with [`backup_db`]/[`restore_db`] (implemented + directly tested now)
/// for the first `from >= 1` raise. (§16)
pub fn run(conn: &mut Connection) -> Result<(), EventStoreError> {
    migrations()
        .to_latest(conn)
        .map_err(|e| EventStoreError::Migration(e.to_string()))
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
