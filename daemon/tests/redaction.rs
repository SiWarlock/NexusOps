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

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{RedactionStatus, Sensitivity, SourceType};
use nexusops_shared::events::SensitiveOutputRedacted;
use nexusops_shared::ids::WorkspaceId;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{
    open_read_only, AppendIntent, EventStore, EventStoreError, PrefixRedactor, QuarantineSignal,
    RedactionOutcome, Redactor,
};
use nexusopsd::idgen::UlidGen;

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

/// The `BARE_MIN_LEN` (40) bare-run floor in `mask_tokens` is pinned at the boundary: a 39-char
/// high-entropy run is NOT masked (below the floor → spares short tokens with margin); a 40-char
/// one IS. Tested as a **bare string** (no JSON `"key":"value"` context) to ISOLATE the bare-run
/// pass from the 2.0-SEC L3 JSON-value pass — which masks JSON values at the lower KV bar (≥20),
/// so a 39-char *JSON value* is now (correctly) masked (residual (a) closure). An off-by-one edit
/// to the load-bearing bare-run threshold would still break this.
#[test]
fn test_bare_run_length_boundary() {
    // both high-entropy (≈4.8 bits/char); length is the ONLY differentiator at the boundary.
    let under = "Ab1Cd2Ef3Gh4Ij5Kl6Mn7Op8Qr9St0Uv1Wx2Yz3"; // 39 chars
    let over = "Ab1Cd2Ef3Gh4Ij5Kl6Mn7Op8Qr9St0Uv1Wx2Yz3A"; // 40 chars
    assert_eq!(under.len(), 39);
    assert_eq!(over.len(), 40);

    let out_under = PrefixRedactor.redact(under);
    assert!(
        out_under.payload_json.contains(under),
        "a 39-char bare run is below the floor → not masked: {}",
        out_under.payload_json
    );

    let out_over = PrefixRedactor.redact(over);
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

// ---- L2 — quarantine → SensitiveOutputRedacted divert + the §15 property/fuzz pin --------

const MARKER: &str = "QUARANTINE_ME";

/// A test Redactor that QUARANTINES any payload carrying [`MARKER`] (a high-confidence secret
/// it "can't safely bound") and redacts everything else clean. Pins the WRITER divert path
/// independent of the real entropy thresholds — and redacts the content-free
/// `SensitiveOutputRedacted` replacement (no MARKER) so the divert can actually persist.
struct QuarantineOnMarker;
impl Redactor for QuarantineOnMarker {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        if payload_json.contains(MARKER) {
            RedactionOutcome {
                status: RedactionStatus::Unredacted,
                payload_json: payload_json.to_string(),
                engine_version: "test-quarantine".to_string(),
                // structural reason + detector — NO secret content (§15).
                quarantine: Some(QuarantineSignal {
                    reason: "high-confidence secret could not be safely bounded".to_string(),
                    detector: "test-marker".to_string(),
                }),
            }
        } else {
            RedactionOutcome {
                status: RedactionStatus::Redacted,
                payload_json: payload_json.to_string(),
                engine_version: "test-quarantine".to_string(),
                quarantine: None,
            }
        }
    }
}

/// A test Redactor that QUARANTINES every payload — including a `SensitiveOutputRedacted`
/// replacement. Exercises the writer's no-recurse guard (a divert-of-a-divert must fail closed).
struct AlwaysQuarantine;
impl Redactor for AlwaysQuarantine {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        RedactionOutcome {
            status: RedactionStatus::Unredacted,
            payload_json: payload_json.to_string(),
            engine_version: "test-always-q".to_string(),
            quarantine: Some(QuarantineSignal {
                reason: "always".to_string(),
                detector: "test".to_string(),
            }),
        }
    }
}

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn intent(event_type: &str, payload: &str) -> AppendIntent {
    AppendIntent {
        event_type: event_type.to_string(),
        event_version: 1,
        occurred_at: "2026-06-07T00:00:00Z".to_string(),
        workspace_id: WorkspaceId::new(),
        actor_type: ActorType::User,
        actor_id: "u_1".to_string(),
        source_type: SourceType::DesktopUi,
        source_id: "src_1".to_string(),
        correlation_id: "corr_1".to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json: payload.to_string(),
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: None,
        session_id: None,
        agent_team_id: None,
        visibility: None,
    }
}

