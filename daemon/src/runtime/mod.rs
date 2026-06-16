//! The daemon runtime (§12 / §16) — the async task topology around the single write-actor.
//!
//! `main.rs` calls `cold_start()` (1.6a) then stands up:
//! - the **write-actor** ([`writer`]): a dedicated thread owning the writable `EventStore`; every
//!   mutation routes through its [`WriteHandle`] (forbidden #3 / LESSON §3),
//! - the **drainer + reaper interval loops** (L2),
//! - the **UDS bind + accept-loop** (L3),
//! - the **live subscribe broadcast fanout** (L4),
//!
//! then blocks on a shutdown signal (SIGTERM/SIGINT) → graceful drain + exit.
//!
//! L1 ships the write-actor + the `main.rs` entry + graceful shutdown; L2–L4 add the loops,
//! the accept-loop, and the subscribe source.

mod drainer;
mod git_watcher;
pub mod jobs;
mod listener;
pub mod recovery;
pub mod session_death;
mod telemetry_sink;
mod wait_class;
mod writer;

pub use drainer::{spawn_drainer, spawn_reaper};
pub use git_watcher::spawn_git_watcher;
pub use jobs::{
    is_stale, recompute_staleness_once, spawn_staleness_poller, spawn_wal_checkpointer,
};
pub use listener::{bind, spawn_accept_loop};
pub use telemetry_sink::{
    TelemetryHandleSlot, WriteActorTelemetrySink, WriteActorTelemetrySinkFactory,
};
pub use wait_class::{InterceptWaitClass, MAX_CONNECTIONS, RESERVED_GENERAL};
pub use writer::{
    compute_worktree_cache, RuntimeError, WriteActor, WriteHandle, BROADCAST_CAPACITY,
};
