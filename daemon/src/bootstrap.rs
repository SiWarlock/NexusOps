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

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType, Visibility};
use nexusops_shared::events::{DeviceRegistered, LocalRunnerRegistered};
use nexusops_shared::ids::WorkspaceId;
use nexusops_shared::ipc::{SUPPORTED_PROTOCOL_MAX, SUPPORTED_PROTOCOL_MIN};
use nexusops_shared::objects::{DeviceId, LocalRunnerId};
use nexusops_shared::CONTRACT_VERSION;

use crate::clock::Clock;
use crate::eventstore::{AppendIntent, EventStore, EventStoreError, Redactor};
use crate::idgen::IdGen;
use crate::locks::{PidLock, PidLockError};

/// the `DeviceRegistered` EventTypeRegistry name (§7.1). NOTE: the `"SessionStarted"`/these
/// event-type string literals dup across `bootstrap.rs` + the 1.2 projectors → a shared const
/// is a tracked Carry-forward consolidation (1.6c, next event-touching slice).
const DEVICE_REGISTERED: &str = "DeviceRegistered";
/// the `LocalRunnerRegistered` EventTypeRegistry name (§7.1).
const LOCAL_RUNNER_REGISTERED: &str = "LocalRunnerRegistered";

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
    /// the stable desktop-host Device id (register-if-absent across restarts, §5.3).
    pub device_id: DeviceId,
    /// the LocalRunner id minted for THIS daemon start (§5.3 minted-per-start).
    pub local_runner_id: LocalRunnerId,
}

impl DaemonContext {
    /// Consume the context into its parts for the runtime (1.6b): the held [`PidLock`] (the
    /// runtime MUST keep it bound for the daemon's lifetime — dropping it releases single-
    /// instance), the writable [`EventStore`] (→ the write-actor), and the version facts.
    /// `device_id`/`local_runner_id` are intentionally NOT in the tuple: they are durably
    /// recorded in the event log (and readable off [`DaemonContext`] before this handoff). A
    /// future consumer that needs the live runtime identity (e.g. Phase-3 session→runner
    /// binding) reads them off the context or re-queries via `first_event_of_type` (Step-9 TODO).
    pub fn into_parts(self) -> (PidLock, EventStore, DaemonVersionInfo) {
        (self._pidlock, self.store, self.version)
    }
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
    /// a registration event could not be (de)serialized — a corrupt stored registration
    /// payload (register-if-absent read) or an unencodable one. Fail-closed: the daemon does
    /// not start half-identified (§16).
    #[error("bootstrap registration error: {0}")]
    Registration(String),
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

    // (3) open + migrate + version-floor + catch-up replay (all inside open). Capture the
    // injected-clock `occurred_at` for the registration events BEFORE the clock moves into open.
    // Both registration events share this one bootstrap timestamp; their total order is the
    // canonical `seq` the store assigns (not `occurred_at`), so the shared value is fine (§7.1).
    let db_path = cfg.base_dir.join(DB_FILENAME);
    let occurred_at = cfg.clock.now_rfc3339();
    let mut store = EventStore::open(&db_path, cfg.idgen, cfg.clock, cfg.redactor)?;