fn open_with(path: &std::path::Path, redactor: Box<dyn Redactor>) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-07T00:00:00Z")),
        redactor,
    )
    .expect("open event store")
}

/// A quarantine outcome DIVERTS: the original event is NOT persisted; a
/// `SensitiveOutputRedacted` is appended in its place (status Redacted). Pins §15
/// quarantine-divert + `event_envelope.rs:49-55` (a quarantined event is diverted, not
/// persisted, and a SensitiveOutputRedacted recorded instead). The §15 gate invariant holds:
/// the persisted row is Redacted, never unredacted.
#[test]
fn test_unredactable_secret_quarantines_and_records() {
    let (_d, path) = temp_db();
    let mut store = open_with(&path, Box::new(QuarantineOnMarker));
    let id = store
        .append(intent(
            "SessionStarted",
            "{\"blob\":\"QUARANTINE_ME-SUPERSECRETBYTES\"}",
        ))
        .expect("a quarantine DIVERTS (records SOR), it does not error");

    let events = store.read_all().expect("read");
    assert_eq!(
        events.len(),
        1,
        "exactly one event persisted — the SOR, NOT the original SessionStarted"
    );
    assert_eq!(events[0].event_type, SensitiveOutputRedacted::EVENT_TYPE);
    assert_eq!(events[0].redaction_status, RedactionStatus::Redacted);
    assert_eq!(id, events[0].event_id, "append returns the SOR's id");
    assert!(
        !events[0].payload_json.contains("SUPERSECRETBYTES"),
        "the diverted secret never reaches the log: {}",
        events[0].payload_json
    );
}

/// The `SensitiveOutputRedacted` record carries NO byte of the original secret/payload — only
/// forensic metadata (`original_event_type` + a structural `reason`/`detector`). Pins §15: the
/// audit record must not leak what it redacted (mirrors the 1.6c AuditIntegrityViolation
/// content-free reason).
#[test]
fn test_quarantine_reason_is_content_free() {
    let (_d, path) = temp_db();
    let mut store = open_with(&path, Box::new(QuarantineOnMarker));
    store
        .append(intent(
            "SessionStarted",
            "{\"k\":\"QUARANTINE_ME-TopSecretValue12345\"}",
        ))
        .unwrap();

    let events = store.read_all().unwrap();
    let sor = &events[0];
    assert_eq!(sor.event_type, SensitiveOutputRedacted::EVENT_TYPE);
    assert!(
        !sor.payload_json.contains("TopSecretValue12345") && !sor.payload_json.contains(MARKER),
        "no byte of the original secret/payload may survive: {}",
        sor.payload_json
    );
    // forensic metadata only: the original event type is preserved; the payload is not.
    let v: serde_json::Value = serde_json::from_str(&sor.payload_json).unwrap();
    assert_eq!(v["original_event_type"], "SessionStarted");
    assert!(v["reason"].is_string(), "structural reason present");
    assert!(v["detector"].is_string(), "detector present");
}

