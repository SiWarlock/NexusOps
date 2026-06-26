//! `integrations/auth.rs` (P4.7/083) — the GitHub auth-bootstrap (the (Y) gh-reuse interim).
//!
//! The daemon SOURCES the token itself — it reads the user's existing `gh auth token` (a DAEMON-side
//! shell-out) and copies it into the OS keychain ([`SecretStore`]); there is NO inbound-IPC secret (the
//! UI "Connect via gh" is a trigger that carries NO token — the LESSON §49 posture). Everything
//! downstream selects the token PER RESOLVED ACCOUNT via a structured `keychain_ref` (the confused-deputy
//! axis, post-082): a write to repo X uses X's account token, NEVER a single global token.
//!
//! **The single live-writes gate ([`resolve_authed_token`]):** BOTH the authed READ client and the authed
//! WRITE client consult this ONE function; with the per-connection `live_writes_enabled` toggle OFF it
//! returns `None` (the client stays unauth → the existing typed `AuthFailed`/401 path — fail-closed),
//! regardless of whether a token is present. No authed call (read OR write) goes live until the toggle is
//! ON (post the mandatory in-arc security re-review — it certifies the cross-account read-path
//! confused-deputy too, LESSON §59). Public-repo reads are unaffected (they never hold a token).
//!
//! Deterministic core (`keychain_ref_for` / `acquire_via_gh` / `resolve_authed_token`) is unit-tested via
//! the injected [`FakeSecretStore`] + [`FakeGhTokenSource`] seams (`tests/keychain.rs`); the LIVE
//! `gh`-shell-out ([`GhCliTokenSource`]) is the non-deterministic edge (mechanism-first, no unit test).

use std::process::Command;
use std::sync::Arc;

use zeroize::Zeroizing;

use nexusops_shared::events::Provider;

use super::keychain::{SecretStore, SecretStoreError};

