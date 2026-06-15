//! P7.1 Wave-C (edges-029) — the `integration.connect` Gateway mutator (`ExecutorKind::Integration`).
//!
//! A REGISTRATION-ONLY mutator: it records an integration connection and emits
//! `IntegrationConnectionRegistered` (the `keychain_ref` is a §15 #4 NON-SECRET pointer). The token
//! NEVER flows through the action — a risk-2 action is approval-gated, so `execute()` runs off the
//! §15-REDACTED durable row (LESSON 20 §7.2-split), where a secret in `inputs_json` is masked by
//! execute-time → §15 #4 holds BY CONSTRUCTION (the executor has no token field). The token→keychain
//! WRITE is a separate, deferred non-Gateway mechanism (HITL; the `keyring` crate is not yet a dep).
//!
//! Tests call `IntegrationExecutor::execute()` DIRECTLY with raw inputs (the project_executor.rs /
//! git_executor.rs unit precedent); the catalog + CONTRACT-surface assertions live in
//! `shared/tests/contract.rs`. INV-SEC-1 no-bypass is pinned structurally (the executor holds no
//! `WriteHandle`/SQL — emits only via `emitted_events`) + by `/wired` (the live catalog-gated path).

use nexusops_shared::actions::{ActionRequest, RequesterType, RiskLevel};
use nexusops_shared::catalog::{lookup, ExecutorKind};
use nexusops_shared::events::{IntegrationConnectionRegistered, Provider};
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::gateway::executor::{ActionExecutor, EmittedEvent, ExecutionOutcome};
use nexusopsd::idgen::FixedIdGen;
use nexusopsd::integrations::connect::IntegrationExecutor;

const FIXED_TS: &str = "2026-06-11T00:00:00Z";

// ---- harness helpers -------------------------------------------------------

/// A fresh `IntegrationExecutor` with a deterministic IdGen (conn_ ids derived from a counter).
fn executor() -> IntegrationExecutor {
    IntegrationExecutor::new(Box::new(FixedIdGen::new()))
}

/// An `integration.connect` ActionRequest carrying `inputs` verbatim (raw — the direct-execute path).
fn connect_req(inputs: serde_json::Value) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "integration.connect".to_string(),
        requester_type: RequesterType::User, // UI/IPC
        requester_id: "u_local".to_string(),
        resource_refs: vec![], // integration.connect: requires_resource_refs = false
        inputs,
        risk_level: RiskLevel::Level2,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

/// Extract the single `IntegrationConnectionRegistered` from a Succeeded outcome (asserts the shape).
fn registration_event(outcome: &ExecutionOutcome) -> IntegrationConnectionRegistered {
    match outcome {
        ExecutionOutcome::Succeeded {
            emitted_events,
            side_effect_applied,
            ..
        } => {
            assert!(
                !side_effect_applied,
                "registration-only: no durable EXTERNAL side effect (the token→keychain write is deferred)"
            );
            assert_eq!(emitted_events.len(), 1, "exactly one event emitted");
            match &emitted_events[0] {
                EmittedEvent::Namespaced {
                    event_type,
                    payload_json,
                } => {
                    assert_eq!(*event_type, IntegrationConnectionRegistered::EVENT_TYPE);
                    serde_json::from_str(payload_json)
                        .expect("IntegrationConnectionRegistered payload parses")
                }
                _ => panic!("expected a Namespaced event"),
            }
        }
        _ => panic!("expected ExecutionOutcome::Succeeded"),
    }
}

fn is_failed(outcome: &ExecutionOutcome) -> bool {
    matches!(outcome, ExecutionOutcome::Failed(_))
}

// ---- Test 1 — the catalog entry (also asserted in shared/tests/contract.rs) ----

#[test]
fn test_catalog_integration_connect_entry() {
    // spec(§6.3): integration.connect has a binding catalog entry — risk-2, ExecutorKind::Integration,
    // requires_resource_refs=false, NON-standing-grantable (a credential/authorization-establishing
    // action always gets a per-action approval — the §6.2 floor; security-reviewer-ruled, LESSON 32).
    let e = lookup("integration.connect").expect("integration.connect catalogued");
    assert_eq!(e.locked_risk, RiskLevel::Level2);
    assert_eq!(e.executor, ExecutorKind::Integration);
    assert!(!e.requires_resource_refs);
    assert!(
        !e.standing_grant_eligible,
        "credential registration is never folded into an approve-all (§6.2 floor)"
    );
}

// ---- Test 2 — execute() emits the registration event -----------------------

#[test]
fn test_executor_emits_registration() {
    // spec(§7.2): execute() with {provider, keychain_ref, account} mints a conn_ id and emits
    // IntegrationConnectionRegistered carrying the pointer (the edges-019 emit precedent).
    let outcome = executor().execute(&connect_req(serde_json::json!({
        "provider": "github",
        "keychain_ref": "nexusops/github/octocat",
        "account": "octocat",
    })));
    let ev = registration_event(&outcome);
    assert!(
        ev.connection_id.starts_with("conn_"),
        "connection_id is conn_-prefixed (IdGen), got {}",
        ev.connection_id
    );
    assert_eq!(ev.provider, Provider::Github);
    assert_eq!(ev.keychain_ref, "nexusops/github/octocat");
    assert_eq!(ev.account.as_deref(), Some("octocat"));
}

#[test]
fn test_executor_emits_registration_no_account() {
    // spec(§7.2): account is optional → None folds through (the Option→null binding path).
    let outcome = executor().execute(&connect_req(serde_json::json!({
        "provider": "linear",
        "keychain_ref": "nexusops/linear/team",
    })));
    let ev = registration_event(&outcome);
    assert_eq!(ev.provider, Provider::Linear);
    assert_eq!(ev.account, None);
}

