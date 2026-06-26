//! The `profile.set_keychain_ref` executor (P5.3b/085, cat-1 — `ExecutorKind::Profile`) — the audited
//! pointer-record half of the execution-profile SECRET vertical.
//!
//! The user-typed SECRET was already written to the OS keychain by the peer-authed `profile.set_secret`
//! IPC trigger (substrate, NOT audited — the secret can't be audited). THIS audited Gateway action records
//! the §15 #4 keychain POINTER onto the canonical `execution_profiles` row (registration-only, LESSON
//! §49/§64 — the secret is already in the keychain; the action carries only the pointer). The
//! `keychain_ref` is RE-DERIVED from the AUDITED resource_ref profile id (NEVER `inputs` —
//! confused-deputy-safe, LESSON §63); the canonical-row UPDATE is applied in the pipeline's txn-B via
//! [`crate::profiles::secret::apply_secret_set`] (ATOMIC with the `ProfileSecretSet` append — fail-closed).
//!
//! A SEPARATE executor (NOT folded into `SessionExecutor`) because a profile-registry mutation is its own
//! domain with no session — the `ExecutorKind::Integration` precedent. Lives in `gateway/` (not `profiles/`)
//! to avoid a `gateway → profiles → gateway` import cycle (the `SessionExecutor` precedent): it imports
//! `profiles::{ProfileLookup, secret::profile_keychain_ref}` downward, never the reverse.

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::ids::ExecutionProfileId;
use nexusops_shared::time::Timestamp;

use crate::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecError, ExecutionOutcome,
};
use crate::profiles::ProfileLookup;

/// The action type this executor handles specially (else it delegates to the inner catalog stub).
const SET_KEYCHAIN_REF: &str = "profile.set_keychain_ref";

/// The `profile.set_keychain_ref` executor. Holds the §15 #8 registry read seam (to refuse an unregistered
/// target — fail-closed-on-unknown, LESSON §62) and the inner [`CatalogExecutor`] (the catalog precondition
/// plus delegation). NO keychain store / NO DB handle — the SECRET write is the separate trigger; the
/// canonical-row UPDATE is the pipeline's txn-B job (the executor only emits the typed `ProfileSecretSet`).
pub struct ProfileExecutor {
    profile_lookup: Box<dyn ProfileLookup>,
    inner: CatalogExecutor,
}

impl ProfileExecutor {
    pub fn new(profile_lookup: Box<dyn ProfileLookup>) -> Self {
        Self {
            profile_lookup,
            inner: CatalogExecutor::new(),
        }
    }

    fn execute_set_keychain_ref(&self, req: &ActionRequest) -> ExecutionOutcome {
        // validate the catalog `requires_resource_refs` precondition FIRST (this path runs its own
        // emit, never reaching `inner.execute`'s validation) — a missing resource_ref → Failed.
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // the target profile is the AUDITED resource_ref (NaturalResourceRef), NEVER `inputs` — the
        // confused-deputy pin (LESSON §63): the recorded ref is bound to the audited id, not attacker input.
        let Some(rref) = req.resource_refs.first() else {
            return ExecutionOutcome::Failed(
                "profile.set_keychain_ref requires the target profile resource_ref".to_string(),
            );
        };
        let id = match ExecutionProfileId::parse(&rref.id) {
            Ok(id) => id,
            Err(_) => {
                return ExecutionOutcome::Failed(format!(
                    "profile.set_keychain_ref: invalid execution profile id '{}'",
                    rref.id
                ))
            }
        };
        // fail-closed-on-unknown (§15 #8 / LESSON §62): never record a pointer for an unregistered profile.
        match self.profile_lookup.exists(&id) {
            Ok(true) => {}
            Ok(false) => {
                return ExecutionOutcome::Failed(format!(
                    "execution profile not found: {}",
                    rref.id
                ))
            }
            Err(e) => {
                return ExecutionOutcome::Failed(format!("execution profile lookup failed: {e}"))
            }
        }
        // the POINTER is daemon-derived from the AUDITED id by `apply_secret_set` in txn-B — it is NOT
        // carried on the event payload (the §15 JSON-value-redactor false-positive avoidance; the bare id
        // is the audited identity, the ref is recomputed). The executor only records WHICH profile.
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: format!(
                "profile.set_keychain_ref — recorded the keychain pointer for {}",
                id.as_str()
            ),
            // side_effect_applied=false: the canonical-row UPDATE + the ProfileSecretSet append BOTH run in
            // the pipeline's txn-B (atomic). If txn-B can't write, NEITHER lands → a clean rollback (the
            // keychain SECRET write was the separate trigger, already idempotent) → a lost terminal event
            // rolls back cleanly, NOT a false ActionPartiallySucceeded (LESSON §21).
            side_effect_applied: false,
            emitted_events: vec![EmittedEvent::ProfileSecretSet {
                execution_profile_id: id,
            }],
        }
    }
}

impl ActionExecutor for ProfileExecutor {
    fn validate(&self, req: &ActionRequest) -> Result<(), ExecError> {
        self.inner.validate(req)
    }

    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        match req.action_type.as_str() {
            SET_KEYCHAIN_REF => self.execute_set_keychain_ref(req),
            // any other action routed here (shouldn't happen — registered only for ExecutorKind::Profile)
            // delegates to the inner catalog executor (which validates internally).
            _ => self.inner.execute(req),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        self.inner.preview(req, generated_at)
    }
}