/// An auth-bootstrap failure. `GhUnavailable` = the user has no usable `gh` login (not installed / not
/// authed / empty token) → the typed "device-flow (084) is your path" signal (NEVER a panic, NEVER a
/// partial connection). `Keychain` wraps a [`SecretStoreError`] from the copy-to-keychain write. No
/// variant carries the secret.
#[derive(Debug)]
pub enum AuthError {
    /// `gh` is not installed / not authenticated / returned an empty token — device-flow is the path.
    GhUnavailable,
    /// the copy-to-keychain write failed (a structural backend class, never the token).
    Keychain(SecretStoreError),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::GhUnavailable => write!(
                f,
                "gh CLI is unavailable or not authenticated (use the device-flow connect)"
            ),
            AuthError::Keychain(e) => write!(f, "keychain write failed: {e}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// The provider's keychain-ref slug (the closed `Provider` wire vocabulary — github | linear).
fn provider_slug(provider: Provider) -> &'static str {
    match provider {
        Provider::Github => "github",
        Provider::Linear => "linear",
    }
}

/// The structured per-account keychain POINTER (§15 #4): `nexusops/<provider>/<account>`. Per-account
/// keyed so token SELECTION is by the 082-resolved account (the confused-deputy pin — a different
/// account's ref reads a different entry, never a shared global token). A POINTER only — never the token.
pub fn keychain_ref_for(provider: Provider, account: &str) -> String {
    format!("nexusops/{}/{}", provider_slug(provider), account)
}

/// Reads a provider access token DAEMON-side (never inbound over IPC). The prod impl shells `gh auth
/// token`; the test fake returns a canned token / `GhUnavailable`. `Send + Sync` for sharing into the
/// live "Connect via gh" path. The token is returned in a [`Zeroizing`] wrapper so its heap allocation is
/// scrubbed on drop (§15 — the transient is not left in freed memory after the keychain write).
pub trait GhTokenSource: Send + Sync {
    /// The user's current `gh` token, transiently — `Err(AuthError::GhUnavailable)` when `gh` has none.
    fn token(&self) -> Result<Zeroizing<String>, AuthError>;
}

/// The LIVE `gh auth token` source (the (Y) interim — `gh` owns its own token refresh; the daemon only
/// READS the current token). Non-deterministic edge (shells out) — no unit test; mechanism-first.
pub struct GhCliTokenSource;

impl GhTokenSource for GhCliTokenSource {
    fn token(&self) -> Result<Zeroizing<String>, AuthError> {
        // `gh auth token` prints the active token to stdout (and exits non-zero if not logged in). Any
        // spawn/exit/empty-output failure → GhUnavailable (the device-flow signal); we NEVER log stdout
        // (it is the secret). Trim the trailing newline; an empty trimmed token is "no usable login".
        // The stdout buffer + the trimmed token both ride Zeroizing so the plaintext is scrubbed on drop.
        let out = Command::new("gh")
            .args(["auth", "token"])
            .output()
            .map_err(|_| AuthError::GhUnavailable)?;
        if !out.status.success() {
            return Err(AuthError::GhUnavailable);
        }
        let raw =
            Zeroizing::new(String::from_utf8(out.stdout).map_err(|_| AuthError::GhUnavailable)?);
        let token = Zeroizing::new(raw.trim().to_string());
        if token.is_empty() {
            return Err(AuthError::GhUnavailable);
        }
        Ok(token)
    }
}

/// The (Y) gh-reuse acquisition: read the `gh` token TRANSIENTLY → copy it into the keychain under the
/// per-account [`keychain_ref_for`] → return the `keychain_ref` POINTER (the caller then registers the
/// connection with the pointer; the token NEVER enters an event/row). The keychain is the ONLY place the
/// secret lands. `gh`-absent → `Err(GhUnavailable)` BEFORE any keychain write → the keychain is left
/// untouched (no partial connection). The transient `token` is dropped at end of scope.
pub fn acquire_via_gh(
    store: &dyn SecretStore,
    gh: &dyn GhTokenSource,
    provider: Provider,
    account: &str,
) -> Result<String, AuthError> {
    // the token rides Zeroizing — its heap allocation is scrubbed when this scope ends (after the keychain
    // write), so the plaintext is not left in freed memory (§15 "zeroized after the keychain write").
    let token = gh.token()?; // gh-absent fails here — keychain untouched (no partial connection).
    let keychain_ref = keychain_ref_for(provider, account);
    store
        .store(&keychain_ref, &token)
        .map_err(AuthError::Keychain)?;
    Ok(keychain_ref) // the POINTER — never the token (the caller emits this into the registration event).
}

/// The SINGLE live-writes authorization gate consulted by BOTH the authed read + write clients (Q4 — one
/// gate). Returns the per-account token IFF the connection's `live_writes_enabled` toggle is ON AND a
/// token is present for the 082-resolved account; otherwise `None` (fail-closed unauth — the client falls
/// back to the existing typed `AuthFailed`/401 path, never a fabricated success):
///  * toggle OFF → `None` even with a token present (the post-re-review live-enablement gate; default OFF);
///  * toggle ON, account has a token → `Some(token)` (per-account — a divergent account reads its own
///    entry, NEVER another account's token: the confused-deputy pin, post-082);
///  * toggle ON, no token for the account → `None` (fail-closed; never another account's token).
pub fn resolve_authed_token(
    store: &dyn SecretStore,
    live_writes_enabled: bool,
    provider: Provider,
    account: &str,
) -> Result<Option<String>, AuthError> {
    if !live_writes_enabled {
        // the live-enablement gate: no authed token (read OR write) until the toggle is ON, even if one
        // is stored. Public-repo reads keep working unauth (they never reach for a token).
        return Ok(None);
    }
    let keychain_ref = keychain_ref_for(provider, account);
    store.read(&keychain_ref).map_err(AuthError::Keychain)
}

/// The per-connection live-writes gate read seam: whether live writes are ENABLED for the connection of
/// `(provider, account)` — reads `proj_integration_connection.live_writes_enabled`. Injected so the
/// decision is deterministically testable; the production impl ([`super::connections::SqliteLiveWritesGate`])
/// reads read-only WAL. A missing connection / a read fault → `false` (fail-closed — never default-ON).
pub trait LiveWritesGate: Send + Sync {
    fn is_enabled_for_account(&self, provider: Provider, account: &str) -> bool;
}

/// Resolves the per-account GitHub auth handle for a live call (the C3 CONSUMER of the keychain token +
/// the toggle). Composes [`SecretStore`] (the per-account token) + [`LiveWritesGate`] (the toggle) via the
/// single [`resolve_authed_token`] gate, then builds an `octocrab` handle. The DECISION (token-selected?)
/// is deterministic + seamed; the octocrab BUILD is the non-deterministic network edge (no unit test).
pub struct GithubAuthResolver {
    store: Arc<dyn SecretStore>,
    gate: Arc<dyn LiveWritesGate>,
}

impl GithubAuthResolver {
    pub fn new(store: Arc<dyn SecretStore>, gate: Arc<dyn LiveWritesGate>) -> Self {
        Self { store, gate }
    }

    /// The deterministic, unit-testable half: would a live call to a repo owned by `owner` be AUTHED?
    /// `Some(token)` iff the connection's toggle is ON AND a per-account token is present (the
    /// confused-deputy-safe per-`owner` selection); else `None` (toggle OFF / no token / read fault →
    /// fail-closed unauth — the existing 401→AuthFailed path). The token never logs.
    pub fn resolve_token_for(&self, owner: &str) -> Option<String> {
        let enabled = self.gate.is_enabled_for_account(Provider::Github, owner);
        // a keychain read fault → None (fail-closed unauth), never a panic / never another account's token.
        resolve_authed_token(self.store.as_ref(), enabled, Provider::Github, owner)
            .ok()
            .flatten()
    }