// ---- Test 3 — §15 #4: keychain_ref pointer-ONLY, secret-shaped ref rejected (OWN safety pin) ----

#[test]
fn test_keychain_ref_pointer_only_no_token() {
    // spec(§15 #4): the emitted event carries ONLY the keychain_ref pointer — there is NO token field
    // (structural: IntegrationConnectionRegistered has no secret slot). Defense-in-depth: a keychain_ref
    // that LOOKS like a secret (a known prefix / high-entropy run — the LESSON 13 detector, reused
    // read-only via PrefixRedactor) is REJECTED, so a caller can't smuggle a token into the pointer slot.
    // (a) a clean pointer registers; the raw payload holds no secret-shaped token.
    let outcome = executor().execute(&connect_req(serde_json::json!({
        "provider": "github",
        "keychain_ref": "nexusops/github/octocat",
    })));
    let ev = registration_event(&outcome);
    let payload = serde_json::to_string(&ev).unwrap();
    assert!(
        !payload.contains("ghp_") && !payload.contains("github_pat_"),
        "no secret-shaped token in the event payload"
    );

    // (b) a secret-shaped keychain_ref (a real GitHub PAT in the pointer slot) is REJECTED → no event.
    let outcome = executor().execute(&connect_req(serde_json::json!({
        "provider": "github",
        "keychain_ref": "ghp_016C7f8e9a0b1c2d3e4f5061728394a5b6c7d8e9f0",
    })));
    assert!(
        is_failed(&outcome),
        "a known-prefix secret keychain_ref must be rejected (§15 #4 defense-in-depth)"
    );

    // (c) a BARE high-entropy run (no known prefix, ≥40 chars, ≥4.5 bits/char) is also REJECTED — the
    // entropy pass of the §15 detector, not just the prefix pass (covers the non-prefixed token form).
    let outcome = executor().execute(&connect_req(serde_json::json!({
        "provider": "github",
        "keychain_ref": "aB3xK9mP2qR7wL5nT8vY1cD4fG6hJ0kZ3sQ7uW9eR2tX6y",
    })));
    assert!(
        is_failed(&outcome),
        "a bare high-entropy keychain_ref must be rejected (§15 #4 — the entropy pass, LESSON 13)"
    );
}

// ---- Test 4 — the arg guard: closed Provider enum + account validation -----

#[test]
fn test_arg_guard_rejects_unknown_provider_and_bad_account() {
    // spec(§5.0/§15 + LESSON 31): provider is the CLOSED Provider enum (reject-unknown); a malformed
    // account (control chars / newline injection into the connection identity) fails closed.
    let unknown_provider = executor().execute(&connect_req(serde_json::json!({
        "provider": "gitlab",
        "keychain_ref": "nexusops/gitlab/me",
    })));
    assert!(
        is_failed(&unknown_provider),
        "an unknown provider is rejected (closed Provider enum)"
    );

    let bad_account = executor().execute(&connect_req(serde_json::json!({
        "provider": "github",
        "keychain_ref": "nexusops/github/me",
        "account": "octo\ncat",
    })));
    assert!(
        is_failed(&bad_account),
        "a control-char account is rejected (no injection into the connection identity)"
    );

    // an explicitly-empty account string (present but blank) fails closed — distinct from absent (None).
    let empty_account = executor().execute(&connect_req(serde_json::json!({
        "provider": "github",
        "keychain_ref": "nexusops/github/me",
        "account": "   ",
    })));
    assert!(
        is_failed(&empty_account),
        "a blank account string is rejected (a present-but-empty identity is malformed)"
    );
}

// ---- Test 5 — missing required inputs fail closed (no malformed event) ------

#[test]
fn test_invalid_inputs_fail_closed() {
    // spec(§15): missing provider OR keychain_ref → Failed, NO event (the edges-019 serialize-fault
    // precedent — the executor never emits a malformed audit event).
    let no_provider = executor().execute(&connect_req(serde_json::json!({
        "keychain_ref": "nexusops/github/me",
    })));
    assert!(is_failed(&no_provider), "missing provider → Failed");

    let no_ref = executor().execute(&connect_req(serde_json::json!({
        "provider": "github",
    })));
    assert!(is_failed(&no_ref), "missing keychain_ref → Failed");

    let blank_ref = executor().execute(&connect_req(serde_json::json!({
        "provider": "github",
        "keychain_ref": "   ",
    })));
    assert!(is_failed(&blank_ref), "blank keychain_ref → Failed");
}

// ---- Test 6 — INV-SEC-1: the executor holds no WriteHandle / SQL -----------

#[test]
fn test_inv_sec_1_executor_holds_no_writehandle() {
    // spec(INV-SEC-1 #1/#2): the IntegrationExecutor is a Gateway EDGE — it emits ONLY via
    // emitted_events and holds NO write surface (no WriteHandle, no rusqlite/Connection, no write-actor).
    // A forbidden-token grep over the source (the session/ cat-1 import-grep precedent). The pure
    // read-only §15 detector (PrefixRedactor) is intentionally NOT forbidden — it is a string predicate,
    // not a mutation path. The live no-bypass path is proven by /wired (submit→policy→approval→execute).
    let src = include_str!("../src/integrations/connect.rs");
    // `rusqlite` covers any DB `Connection` precisely (a bare "Connection" substring would false-positive
    // on the legit event name `IntegrationConnectionRegistered`); these three are the exact write surface.
    for tok in ["WriteHandle", "rusqlite", "INSERT INTO"] {
        assert!(
            !src.contains(tok),
            "integrations/connect.rs must not name `{tok}` — the executor emits only via emitted_events \
             (INV-SEC-1: no second mutator; no DB write)"
        );
    }
}
