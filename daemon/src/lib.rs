//! nexusopsd — the Rust daemon (trust core).
//!
//! The single audited mutator + the sole writer of `nexusops.db` (§4, §15
//! INV-SEC-1). 1.1 lands the event-store spine: a single-writer WAL SQLite
//! `events` log (the §7.1 envelope), `user_version` migrations with
//! backup/rollback (§16), read-only WAL readers, and injectable `Clock`+`IdGen`
//! for deterministic golden-log replay (§14).

pub mod bootstrap;
pub mod clock;
pub mod eventstore;
/// §14 deterministic fault-injection (2.4) — present ONLY under the `fault-injection` feature (the
/// daemon's own test targets); compiled out of every production build (no prod fault surface).
#[cfg(feature = "fault-injection")]
pub mod fault;
pub mod gateway;
pub mod git;
pub mod harness;
pub mod idgen;
pub mod integrations;
pub mod ipc;
pub mod locks;
pub mod projections;
pub mod runtime;
pub mod workflow;
