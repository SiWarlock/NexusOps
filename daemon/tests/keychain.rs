//! P4.7 (083) — the GitHub auth-bootstrap foundation + the (Y) gh-reuse interim. cat-1 / secret-touching.
//!
//! Deterministic seams (CLAUDE.md: real keychain / real `gh` shell-out are non-deterministic edges →
//! injected fakes): `SecretStore` (the keychain-write primitive; prod `KeyringSecretStore`, test
//! `FakeSecretStore`) + `GhTokenSource` (the `gh auth token` reader; prod `GhCliTokenSource`, test fake).
//! These pin the §15 #3/#4 invariants (no secret in events/rows/logs; keychain-ref-only), the gh-reuse
//! acquisition, gh-absent handling, per-account token selection (the confused-deputy axis, post-082),
//! fail-closed-on-no-token, and the live-writes toggle gate.

use std::sync::Arc;

use nexusops_shared::events::{IntegrationConnectionRegistered, Provider};
use nexusopsd::integrations::auth::{
    acquire_via_gh, keychain_ref_for, resolve_authed_token, AuthError, FakeGhTokenSource,
    GithubAuthResolver, LiveWritesGate,
};
use nexusopsd::integrations::keychain::{FakeSecretStore, SecretStore};

const GH_TOKEN: &str = "ghp_0123456789abcdefghijklmnopqrstuvwxyzABCD";

/// A fake [`LiveWritesGate`] — reports a fixed toggle for every account (the deterministic seam for the
/// C3 `GithubAuthResolver::resolve_token_for` decision; the SqliteLiveWritesGate is the live read edge).
struct FakeLiveWritesGate {
    enabled: bool,
}

impl LiveWritesGate for FakeLiveWritesGate {
    fn is_enabled_for_account(&self, _provider: Provider, _account: &str) -> bool {
        self.enabled
    }
}

/// Build a `GithubAuthResolver` over a [`FakeSecretStore`] (pre-seeded) + a fixed-toggle gate.
fn resolver(store: Arc<FakeSecretStore>, toggle_enabled: bool) -> GithubAuthResolver {
    GithubAuthResolver::new(
        store,
        Arc::new(FakeLiveWritesGate {
            enabled: toggle_enabled,
        }),
    )
}

#[test]
fn keychain_store_then_read_roundtrips() {
    // spec(§15 #4) — the keychain-write primitive: a stored secret reads back by the same keychain_ref;
    // an unknown ref reads None (fail-fast, never a wrong secret).
    let store = FakeSecretStore::new();
    let r = keychain_ref_for(Provider::Github, "octocat");
    store.store(&r, GH_TOKEN).expect("store");
    assert_eq!(store.read(&r).expect("read").as_deref(), Some(GH_TOKEN));
    assert_eq!(
        store.read("nexusops/github/nobody").expect("read"),
        None,
        "an unknown keychain_ref reads None"
    );
}

#[test]
fn keychain_store_emits_ref_only_no_secret() {
    // spec(§15 #3/#4) — the acquisition's audit/registration record (IntegrationConnectionRegistered)
    // carries ONLY the keychain_ref POINTER; the token NEVER appears in the serialized event payload.
    let store = FakeSecretStore::new();
    let gh = FakeGhTokenSource::ok(GH_TOKEN);
    let keychain_ref =
        acquire_via_gh(&store, &gh, Provider::Github, "octocat").expect("gh-reuse acquire");
    let payload = IntegrationConnectionRegistered {
        connection_id: "conn_gh_octocat".to_string(),
        provider: Provider::Github,
        keychain_ref: keychain_ref.clone(),
        account: Some("octocat".to_string()),
    };
    let json = serde_json::to_string(&payload).expect("serialize");
    assert!(
        !json.contains(GH_TOKEN),
        "§15 #3 — the token must NEVER appear in the registration event payload"
    );
    assert!(
        json.contains(&keychain_ref),
        "§15 #4 — the event carries the keychain_ref POINTER"
    );
    assert!(
        !keychain_ref.contains(GH_TOKEN),
        "§15 #4 — the keychain_ref is a pointer, not the token"
    );
}

#[test]
fn gh_reuse_acquires_token_to_keychain() {
    // spec(§9 / §15 — the (Y) interim): the gh-reuse path reads the gh token (transient) → stores it in
    // the keychain under the per-account ref → returns the ref. The keychain now holds the token; the
    // returned ref does not.
    let store = FakeSecretStore::new();
    let gh = FakeGhTokenSource::ok(GH_TOKEN);
    let keychain_ref =
        acquire_via_gh(&store, &gh, Provider::Github, "octocat").expect("gh-reuse acquire");
    assert_eq!(
        keychain_ref,
        keychain_ref_for(Provider::Github, "octocat"),
        "the per-account keychain_ref is structured (service/provider/account)"
    );
    assert_eq!(
        store.read(&keychain_ref).expect("read").as_deref(),
        Some(GH_TOKEN),
        "the gh token is now in the keychain under the per-account ref"
    );
}

#[test]
fn gh_absent_yields_typed_error() {
    // spec(§9 — gh-absent handling): gh not installed / not authed → a typed structural error (the
    // "device-flow is your path" signal), NEVER a panic, NEVER a partial connection (the keychain stays
    // untouched). Confirms gh-absent IS cleanly handleable → device-flow (084) stays the fast-follow.
    let store = FakeSecretStore::new();
    let gh = FakeGhTokenSource::unavailable();
    let err = acquire_via_gh(&store, &gh, Provider::Github, "octocat")
        .expect_err("gh-absent → typed error");
    assert!(
        matches!(err, AuthError::GhUnavailable),
        "gh-absent is the typed AuthError::GhUnavailable, not a panic/partial"
    );
    assert_eq!(
        store
            .read(&keychain_ref_for(Provider::Github, "octocat"))
            .expect("read"),
        None,
        "gh-absent leaves the keychain untouched (no partial connection)"
    );
}

