//! nexusopsd — the Rust daemon (trust core).
//!
//! The single audited mutator + the sole writer of `nexusops.db` (§4, §15
//! INV-SEC-1). 1.1 lands the event-store spine: a single-writer WAL SQLite
//! `events` log (the §7.1 envelope), `user_version` migrations with
//! backup/rollback (§16), read-only WAL readers, and injectable `Clock`+`IdGen`
//! for deterministic golden-log replay (§14).

pub mod clock;
pub mod eventstore;
pub mod idgen;
