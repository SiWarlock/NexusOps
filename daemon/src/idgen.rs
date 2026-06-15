//! Injectable event-id generator seam (§14 deterministic golden-log replay).
//!
//! The writer takes a `&dyn IdGen` so tests can inject a deterministic sequence
//! of `evt_` ULIDs and replay a byte-identical log. Production uses [`UlidGen`]
//! (real random ULIDs); tests use [`FixedIdGen`].

use std::sync::atomic::{AtomicU64, Ordering};

use nexusops_shared::ids::EventId;

/// Mints platform `event_id`s (`evt_<ULID>`) + daemon-internal `out_` outbox ids.
pub trait IdGen: Send + Sync {
    fn new_event_id(&self) -> EventId;
    /// A fresh `out_<ULID>` outbox id (DATA_MODEL §2.5). Daemon-internal — NOT one of
    /// the 22 frozen `shared/` IDs, so it is a plain `String`, not a newtype.
    fn new_outbox_id(&self) -> String;
    /// A fresh `term_<ULID>` terminal runtime-handle id (§6.4 Terminal Channel, 3.4). Daemon-internal
    /// — NOT one of the 22 frozen `shared/` IDs (a connection/session-scoped handle, re-minted on
    /// resume; the `subscription_id` precedent), so it is a plain `String` wrapped by `terminal::TerminalId`.
    fn new_terminal_id(&self) -> String;
    /// A fresh `conn_<ULID>` integration-connection id (P7.1 Wave-C, edges-029). Daemon-internal — NOT
    /// one of the 22 frozen `shared/` IDs (an edges-private connection-registry id; the `out_`/`term_`
    /// bare-String precedent), so it is a plain `String` carried in `IntegrationConnectionRegistered`.
    fn new_connection_id(&self) -> String;
}

/// Production generator — real time+random ULIDs.
#[derive(Debug, Default, Clone, Copy)]
pub struct UlidGen;

impl IdGen for UlidGen {
    fn new_event_id(&self) -> EventId {
        EventId::new()
    }
    fn new_outbox_id(&self) -> String {
        format!("out_{}", ulid::Ulid::new())
    }
    fn new_terminal_id(&self) -> String {
        format!("term_{}", ulid::Ulid::new())
    }
    fn new_connection_id(&self) -> String {
        format!("conn_{}", ulid::Ulid::new())
    }
}

/// Deterministic generator for tests / golden-log replay — yields `evt_` ULIDs
/// derived from a monotonic counter, so a fixed input sequence produces a fixed
/// id sequence.
#[derive(Debug, Default)]
pub struct FixedIdGen {
    counter: AtomicU64,
    // a SEPARATE counter for outbox ids so writing outbox rows never perturbs the
    // event-id sequence — the golden-log replay stays byte-identical (§14).
    outbox_counter: AtomicU64,
    // likewise a SEPARATE counter for terminal runtime-handle ids (3.4) — minting a
    // terminal id never shifts the event-id sequence (golden-log replay byte-identical).
    terminal_counter: AtomicU64,
    // likewise a SEPARATE counter for integration-connection ids (P7.1 Wave-C) — minting a
    // connection id never shifts the event-id sequence (golden-log replay byte-identical).
    connection_counter: AtomicU64,
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
    fn new_outbox_id(&self) -> String {
        let n = self.outbox_counter.fetch_add(1, Ordering::Relaxed);
        format!("out_{}", ulid::Ulid::from(n as u128))
    }
    fn new_terminal_id(&self) -> String {
        let n = self.terminal_counter.fetch_add(1, Ordering::Relaxed);
        format!("term_{}", ulid::Ulid::from(n as u128))
    }
    fn new_connection_id(&self) -> String {
        let n = self.connection_counter.fetch_add(1, Ordering::Relaxed);
        format!("conn_{}", ulid::Ulid::from(n as u128))
    }
}