/// The divert PRESERVES the diverted event's routing / identity envelope fields for forensics
/// (workspace/actor/source/correlation) but RECLASSIFIES the content-free record to
/// `Sensitivity::Internal` (not the original's, which may be Secret) and NAMESPACES the dedup
/// key (`divert-<original>`) so a retry dedups without colliding with an unrelated event. Pins
/// the §15 forensic + audit-classification contract of the divert.
#[test]
fn test_divert_preserves_routing_reclassifies_and_namespaces_key() {
    let (_d, path) = temp_db();
    let mut store = open_with(&path, Box::new(QuarantineOnMarker));
    let ws = WorkspaceId::new();
    let original = AppendIntent {
        event_type: "SessionStarted".to_string(),
        event_version: 1,
        occurred_at: "2026-06-07T00:00:00Z".to_string(),
        workspace_id: ws.clone(),
        actor_type: ActorType::User,
        actor_id: "u_specific".to_string(),
        source_type: SourceType::DesktopUi,
        source_id: "src_specific".to_string(),
        correlation_id: "corr_specific".to_string(),
        sensitivity: Sensitivity::Secret, // a high classification — must NOT be inherited
        payload_json: "{\"k\":\"QUARANTINE_ME-x\"}".to_string(),
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: Some("idem-original".to_string()),
        project_id: None,
        session_id: None,
        agent_team_id: None,
        visibility: None,
    };
    store.append(original).unwrap();

    let sor = &store.read_all().unwrap()[0];
    // routing/identity preserved for forensics
    assert_eq!(sor.workspace_id, ws);
    assert_eq!(sor.actor_id, "u_specific");
    assert_eq!(sor.source_type, SourceType::DesktopUi);
    assert_eq!(sor.correlation_id, "corr_specific");
    // content-free record → reclassified Internal (NOT the original Secret)
    assert_eq!(sor.sensitivity, Sensitivity::Internal);
    // dedup key namespaced so a retry dedups without colliding with an unrelated event
    assert_eq!(sor.idempotency_key.as_deref(), Some("divert-idem-original"));
}

/// A divert-of-a-divert FAILS CLOSED: if the Redactor quarantines the `SensitiveOutputRedacted`
/// replacement itself, the writer's recursion guard returns `RedactionRequired` and persists
/// NOTHING (no half-write). Pins the §15 fail-closed bound on the divert path.
#[test]
fn test_divert_recursion_guard_fails_closed() {
    let (_d, path) = temp_db();
    let mut store = open_with(&path, Box::new(AlwaysQuarantine));
    let res = store.append(intent("SessionStarted", "{\"x\":1}"));
    assert!(
        matches!(res, Err(EventStoreError::RedactionRequired)),
        "a divert-of-a-divert must fail closed: {res:?}"
    );
    assert_eq!(
        store.read_all().unwrap().len(),
        0,
        "nothing persists on the fail-closed path (no half-write)"
    );
}

/// The REAL `PrefixRedactor` MASKS in place and NEVER quarantines (Q2 ruling): the divert
/// path stays a tested-but-MVP-unreached safety net. Pins that the production redactor on a
/// bare blob returns `quarantine: None` + a masked payload.
#[test]
fn test_real_redactor_masks_never_quarantines() {
    let blob = "Qz7Z9pX2mK4vL8nR6wT3yB5cF1dG0hJ7aS4eD2fH9kM6oP1";
    let out = PrefixRedactor.redact(&format!("{{\"d\":\"{blob}\"}}"));
    assert!(
        out.quarantine.is_none(),
        "the MVP redactor masks in place, never diverts"
    );
    assert_eq!(out.status, RedactionStatus::Redacted);
    assert!(!out.payload_json.contains(blob), "the bare blob is masked");
}

