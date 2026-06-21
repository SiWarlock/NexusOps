//! P5.3a — the `execution_profiles` durable registry (DATA_MODEL §2.8, SOM §15; Option B, USER-DECIDED).
//!
//! The daemon's FIRST canonical OBJECT registry: the ROW is the source of truth (NOT a projection), written
//! atomically with an `ExecutionProfileRegistered` audit event (the LESSON-16 dual-gate — the event is the
//! trail). [`register_profile`] is the mutator; [`seed_default_profile`] is the cold-start register-if-absent
//! seed (System-actor, idempotent across restarts — the `register_device` precedent, LESSON 10); the
//! [`ProfileLookup`] seam is what `SessionExecutor` resolves against at session.create (§15 #8, fail-closed
//! on an unknown id).
//!
//! **Non-secret (5.3a):** `keychain_ref` is a §15 #4 POINTER, never a token. The keychain SECRET write, the
//! startup keychain self-test, and runtime `status` re-derivation (§7.2) are 5.3b (their own commit).

use std::path::PathBuf;

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType, Visibility};
use nexusops_shared::events::ExecutionProfileRegistered;
use nexusops_shared::ids::{ExecutionProfileId, WorkspaceId};
use nexusops_shared::status::ExecutionProfile;
use nexusops_shared::time::Timestamp;

use crate::eventstore::{open_read_only, AppendIntent, EventStore, EventStoreError};

/// The seeded default profile's provider/harness (Q2: claude-first — the MVP cat-4 PTY-primary Claude).
const DEFAULT_PROVIDER: &str = "anthropic";
const DEFAULT_HARNESS: &str = "claude_code";

/// Typed profile-registry failures — fail-closed.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    /// a store/txn write failure (incl. the §14 injected `RegistryEventWrite` audit-write fault).
    #[error("execution-profile store error: {0}")]
    Store(#[from] EventStoreError),
    /// a profile payload could not be (de)serialized (an unencodable spec, or a corrupt stored seed).
    #[error("execution-profile encode error: {0}")]
    Encode(String),
}

/// The non-secret config one profile registration carries (DATA_MODEL §2.8). `keychain_ref` is a §15 #4
/// POINTER (the keychain entry name/ref), NEVER the token — the secret WRITE is 5.3b.
#[derive(Clone, Debug)]
pub struct ProfileSpec {
    pub workspace_id: WorkspaceId,
    pub provider: String,
    pub harness: String,
    pub model: Option<String>,
    pub account_alias: Option<String>,
    pub keychain_ref: Option<String>,
    pub usage_policy_json: Option<String>,
    pub status: ExecutionProfile,
}

/// The cold-start default profile (Q2). A usable claude-first profile whose SECRET/account binding lands
/// in 5.3b — non-secret here (`keychain_ref = None`).
fn default_spec() -> ProfileSpec {
    ProfileSpec {
        workspace_id: WorkspaceId::system(),
        provider: DEFAULT_PROVIDER.to_string(),
        harness: DEFAULT_HARNESS.to_string(),
        model: None,
        account_alias: None,
        keychain_ref: None,
        usage_policy_json: None,
        status: ExecutionProfile::Available,
    }
}

/// The snake_case wire string of a frozen §5.1 [`ExecutionProfile`] (serde `rename_all`); the fallback is
/// unreachable for a unit enum but avoids an unwrap (the `Unknown` wire value is itself a valid state).
fn status_wire(status: &ExecutionProfile) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Register one execution profile: write the canonical `execution_profiles` row AND append the
/// `ExecutionProfileRegistered` audit event ATOMICALLY (the LESSON-16 dual-gate over [`EventStore::gateway_txn`]),
/// §15-redacting the row payload BEFORE the INSERT (§15 #4 defense-in-depth). Fail-closed: an event-write
/// fault (or an unredactable payload) rolls back the whole txn → NO row persists (the row is durable only if
/// its audit trail is).
pub fn register_profile(
    store: &mut EventStore,
    id: &ExecutionProfileId,
    spec: &ProfileSpec,
    occurred_at: &str,
) -> Result<(), ProfileError> {
    let created_at = Timestamp::parse(occurred_at)
        .map_err(|e| ProfileError::Encode(format!("created_at: {e}")))?;
    let payload = ExecutionProfileRegistered {
        execution_profile_id: id.clone(),
        workspace_id: spec.workspace_id.clone(),
        provider: spec.provider.clone(),
        harness: spec.harness.clone(),
        model: spec.model.clone(),
        account_alias: spec.account_alias.clone(),
        keychain_ref: spec.keychain_ref.clone(),
        usage_policy_json: spec.usage_policy_json.clone(),
        status: spec.status,
        created_at,
    };
    let raw_json =
        serde_json::to_string(&payload).map_err(|e| ProfileError::Encode(e.to_string()))?;

    store.gateway_txn(|gtx| {
        // §15 #4 dual-gate: redact the row payload BEFORE the INSERT (the canonical Redactor; an
        // unredactable payload is REFUSED → the whole register fails closed). The event append below
        // re-redacts through the same gate (idempotent on already-redacted text).
        let redacted_json = gtx.redact_row(&raw_json)?;
        // re-hydrate the REDACTED payload into the typed struct, then bind its fields into the row INSERT
        // below — so the row stores the masked values, never the raw input. COUPLING: this round-trip is
        // safe only because every `ExecutionProfileRegistered` field is also a DDL column AND its
        // `Option`/`skip_serializing_if` fields carry `#[serde(default)]` (a skipped None re-hydrates to
        // None). A future event field WITHOUT a matching DDL column (or without `default`) would break this
        // bind — keep the struct's field set and the `execution_profiles` columns in lock-step.
        let row: ExecutionProfileRegistered = serde_json::from_str(&redacted_json)
            .map_err(|e| ProfileError::Encode(e.to_string()))?;
        // the canonical row (the source of truth) — bind the REDACTED values.
        gtx.tx()
            .execute(
                "INSERT INTO execution_profiles (execution_profile_id, workspace_id, provider, harness, \
                 model, account_alias, keychain_ref, usage_policy_json, status, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    row.execution_profile_id.as_str(),
                    row.workspace_id.as_str(),
                    row.provider,
                    row.harness,
                    row.model,
                    row.account_alias,
                    row.keychain_ref,
                    row.usage_policy_json,
                    status_wire(&row.status),
                    row.created_at.as_str(),
                ],
            )
            .map_err(EventStoreError::Write)?;
        // the audit TRAIL — appended in the SAME txn (atomic; an audit-write fault rolls back the row too).
        gtx.append(&registered_intent(
            id,
            &redacted_json,
            occurred_at,
            &spec.workspace_id,
        ))?;
        Ok::<(), ProfileError>(())
    })
}

