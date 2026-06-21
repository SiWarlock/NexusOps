//! `integrations/connections.rs` (P4.7/083) — the production [`ConnectionLookup`] read seam.
//!
//! Holds the sqlite-backed registered-connection gate for `integration.set_live_writes`. It lives in a
//! SIBLING module to `connect.rs` (the executor) ON PURPOSE: the executor module must name NO DB surface
//! (`rusqlite`/`INSERT`) — the INV-SEC-1 no-second-mutator structural guard (`integration_connect.rs`
//! Test 6) greps `connect.rs` for those tokens. This is a READ-only WAL query (forbidden #3 governs
//! WRITES); the `SqliteProfileLookup`-in-`profiles/` precedent.

use std::path::PathBuf;

use nexusops_shared::events::Provider;

use crate::eventstore::{open_read_only, EventStoreError};
use crate::integrations::auth::LiveWritesGate;
use crate::integrations::connect::ConnectionLookup;

/// The production [`ConnectionLookup`]: reads `proj_integration_connection` over a fresh read-only WAL
/// connection (the executor runs on the write-actor's std::thread; a 2nd READ connection is
/// single-writer-safe — forbidden #3 governs WRITES; the `SqliteProfileLookup` precedent).
pub struct SqliteConnectionLookup {
    db_path: PathBuf,
}

impl SqliteConnectionLookup {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

impl ConnectionLookup for SqliteConnectionLookup {
    fn is_registered(&self, connection_id: &str) -> Result<bool, EventStoreError> {
        let conn = open_read_only(&self.db_path)?;
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM proj_integration_connection WHERE connection_id = ?1",
                rusqlite::params![connection_id],
                |r| r.get(0),
            )
            .map_err(EventStoreError::Write)?;
        Ok(n > 0)
    }
}

/// The provider's stored wire value (matches the projector's `wire_value(&Provider)` — the closed enum).
fn provider_wire(provider: Provider) -> &'static str {
    match provider {
        Provider::Github => "github",
        Provider::Linear => "linear",
    }
}

/// The production [`LiveWritesGate`]: reads `proj_integration_connection.live_writes_enabled` for the
/// connection keyed by `(provider, account)` over a fresh read-only WAL connection. **Fail-closed by
/// construction:** a connect-open fault, a missing connection row, OR a read error ALL yield `false`
/// (never default-ON; the authed client then stays unauth — the existing 401/AuthFailed path).
pub struct SqliteLiveWritesGate {
    db_path: PathBuf,
}

impl SqliteLiveWritesGate {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

impl LiveWritesGate for SqliteLiveWritesGate {
    fn is_enabled_for_account(&self, provider: Provider, account: &str) -> bool {
        let Ok(conn) = open_read_only(&self.db_path) else {
            return false; // can't open → fail-closed (unauth)
        };
        // NOTE — `account` is a NULLABLE column. SQL `NULL = ?2` evaluates to NULL (never TRUE), so a
        // connection registered WITHOUT an account can NEVER match this gate → it is permanently
        // fail-closed (unauth), regardless of its toggle. That is the intended posture: per-account token
        // selection requires a concrete account (the github clients only ever call with a non-empty owner;
        // the gh-reuse acquisition + a github connection always carry the account). A future account-less
        // live-auth path would need its own keying, not this gate.
        conn.query_row(
            "SELECT live_writes_enabled FROM proj_integration_connection \
             WHERE provider = ?1 AND account = ?2",
            rusqlite::params![provider_wire(provider), account],
            |r| r.get::<_, i64>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(false) // no row (unknown / NULL-account connection) / read fault → fail-closed (unauth)
    }
}
