//! The `integration.connect` executor (P7.1 Wave-C, edges-029) — `ExecutorKind::Integration`.
//!
//! REGISTERS an integration connection and emits `IntegrationConnectionRegistered` through the Gateway's
//! in-txn §15 redaction gate (`ExecutionOutcome.emitted_events` → the pipeline appends it ATOMIC with
//! `ActionSucceeded`; the `ProjectExecutor`/edges-019 precedent, Q1=B generic bridge).
//!
//! **§15 #4 — registration-only, the token NEVER flows through the action (load-bearing):** a risk-2
//! action is approval-gated, so `execute()` runs off the §15-REDACTED durable `action_requests` row
//! (LESSON 20 §7.2-split) — a secret placed in `inputs_json` is masked BEFORE persist (LESSON 16
//! dual-gate) → GONE by execute-time. So the mutator CANNOT carry a token: the inputs carry only
//! `{provider, keychain_ref (a pre-existing POINTER), account?}`, and the emitted event holds ONLY the
//! pointer (the event type has no token slot). The token→keychain WRITE is a SEPARATE, DEFERRED
//! non-Gateway mechanism (the `keyring` crate + a secret-store path; HITL). §15 #4 holds BY
//! CONSTRUCTION; defense-in-depth, a `keychain_ref` that LOOKS like a secret (a known prefix /
//! high-entropy run — the canonical LESSON-13 §15 detector reused read-only via [`PrefixRedactor`]: if
//! it MASKS any span, the redacted output differs from the input → reject) is REJECTED, so a caller
//! can't smuggle a token into the pointer slot.
//!
//! **INV-SEC-1:** the executor holds no write-actor handle and no DB-write surface — it emits ONLY via
//! `emitted_events` (the edges-executor boundary). It is reachable ONLY via the catalog-gated pipeline
//! (risk-2 → approval; the `requires_resource_refs`/Adjudication guards run before dispatch). There is
//! NO CLI here (registration-only emits an event; nothing is shelled), so the LESSON-31 arg-guard
//! discipline maps to operand validation — the closed `Provider` enum (reject-unknown) + account
//! control-char validation + the keychain_ref secret-shape reject — not a literal dash-operand guard.

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::events::{
    IntegrationConnectionRegistered, IntegrationLiveWritesSet, Provider,
};
use nexusops_shared::time::Timestamp;

use crate::eventstore::{EventStoreError, PrefixRedactor, Redactor};
use crate::gateway::executor::{ActionExecutor, EmittedEvent, ExecError, ExecutionOutcome};
use crate::idgen::IdGen;

/// The `ExecutorKind::Integration` action types this leaf executor handles.
const INTEGRATION_CONNECT: &str = "integration.connect";
const INTEGRATION_SET_LIVE_WRITES: &str = "integration.set_live_writes";

/// The read seam for the registered-connection gate: whether a `connection_id` is a REGISTERED connection
/// (a `proj_integration_connection` row exists). Injected so `integration.set_live_writes` validation is
/// deterministically unit-testable (a fake in tests; the sqlite-backed reader in production) — the
/// `ProfileLookup`/§15 #8 `resolve_profile`-on-unknown precedent (LESSON §62). The production impl
/// ([`super::connections::SqliteConnectionLookup`]) lives in a SIBLING module so this executor module
/// stays free of any DB read/write surface — the INV-SEC-1 no-second-mutator structural guard (Test 6).
pub trait ConnectionLookup: Send + Sync {
    /// Whether `connection_id` references a registered connection — the fail-closed-on-unknown gate (a
    /// flip on an unregistered connection is rejected, never silently emitted).
    fn is_registered(&self, connection_id: &str) -> Result<bool, EventStoreError>;
}

/// Registers integration connections (emits `IntegrationConnectionRegistered`) + flips the per-connection
/// live-writes toggle (emits `IntegrationLiveWritesSet`). Holds an injected [`IdGen`] (mints the `conn_`
/// id, deterministic in tests) + a [`ConnectionLookup`] (the registered-connection gate for the toggle).
/// No write-actor handle — emits only via `emitted_events` (INV-SEC-1: the executor is not a second
/// mutator).
pub struct IntegrationExecutor {
    idgen: Box<dyn IdGen>,
    connections: Box<dyn ConnectionLookup>,
}

