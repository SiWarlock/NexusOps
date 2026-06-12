//! P7.1 — the §17 integration-failure classifier (the deterministic core of Phase 7.1's git/integration
//! read side). `classify` maps an HTTP/transport delivery outcome → a daemon-internal
//! `IntegrationOutcomeClass` faithful to §17 line-450 (transient 429/5xx vs auth-terminal 401/403 vs
//! client-terminal 4xx) + line-452 (transport/offline → retryable); `to_delivery_outcome` maps that
//! INTO the existing 1.3 `DeliveryOutcome` the outbox drainer already consumes (no drainer change);
//! `parse_retry_after` parses the RFC-7231 Retry-After header (delta-seconds | HTTP-date), pure (no
//! Clock — the absolute/relative hint is resolved against `now` by the caller later).
//!
//! The github/linear `Destination` adapters that CALL this, and the §17 auth path
//! (`AuthFailed` → `SyncFailed` + `ExecutionProfile→auth_expired`), are deferred/gated.

use nexusopsd::eventstore::DeliveryOutcome;
use nexusopsd::integrations::classifier::{
    classify, parse_retry_after, IntegrationOutcomeClass, RetryAfter,
};
use time::format_description::well_known::Rfc3339;
use time::{Date, Month, OffsetDateTime};

// ---- classify --------------------------------------------------------------

#[test]
fn classify_429_rate_limited() {
    // spec(§17): 429 + Retry-After → RateLimited carrying the parsed hint (transient).
    assert_eq!(
        classify(Some(429), Some("120"), false),
        IntegrationOutcomeClass::RateLimited {
            retry_after: Some(RetryAfter::Delta(120))
        }
    );
}

#[test]
fn classify_429_no_retry_after() {
    // spec(§17): a 429 with NO Retry-After header (e.g. GitHub omits it) → RateLimited{None} via
    // classify's own parse path — still transient.
    assert_eq!(
        classify(Some(429), None, false),
        IntegrationOutcomeClass::RateLimited { retry_after: None }
    );
}

#[test]
fn classify_5xx_server_error() {
    // spec(§17): 5xx → ServerError (transient).
    assert_eq!(
        classify(Some(503), None, false),
        IntegrationOutcomeClass::ServerError
    );
}

#[test]
fn classify_transport_error() {
    // spec(§17): a transport/connection failure → TransportError (line 452 — offline queues, not failed).
    assert_eq!(
        classify(None, None, true),
        IntegrationOutcomeClass::TransportError
    );
}

#[test]
fn classify_401_auth() {
    // spec(§17): 401 → AuthFailed (line-450 auth-terminal).
    assert_eq!(
        classify(Some(401), None, false),
        IntegrationOutcomeClass::AuthFailed
    );
}

#[test]
fn classify_403_auth() {
    // spec(§17): 403 → AuthFailed (line-450 auth-terminal).
    assert_eq!(
        classify(Some(403), None, false),
        IntegrationOutcomeClass::AuthFailed
    );
}

#[test]
fn classify_400_client() {
    // spec(§17): 400 → ClientError (terminal payload error).
    assert_eq!(
        classify(Some(400), None, false),
        IntegrationOutcomeClass::ClientError { status: 400 }
    );
}

#[test]
fn classify_404_client() {
    // spec(§17): 404 → ClientError (terminal, NOT auth).
    assert_eq!(
        classify(Some(404), None, false),
        IntegrationOutcomeClass::ClientError { status: 404 }
    );
}

#[test]
fn classify_422_client() {
    // spec(§17): 422 → ClientError (terminal payload error).
    assert_eq!(
        classify(Some(422), None, false),
        IntegrationOutcomeClass::ClientError { status: 422 }
    );
}

#[test]
fn classify_2xx_success() {
    // spec(§17): 2xx → Success.
    assert_eq!(
        classify(Some(200), None, false),
        IntegrationOutcomeClass::Success
    );
    assert_eq!(
        classify(Some(204), None, false),
        IntegrationOutcomeClass::Success
    );
}