    /// Build an `octocrab` for a call to a repo owned by `owner`: AUTHED with the per-account keychain
    /// token when [`resolve_token_for`] yields one; else UNAUTHENTICATED (fail-closed — the existing
    /// unauth client behavior). A builder fault also degrades to unauth (never breaks the call). The live
    /// HTTP round-trip is the non-deterministic edge — only this build branch is exercised by tests (via
    /// [`resolve_token_for`]); the authed handle's network behavior is fixture/manual-verified.
    pub fn octocrab_for(&self, owner: &str) -> octocrab::Octocrab {
        match self.resolve_token_for(owner) {
            Some(token) => octocrab::Octocrab::builder()
                .personal_token(token)
                .build()
                // a build fault → unauth fail-closed (degrade availability, never break the invariant).
                .unwrap_or_default(),
            None => octocrab::Octocrab::default(),
        }
    }
}

/// The "Connect via gh" daemon-side connector (the IPC trigger's worker, P4.7/083 C3b): acquire the `gh`
/// token DAEMON-side → the OS keychain → return the `keychain_ref` POINTER, or a typed [`AuthError`]
/// (gh-absent → the device-flow signal). Injected so the IPC dispatch is test-seamed; the prod impl wires
/// [`KeyringSecretStore`](super::keychain::KeyringSecretStore) + [`GhCliTokenSource`].
pub trait GhConnector: Send + Sync {
    fn connect(&self, provider: Provider, account: &str) -> Result<String, AuthError>;
}

/// The production [`GhConnector`]: `gh auth token` ([`GhCliTokenSource`]) → the OS keychain (the SAME
/// [`SecretStore`] the github clients read, so the token the trigger writes is the one the clients use).
/// NO token over IPC — the daemon SOURCES it. Shells `gh` (the non-deterministic edge).
pub struct GhCliConnector {
    store: Arc<dyn SecretStore>,
}

impl GhCliConnector {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self { store }
    }
}

impl GhConnector for GhCliConnector {
    fn connect(&self, provider: Provider, account: &str) -> Result<String, AuthError> {
        acquire_via_gh(self.store.as_ref(), &GhCliTokenSource, provider, account)
    }
}

/// A test [`GhTokenSource`] — `ok(token)` returns it; `unavailable()` simulates a missing/failed `gh`
/// login (the device-flow path). Test-support double (the `FakeSecretStore` sibling) — gated OUT of the
/// production binary (`cfg(test)` for the lib's own units, `test-support` for the integration tests).
#[cfg(any(test, feature = "test-support"))]
pub struct FakeGhTokenSource {
    token: Option<String>,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeGhTokenSource {
    /// A source that yields `token` (a usable `gh` login).
    pub fn ok(token: &str) -> Self {
        Self {
            token: Some(token.to_string()),
        }
    }

    /// A source with no usable login (`gh` not installed / not authed) → `GhUnavailable`.
    pub fn unavailable() -> Self {
        Self { token: None }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl GhTokenSource for FakeGhTokenSource {
    fn token(&self) -> Result<Zeroizing<String>, AuthError> {
        self.token
            .clone()
            .map(Zeroizing::new)
            .ok_or(AuthError::GhUnavailable)
    }
}

/// The canned outcome a [`FakeGhConnector`] returns.
#[cfg(any(test, feature = "test-support"))]
enum FakeConnectOutcome {
    Connected(String),
    GhUnavailable,
    KeychainFault,
}

/// A test [`GhConnector`] — `connected(ref)` returns the keychain_ref; `gh_unavailable()` returns the
/// device-flow signal; `keychain_fault()` simulates a backend write failure. The IPC tests inject it
/// (most only to satisfy the serve/dispatch signature).
#[cfg(any(test, feature = "test-support"))]
pub struct FakeGhConnector {
    outcome: FakeConnectOutcome,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeGhConnector {
    /// A connector that "connects" → returns `keychain_ref` (the POINTER).
    pub fn connected(keychain_ref: &str) -> Self {
        Self {
            outcome: FakeConnectOutcome::Connected(keychain_ref.to_string()),
        }
    }

    /// A connector with no usable `gh` login → `GhUnavailable` (the device-flow path).
    pub fn gh_unavailable() -> Self {
        Self {
            outcome: FakeConnectOutcome::GhUnavailable,
        }
    }

    /// A connector whose keychain WRITE fails → `AuthError::Keychain` (the IPC trigger maps this to
    /// `internal_error`, never leaking the token).
    pub fn keychain_fault() -> Self {
        Self {
            outcome: FakeConnectOutcome::KeychainFault,
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl GhConnector for FakeGhConnector {
    fn connect(&self, _provider: Provider, _account: &str) -> Result<String, AuthError> {
        match &self.outcome {
            FakeConnectOutcome::Connected(r) => Ok(r.clone()),
            FakeConnectOutcome::GhUnavailable => Err(AuthError::GhUnavailable),
            FakeConnectOutcome::KeychainFault => Err(AuthError::Keychain(
                SecretStoreError::Backend("fake keychain fault".to_string()),
            )),
        }
    }
}
