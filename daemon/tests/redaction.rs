//! Phase 1.7 — Redactor entropy fallback (OQ-SEC-2) + quarantine → `SensitiveOutputRedacted`
//! divert path (§15 redaction-before-persist). RED first.
//!
//! L1 (this file, first sub-cycle): the Shannon-entropy fallback on `KEY=value` lines —
//! masks a high-entropy value the 1.1 prefix set misses, WITHOUT a false-positive storm on
//! low-entropy config (`DEBUG=true`, paths, URLs). Daemon-internal, no contract bump.
//!
//! L2 (added at the L2 sub-cycle): the writer DIVERT path — a high-confidence secret that
//! can't be safely bounded → the original is NOT persisted; a content-free
//! `SensitiveOutputRedacted` event is recorded instead (CONTRACT 0.13.0→0.14.0). Plus the
//! §15 property/fuzz pin (no secret ever persists `unredacted`) + the three-sink confirm.

use nexusops_shared::event_envelope::RedactionStatus;
use nexusopsd::eventstore::{PrefixRedactor, Redactor};

// ---- L1 — Shannon-entropy fallback on KEY=value (OQ-SEC-2, §15) -----------------------

/// A high-entropy `KEY=value` secret with NO known prefix (the prefix set would miss it)
/// → the value is masked. Pins OQ-SEC-2 recall: secret detection can't drift below the §15
/// bar just because a secret lacks a recognized token prefix.
#[test]
fn test_entropy_fallback_catches_prefixless_secret() {
    // 40-char mixed-case+digit base64-ish token: high Shannon entropy, no known prefix.
    let secret = "Zx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss4Dd";
    let payload = format!("{{\"env\":\"API_SECRET={secret}\"}}");

    let out = PrefixRedactor.redact(&payload);

    assert_eq!(out.status, RedactionStatus::Redacted, "still persistable");
    assert!(
        !out.payload_json.contains(secret),
        "the high-entropy value must not survive: {}",
        out.payload_json
    );
    assert!(
        out.payload_json.contains("[REDACTED]"),
        "the value is masked in place: {}",
        out.payload_json
    );
    // structure (the KEY= and the JSON punctuation) stays intact — only the value is masked.
    assert!(
        out.payload_json.contains("API_SECRET="),
        "the key + structure survive: {}",
        out.payload_json
    );
}

/// Low-entropy `KEY=value` config (and URLs / paths) is NOT redacted — no false-positive
/// storm. Pins the §15 recall-vs-precision balance: the entropy fallback raises recall
/// without shredding ordinary config values.
#[test]
fn test_low_entropy_config_not_redacted() {
    // (input payload, the substring that must survive unmasked)
    let cases = [
        ("{\"env\":\"DEBUG=true\"}", "DEBUG=true"),
        ("{\"env\":\"PORT=8080\"}", "PORT=8080"),
        ("{\"env\":\"LOG_LEVEL=info\"}", "LOG_LEVEL=info"),
        // a URL value — the token after `=` breaks at `:` → low-entropy/short, never masked.
        (
            "{\"env\":\"ENDPOINT=https://example.com/v1/path\"}",
            "https://example.com/v1/path",
        ),
        // a filesystem path — split into short low-entropy segments, never masked.
        (
            "{\"env\":\"DATA_DIR=/usr/local/share/app\"}",
            "/usr/local/share/app",
        ),
    ];
    for (payload, must_survive) in cases {
        let out = PrefixRedactor.redact(payload);
        // (no status assert here — `PrefixRedactor` always returns Redacted, so asserting it
        // on a NOT-masked payload would give false confidence; the real invariant is that the
        // value survives + no mask fired — pinned below.)
        assert!(
            out.payload_json.contains(must_survive),
            "low-entropy config must NOT be redacted: {must_survive:?} vanished from {}",
            out.payload_json
        );
        assert!(
            !out.payload_json.contains("[REDACTED]"),
            "no mask should fire for {payload:?} → {}",
            out.payload_json
        );
    }
}

