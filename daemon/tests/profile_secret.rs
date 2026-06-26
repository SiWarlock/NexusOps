//! P5.3b/085 — the execution-profile SECRET vertical (cat-1). C2: the `profile.set_secret` inbound-secret
//! keychain-write trigger (the ⚠️ NEW POSTURE — the FIRST inbound-secret surface). C3: the audited
//! `profile.set_keychain_ref` pointer-record Gateway action (added below in the C3 block).
//!
//! The deterministic core is pinned via the injected `FakeSecretStore` seam; the live keychain round-trip
//! is the non-deterministic edge (no unit test). §15 #4 (POINTER-only) / §15 #7 (getpeereid-first) /
//! LESSON §62 (fail-closed-on-unknown) / LESSON §64 (no-echo).

use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::Path;

use zeroize::Zeroizing;

use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::idgen::UlidGen;
use nexusopsd::integrations::keychain::{FakeSecretStore, SecretStore, SecretStoreError};
use nexusopsd::ipc::serve_connection;
use nexusopsd::profiles::secret::{profile_keychain_ref, write_profile_secret};
use nexusopsd::profiles::{self, ProfileSpec};

use nexusops_shared::ids::{ExecutionProfileId, WorkspaceId};
use nexusops_shared::ipc::{IpcErrorCode, SetProfileSecretParams};
use nexusops_shared::status::ExecutionProfile;

const OCCURRED_AT: &str = "2026-06-25T00:00:00Z";
const SECRET: &str = "sk-ant-the-inbound-profile-credential-0xDEADBEEF";

fn open(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new(OCCURRED_AT)),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// register a real profile in a temp db (mirrors `tests/execution_profiles.rs`) so the fail-closed-on-unknown
/// gate has a known id to accept; returns the registered id.
fn register_profile(store: &mut EventStore) -> ExecutionProfileId {
    let id = ExecutionProfileId::new();
    let spec = ProfileSpec {
        workspace_id: WorkspaceId::system(),
        provider: "anthropic".to_string(),
        harness: "claude_code".to_string(),
        model: None,
        account_alias: None,
        keychain_ref: None,
        usage_policy_json: None,
        status: ExecutionProfile::Available,
    };
    profiles::register_profile(store, &id, &spec, OCCURRED_AT).expect("register profile");
    id
}

// ---- C2 RED — the inbound keychain-write trigger (§15 #4/#7, LESSON §62/§64) -----------------------

#[test]
fn set_secret_writes_keychain_and_returns_only_pointer() {
    // spec(§15 #4 / LESSON §64) — the inbound secret lands in the keychain under the daemon-derived
    // per-profile ref; the result carries ONLY the keychain_ref POINTER (no secret echoed).
    let store = FakeSecretStore::new();
    let id = ExecutionProfileId::new();
    let result = write_profile_secret(&store, &id, Zeroizing::new(SECRET.to_string()))
        .expect("write succeeds");
    // the pointer is the daemon-derived id-keyed ref.
    assert_eq!(result.keychain_ref, profile_keychain_ref(&id));
    assert_eq!(
        result.keychain_ref,
        format!("nexusops/profile/{}", id.as_str())
    );
    // the SECRET is in the keychain under that ref — and ONLY there.
    assert_eq!(
        store.read(&result.keychain_ref).expect("read").as_deref(),
        Some(SECRET),
    );
    // the result serializes to a pointer ONLY — the secret never appears in the wire body (§15 #4 no-echo).
    let body = serde_json::to_string(&result).unwrap();
    assert!(
        !body.contains(SECRET),
        "the result body must NOT contain the secret"
    );
    assert!(body.contains(&result.keychain_ref));
}

#[test]
fn set_secret_keychain_fault_is_structural_no_secret_leak() {
    // spec(§15 / LESSON §64) — a keychain backend fault surfaces a STRUCTURAL error; the secret is NEVER
    // in the error message (Display or Debug). The fail-closed write returns Err, nothing partial.
    let store = FailingSecretStore;
    let id = ExecutionProfileId::new();
    let err = write_profile_secret(&store, &id, Zeroizing::new(SECRET.to_string()))
        .expect_err("a backend fault fails the write");
    let shown = format!("{err} | {err:?}");
    assert!(
        !shown.contains(SECRET),
        "the structural error must NOT echo the secret"
    );
}

