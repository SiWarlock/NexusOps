//! The session-lifecycle spine (P4.0a, opt-3; ARCHITECTURE §5.1 / §10 / §0.1 O-2).
//!
//! An **edge module** (depends on `harness` + `terminal` + `idgen`; **never writes the DB** — the
//! layer rule). The opt-3 shape: a [`SessionActor`](actor::SessionActorHandle) per agent session
//! (a Tokio task + an mpsc mailbox + the §5.1 status state), spawned + supervised by a
//! `SessionSupervisor` (L3), behind a `SessionLauncher` seam (L2; the B2-strict survival broker
//! swaps in at 4.1). This is the foundation the live drive loop (4.0b: launch + INV-SEC-1
//! interception + the Gateway `session.create` executor) builds on.
//!
//! **Cat-1 boundary (4.0a)** — FakeHarness/FakePty-driven; NO live agent, NO event emission, NO
//! mutation. The module takes no `WriteHandle` and no live-interception hook, so emission + mutation
//! are compile-time impossible (the live launch + interception + executor are 4.0b, deep-dive §8).

pub mod actor;
pub mod launcher;

pub use actor::{spawn_session_actor, SessionActorHandle, SessionCommand};
pub use launcher::{FakeLauncher, LaunchedSession, NullTerminalSink, PtyLauncher, SessionLauncher};