#[test]
fn classify_unknown_conservative() {
    // spec(§17): an unexpected status (308) → the conservative transient default (retry, never dead-letter).
    assert_eq!(
        classify(Some(308), None, false),
        IntegrationOutcomeClass::ServerError
    );
}

#[test]
fn classify_auth_distinct_from_client() {
    // spec(§17): THE load-bearing line-450 distinction — 401/403 → AuthFailed, other 4xx → ClientError
    // (never collapsed; auth → the gated SyncFailed/auth_expired path, client → payload-fix).
    for s in [401, 403] {
        assert_eq!(
            classify(Some(s), None, false),
            IntegrationOutcomeClass::AuthFailed
        );
    }
    for s in [400, 404, 422] {
        assert_eq!(
            classify(Some(s), None, false),
            IntegrationOutcomeClass::ClientError { status: s }
        );
    }
}

// ---- to_delivery_outcome ---------------------------------------------------

#[test]
fn rate_limited_to_retryable() {
    // spec(§17): RateLimited → Retryable (drainer backs off).
    assert!(matches!(
        IntegrationOutcomeClass::RateLimited { retry_after: None }.to_delivery_outcome(),
        DeliveryOutcome::Retryable(_)
    ));
}

#[test]
fn server_error_to_retryable() {
    assert!(matches!(
        IntegrationOutcomeClass::ServerError.to_delivery_outcome(),
        DeliveryOutcome::Retryable(_)
    ));
}

#[test]
fn transport_to_retryable() {
    assert!(matches!(
        IntegrationOutcomeClass::TransportError.to_delivery_outcome(),
        DeliveryOutcome::Retryable(_)
    ));
}

#[test]
fn auth_to_terminal() {
    // spec(§17): AuthFailed → Terminal (dead after budget; the auth distinction is preserved on the class).
    assert!(matches!(
        IntegrationOutcomeClass::AuthFailed.to_delivery_outcome(),
        DeliveryOutcome::Terminal(_)
    ));
}

#[test]
fn client_to_terminal() {
    assert!(matches!(
        IntegrationOutcomeClass::ClientError { status: 404 }.to_delivery_outcome(),
        DeliveryOutcome::Terminal(_)
    ));
}

#[test]
fn success_to_delivered() {
    assert_eq!(
        IntegrationOutcomeClass::Success.to_delivery_outcome(),
        DeliveryOutcome::Delivered
    );
}

// ---- parse_retry_after -----------------------------------------------------

#[test]
fn retry_after_delta_seconds() {
    // spec(§17): RFC-7231 delta-seconds form.
    assert_eq!(parse_retry_after(Some("120")), Some(RetryAfter::Delta(120)));
}

#[test]
fn retry_after_http_date() {
    // spec(§17): RFC-7231 HTTP-date form → an absolute instant. Compared as an INSTANT (not a string)
    // to stay agnostic to the RFC3339 `Z` vs `+00:00` rendering.
    let Some(RetryAfter::Until(ts)) = parse_retry_after(Some("Sun, 06 Nov 1994 08:49:37 GMT"))
    else {
        panic!("expected Until(_)");
    };
    let got = OffsetDateTime::parse(ts.as_str(), &Rfc3339).unwrap();
    let expected = Date::from_calendar_date(1994, Month::November, 6)
        .unwrap()
        .with_hms(8, 49, 37)
        .unwrap()
        .assume_utc();
    assert_eq!(got, expected);
}

#[test]
fn retry_after_absent_or_malformed() {
    // spec(§17): absent / malformed → None (robustness).
    assert_eq!(parse_retry_after(None), None);
    assert_eq!(parse_retry_after(Some("")), None);
    assert_eq!(parse_retry_after(Some("not-a-date")), None);
    assert_eq!(parse_retry_after(Some("999999999999999999999")), None); // u64 overflow → None
}
