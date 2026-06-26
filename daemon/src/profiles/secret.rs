//! P5.3b — the execution-profile SECRET surface (cat-1 / secret-touching) + the binding-hardening
//! classifiers (§15 #4/#7/#8, §5.1, §7.2).
//!
//! **The inbound-secret posture (the ⚠️ NEW POSTURE, 085):** unlike 083 (a daemon-sourced `gh auth
//! token`), a user-typed profile credential arrives INBOUND over the getpeereid-authed local UDS
//! (the `profile.set_secret` trigger). [`write_profile_secret`] holds it in [`Zeroizing`] and drops it
//! immediately after the keychain write; ONLY the [`profile_keychain_ref`] POINTER flows downstream
//! (§15 #4 — the secret never enters an event, a row, or a log; LESSON §64 no-echo). The local-trust-
//! boundary (the secret was already on the user's machine; the peer is uid-checked) is the
//! acceptability basis, mirroring 083's ruling on its own merits.
//!
//! The deterministic core ([`profile_keychain_ref`] / [`write_profile_secret`] / [`self_test_status`] /
//! [`derive_runtime_status`]) is unit-tested via the injected [`crate::integrations::keychain::FakeSecretStore`]
//! seam (`tests/profile_secret.rs` / `tests/profile_self_test.rs` / `tests/profile_runtime_status.rs`);
//! the live keychain round-trip + the live session/telemetry inputs are the non-deterministic edges.

use zeroize::Zeroizing;

use nexusops_shared::ids::ExecutionProfileId;
use nexusops_shared::ipc::SetProfileSecretResult;
use nexusops_shared::status::ExecutionProfile;

use crate::eventstore::{EventStoreError, GatewayTxn};
use crate::integrations::keychain::{SecretStore, SecretStoreError};

/// The per-profile keychain POINTER (§15 #4): `nexusops/profile/<prof_id>`. **Daemon-derived, id-keyed,
/// stable** — NEVER proposer-supplied (the confused-deputy pin, LESSON §63: a profile's secret entry is
/// bound to the AUDITED profile id, never to `inputs`). A POINTER only — never the token. Id-keyed (vs the
/// 083 account-keyed `keychain_ref_for`) because profiles are id-scoped, integrations account-scoped.
pub fn profile_keychain_ref(id: &ExecutionProfileId) -> String {
    format!("nexusops/profile/{}", id.as_str())
}

/// Write the INBOUND profile secret to the OS keychain under the daemon-derived [`profile_keychain_ref`]
/// and return ONLY the `keychain_ref` POINTER (§15 #4 / LESSON §64 no-echo — the secret NEVER appears in
/// the result, an event, a row, or a log). The `secret` rides [`Zeroizing`] BY VALUE → its heap
/// allocation is scrubbed when this scope ends (after the keychain write), so the plaintext is not left
/// in freed memory. A backend fault surfaces a STRUCTURAL [`SecretStoreError`] (no secret in the message).
///
/// This core does NOT validate the profile id is registered — the IPC trigger
/// ([`crate::ipc::methods::set_profile_secret`]) runs the fail-closed-on-unknown gate FIRST (LESSON §62),
/// so a secret is never written for an unregistered profile on the production path.
pub fn write_profile_secret(
    store: &dyn SecretStore,
    id: &ExecutionProfileId,
    secret: Zeroizing<String>,
) -> Result<SetProfileSecretResult, SecretStoreError> {
    let keychain_ref = profile_keychain_ref(id);
    // the secret crosses into the keychain ONCE; `secret` (Zeroizing) drops at the end of this scope.
    store.store(&keychain_ref, &secret)?;
    Ok(SetProfileSecretResult { keychain_ref })
}