    // (4) compose the report-only version facts (the enforcing floor is open's DB check).
    let version = DaemonVersionInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_range: (SUPPORTED_PROTOCOL_MIN, SUPPORTED_PROTOCOL_MAX),
        db_user_version: store.user_version()?,
        contract_version: CONTRACT_VERSION.to_string(),
    };

    // (5) register the desktop-host Device (register-if-absent) + a fresh LocalRunner (minted
    // per start) as System-actor system events (Option B, USER-RULED 2026-06-08): daemon self-
    // registration is the daemon establishing its OWN runtime identity — substrate, NOT a
    // policy-gated Gateway Action from an untrusted proposer (which is also circular pre-Phase-2).
    // Each is appended through `EventStore::append`, so the §15 redaction gate + the projector
    // fold (object_refs) both still run; INV-SEC-1 governs proposer intents, not the daemon's
    // own lifecycle events (the layer those Actions are recorded into).
    let device_id = register_device(&mut store, &occurred_at)?;
    let local_runner_id = register_local_runner(&mut store, &occurred_at)?;

    // (6) emit a loud audit-integrity event for any row startup replay quarantined (§17 Option C,
    // 1.6c L2). Emitted HERE (the caller, after open) — replay itself stays append-free; idempotent
    // via the quarantine record (`audit_emitted`) + the `audit-integrity-{seq}` idempotency_key.
    store.emit_quarantine_audit_events()?;

    Ok(DaemonContext {
        _pidlock: pidlock,
        store,
        version,
        device_id,
        local_runner_id,
    })
}

/// Register the desktop-host Device — register-if-absent (§5.3 stable host): reuse an existing
/// `DeviceRegistered`'s id if one is recorded, else mint a fresh `dev_` + append the event.
fn register_device(store: &mut EventStore, occurred_at: &str) -> Result<DeviceId, BootstrapError> {
    if let Some(env) = store.first_event_of_type(DEVICE_REGISTERED)? {
        let payload: DeviceRegistered = serde_json::from_str(&env.payload_json)
            .map_err(|e| BootstrapError::Registration(format!("corrupt DeviceRegistered: {e}")))?;
        return Ok(payload.device_id);
    }
    // runtime-fresh identity, read back from the payload on replay — deliberately NOT minted via
    // the injected IdGen, whose seam is scoped to golden-log byte-identical replay (event/outbox
    // ids only). Rebuild-equivalence (LESSON §4) holds regardless (ids are stored data).
    let device_id = DeviceId::new();
    let payload = serde_json::to_string(&DeviceRegistered {
        device_id: device_id.clone(),
    })
    .map_err(|e| BootstrapError::Registration(e.to_string()))?;
    store.append(system_intent(
        DEVICE_REGISTERED,
        device_id.as_str(),
        payload,
        occurred_at,
    ))?;
    Ok(device_id)
}

/// Register a fresh LocalRunner — minted per daemon start (§5.3): always a new `lr_` + event.
fn register_local_runner(
    store: &mut EventStore,
    occurred_at: &str,
) -> Result<LocalRunnerId, BootstrapError> {
    // runtime-fresh, minted per start (see `register_device` on the deliberate IdGen bypass).
    let local_runner_id = LocalRunnerId::new();
    let payload = serde_json::to_string(&LocalRunnerRegistered {
        local_runner_id: local_runner_id.clone(),
    })
    .map_err(|e| BootstrapError::Registration(e.to_string()))?;
    store.append(system_intent(
        LOCAL_RUNNER_REGISTERED,
        local_runner_id.as_str(),
        payload,
        occurred_at,
    ))?;
    Ok(local_runner_id)
}

/// Build a System-actor bootstrap-registration event intent (Option B): `actor_type=System`,
/// `source_type=local_daemon`, the reserved system-workspace sentinel ([`WorkspaceId::system`]),
/// `visibility=system`. `object_id` (the registered `dev_`/`lr_`) populates the audit
/// actor/source/correlation fields; the typed payload carries the identity the projector reads.
fn system_intent(
    event_type: &str,
    object_id: &str,
    payload_json: String,
    occurred_at: &str,
) -> AppendIntent {
    AppendIntent {
        event_type: event_type.to_string(),
        event_version: 1,
        occurred_at: occurred_at.to_string(),
        workspace_id: WorkspaceId::system(),
        actor_type: ActorType::System,
        actor_id: object_id.to_string(),
        source_type: SourceType::LocalDaemon,
        source_id: object_id.to_string(),
        correlation_id: object_id.to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json,
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: None,
        session_id: None,
        agent_team_id: None,
        visibility: Some(Visibility::System),
    }
}
