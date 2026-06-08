//! Cold-start bootstrap orchestration (§16).
//!
//! [`cold_start`] runs the daemon's first-run / restart startup in §16-binding order —
//! acquire the single-instance pidlock → create the app-support dir → open+migrate the
//! event store → (L3: register the desktop-host Device + a fresh LocalRunner) → return an
//! initialized [`DaemonContext`] the 1.6c runtime drives. It **composes** the already-
//! shipped primitives ([`crate::locks::PidLock::acquire`], [`EventStore::open`]) — the DB
//! lifecycle (migrate / version-floor / catch-up replay) all happens inside `open`; this
//! layer does not re-implement it.
//!
//! Stale-socket reclaim, the UDS `bind()` + accept-loop, and the Tokio task spawns are 1.6c
//! (they consume the `DaemonContext`); the §17 degradable replay is 1.6b (it refines `open`'s
//! read path — bootstrap is agnostic to which strategy `open` uses).

use std::path::PathBuf;

use nexusops_shared::ipc::{SUPPORTED_PROTOCOL_MAX, SUPPORTED_PROTOCOL_MIN};
use nexusops_shared::CONTRACT_VERSION;

use crate::clock::Clock;
use crate::eventstore::{EventStore, EventStoreError, Redactor};
use crate::idgen::IdGen;
use crate::locks::{PidLock, PidLockError};

/// The daemon's event-store DB filename within the app-support dir.
pub const DB_FILENAME: &str = "nexusops.db";
/// The single-instance pidlock filename within the app-support dir.
pub const PID_FILENAME: &str = "daemon.pid";

/// Injected cold-start inputs (§14 seam): the app-support base dir + the `Clock`/`IdGen`/
/// `Redactor` the [`EventStore`] takes. Tests inject a tempdir base so no real `~/Library`
/// path is touched; production resolves the macOS Application Support dir (1.6c wires the
/// real-path resolver into `main.rs`).
pub struct BootstrapConfig {
    pub base_dir: PathBuf,
    pub idgen: Box<dyn IdGen>,
    pub clock: Box<dyn Clock>,
    pub redactor: Box<dyn Redactor>,
}

/// Version facts composed at cold-start for the handshake / diagnostics (§16 version-compat).
/// **Report-only:** the ENFORCING floor is the DB `user_version` check inside
/// [`EventStore::open`] (`refuses_db_newer_than_supported`); the protocol range is enforced
/// at the 1.5 handshake. The agent-CLI/SDK tuple + sidecar-MCP `initialize` rows of the §16
/// matrix are §9.1/§13.1 — deferred (named, not built here).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonVersionInfo {
    pub app_version: String,
    pub protocol_range: (u32, u32),
    pub db_user_version: u32,
    pub contract_version: String,
}

/// The initialized daemon runtime context the 1.6c runtime drives. Holds the live
/// [`PidLock`] (kept alive for the daemon lifetime — `Drop` releases the OS advisory lock,
/// even on crash) + the single-writer [`EventStore`] + the composed version facts.
pub struct DaemonContext {
    // the OS advisory lock is released when this field drops — single-instance for the
    // daemon's lifetime (§16). Held for its Drop, never read.
    _pidlock: PidLock,
    pub store: EventStore,
    pub version: DaemonVersionInfo,
}

/// Typed cold-start failures — fail-closed (§16): a start that cannot prove single-instance
/// AND a sound DB returns an error, never a half-initialized context.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    /// another daemon instance already holds the single-instance lock (§16 / forbidden #3).
    #[error("another daemon instance is already running")]
    AlreadyRunning,
    /// the app-support dir could not be created.
    #[error("bootstrap io error: {0}")]
    Io(std::io::Error),
    /// the event store could not be opened/migrated — incl. the §16 DB-newer refusal
    /// ([`EventStoreError::DbNewerThanSupported`]) and a failed migration rollback.
    #[error("bootstrap event-store error: {0}")]
    Store(#[from] EventStoreError),
}

impl From<PidLockError> for BootstrapError {
    fn from(e: PidLockError) -> Self {
        match e {
            // a held lock is a clean single-instance refusal (NOT an error to surface raw).
            PidLockError::AlreadyHeld => BootstrapError::AlreadyRunning,
            // an IO failure to acquire fails closed — never treated as "started".
            PidLockError::Io(io) => BootstrapError::Io(io),
        }
    }
}

/// Cold-start the daemon in §16-binding order.
///
/// **Ordering (forbidden #3 / single-writer):** `create_dir_all` is the idempotent,
/// race-safe prerequisite for the lock file's home — two racing instances both succeed at
/// it (it touches no DB). The [`PidLock`] is then the FIRST step that can REFUSE a second
/// instance, and it STRICTLY PRECEDES [`EventStore::open`] (the migrate/write step) — so a
/// second instance can never reach a concurrent migration (the corruption risk forbidden #3
/// guards). Device + LocalRunner registration is L3 (held on the INV-SEC-1 ruling).
pub fn cold_start(cfg: BootstrapConfig) -> Result<DaemonContext, BootstrapError> {
    // (1) prerequisite: the app-support dir must exist for the lock + DB to live in it.
    // Idempotent (exists-ok), race-safe, touches no DB.
    std::fs::create_dir_all(&cfg.base_dir).map_err(BootstrapError::Io)?;

    // (2) single-instance gate — BEFORE any DB open, so only one instance ever migrates.
    let pidlock = PidLock::acquire(&cfg.base_dir.join(PID_FILENAME))?;

    // (3) open + migrate + version-floor + catch-up replay (all inside open).
    let db_path = cfg.base_dir.join(DB_FILENAME);
    let store = EventStore::open(&db_path, cfg.idgen, cfg.clock, cfg.redactor)?;

    // (4) compose the report-only version facts (the enforcing floor is open's DB check).
    let version = DaemonVersionInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_range: (SUPPORTED_PROTOCOL_MIN, SUPPORTED_PROTOCOL_MAX),
        db_user_version: store.user_version()?,
        contract_version: CONTRACT_VERSION.to_string(),
    };

    // (5) Device + LocalRunner registration → L3 (held on the lead/user INV-SEC-1 ruling).

    Ok(DaemonContext {
        _pidlock: pidlock,
        store,
        version,
    })
}