/// C3 — record the §15 #4 keychain POINTER onto the CANONICAL `execution_profiles` row, INSIDE the gateway's
/// txn-B (called from the pipeline ATOMIC with appending the `ProfileSecretSet` audit event; a write fault
/// rolls BOTH back → fail-closed, no half-written keychain_ref; §15 #5 / LESSON §16). `execution_profiles` is
/// the CANONICAL source of truth (NOT a projection — LESSON §62), so the pointer is recorded by a direct
/// UPDATE here, not a projector fold (the SQL lives in `profiles/`; the pipeline just dispatches). Defense-in-
/// depth: the pointer is routed through the §15 redact gate (a daemon-derived `nexusops/profile/<id>` pointer
/// is non-secret → a no-op in practice, but the dual-gate discipline holds — the `register_profile` precedent).
/// `keychain_ref` is daemon-derived (re-derived by the `ProfileExecutor` from the AUDITED resource_ref id),
/// daemon-DERIVED from the audited id (`profile_keychain_ref(id)`), NEVER proposer-supplied (confused-deputy-
/// safe §63 — the id is the only input). An UPDATE of a missing row is a healthy 0-row no-op (the executor's
/// fail-closed-on-unknown gate already refused an unregistered profile before emitting).
pub fn apply_secret_set(gtx: &GatewayTxn, id: &ExecutionProfileId) -> Result<(), EventStoreError> {
    // the POINTER is recomputed from the id (deterministic) — it is NOT carried on the event payload.
    let keychain_ref = profile_keychain_ref(id);
    let redacted_ref = gtx.redact_row(&keychain_ref)?;
    gtx.tx()
        .execute(
            "UPDATE execution_profiles SET keychain_ref = ?1 WHERE execution_profile_id = ?2",
            rusqlite::params![redacted_ref, id.as_str()],
        )
        .map_err(EventStoreError::Write)?;
    Ok(())
}

/// The outcome of reading a profile's keychain entry during the startup self-test (piece 2). Modeled as
/// a typed input to the pure [`self_test_status`] classifier so the §5.1 mapping is unit-testable without
/// a real keychain.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeychainReadOutcome {
    /// the keychain entry resolved (the secret is present).
    Resolved,
    /// the keychain entry did NOT resolve (`SecretStore::read` → `Ok(None)`) — a configured-but-missing secret.
    Unresolved,
    /// the keychain backend faulted (`SecretStore::read` → `Err`).
    BackendFault,
}

/// Piece 2 — the pure startup keychain self-test classifier (§5.1; the LESSON §36/§41 pure-classifier
/// family). A profile with NO configured secret (`keychain_ref` IS `None`) is **ambient-auth** (the seeded
/// default) → NOT misconfigured (`None` here = "the runtime self-test contributes nothing"). A profile WITH
/// a `keychain_ref` whose entry does not resolve, or whose read faults, is **`misconfigured`** (§5.1).
/// Returns `Some(Misconfigured)` only — a healthy resolve returns `None` (the self-test asserts no problem;
/// the runtime status then derives from the live session/telemetry inputs, [`derive_runtime_status`]).
pub fn self_test_status(
    has_keychain_ref: bool,
    read_outcome: Option<KeychainReadOutcome>,
) -> Option<ExecutionProfile> {
    match (has_keychain_ref, read_outcome) {
        // ambient-auth (no configured secret) → the self-test contributes nothing (NOT misconfigured).
        (false, _) => None,
        // a configured secret that resolves → healthy (the self-test sees no problem).
        (true, Some(KeychainReadOutcome::Resolved)) => None,
        // a configured secret that does NOT resolve, or whose read faults → misconfigured (§5.1).
        (true, Some(KeychainReadOutcome::Unresolved | KeychainReadOutcome::BackendFault)) => {
            Some(ExecutionProfile::Misconfigured)
        }
        // a configured ref but the self-test wasn't run (no outcome) → contributes nothing (conservative).
        (true, None) => None,
    }
}

/// The live inputs the §7.2 runtime-status re-derivation reads — recomputed each read (NOT trusted from the
/// persisted row). A profile's RUNTIME status is a function of these live signals, never the stored config
/// `status` alone (the LESSON §41/§47 live-read-recompute precedent; the worktree-status axis pattern).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct RuntimeInputs {
    /// the piece-2 keychain self-test verdict (`Some(Misconfigured)` when a configured secret is
    /// unreadable; `None` when healthy / ambient-auth).
    pub self_test: Option<ExecutionProfile>,
    /// a live session is currently bound to this profile (drives `InUse`).
    pub in_use: bool,
    /// the adapter reported a soft, auto-recovering throttle (drives `RateLimited`).
    pub rate_limited: bool,
    /// the adapter reported an auth failure (drives `AuthExpired`).
    pub auth_expired: bool,
}

