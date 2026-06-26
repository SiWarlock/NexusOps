//! P5.3b/085 — piece 2: the startup keychain self-test (§5.1 `misconfigured`). The pure `self_test_status`
//! classifier (LESSON §36/§41 pure-classifier family) + the cold-start `run_profile_self_test_pass` over the
//! injected `FakeSecretStore` seam (NO real keychain). A configured `keychain_ref` whose entry doesn't resolve
//! → `misconfigured`; a profile with NO ref → ambient-auth (NOT misconfigured).

use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::idgen::UlidGen;
use nexusopsd::integrations::keychain::{FakeSecretStore, SecretStore};
use nexusopsd::profiles::secret::{
    run_profile_self_test_pass, self_test_status, KeychainReadOutcome,
};
use nexusopsd::profiles::{self, ProfileSpec};

use nexusops_shared::ids::{ExecutionProfileId, WorkspaceId};
use nexusops_shared::status::ExecutionProfile;

const OCCURRED_AT: &str = "2026-06-25T00:00:00Z";

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new(OCCURRED_AT)),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// register a profile with an explicit `keychain_ref` (a NON-ulid test ref so the §15 row-redactor doesn't
/// mask it — the production ref is set later via apply_secret_set; here we control the row directly).
fn register_with_ref(store: &mut EventStore, keychain_ref: Option<&str>) -> ExecutionProfileId {
    let id = ExecutionProfileId::new();
    let spec = ProfileSpec {
        workspace_id: WorkspaceId::system(),
        provider: "anthropic".to_string(),
        harness: "claude_code".to_string(),
        model: None,
        account_alias: None,
        keychain_ref: keychain_ref.map(|s| s.to_string()),
        usage_policy_json: None,
        status: ExecutionProfile::Available,
    };
    profiles::register_profile(store, &id, &spec, OCCURRED_AT).expect("register");
    id
}

// ---- the pure self_test_status classifier (§5.1) -------------------------------------------------

#[test]
fn self_test_none_ref_is_available_not_misconfigured() {
    // spec(§5.1) — a profile with NO configured secret (keychain_ref = None) is ambient-auth → the self-test
    // contributes NOTHING (None), never `misconfigured` (the seeded default's path).
    assert_eq!(self_test_status(false, None), None);
    assert_eq!(
        self_test_status(false, Some(KeychainReadOutcome::Resolved)),
        None
    );
}

#[test]
fn self_test_unresolvable_or_fault_is_misconfigured() {
    // spec(§5.1) — a CONFIGURED keychain_ref whose entry does NOT resolve (Ok(None)) or whose read FAULTS
    // → `misconfigured`; a configured ref that resolves → healthy (None — no problem surfaced).
    assert_eq!(
        self_test_status(true, Some(KeychainReadOutcome::Unresolved)),
        Some(ExecutionProfile::Misconfigured)
    );
    assert_eq!(
        self_test_status(true, Some(KeychainReadOutcome::BackendFault)),
        Some(ExecutionProfile::Misconfigured)
    );
    assert_eq!(
        self_test_status(true, Some(KeychainReadOutcome::Resolved)),
        None
    );
}

// ---- the cold-start pass over a FakeSecretStore --------------------------------------------------

#[test]
fn self_test_pass_marks_missing_entry_misconfigured_and_present_available() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);
    // (a) a profile with a configured ref whose keychain entry IS present → available (resolved).
    let healthy = register_with_ref(&mut store, Some("nexusops/profile/test-healthy"));
    // (b) a profile with a configured ref whose keychain entry is ABSENT → misconfigured.
    let broken = register_with_ref(&mut store, Some("nexusops/profile/test-broken"));
    // (c) a profile with NO configured ref → ambient-auth (available, not misconfigured).
    let ambient = register_with_ref(&mut store, None);

    let keychain = FakeSecretStore::new();
    keychain
        .store("nexusops/profile/test-healthy", "x")
        .expect("seed the healthy entry"); // ONLY the healthy ref resolves.

    let statuses = run_profile_self_test_pass(&path, &keychain).expect("pass runs");
    let lookup = |id: &ExecutionProfileId| {
        statuses
            .iter()
            .find(|(i, _)| i == id)
            .map(|(_, s)| *s)
            .expect("profile in the pass result")
    };
    assert_eq!(
        lookup(&broken),
        ExecutionProfile::Misconfigured,
        "absent entry → misconfigured"
    );
    assert_eq!(
        lookup(&ambient),
        ExecutionProfile::Available,
        "no ref → ambient-auth (available)"
    );
    assert_eq!(
        lookup(&healthy),
        ExecutionProfile::Available,
        "present entry → available (the self-test surfaces no problem)"
    );
}