#[test]
fn authed_call_uses_resolved_account_token() {
    // spec(confused-deputy, post-082): the token for an authed call is the RESOLVED account's token — a
    // different account's token is NOT used. account X's ref → X's token; account Y (no token) → None.
    let store = FakeSecretStore::new();
    store
        .store(&keychain_ref_for(Provider::Github, "acme"), GH_TOKEN)
        .expect("store acme token");
    // a call to acme's repo (toggle ON) resolves acme's token...
    assert_eq!(
        resolve_authed_token(&store, true, Provider::Github, "acme")
            .expect("resolve")
            .as_deref(),
        Some(GH_TOKEN),
        "the resolved account's token is used"
    );
    // ...and a DIFFERENT account (other-corp, no stored token) does NOT get acme's token.
    assert_eq!(
        resolve_authed_token(&store, true, Provider::Github, "other-corp").expect("resolve"),
        None,
        "a divergent account does NOT reuse acme's token (no global token)"
    );
}

#[test]
fn no_token_fails_closed() {
    // spec(fail-closed): no token for the resolved account (toggle ON) → None → the caller stays on the
    // existing unauth/AuthFailed path, NEVER a silent success.
    let store = FakeSecretStore::new();
    assert_eq!(
        resolve_authed_token(&store, true, Provider::Github, "octocat").expect("resolve"),
        None,
        "no stored token → None (fail-closed unauth), never a fabricated success"
    );
}

#[test]
fn live_writes_toggle_off_blocks_authed_write() {
    // spec(the live-enablement gate): live_writes_enabled OFF → the authed token is NOT resolved (the
    // WRITE stays fail-closed/unauth) EVEN when a token is present; ON → it resolves. Default OFF is the
    // post-re-review gate.
    let store = FakeSecretStore::new();
    store
        .store(&keychain_ref_for(Provider::Github, "acme"), GH_TOKEN)
        .expect("store");
    assert_eq!(
        resolve_authed_token(&store, false, Provider::Github, "acme").expect("resolve"),
        None,
        "toggle OFF → no authed write token even with a token present (fail-closed)"
    );
    assert_eq!(
        resolve_authed_token(&store, true, Provider::Github, "acme")
            .expect("resolve")
            .as_deref(),
        Some(GH_TOKEN),
        "toggle ON → the authed write token resolves"
    );
}

#[test]
fn read_also_gated_on_toggle() {
    // spec(Q4 TWEAK — single gate): the authed READ path (get_pr_diff/sync on private repos) is gated on
    // the SAME toggle as writes — NO authed call (read OR write) goes live until the toggle is ON
    // (post-re-review certifies the cross-account confused-deputy on the read path too, LESSON §59).
    // `resolve_authed_token` is the ONE gate both client builds consult; OFF → None even with a token.
    let store = FakeSecretStore::new();
    store
        .store(&keychain_ref_for(Provider::Github, "acme"), GH_TOKEN)
        .expect("store");
    // toggle OFF → the read client gets no token (public-repo reads still work unauth; private fail-closed).
    assert_eq!(
        resolve_authed_token(&store, false, Provider::Github, "acme").expect("resolve"),
        None,
        "toggle OFF → the authed READ path is gated too (no token), not just writes"
    );
    // toggle ON → both read + write resolve the same per-account token.
    assert_eq!(
        resolve_authed_token(&store, true, Provider::Github, "acme")
            .expect("resolve")
            .as_deref(),
        Some(GH_TOKEN),
        "toggle ON → the authed read path resolves the per-account token"
    );
}

// ---- P4.7 (083 C3) — the GithubAuthResolver decision seam (account-selection + toggle-gate + fail-closed)

#[test]
fn resolver_authes_per_account_when_toggle_on() {
    // spec(C3 / confused-deputy): the resolver's deterministic decision selects the token PER OWNER —
    // a call to a repo owned by `acme` resolves acme's token; a DIFFERENT owner (no token) resolves None
    // (never acme's token). This is the live client's authed-vs-unauth decision (the octocrab build is the
    // non-deterministic edge; only this decision is unit-pinned).
    let store = Arc::new(FakeSecretStore::new());
    store
        .store(&keychain_ref_for(Provider::Github, "acme"), GH_TOKEN)
        .expect("store");
    let r = resolver(store, true);
    assert_eq!(
        r.resolve_token_for("acme").as_deref(),
        Some(GH_TOKEN),
        "toggle ON + acme has a token → the call to acme's repo is authed with acme's token"
    );
    assert_eq!(
        r.resolve_token_for("other-corp"),
        None,
        "a divergent owner has no token → unauth (never acme's token — confused-deputy-safe)"
    );
}

#[test]
fn resolver_unauth_when_toggle_off_or_no_token() {
    // spec(C3 / fail-closed): the resolver yields None (→ the client stays UNAUTH, the existing
    // 401→AuthFailed path) when the toggle is OFF (even with a token present) OR no token is stored.
    let store = Arc::new(FakeSecretStore::new());
    store
        .store(&keychain_ref_for(Provider::Github, "acme"), GH_TOKEN)
        .expect("store");
    assert_eq!(
        resolver(store.clone(), false).resolve_token_for("acme"),
        None,
        "toggle OFF → unauth even with a token present (the default-OFF live-enablement gate)"
    );

    let empty = Arc::new(FakeSecretStore::new());
    assert_eq!(
        resolver(empty, true).resolve_token_for("acme"),
        None,
        "toggle ON but no token for the owner → unauth (fail-closed, never a fabricated auth)"
    );
}
