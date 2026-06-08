//! Injectable clock seam (§14 deterministic golden-log replay).
//!
//! The event-store writer takes a `&dyn Clock` so tests can inject a fixed clock
//! and replay a byte-identical event log. Production uses [`SystemClock`] (system
//! UTC, RFC3339); tests use [`FixedClock`].

/// Yields RFC3339 UTC timestamps (the `occurred_at`/`recorded_at` form, DATA_MODEL §2.1).
pub trait Clock: Send + Sync {
    fn now_rfc3339(&self) -> String;
}

/// Production clock — system UTC formatted as RFC3339.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_rfc3339(&self) -> String {
        // infallible: a valid `OffsetDateTime` always formats as RFC3339.
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("RFC3339 formatting of now_utc() is infallible")
    }
}

/// Deterministic clock for tests / golden-log replay — returns a fixed timestamp.
#[derive(Debug, Clone)]
pub struct FixedClock {
    ts: String,
}

impl FixedClock {
    pub fn new(rfc3339: impl Into<String>) -> Self {
        Self { ts: rfc3339.into() }
    }
}

impl Clock for FixedClock {
    fn now_rfc3339(&self) -> String {
        self.ts.clone()
    }
}