#[test]
fn set_secret_inbound_secret_held_zeroizing() {
    // spec(§15 / 085 NEW POSTURE) — the trigger core takes the inbound secret BY VALUE in `Zeroizing`
    // (the type-level scrub guarantee — the heap allocation is wiped on drop after the keychain write).
    // The no-echo corollary: neither the returned result nor its Debug ever contains the secret.
    // Out of scope: a runtime proof of the freed-memory wipe (a type-level pin, the 083 precedent).
    let store = FakeSecretStore::new();
    let id = ExecutionProfileId::new();
    let secret: Zeroizing<String> = Zeroizing::new(SECRET.to_string());
    let result = write_profile_secret(&store, &id, secret).expect("write succeeds");
    assert!(
        !format!("{result:?}").contains(SECRET),
        "result Debug must not echo the secret"
    );
}

#[test]
fn set_secret_unknown_profile_fails_closed() {
    // spec(§15 #4 / LESSON §62) — the trigger orchestration refuses an unknown/unparseable profile id
    // BEFORE any keychain write (no entry for an unregistered profile); only a REGISTERED id writes.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let registered = {
        let mut store = open(&path);
        register_profile(&mut store)
    };
    let store = FakeSecretStore::new();

    // (a) an unparseable id → protocol_error, keychain untouched.
    let unparseable = nexusopsd::ipc::set_profile_secret(
        &store,
        &path,
        SetProfileSecretParams {
            execution_profile_id: "not-a-prof-id".to_string(),
            secret: SECRET.to_string(),
        },
    );
    assert!(matches!(unparseable, Err(IpcErrorCode::ProtocolError)));

    // (b) a WELL-FORMED but UNREGISTERED id → not_found, keychain untouched (fail-closed-on-unknown).
    let unknown_id = ExecutionProfileId::new();
    let unknown = nexusopsd::ipc::set_profile_secret(
        &store,
        &path,
        SetProfileSecretParams {
            execution_profile_id: unknown_id.as_str().to_string(),
            secret: SECRET.to_string(),
        },
    );
    assert!(matches!(unknown, Err(IpcErrorCode::NotFound)));

    // NO keychain entry was written for EITHER rejected profile (the secret never reached the store).
    assert_eq!(
        store.read(&profile_keychain_ref(&unknown_id)).unwrap(),
        None
    );

    // (c) the REGISTERED id writes + returns the pointer (the positive control — the gate isn't blanket-deny).
    let ok = nexusopsd::ipc::set_profile_secret(
        &store,
        &path,
        SetProfileSecretParams {
            execution_profile_id: registered.as_str().to_string(),
            secret: SECRET.to_string(),
        },
    )
    .expect("a registered profile accepts the secret");
    assert_eq!(ok.keychain_ref, profile_keychain_ref(&registered));
    assert_eq!(
        store.read(&ok.keychain_ref).unwrap().as_deref(),
        Some(SECRET)
    );
}

#[test]
fn set_secret_rejects_non_daemon_peer_before_reading_secret() {
    // spec(§15 #7) — getpeereid is connection-scoped: a foreign-uid peer is rejected by the rule-#7 gate
    // FIRST (before any frame is read), so `profile.set_secret` (and thus the keychain store) is unreachable
    // by construction. The handler rejects + disconnects; the keychain stays untouched.
    let (server, client) = UnixStream::pair().unwrap();
    let daemon_uid = 1000u32;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let store = FakeSecretStore::new();

    let outcome = serve_connection(
        server,
        daemon_uid + 1, // a FOREIGN uid
        daemon_uid,
        &path,
        sc_no_deltas(),
        &nexusopsd::runtime::WriteHandle::disconnected(),
        &nexusopsd::decisions::DecisionRegistry::new(),
        &nexusopsd::runtime::InterceptWaitClass::production_default(),
        &sc_fake_github(),
        &nexusopsd::integrations::auth::FakeGhConnector::connected("nexusops/github/test"),
        &store,
    );
    assert!(
        matches!(
            outcome,
            Err(nexusopsd::ipc::IpcError::UnauthorizedPeer { .. })
        ),
        "a foreign-uid peer is rejected before any method (the secret path is behind getpeereid)"
    );
    // the connection disconnected without serving a method → the client sees EOF.
    let mut client = client;
    let mut buf = [0u8; 1];
    assert_eq!(
        client.read(&mut buf).unwrap(),
        0,
        "server disconnected unserved"
    );
    // the keychain store was never touched (no profile.set_secret ran).
    let any = ExecutionProfileId::new();
    assert_eq!(store.read(&profile_keychain_ref(&any)).unwrap(), None);
}