/// Cold-start the ONE default profile (register-if-absent, sub-decision 1): if an `ExecutionProfileRegistered`
/// is already recorded, REUSE its id (no duplicate row/event); else mint a fresh `prof_` + register the
/// default. Idempotent across restarts (the `register_device` precedent, LESSON 10).
pub fn seed_default_profile(
    store: &mut EventStore,
    occurred_at: &str,
) -> Result<ExecutionProfileId, ProfileError> {
    if let Some(env) = store.first_event_of_type(ExecutionProfileRegistered::EVENT_TYPE)? {
        let payload: ExecutionProfileRegistered =
            serde_json::from_str(&env.payload_json).map_err(|e| {
                ProfileError::Encode(format!("corrupt ExecutionProfileRegistered: {e}"))
            })?;
        return Ok(payload.execution_profile_id);
    }
    let id = ExecutionProfileId::new();
    register_profile(store, &id, &default_spec(), occurred_at)?;
    Ok(id)
}

/// Build the System-actor registration event intent (mirrors `bootstrap::system_intent`): the durable-
/// registry register-mutator is the daemon's OWN substrate write — `actor_type=System`, the reserved
/// system workspace (for the seed), NOT a policy-gated Gateway Action (INV-SEC-1 governs proposer intents,
/// not the daemon's lifecycle/registry events).
fn registered_intent(
    id: &ExecutionProfileId,
    payload_json: &str,
    occurred_at: &str,
    workspace_id: &WorkspaceId,
) -> AppendIntent {
    AppendIntent {
        event_type: ExecutionProfileRegistered::EVENT_TYPE.to_string(),
        event_version: 1,
        occurred_at: occurred_at.to_string(),
        workspace_id: workspace_id.clone(),
        actor_type: ActorType::System,
        actor_id: id.as_str().to_string(),
        source_type: SourceType::LocalDaemon,
        source_id: id.as_str().to_string(),
        correlation_id: id.as_str().to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json: payload_json.to_string(),
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: None,
        session_id: None,
        agent_team_id: None,
        visibility: Some(Visibility::System),
        // a registry register-mutator is not a gateway action — no FK edges.
        action_request_id: None,
        approval_id: None,
        causation_id: None,
    }
}

/// The registry read seam the `SessionExecutor` resolves an `ExecutionProfile` against at session.create
/// (§15 #8). Injected so resolution is deterministically unit-testable (a fake in tests; the sqlite-backed
/// reader in production).
pub trait ProfileLookup: Send + Sync {
    /// The seeded default profile id — the `None` (no-profile-requested) resolve path.
    fn default_id(&self) -> Result<ExecutionProfileId, ProfileError>;
    /// Whether `id` is a registered profile — the fail-closed-on-unknown gate (§15 #8 no-account-hop).
    fn exists(&self, id: &ExecutionProfileId) -> Result<bool, ProfileError>;
}

/// The production [`ProfileLookup`]: reads the canonical `execution_profiles` table over a fresh read-only
/// WAL connection (the executor runs on the write-actor; a 2nd READ connection is single-writer-safe —
/// forbidden #3 governs WRITES). The default id is DETERMINED at cold-start (the seeded
/// `ExecutionProfileRegistered`, stable across restarts) and passed at construction, not re-derived per call.
pub struct SqliteProfileLookup {
    db_path: PathBuf,
    default_id: ExecutionProfileId,
}

impl SqliteProfileLookup {
    pub fn new(db_path: PathBuf, default_id: ExecutionProfileId) -> Self {
        Self {
            db_path,
            default_id,
        }
    }
}

impl ProfileLookup for SqliteProfileLookup {
    fn default_id(&self) -> Result<ExecutionProfileId, ProfileError> {
        Ok(self.default_id.clone())
    }

    fn exists(&self, id: &ExecutionProfileId) -> Result<bool, ProfileError> {
        let conn = open_read_only(&self.db_path)?;
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM execution_profiles WHERE execution_profile_id = ?1",
                rusqlite::params![id.as_str()],
                |r| r.get(0),
            )
            .map_err(EventStoreError::Write)?;
        Ok(n > 0)
    }
}
