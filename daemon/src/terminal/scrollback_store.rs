//! 075c — the `ScrollbackStore` seam connecting the VT snapshot PRODUCER (the `SessionActor`
//! read-pump tap) to the restart-recovery CONSUMER (`runtime::recovery`).
//!
//! **Cat-1 placement.** The trait lives in `terminal/` (NOT `runtime/`/`eventstore/`/`gateway`) so the
//! `session/` import-grep still passes — the producer holds a `ScrollbackStore` trait object, NEVER a
//! `WriteHandle` (the §35 opaque-sink pattern; LESSONS §28/§35). It is shared as
//! `Arc<dyn ScrollbackStore>` (the `Arc<dyn Clock>` precedent) — ONE instance written by every live
//! `SessionActor` and read by recovery.
//!
//! **Mechanism-wired-test-first (075c) → durable behind it (075d).** The full producer→store→
//! `Replayed` path is proven NOW with [`FakeScrollbackStore`]; production injects the no-op
//! [`NoopScrollbackStore`] (so recovery stays on the `Relaunched` rung exactly as before 075c). 075d
//! swaps the durable, §15-redaction-gated store in behind this seam → `Replayed`-after-restart goes
//! live (the 4.0a/043 "mechanism wired test-first, real impl next slice" precedent).

use nexusops_shared::ids::SessionId;

use super::VtSnapshot;

/// The producer→recovery seam for per-session VT snapshots. **Daemon-internal** (NOT a `shared/`
/// contract). `Send + Sync` because ONE shared instance (`Arc<dyn ScrollbackStore>`) is written by
/// every live `SessionActor` and read by the restart-recovery consumer.
pub trait ScrollbackStore: Send + Sync {
    /// Persist the latest snapshot for a session (overwrites any prior). Called from the producer tap
    /// (periodic tick + a final save on reap).
    fn save(&self, session_id: &SessionId, snapshot: &VtSnapshot);
    /// Load the latest snapshot for a session, or `None` if none was saved (→ the `Relaunched` rung).
    fn load(&self, session_id: &SessionId) -> Option<VtSnapshot>;
}

/// The production PLACEHOLDER (075c): `save` drops, `load` returns `None`. Production recovery stays on
/// the `Relaunched` rung exactly as before 075c — NO behavior change. 075d replaces this with the
/// durable §15-gated store, at which point `Replayed`-after-restart goes live.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopScrollbackStore;

impl ScrollbackStore for NoopScrollbackStore {
    fn save(&self, _session_id: &SessionId, _snapshot: &VtSnapshot) {}
    fn load(&self, _session_id: &SessionId) -> Option<VtSnapshot> {
        None
    }
}

/// An in-memory [`ScrollbackStore`] for tests — the producer→recovery seam without persistence.
/// Internally `Arc<Mutex<…>>` so a clone SHARES the same backing: the test holds one handle to inspect
/// saves while the producer holds another (as `Arc<dyn ScrollbackStore>`). `test-support`-gated (the
/// `FakePty`/`FakeHarness` precedent).
#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Default)]
pub struct FakeScrollbackStore {
    inner: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<SessionId, VtSnapshot>>>,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeScrollbackStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of sessions with a saved snapshot (test inspection).
    #[must_use]
    pub fn saved_count(&self) -> usize {
        self.inner.lock().expect("scrollback store lock").len()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl ScrollbackStore for FakeScrollbackStore {
    fn save(&self, session_id: &SessionId, snapshot: &VtSnapshot) {
        self.inner
            .lock()
            .expect("scrollback store lock")
            .insert(session_id.clone(), snapshot.clone());
    }

    fn load(&self, session_id: &SessionId) -> Option<VtSnapshot> {
        self.inner
            .lock()
            .expect("scrollback store lock")
            .get(session_id)
            .cloned()
    }
}
