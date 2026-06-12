//! The §17 integration-failure classifier — pure + deterministic.
//!
//! `classify` maps an HTTP/transport delivery outcome → `IntegrationOutcomeClass`, faithful to §17
//! line-450 (transient 429/5xx vs auth-terminal 401/403 vs client-terminal 4xx) + line-452 (transport/
//! offline → transient, "queues, not failed"). `to_delivery_outcome` maps that INTO the existing 1.3
//! `DeliveryOutcome` the outbox drainer already consumes — adding no outbox/drainer type. The github/
//! linear `Destination` adapters that call this, and the §17 auth path (`AuthFailed` → `SyncFailed` +
//! `ExecutionProfile→auth_expired`), are deferred/gated.

use nexusops_shared::time::Timestamp;

use crate::eventstore::DeliveryOutcome;

/// A parsed RFC-7231 `Retry-After` hint, carried as-is — **no `Clock`**; the caller resolves it
/// against `now` later. `Delta` = relative seconds; `Until` = an absolute RFC3339-Z instant
/// (lexical-compare-ready against the drainer's due-times, LESSON §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryAfter {
    Delta(u64),
    Until(Timestamp),
}

/// The §17 classification of one delivery attempt. Auth-terminal (401/403) is kept DISTINCT from
/// client-terminal (other 4xx) per §17 line-450, so the gated auth path (`SyncFailed`/`auth_expired`)
/// can route `AuthFailed` differently from a payload-fix `ClientError` — both still dead-letter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationOutcomeClass {
    /// 2xx.
    Success,
    /// 429 — transient; the parsed `Retry-After` hint is carried for the caller to honor later.
    RateLimited { retry_after: Option<RetryAfter> },
    /// 5xx (or an unexpected status) — transient.
    ServerError,
    /// connection/offline failure — transient (§17 line-452: writes queue, not fail).
    TransportError,
    /// 401/403 — terminal; drives the gated `SyncFailed`/`auth_expired` re-auth path.
    AuthFailed,
    /// other 4xx — terminal payload/request error (a fix, not a retry).
    ClientError { status: u16 },
}

impl IntegrationOutcomeClass {
    /// Map into the existing 1.3 `DeliveryOutcome` (§17): transient → `Retryable` (the drainer backs
    /// off); auth/client-terminal → `Terminal` (dead after the retry budget). The auth-vs-client
    /// distinction stays on `IntegrationOutcomeClass` for the gated `SyncFailed` wiring.
    pub fn to_delivery_outcome(&self) -> DeliveryOutcome {
        match self {
            Self::Success => DeliveryOutcome::Delivered,
            Self::RateLimited { .. } => DeliveryOutcome::Retryable("rate limited (429)".to_owned()),
            Self::ServerError => DeliveryOutcome::Retryable("server error (5xx)".to_owned()),
            Self::TransportError => {
                DeliveryOutcome::Retryable("transport error (offline)".to_owned())
            }
            Self::AuthFailed => DeliveryOutcome::Terminal("auth failed (401/403)".to_owned()),
            Self::ClientError { status } => {
                DeliveryOutcome::Terminal(format!("client error ({status})"))
            }
        }
    }
}

/// Classify a delivery attempt (§17 line-450/452). `transport_error` (a connection failure with no
/// HTTP response) short-circuits to `TransportError`. An unexpected status (1xx/3xx, or no status and
/// no transport error) folds to the conservative transient `ServerError` — retry, never dead-letter.
pub fn classify(
    status: Option<u16>,
    retry_after: Option<&str>,
    transport_error: bool,
) -> IntegrationOutcomeClass {
    if transport_error {
        return IntegrationOutcomeClass::TransportError;
    }
    let Some(status) = status else {
        return IntegrationOutcomeClass::ServerError;
    };
    match status {
        200..=299 => IntegrationOutcomeClass::Success,
        429 => IntegrationOutcomeClass::RateLimited {
            retry_after: parse_retry_after(retry_after),
        },
        401 | 403 => IntegrationOutcomeClass::AuthFailed,
        400..=499 => IntegrationOutcomeClass::ClientError { status },
        500..=599 => IntegrationOutcomeClass::ServerError,
        _ => IntegrationOutcomeClass::ServerError,
    }
}

/// Parse an RFC-7231 `Retry-After` value → a `RetryAfter` hint. `delta-seconds` (all ASCII digits)
/// → `Delta`; an HTTP-date (IMF-fixdate) → `Until` (normalized to RFC3339-Z). Absent / malformed →
/// `None`. **Pure** — no `Clock`.
pub fn parse_retry_after(value: Option<&str>) -> Option<RetryAfter> {
    let raw = value?.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.bytes().all(|b| b.is_ascii_digit()) {
        // A pathological 20+-digit value overflows `u64` → `None` (treated as no hint; u64::MAX
        // seconds is operationally infinite anyway). Never a panic.
        return raw.parse::<u64>().ok().map(RetryAfter::Delta);
    }
    parse_http_date(raw).map(RetryAfter::Until)
}

/// Parse an RFC-7231 HTTP-date (IMF-fixdate, e.g. `Sun, 06 Nov 1994 08:49:37 GMT`) → an RFC3339-Z
/// `Timestamp`. HTTP-date is always GMT/UTC → parse offset-less, then `assume_utc`. Uses the existing
/// `time` dep (no new dependency).
fn parse_http_date(raw: &str) -> Option<Timestamp> {
    let fmt = time::format_description::parse(
        "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT",
    )
    .ok()?;
    let utc = time::PrimitiveDateTime::parse(raw, &fmt).ok()?.assume_utc();
    let rfc3339 = utc
        .format(&time::format_description::well_known::Rfc3339)
        .ok()?;
    Timestamp::parse(&rfc3339).ok()
}