// ---- test doubles + serve_connection plumbing helpers ---------------------------------------------

/// a `SecretStore` whose `store` ALWAYS faults — for the structural-error / no-leak pin (T4).
struct FailingSecretStore;
impl SecretStore for FailingSecretStore {
    fn store(&self, _keychain_ref: &str, _secret: &str) -> Result<(), SecretStoreError> {
        // the error class carries NO secret (the secret is an argument, never the message).
        Err(SecretStoreError::Backend(
            "simulated keychain backend fault".to_string(),
        ))
    }
    fn read(&self, _keychain_ref: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(None)
    }
}

fn sc_no_deltas() -> tokio::sync::broadcast::Sender<nexusops_shared::ipc::ProjectionDelta> {
    tokio::sync::broadcast::channel(1).0
}

fn sc_fake_github() -> nexusopsd::integrations::github::FakeGithubReadClient {
    nexusopsd::integrations::github::FakeGithubReadClient::new(Err(
        nexusopsd::integrations::github::GithubReadError {
            class: nexusopsd::integrations::classifier::IntegrationOutcomeClass::ServerError,
            message: "unused".into(),
        },
    ))
}

// =================================================================================================
// C3 — the audited `profile.set_keychain_ref` pointer-record Gateway action (§15 #4/#8, LESSON §49/§62/§63)
// =================================================================================================

use nexusopsd::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecutionOutcome,
};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::profile_executor::ProfileExecutor;
use nexusopsd::gateway::Gateway;
use nexusopsd::profiles::secret::apply_secret_set;

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::events::ProfileSecretSet;
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequest as ActionRequestStatus;
use nexusops_shared::time::Timestamp;

/// An in-memory [`ProfileLookup`] double (mirrors the session_executor test fake) — a known seeded default
/// + a set of registered ids. Lets the executor's fail-closed-on-unknown gate run with NO real table.
struct FakeProfileLookup {
    default: ExecutionProfileId,
    known: std::collections::HashSet<String>,
}
impl FakeProfileLookup {
    fn with(known: &ExecutionProfileId) -> Self {
        let mut set = std::collections::HashSet::new();
        set.insert(known.as_str().to_string());
        Self {
            default: ExecutionProfileId::new(),
            known: set,
        }
    }
}
impl nexusopsd::profiles::ProfileLookup for FakeProfileLookup {
    fn default_id(&self) -> Result<ExecutionProfileId, nexusopsd::profiles::ProfileError> {
        Ok(self.default.clone())
    }
    fn exists(&self, id: &ExecutionProfileId) -> Result<bool, nexusopsd::profiles::ProfileError> {
        Ok(self.known.contains(id.as_str()))
    }
}

/// a `profile.set_keychain_ref` request: the target profile is the AUDITED resource_ref (NaturalResourceRef);
/// `extra_inputs` lets a test plant a DIVERGENT inputs field (the confused-deputy probe — it MUST be ignored).
fn set_keychain_ref_req(
    profile: &ExecutionProfileId,
    extra_inputs: serde_json::Value,
) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "profile.set_keychain_ref".to_string(),
        requester_type: RequesterType::User, // UI/IPC (§15 #8 — profile mutations are UI-only)
        requester_id: "u_local".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::ExecutionProfile,
            id: profile.as_str().to_string(),
            uri: None,
        }],
        inputs: extra_inputs,
        risk_level: RiskLevel::Level2,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-25T00:00:00Z").unwrap(),
    }
}

