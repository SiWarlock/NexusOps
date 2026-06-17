//! nexusopsd — the Rust daemon (trust core).
//!
//! The single audited mutator + the sole writer of `nexusops.db` (§4, §15
//! INV-SEC-1). 1.1 lands the event-store spine: a single-writer WAL SQLite
//! `events` log (the §7.1 envelope), `user_version` migrations with
//! backup/rollback (§16), read-only WAL readers, and injectable `Clock`+`IdGen`
//! for deterministic golden-log replay (§14).

pub mod bootstrap;
pub mod clock;
pub mod decisions;
pub mod eventstore;
/// §14 deterministic fault-injection (2.4) — present ONLY under the `fault-injection` feature (the
/// daemon's own test targets); compiled out of every production build (no prod fault surface).
#[cfg(feature = "fault-injection")]
pub mod fault;
pub mod gateway;
pub mod git;
pub mod harness;
pub mod hook;
pub mod idgen;
pub mod integrations;
pub mod integrity;
pub mod ipc;
pub mod locks;
pub mod project;
pub mod projections;
pub mod runtime;
/// the §15-redacted durable scrollback store (075d) — the production `ScrollbackStore` behind the
/// 075c seam; persists redacted plain-text sidecars (0700/0600) → `Replayed`-after-restart live.
pub mod scrollback;
pub mod session;
/// the `nexusopsd smoke` dev-client subcommand (P4.0b-2-smoke / brief 053) — the 0.1-HITL "see it
/// work" rig. Feature-gated (`dev-client`): compiled out of a default/release build (prod hygiene).
#[cfg(feature = "dev-client")]
pub mod smoke;
pub mod terminal;
pub mod workflow;