/// A high-entropy secret in a `KEY = value` line with whitespace and/or quotes around the
/// `=` is STILL masked — the env-style form the entropy fallback targets has common spaced
/// and quoted renderings. Pins §15 OQ-SEC-2 recall against the spaced/quoted `KEY=value`
/// variant (a delimiter-immediacy assumption would miss it).
#[test]
fn test_spaced_and_quoted_kv_masked() {
    let cases = [
        // spaces around `=`
        "{\"env\":\"SECRET = Zx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss4Dd\"}",
        // a quoted value (escaped quotes inside the JSON string)
        "{\"env\":\"TOKEN=\\\"Qm9nVx4Lp8Zr2Kt6Wd0Hn3Cf7Ms1Ej5Gb9Uu\\\"\"}",
    ];
    for payload in cases {
        let out = PrefixRedactor.redact(payload);
        assert!(
            out.payload_json.contains("[REDACTED]"),
            "spaced/quoted KEY=value secret must mask: {}",
            out.payload_json
        );
        assert!(
            !out.payload_json
                .contains("Zx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss4Dd")
                && !out
                    .payload_json
                    .contains("Qm9nVx4Lp8Zr2Kt6Wd0Hn3Cf7Ms1Ej5Gb9Uu"),
            "no secret may survive: {}",
            out.payload_json
        );
    }
}

/// A standard-base64 secret containing `+`/`/` (e.g. an AWS secret access key) in a
/// `KEY=value` line is masked — the value span is base64-aware, not split at `+`/`/`. Pins
/// §15 OQ-SEC-2: "entropy fallback on KEY=value lines" must catch the dominant high-value
/// secret shape (an AWS secret access key) that a `[A-Za-z0-9_-]`-only tokenizer fragments.
#[test]
fn test_base64_secret_with_slashes_in_kv_masked() {
    let secret = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    let payload = format!("{{\"env\":\"AWS_SECRET_ACCESS_KEY={secret}\"}}");
    let out = PrefixRedactor.redact(&payload);
    assert!(
        !out.payload_json.contains(secret),
        "a `+`/`/` base64 secret in KEY=value must not survive: {}",
        out.payload_json
    );
    assert!(out.payload_json.contains("[REDACTED]"));
}

/// Entropy-dilution evasion is resisted: a `KEY=value` whose value glues low-entropy padding
/// (`=`/`+`/`/`-joined) to a real secret so the WHOLE-span average entropy drops below the
/// bar STILL masks — because a high-entropy contiguous sub-run inside the span is scored, not
/// just the average. Pins §15 against an adversary-controllable evasion of the primary KV path
/// (security-reviewer finding d).
#[test]
fn test_kv_entropy_dilution_resisted() {
    // value = 30 low-entropy 'A' chars, an `=`, then a 39-char high-entropy secret. Whole-span
    // average H ≈ 3.8 (< 4.0), but the embedded secret sub-run is H ≈ 5.1.
    let secret = "Zx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss4Dd";
    let payload = format!("{{\"e\":\"SECRET=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA={secret}\"}}");
    let out = PrefixRedactor.redact(&payload);
    assert!(
        !out.payload_json.contains(secret),
        "a padding-diluted KEY=value secret must not survive: {}",
        out.payload_json
    );
    assert!(out.payload_json.contains("[REDACTED]"));
}

/// The `BARE_MIN_LEN` (40) bare-run floor is pinned at the boundary: a 39-char high-entropy
/// run is NOT masked (below the floor → spares ≤31-char IDs and short tokens with margin); a
/// 40-char one IS. An off-by-one edit to the load-bearing threshold would break this.
#[test]
fn test_bare_run_length_boundary() {
    // both high-entropy (≈4.8 bits/char); length is the ONLY differentiator at the boundary.
    let under = "Ab1Cd2Ef3Gh4Ij5Kl6Mn7Op8Qr9St0Uv1Wx2Yz3"; // 39 chars
    let over = "Ab1Cd2Ef3Gh4Ij5Kl6Mn7Op8Qr9St0Uv1Wx2Yz3A"; // 40 chars
    assert_eq!(under.len(), 39);
    assert_eq!(over.len(), 40);

    let out_under = PrefixRedactor.redact(&format!("{{\"d\":\"{under}\"}}"));
    assert!(
        out_under.payload_json.contains(under),
        "a 39-char bare run is below the floor → not masked: {}",
        out_under.payload_json
    );

    let out_over = PrefixRedactor.redact(&format!("{{\"d\":\"{over}\"}}"));
    assert!(
        !out_over.payload_json.contains(over),
        "a 40-char bare run is at the floor → masked: {}",
        out_over.payload_json
    );
}