/// the emitted ProfileSecretSet's execution_profile_id from a Succeeded outcome (the executor-level pin).
/// The keychain POINTER is daemon-derived (NOT on the event) → recompute `profile_keychain_ref(id)` to check it.
fn emitted_secret_set(outcome: &ExecutionOutcome) -> ExecutionProfileId {
    match outcome {
        ExecutionOutcome::Succeeded {
            emitted_events,
            side_effect_applied,
            ..
        } => {
            assert!(
                !side_effect_applied,
                "registration-only: the row UPDATE + event are both in txn-B (no external side effect)"
            );
            assert_eq!(emitted_events.len(), 1, "exactly one event emitted");
            match &emitted_events[0] {
                EmittedEvent::ProfileSecretSet {
                    execution_profile_id,
                } => execution_profile_id.clone(),
                _ => panic!("expected EmittedEvent::ProfileSecretSet"),
            }
        }
        _ => panic!("expected ExecutionOutcome::Succeeded, got a non-success outcome"),
    }
}

fn approval_id_of(path: &Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("an approval row")
}

fn row_keychain_ref(path: &Path, id: &ExecutionProfileId) -> Option<String> {
    use rusqlite::OptionalExtension as _;
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row(
        "SELECT keychain_ref FROM execution_profiles WHERE execution_profile_id = ?1",
        [id.as_str()],
        |r| r.get::<_, Option<String>>(0),
    )
    .optional()
    .expect("query")
    .flatten()
}

#[test]
fn pointer_record_emits_pointer_only_no_secret() {
    // spec(§15 #4 / LESSON §49) — the executor emits ProfileSecretSet carrying the daemon-derived POINTER;
    // there is NO secret field (the secret is already in the keychain via the trigger; this is the audit).
    let profile = ExecutionProfileId::new();
    let exec = ProfileExecutor::new(Box::new(FakeProfileLookup::with(&profile)));
    let outcome = exec.execute(&set_keychain_ref_req(&profile, serde_json::json!({})));
    let id = emitted_secret_set(&outcome);
    assert_eq!(id, profile, "the event records the target profile id");
    // the POINTER is daemon-derived from that id (recomputed, not on the event) + carries no secret.
    let keychain_ref = profile_keychain_ref(&id);
    assert!(!keychain_ref.contains("sk-") && !keychain_ref.contains("ghp_"));
}

#[test]
fn pointer_record_derives_ref_from_audited_id_not_inputs() {
    // spec(§15 #4 / LESSON §63 confused-deputy) — a DIVERGENT inputs-supplied execution_profile_id /
    // keychain_ref is IGNORED; the executor re-derives the ref from the AUDITED resource_ref id.
    let audited = ExecutionProfileId::new();
    let attacker = ExecutionProfileId::new();
    let exec = ProfileExecutor::new(Box::new(FakeProfileLookup::with(&audited)));
    let req = set_keychain_ref_req(
        &audited,
        serde_json::json!({
            // a malicious caller plants a different profile id + a forged pointer — both MUST be ignored.
            "execution_profile_id": attacker.as_str(),
            "keychain_ref": "nexusops/profile/attacker",
        }),
    );
    let id = emitted_secret_set(&exec.execute(&req));
    assert_eq!(
        id, audited,
        "the recorded id is the AUDITED resource_ref, not inputs"
    );
    assert_ne!(id, attacker, "the forged inputs profile id is IGNORED");
    // the daemon-derived pointer keys off the audited id, never the forged inputs pointer.
    assert_eq!(profile_keychain_ref(&id), profile_keychain_ref(&audited));
}

#[test]
fn pointer_record_unknown_profile_fails_closed() {
    // spec(§15 #8 / LESSON §62) — an UNREGISTERED target profile → Failed (no pointer recorded).
    let registered = ExecutionProfileId::new();
    let unknown = ExecutionProfileId::new();
    let exec = ProfileExecutor::new(Box::new(FakeProfileLookup::with(&registered)));
    let outcome = exec.execute(&set_keychain_ref_req(&unknown, serde_json::json!({})));
    assert!(
        matches!(outcome, ExecutionOutcome::Failed(_)),
        "unknown profile fails closed"
    );
}

#[test]
fn apply_secret_set_updates_canonical_row() {
    // spec(§15 #4 / LESSON §62) — apply_secret_set UPDATEs the CANONICAL execution_profiles.keychain_ref
    // (the row is the source of truth, not a projection). Driven inside a gateway_txn (the txn-B home).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);
    let id = register_profile(&mut store);
    assert_eq!(
        row_keychain_ref(&path, &id),
        None,
        "seeded with no secret (keychain_ref NULL)"
    );

    let kref = profile_keychain_ref(&id);
    store
        .gateway_txn(|gtx| apply_secret_set(gtx, &id))
        .expect("apply_secret_set commits");
    assert_eq!(
        row_keychain_ref(&path, &id),
        Some(kref),
        "the canonical row now carries the keychain POINTER"
    );
}

