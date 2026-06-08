//! Injectable clock seam (§14 deterministic golden-log replay).
//!
//! The event-store writer takes a `&dyn Clock` so tests can inject a fixed clock
//! and replay a byte-identical event log. Production uses [`SystemClock`] (system
//! UTC, RFC3339); tests use [`FixedClock`].

/// Yields RFC3339 UTC timestamps (the `occurred_at`/`recorded_at` form, DATA_MODEL §2.1).
///
/// **Contract:** `now_rfc3339` MUST return a `Z`-suffix UTC RFC3339 string (not a
/// numeric `+00:00` offset). The outbox due-selection compares `next_attempt_at`
/// against `now` **lexically** (RFC3339 sorts chronologically only at a single,
/// fixed offset form); `now_plus_secs` always emits `Z`, so a clock that emitted
/// `+00:00` would make `Z`-vs-`+` mismatch and strand rows. All concrete clocks emit `Z`.
pub trait Clock: Send + Sync {
    fn now_rfc3339(&self) -> String;

    /// `now` + `secs`, RFC3339 — the outbox backoff schedule (§17). The default parses
    /// `now_rfc3339` and adds the duration; the drainer's clock always yields valid
    /// RFC3339, so the parse is infallible. (Override if a clock holds a richer now.)
    fn now_plus_secs(&self, secs: i64) -> String {
        let rfc3339 = time::format_description::well_known::Rfc3339;
        let now = time::OffsetDateTime::parse(&self.now_rfc3339(), &rfc3339)
            .expect("clock now_rfc3339 yields valid RFC3339");
        (now + time::Duration::seconds(secs))
            .format(&rfc3339)
            .expect("RFC3339 formatting of an OffsetDateTime is infallible")
    }
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
