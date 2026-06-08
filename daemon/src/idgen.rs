//! Injectable event-id generator seam (§14 deterministic golden-log replay).
//!
//! The writer takes a `&dyn IdGen` so tests can inject a deterministic sequence
//! of `evt_` ULIDs and replay a byte-identical log. Production uses [`UlidGen`]
//! (real random ULIDs); tests use [`FixedIdGen`].

use std::sync::atomic::{AtomicU64, Ordering};

use nexusops_shared::ids::EventId;

/// Mints platform `event_id`s (`evt_<ULID>`).
pub trait IdGen: Send + Sync {
    fn new_event_id(&self) -> EventId;
}

/// Production generator — real time+random ULIDs.
#[derive(Debug, Default, Clone, Copy)]
pub struct UlidGen;

impl IdGen for UlidGen {
    fn new_event_id(&self) -> EventId {
        EventId::new()
    }
}

/// Deterministic generator for tests / golden-log replay — yields `evt_` ULIDs
/// derived from a monotonic counter, so a fixed input sequence produces a fixed
/// id sequence.
#[derive(Debug, Default)]
pub struct FixedIdGen {
    counter: AtomicU64,
}

impl FixedIdGen {
    pub fn new() -> Self {
        Self::default()
    }
}

impl IdGen for FixedIdGen {
    fn new_event_id(&self) -> EventId {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        // a counter-derived u128 is a valid ULID payload → a valid `evt_` id.
        // NOTE: small counters encode a 1970-epoch ULID timestamp — TEST-ONLY
        // (these sort before any real production id); never use in production.
        let ulid = ulid::Ulid::from(n as u128);
        EventId::parse(&format!("evt_{ulid}")).expect("counter-derived ULID is always valid")
    }
}