/// Piece 3 — the pure §7.2 runtime-status re-derivation (the LESSON §36/§41 pure-classifier family). The
/// RUNTIME status is RE-DERIVED from the live [`RuntimeInputs`] in **strict precedence**, NOT trusted from
/// the persisted `config_status` (which is the CONFIG/intended state). A terminal config (`Disabled`) is
/// ALWAYS honored (a disabled profile is disabled regardless of live signals); otherwise the worst live
/// condition wins, with a total `else` falling back to the config status. Recomputed on every read (a
/// clock/live-input flip on fixed rows changes the result — the LESSON §41 recomputed-not-replayed pin).
pub fn derive_runtime_status(
    config_status: ExecutionProfile,
    inputs: &RuntimeInputs,
) -> ExecutionProfile {
    // a terminal config state is honored first — a disabled profile stays disabled (no live override).
    if config_status == ExecutionProfile::Disabled {
        return ExecutionProfile::Disabled;
    }
    // strict precedence over the live conditions (worst-first): misconfigured > auth_expired >
    // rate_limited > in_use > the config status (the total `else`).
    if inputs.self_test == Some(ExecutionProfile::Misconfigured) {
        return ExecutionProfile::Misconfigured;
    }
    if inputs.auth_expired {
        return ExecutionProfile::AuthExpired;
    }
    if inputs.rate_limited {
        return ExecutionProfile::RateLimited;
    }
    if inputs.in_use {
        return ExecutionProfile::InUse;
    }
    // total else — no live condition → the configured/intended status (typically Available).
    config_status
}

/// Piece 2 + 3 — the startup profile self-test + runtime-status RECOMPUTE pass. For each registered profile,
/// read its keychain entry ([`self_test_status`]) then RE-DERIVE the RUNTIME status from the live inputs
/// ([`derive_runtime_status`]). At cold-start the only live input is the keychain self-test (no live sessions
/// yet → `in_use`/`rate_limited`/`auth_expired` are false), so this pass surfaces `misconfigured` profiles
/// (a configured `keychain_ref` whose entry doesn't resolve). The runtime status is RECOMPUTED-on-read, NOT
/// persisted to the config `status` column (§7.2 — the persisted status stays the CONFIG/intended state).
/// Returns the per-profile `(id, runtime_status)` the caller (main.rs cold-start) logs + holds for the
/// DORMANT-until-ui-reader in-memory view (LESSON §35/§41; the ui profile-status read RPC is the named
/// follow-on). Read-only WAL (the executor stays the sole writer — forbidden #3). A malformed row is skipped
/// defensively (never panics the cold-start).
pub fn run_profile_self_test_pass(
    db_path: &std::path::Path,
    store: &dyn SecretStore,
) -> Result<Vec<(ExecutionProfileId, ExecutionProfile)>, EventStoreError> {
    let conn = crate::eventstore::open_read_only(db_path)?;
    let mut stmt = conn
        .prepare("SELECT execution_profile_id, keychain_ref, status FROM execution_profiles")
        .map_err(EventStoreError::Write)?;
    let rows = stmt
        .query_map([], |r| {
            let id: String = r.get(0)?;
            let keychain_ref: Option<String> = r.get(1)?;
            let status: String = r.get(2)?;
            Ok((id, keychain_ref, status))
        })
        .map_err(EventStoreError::Write)?;
    let mut out = Vec::new();
    for row in rows {
        let (id_str, keychain_ref, status_str) = row.map_err(EventStoreError::Write)?;
        // a malformed id / status is skipped (defensive — a bad row can't crash the cold-start pass).
        let Ok(id) = ExecutionProfileId::parse(&id_str) else {
            continue;
        };
        let config_status: ExecutionProfile =
            serde_json::from_value(serde_json::Value::String(status_str))
                .unwrap_or(ExecutionProfile::Unknown);
        // the keychain self-test — the only live runtime input available at cold-start (None when the profile
        // has no configured secret → ambient-auth).
        let read_outcome = keychain_ref.as_deref().map(|kref| match store.read(kref) {
            Ok(Some(_)) => KeychainReadOutcome::Resolved,
            Ok(None) => KeychainReadOutcome::Unresolved,
            Err(_) => KeychainReadOutcome::BackendFault,
        });
        let self_test = self_test_status(keychain_ref.is_some(), read_outcome);
        let runtime = derive_runtime_status(
            config_status,
            &RuntimeInputs {
                self_test,
                ..Default::default()
            },
        );
        out.push((id, runtime));
    }
    Ok(out)
}