/// §15 fail-closed under fuzzing (THE load-bearing pin — the reason 1.7 is BLOCKING): over
/// seeded-random payloads each embedding a CATCHABLE secret (prefix · KEY=value high-entropy ·
/// padding-diluted KEY=value · `+`/`/` base64 KEY=value · bare ≥40 blob), the writer NEVER
/// persists a row with `redaction_status='unredacted'` AND no raw secret survives in any
/// persisted payload. Deterministic (seeded LCG — no dep, reproducible). The padding/base64
/// shapes are the security-reviewer's forcing function for the in-slice hardening.
#[test]
fn prop_no_secret_persists_unredacted() {
    let (_d, path) = temp_db();
    let mut store = open_with(&path, Box::new(PrefixRedactor));

    // seeded LCG (golden-numbers); fixed seed → reproducible, never flaky.
    let mut rng: u64 = 0x1234_5678_9abc_def0;
    let mut next = || {
        rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (rng >> 33) as u32
    };
    let alphabet: Vec<char> = ('A'..='Z').chain('a'..='z').chain('0'..='9').collect();
    let alen = alphabet.len(); // 62 = 2·31

    // distinct-char tokens (a coprime stride over the alphabet) → entropy ≈ log2(n), reliably
    // above the bare-run bar (≥4.5) and the KV bar (≥4.0). Varying start+stride keeps the
    // fuzzed values distinct while guaranteeing each is a CATCHABLE secret — the test verifies
    // catchable secrets are caught; a sub-threshold token is in the SHA-sparing recall envelope
    // by design (not the catchable set), so the generator must not emit one.
    let token = |n: usize, next: &mut dyn FnMut() -> u32| -> String {
        let start = (next() as usize) % alen;
        let stride = [3usize, 5, 7, 9, 11, 13][(next() as usize) % 6]; // all coprime to 62
        (0..n)
            .map(|k| alphabet[(start + k * stride) % alen])
            .collect()
    };

    let mut secrets: Vec<String> = Vec::new();
    for _ in 0..256 {
        let t = token(40, &mut next);
        let (payload, raw) = match next() % 5 {
            0 => {
                let s = format!("ghp_{t}");
                (format!("{{\"x\":\"{s}\"}}"), s) // known prefix
            }
            1 => (format!("{{\"e\":\"API_KEY={t}\"}}"), t.clone()), // KEY=value high-entropy
            2 => (
                // padding-diluted KEY=value (whole-span avg below the bar; sub-run catches it)
                format!("{{\"e\":\"SECRET=AAAAAAAAAAAAAAAAAAAAAAAA={t}\"}}"),
                t.clone(),
            ),
            3 => {
                // standard-base64 `/`-bearing value (e.g. an AWS secret access key)
                let s = format!("{}/{}/{}", &t[..13], &t[13..26], &t[26..]);
                (format!("{{\"e\":\"AWS={s}\"}}"), s)
            }
            _ => (format!("{{\"blob\":\"{t}\"}}"), t.clone()), // bare ≥40 high-entropy run
        };
        secrets.push(raw);
        store
            .append(intent("SessionStarted", &payload))
            .expect("append");
    }

    let events = store.read_all().expect("read");
    for ev in &events {
        assert_ne!(
            ev.redaction_status,
            RedactionStatus::Unredacted,
            "no row may persist redaction_status='unredacted' (§15 fail-closed)"
        );
        for raw in &secrets {
            assert!(
                !ev.payload_json.contains(raw.as_str()),
                "a catchable secret survived the redactor: {raw} in {}",
                ev.payload_json
            );
        }
    }
}

/// All three §15 sinks (persist · sync · embed) are gated by the ONE redactor — no second
/// un-gated path. A secret-bearing event leaves no raw secret in `events` (persist),
/// `outbox` (sync, derived from the already-redacted event), or `fts_events` (embed — the
/// rendered headline, never raw payload). Confirms §15 "the same redactor gates all three."
#[test]
fn test_all_three_sinks_same_redactor() {
    let (_d, path) = temp_db();
    let secret = "Zx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss4Dd";
    {
        let mut store = open_with(&path, Box::new(PrefixRedactor));
        store
            .append(intent(
                "SessionStarted",
                &format!("{{\"e\":\"API_SECRET={secret}\"}}"),
            ))
            .unwrap();
    }
    let conn = open_read_only(&path).unwrap();
    let count_with_secret = |table: &str, col: &str| -> i64 {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE {col} LIKE '%' || ?1 || '%'"),
            [secret],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(
        count_with_secret("events", "payload_json"),
        0,
        "persist sink redacted"
    );
    assert_eq!(
        count_with_secret("outbox", "payload_json"),
        0,
        "sync (outbox) sink redacted"
    );
    assert_eq!(
        count_with_secret("fts_events", "body"),
        0,
        "embed (fts) sink redacted"
    );
}