#[test]
fn rebuild_preserves_keychain_ref() {
    // spec(LESSON §62 — the inverse of the §48 rebuild-equivalence test) — execution_profiles is CANONICAL
    // (NOT in REBUILD_TABLES), so a rebuild() must NOT revert the keychain_ref the pointer-record UPDATEd.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);
    let id = register_profile(&mut store);
    let kref = profile_keychain_ref(&id);
    store
        .gateway_txn(|gtx| apply_secret_set(gtx, &id))
        .expect("apply");

    store.rebuild_projections().expect("rebuild");

    assert_eq!(
        row_keychain_ref(&path, &id),
        Some(kref),
        "rebuild does NOT clear the canonical execution_profiles.keychain_ref (LESSON §62)"
    );
}

#[test]
fn pointer_record_txn_fault_rolls_back_row_and_event() {
    // spec(§15 #5 / LESSON §16 fail-closed) — within ONE gateway_txn (the txn-B home), the row UPDATE + the
    // ProfileSecretSet append are ATOMIC: a fault rolls back BOTH (no half-written keychain_ref, no event).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);
    let id = register_profile(&mut store);

    // a gateway_txn that applies the pointer record THEN faults → the whole txn rolls back.
    let r: Result<(), nexusopsd::eventstore::EventStoreError> = store.gateway_txn(|gtx| {
        apply_secret_set(gtx, &id)?;
        // model a txn-B audit-write fault AFTER the row UPDATE (e.g. the ActionSucceeded append failing).
        Err(nexusopsd::eventstore::EventStoreError::RedactionRequired)
    });
    assert!(r.is_err(), "the injected txn-B fault propagates");

    // BOTH rolled back: the row's keychain_ref is still NULL.
    assert_eq!(
        row_keychain_ref(&path, &id),
        None,
        "a txn fault rolls back the row UPDATE — no half-written keychain_ref (fail-closed)"
    );
}

#[test]
fn profile_set_keychain_ref_through_gateway_records_pointer_and_audits() {
    // spec(§52 production-path / §15 #4) — the FULL pipeline: submit + approve a risk-2 profile.set_keychain_ref
    // through the gateway → the ProfileExecutor runs in txn-B → the canonical row keychain_ref is UPDATEd AND a
    // ProfileSecretSet audit event (pointer-only, NO secret) is appended ATOMIC with ActionSucceeded.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);
    let id = register_profile(&mut store);

    let mut catalog = CatalogExecutor::new();
    catalog.register(
        nexusops_shared::catalog::ExecutorKind::Profile,
        std::sync::Arc::new(ProfileExecutor::new(Box::new(FakeProfileLookup::with(&id)))),
    );
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(catalog));

    // risk-2 → submit lands awaiting_approval; approve drives execution.
    let ack = gw
        .submit_action(&mut store, set_keychain_ref_req(&id, serde_json::json!({})))
        .expect("submit");
    assert_eq!(ack.status, ActionRequestStatus::AwaitingApproval);
    let final_ack = gw
        .approve(&mut store, &approval_id_of(&path))
        .expect("approve → execute");
    assert_eq!(final_ack.status, ActionRequestStatus::Succeeded);

    // the canonical row now carries the POINTER (recorded via the pipeline txn-B apply_secret_set).
    assert_eq!(
        row_keychain_ref(&path, &id),
        Some(profile_keychain_ref(&id))
    );

    // a ProfileSecretSet audit event was appended — pointer-only, NO secret.
    let events = store.read_all().unwrap();
    let ev = events
        .iter()
        .find(|e| e.event_type == ProfileSecretSet::EVENT_TYPE)
        .expect("a ProfileSecretSet event was appended");
    assert!(
        !ev.payload_json.contains(SECRET),
        "the audit event carries no secret"
    );
    // the event records the target profile id (the keychain POINTER is daemon-derived, not on the event).
    let payload: ProfileSecretSet = serde_json::from_str(&ev.payload_json).unwrap();
    assert_eq!(payload.execution_profile_id, id);
}