impl IntegrationExecutor {
    pub fn new(idgen: Box<dyn IdGen>, connections: Box<dyn ConnectionLookup>) -> Self {
        Self { idgen, connections }
    }

    fn execute_connect(&self, req: &ActionRequest) -> ExecutionOutcome {
        // provider — REQUIRED, the CLOSED Provider enum (reject-unknown, §5.0/§15 — LESSON 31 operand
        // validation: an unknown provider fails closed, never a default).
        let provider =
            match req.inputs.get("provider") {
                Some(v) => match serde_json::from_value::<Provider>(v.clone()) {
                    Ok(p) => p,
                    Err(_) => return ExecutionOutcome::Failed(
                        "integration.connect: missing or unknown provider (expected github|linear)"
                            .to_string(),
                    ),
                },
                None => {
                    return ExecutionOutcome::Failed(
                        "integration.connect requires inputs[\"provider\"]".to_string(),
                    )
                }
            };

        // keychain_ref — REQUIRED, non-empty, a §15 #4 POINTER (never a token). Defense-in-depth: reject
        // a secret-shaped ref via the canonical LESSON-13 §15 detector (PrefixRedactor, read-only — it
        // masks in place, so a changed output means it found a token-shaped span; see the check below).
        let keychain_ref = match req.inputs.get("keychain_ref").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => {
                return ExecutionOutcome::Failed(
                    "integration.connect requires a non-empty inputs[\"keychain_ref\"]".to_string(),
                )
            }
        };
        // the redactor masks in place; if the output DIFFERS from the input it masked a token-shaped
        // span → the "pointer" is actually a secret → reject (§15 #4 defense-in-depth). A clean pointer
        // (`nexusops/github/octocat`) splits into short low-entropy sub-tokens → unchanged → accepted.
        if PrefixRedactor.redact(&keychain_ref).payload_json != keychain_ref {
            return ExecutionOutcome::Failed(
                "integration.connect: keychain_ref must be a non-secret pointer, not a token (§15 #4)"
                    .to_string(),
            );
        }

        // account — OPTIONAL; if present, validate (non-empty + no control chars — no injection into the
        // connection identity). Absent / null → None.
        let account = match req.inputs.get("account") {
            None | Some(serde_json::Value::Null) => None,
            Some(v) => match v.as_str() {
                Some(s) if is_clean_account(s) => Some(s.to_string()),
                _ => {
                    return ExecutionOutcome::Failed(
                        "integration.connect: malformed account (a non-empty, control-char-free string)"
                            .to_string(),
                    )
                }
            },
        };

        let connection_id = self.idgen.new_connection_id();
        let detail =
            format!("integration.connect — registered {provider:?} connection {connection_id}");

