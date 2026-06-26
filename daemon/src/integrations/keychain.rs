//! `integrations/keychain.rs` (P4.7/083) — the **keychain-write primitive** (cat-1 / secret-touching).
//!
//! The daemon's FIRST secret-write surface. A [`SecretStore`] stores/reads a secret by an opaque
//! `keychain_ref` POINTER; the secret lives SOLELY in the OS keychain and is NEVER written to an event,
//! a DB row, an IPC response, or a log line (ARCHITECTURE §15 #3/#4). The secret enters the daemon
//! DAEMON-side ONLY (a `gh auth token` shell-out / an outbound OAuth poll — NEVER inbound across IPC,
//! the LESSON §49 posture) and goes straight to the keychain; everything downstream carries only the
//! `keychain_ref` (the LESSON §62 / §16 dual-gate pattern applied to the keychain instead of a DB row).
//!
//! **Two impls (the OctocrabGithubReadClient precedent — a deterministic core + a non-deterministic
//! edge):**
//!  * [`FakeSecretStore`] — an in-memory store; the DETERMINISTIC seam every unit test injects
//!    (`tests/keychain.rs`), so the §15 invariants + the gh-reuse acquisition + the toggle gate are all
//!    pinned test-first WITHOUT a real keychain.
//!  * [`KeyringSecretStore`] — the LIVE store over the OS keychain (the `keyring` crate; on macOS the
//!    Apple Keychain). The keychain round-trip is the fake-covered NON-DETERMINISTIC edge — no unit test
//!    (it needs a real keychain + entitlements); it ships mechanism-first (the live "Connect via gh"
//!    path drives it). NEVER logs a secret.

// only the test-only FakeSecretStore needs these (gated out of the production binary).
#[cfg(any(test, feature = "test-support"))]
use std::collections::HashMap;
#[cfg(any(test, feature = "test-support"))]
use std::sync::Mutex;

/// A keychain-write/read failure. Carries ONLY a STRUCTURAL reason (never the secret, never the
/// underlying store's raw message verbatim if it could echo a value) — it may surface to a log/error.
#[derive(Debug)]
pub enum SecretStoreError {
    /// the underlying OS keychain backend failed (open/store/read) — a structural class, no secret.
    Backend(String),
}

impl std::fmt::Display for SecretStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretStoreError::Backend(why) => write!(f, "keychain backend error: {why}"),
        }
    }
}

impl std::error::Error for SecretStoreError {}

/// The keychain-write primitive (§15 #4): store/read a secret by an opaque `keychain_ref` POINTER. The
/// secret crosses this trait ONCE (in) and is read back ONCE (out); it lives ONLY in the backing store.
/// `Send + Sync` so it can be shared (`Arc<dyn SecretStore>`) into the github clients + the gh-reuse path.
pub trait SecretStore: Send + Sync {
    /// Store `secret` under `keychain_ref` (overwrites an existing entry for the same ref). The secret
    /// is NEVER logged or echoed.
    fn store(&self, keychain_ref: &str, secret: &str) -> Result<(), SecretStoreError>;
    /// Read the secret stored under `keychain_ref`. `Ok(None)` when no entry exists (fail-fast — never
    /// a wrong/another secret); `Err` only on a backend fault.
    fn read(&self, keychain_ref: &str) -> Result<Option<String>, SecretStoreError>;
}

/// An in-memory [`SecretStore`] — the deterministic test seam. NOT for production (a process-lifetime
/// map, no OS keychain) → gated OUT of the production binary (`cfg(test)` for the lib's own units,
/// `test-support` for the integration tests; never `cargo build`/release — a secret must never live in
/// a volatile map on the live path). `Mutex` so `&self` store/read match the trait (interior mutability).
#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
pub struct FakeSecretStore {
    entries: Mutex<HashMap<String, String>>,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeSecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl SecretStore for FakeSecretStore {
    fn store(&self, keychain_ref: &str, secret: &str) -> Result<(), SecretStoreError> {
        // a poisoned lock is an unrecoverable invariant break — surface it as a backend fault (never panic
        // in the daemon; the secret is not in the message).
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| SecretStoreError::Backend("fake secret-store lock poisoned".into()))?;
        entries.insert(keychain_ref.to_string(), secret.to_string());
        Ok(())
    }

    fn read(&self, keychain_ref: &str) -> Result<Option<String>, SecretStoreError> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| SecretStoreError::Backend("fake secret-store lock poisoned".into()))?;
        Ok(entries.get(keychain_ref).cloned())
    }
}

/// The service namespace under which every NexusOps secret is filed in the OS keychain. The per-secret
/// `keychain_ref` (e.g. `nexusops/github/octocat`) is the keyring `username`; the secret is its password.
const KEYCHAIN_SERVICE: &str = "com.nexusops.daemon";

/// The LIVE [`SecretStore`] over the OS keychain (the `keyring` crate). On macOS this is the Apple
/// Keychain. **Non-deterministic edge** — the round-trip needs a real keychain, so there is no unit test
/// (the `OctocrabGithubReadClient` precedent). NEVER logs the secret.
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeyringSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeyringSecretStore {
    fn store(&self, keychain_ref: &str, secret: &str) -> Result<(), SecretStoreError> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, keychain_ref)
            .map_err(|e| SecretStoreError::Backend(format!("open keychain entry: {e}")))?;
        // set_password writes the secret to the OS keychain; the error path carries the keyring class,
        // NOT the secret (the secret is the argument, never the message).
        entry
            .set_password(secret)
            .map_err(|e| SecretStoreError::Backend(format!("write keychain entry: {e}")))
    }

    fn read(&self, keychain_ref: &str) -> Result<Option<String>, SecretStoreError> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, keychain_ref)
            .map_err(|e| SecretStoreError::Backend(format!("open keychain entry: {e}")))?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            // a missing entry is NOT an error — fail-fast `None` (the caller then stays unauth, §15
            // fail-closed), never a wrong secret.
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretStoreError::Backend(format!(
                "read keychain entry: {e}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    //! C1 safety-pin unit: the keychain-write PRIMITIVE in isolation (the `FakeSecretStore` seam). The
    //! cross-module auth surface (gh-reuse, per-account token selection, the toggle gate) lives in the
    //! integration `tests/keychain.rs` against this same `SecretStore` trait.
    use super::*;

    #[test]
    fn fake_secret_store_roundtrips_and_misses_cleanly() {
        // spec(§15 #4): a stored secret reads back by the same ref; an unknown ref reads None (fail-fast,
        // never another secret); an overwrite replaces.
        let store = FakeSecretStore::new();
        store
            .store("nexusops/github/octocat", "ghp_TOKEN")
            .expect("store");
        assert_eq!(
            store
                .read("nexusops/github/octocat")
                .expect("read")
                .as_deref(),
            Some("ghp_TOKEN"),
        );
        assert_eq!(store.read("nexusops/github/nobody").expect("read"), None);
        store
            .store("nexusops/github/octocat", "ghp_ROTATED")
            .expect("overwrite");
        assert_eq!(
            store
                .read("nexusops/github/octocat")
                .expect("read")
                .as_deref(),
            Some("ghp_ROTATED"),
        );
    }
}