/// The entropy redaction is a pure function of the payload — the SAME input redacts
/// byte-identically on repeat. Pins §14 determinism / LESSON §3 (golden-log-safe): the
/// redacted payload is part of the immutable event, so it must be reproducible.
#[test]
fn test_entropy_redaction_is_deterministic() {
    let payload = "{\"env\":\"TOKEN=Qm9nVx4Lp8Zr2Kt6Wd0Hn3Cf7Ms1Ej5Gb9Uu\"}";
    let a = PrefixRedactor.redact(payload);
    let b = PrefixRedactor.redact(payload);
    assert_eq!(
        a.payload_json, b.payload_json,
        "redaction must be byte-identical on repeat"
    );
    assert_eq!(a.engine_version, b.engine_version);
}

/// A bare high-entropy run (NOT in a `KEY=value` position — e.g. a blob JSON value) is
/// masked IN-PLACE (Q2, orch-ruled: mask, never divert — the run's span IS the run, so it
/// can be safely bounded). Recall is preserved; the false-quarantine event-loss harm is
/// gone. Pins §15 OQ-SEC-2 + the bare-run threshold (≥40 char / ≥4.5 bits) while SPARING a
/// shorter ID-shaped token (`sess_<ULID>`, 31 char) — IDs must survive for forensics.
#[test]
fn test_bare_high_entropy_run_masked_in_place() {
    // a bare ≥40-char high-entropy blob (no `=`): the bare-run pass masks it.
    let blob = "Qz7Z9pX2mK4vL8nR6wT3yB5cF1dG0hJ7aS4eD2fH9kM6oP1";
    let payload = format!("{{\"data\":\"{blob}\"}}");
    let out = PrefixRedactor.redact(&payload);
    assert_eq!(out.status, RedactionStatus::Redacted, "still persistable");
    assert!(
        !out.payload_json.contains(blob),
        "bare high-entropy run must be masked in place: {}",
        out.payload_json
    );
    assert!(out.payload_json.contains("[REDACTED]"));

    // a prefixed-ULID id (31 char) is BELOW the bare-run length floor → survives (spares IDs).
    let id = "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let id_payload = format!("{{\"sid\":\"{id}\"}}");
    let id_out = PrefixRedactor.redact(&id_payload);
    assert!(
        id_out.payload_json.contains(id),
        "an ID-shaped token under the bare-run floor must NOT be masked: {}",
        id_out.payload_json
    );
}

/// The 1.1 prefix secrets (GitHub PAT, OpenAI key, PEM) still mask after the entropy
/// fallback lands — recall only RISES, never regresses. Pins §15 (the entropy pass is
/// ADDITIVE to the prefix set, not a replacement).
#[test]
fn test_prefix_set_still_redacts() {
    let cases = [
        "{\"k\":\"ghp_AbCdEf0123456789AbCdEf0123456789AbCd\"}",
        "{\"k\":\"sk-AbCdEf0123456789AbCdEf0123456789\"}",
    ];
    for payload in cases {
        let out = PrefixRedactor.redact(payload);
        assert_eq!(out.status, RedactionStatus::Redacted);
        assert!(
            out.payload_json.contains("[REDACTED]"),
            "prefix secret must still mask: {}",
            out.payload_json
        );
    }
    // PEM private-key block → whole-region mask (1.1 behavior preserved).
    let pem = "{\"k\":\"-----BEGIN PRIVATE KEY-----\\nMIIabc\\n-----END PRIVATE KEY-----\"}";
    let out = PrefixRedactor.redact(pem);
    assert_eq!(out.status, RedactionStatus::Redacted);
    assert!(
        !out.payload_json.contains("MIIabc"),
        "PEM body must not survive: {}",
        out.payload_json
    );
}