        // the executor serializes its own FROZEN event struct (Q1=B — the generic Namespaced bridge); a
        // serialize fault fails CLOSED (no malformed event reaches the in-txn append). The event carries
        // ONLY the keychain_ref POINTER — there is no token field (§15 #4 by construction).
        let payload = IntegrationConnectionRegistered {
            connection_id,
            provider,
            keychain_ref,
            account,
        };
        let payload_json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => {
                return ExecutionOutcome::Failed(format!(
                    "serialize IntegrationConnectionRegistered: {e}"
                ))
            }
        };

        ExecutionOutcome::Succeeded {
            changed_resources: vec![],
            detail,
            // registration-only: the event IS the record; NO durable EXTERNAL side effect (the
            // token→keychain write is deferred) → a txn-B fault rolls back cleanly, never
            // ActionPartiallySucceeded (§17 — the project.rescan precedent).
            side_effect_applied: false,
            emitted_events: vec![EmittedEvent::Namespaced {
                event_type: IntegrationConnectionRegistered::EVENT_TYPE,
                payload_json,
            }],
        }
    }

    /// `integration.set_live_writes` (P4.7/083 Q3): emit `IntegrationLiveWritesSet{connection_id, enabled}`
    /// — the typed live-writes authorization flip. The connection identity is INPUT-CARRIED (mirroring
    /// `integration.connect`; the recorded audited identity is in `inputs` + the emitted event). NO secret
    /// flows through (an authorization-state flip — the keychain pointer already lives on the registration).
    /// **Fail-closed on an UNKNOWN connection** (ADD, Step-2.5): the `connection_id` MUST reference a
    /// REGISTERED connection (the `resolve_profile`-on-unknown precedent, LESSON §62) — else the flip is
    /// rejected, never emitted for a phantom connection.
    fn execute_set_live_writes(&self, req: &ActionRequest) -> ExecutionOutcome {
        // connection_id — REQUIRED, non-empty (no phantom/empty identity into the authorization record).
        let connection_id =
            match req.inputs.get("connection_id").and_then(|v| v.as_str()) {
                Some(s) if !s.trim().is_empty() => s.to_string(),
                _ => return ExecutionOutcome::Failed(
                    "integration.set_live_writes requires a non-empty inputs[\"connection_id\"]"
                        .to_string(),
                ),
            };
        // enabled — REQUIRED bool (no defaulting an authorization decision; a missing/non-bool fails closed).
        let enabled = match req.inputs.get("enabled") {
            Some(v) => match v.as_bool() {
                Some(b) => b,
                None => {
                    return ExecutionOutcome::Failed(
                        "integration.set_live_writes: inputs[\"enabled\"] must be a bool"
                            .to_string(),
                    )
                }
            },
            None => {
                return ExecutionOutcome::Failed(
                    "integration.set_live_writes requires inputs[\"enabled\"]".to_string(),
                )
            }
        };
        // the registered-connection gate (fail-closed on unknown — §15 #8 no-silent-mint precedent): a flip
        // may only target a connection that was actually registered. A lookup fault also fails closed.
        match self.connections.is_registered(&connection_id) {
            Ok(true) => {}
            Ok(false) => {
                return ExecutionOutcome::Failed(format!(
                    "integration.set_live_writes: connection_id `{connection_id}` is not a registered connection (fail-closed)"
                ))
            }
            Err(e) => {
                return ExecutionOutcome::Failed(format!(
                    "integration.set_live_writes: connection lookup failed: {e}"
                ))
            }
        }

        let detail = format!(
            "integration.set_live_writes — connection {connection_id} live_writes_enabled={enabled}"
        );
        // the executor serializes its own FROZEN event struct (Q1=B — the generic Namespaced bridge); a
        // serialize fault fails CLOSED. The event carries ONLY {connection_id, enabled} — NO secret.
        let payload = IntegrationLiveWritesSet {
            connection_id,
            enabled,
        };
        let payload_json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => {
                return ExecutionOutcome::Failed(format!("serialize IntegrationLiveWritesSet: {e}"))
            }
        };

        ExecutionOutcome::Succeeded {
            changed_resources: vec![],
            detail,
            // an authorization flip recorded as an event; NO durable EXTERNAL side effect → a txn-B fault
            // rolls back cleanly, never ActionPartiallySucceeded (the integration.connect precedent).
            side_effect_applied: false,
            emitted_events: vec![EmittedEvent::Namespaced {
                event_type: IntegrationLiveWritesSet::EVENT_TYPE,
                payload_json,
            }],
        }
    }
}

/// An account is a non-empty, control-char-free identity string (no newline/injection into the
/// connection record). Pure + total.
fn is_clean_account(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(|c| c.is_control())
}

impl ActionExecutor for IntegrationExecutor {
    fn validate(&self, _req: &ActionRequest) -> Result<(), ExecError> {
        // IntegrationExecutor is a LEAF handler for ExecutorKind::Integration — the outer
        // CatalogExecutor.resolve() already enforced the catalog `requires_resource_refs` precondition
        // (false here) BEFORE dispatch. Input validation lives in execute() (fail-closed →
        // ExecutionOutcome::Failed, AUDITED as ActionFailed — the ProjectExecutor precedent), so there
        // is nothing left to validate at this leaf.
        Ok(())
    }

    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        match req.action_type.as_str() {
            INTEGRATION_CONNECT => self.execute_connect(req),
            INTEGRATION_SET_LIVE_WRITES => self.execute_set_live_writes(req),
            // a non-Integration action should never reach this leaf (the outer CatalogExecutor dispatches
            // by ExecutorKind::Integration) — fail CLOSED rather than silently succeed if a future
            // Integration-kind type is catalogued before its arm lands here.
            other => ExecutionOutcome::Failed(format!(
                "IntegrationExecutor has no handler for action_type `{other}`"
            )),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        // never on the production path (the outer CatalogExecutor renders previews via the catalog) — a
        // minimal envelope, mirroring the ProjectExecutor/StubExecutor pattern.
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: format!("integration executor: {}", req.action_type),
            changed_resources: vec![],
            cannot_preview_reason: None,
        }
    }
}
