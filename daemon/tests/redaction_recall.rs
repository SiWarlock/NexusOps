//! Phase 2.0-SEC — §15 redactor-recall hardening (L1: corpus + measurement harness + baseline).
//! RED first.
//!
//! This file turns the qualitatively-accepted §15 recall envelope (Option C, 2026-06-10) into a
//! **measured, owned, regression-pinned** envelope. L1 ships:
//!   * a labeled **synthetic** corpus (correctly-shaped fake secrets + realistic non-secrets) —
//!     NO real credential is committed (rule #5; pinned by `test_corpus_contains_no_real_secrets`);
//!   * a pure deterministic **measurement harness** computing recall / precision / FP-rate of any
//!     `Redactor`, broken down by payload-context category;
//!   * a **baseline** of the live redactor pinned as a regression floor (recall) + ceiling (FP)
//!     — L1 baselined `prefix-entropy-v2`; L3 ratcheted it up to `prefix-entropy-v3`.
//!
//! L1 is verification test-infra only — no production entry point (like the 1.7 fuzz pin). The
//! redactor under measurement is the live `PrefixRedactor`; the harness + corpus live here.
//!
//! The `harness` and `corpus` modules are defined at the bottom of the file.

use nexusops_shared::event_envelope::RedactionStatus;
use nexusopsd::eventstore::{PrefixRedactor, RedactionOutcome, Redactor};

use harness::{measure, Category, Label, Sample};

/// Floats are computed from integer counts over a fixed corpus → exact dyadic fractions
/// (1.0, 0.75, 0.5, 0.0). A tiny epsilon keeps the assertions robust to f64 representation.
fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// ---- Baseline regression pins (the measured envelope of `prefix-entropy-v3`) ----------------
//
// These are set to the MEASURED values of the live redactor against the L1 corpus. They are the
// regression floor/ceiling: recall on the catchable set may not silently drift below the floor,
// FP-rate on the non-secret set may not rise above the ceiling (§15 / OQ-SEC-2, line 360).

/// Recall on the **catchable** secret set (prefix · KV-high-entropy · bare-≥40 · long-JSON-value).
/// Measured = 1.0: every secret the redactor claims to catch is caught.
const BASELINE_RECALL_FLOOR: f64 = 1.0;

/// FP-rate on the non-secret set (prefixed-ULIDs, git-SHAs, UUIDs, URLs, paths, config, log lines).
/// Measured = 0.0: no realistic non-secret is masked (the precision half of the envelope).
const BASELINE_FP_CEILING: f64 = 0.0;

// Per-category recall of `prefix-entropy-v3` — the MEASURED residual map (line-360 residuals
// turned into numbers). Catchable categories = 1.0; residual shapes drag their category below 1.0.
const RECALL_PREFIXED: f64 = 1.0; // ghp_/eyJ/AKIA prefixes — all caught.
const RECALL_KV_VALUE: f64 = 0.75; // base64/AWS/dilution caught; adversarial-split (<20 pieces) missed (residual c).
const RECALL_BARE_RUN: f64 = 0.5; // ≥40 high-entropy blob caught; hex≈git-SHA (~4.0 bits) missed (residual b).
const RECALL_JSON_VALUE: f64 = 2.0 / 3.0; // L3: ≥20 JSON values masked-in-place (a closed for ≥20ch); a <20ch value stays a retained accepted sub-residual.

// ---- Test 1 — the measurement math, pinned before it judges the real redactor ---------------

/// A stub Redactor with a KNOWN catch pattern: it masks any payload containing the sentinel
/// `CATCHME` (replacing the whole payload so the value cannot survive) and leaves everything else
/// untouched. Lets the harness math be verified against a hand-computed expectation.
struct CatchMe;
impl Redactor for CatchMe {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        let masked = if payload_json.contains("CATCHME") {
            "\"[MASKED]\"".to_string()
        } else {
            payload_json.to_string()
        };
        RedactionOutcome {
            status: RedactionStatus::Redacted,
            payload_json: masked,
            engine_version: "stub-catchme".to_string(),
            quarantine: None,
        }
    }
}

/// The harness computes recall / precision / FP-rate arithmetically-correctly on a known fixture
/// against a stub redactor — pins the measurement math BEFORE it is trusted to judge the real
/// redactor. recall = caught/total-secrets; precision = caught/(caught+false-catches);
/// FP-rate = falsely-masked-non-secrets/total-non-secrets. (`ARCHITECTURE.md §15` measurement.)
#[test]
fn test_measurement_harness_computes_known_metrics() {
    let samples = vec![
        // a secret that IS caught (payload carries the sentinel) → true positive
        Sample::secret(
            "CATCHME_secretAAA",
            "{\"v\":\"CATCHME_secretAAA\"}",
            Category::BareRun,
            true,
        ),
        // a secret that is NOT caught (no sentinel) → false negative
        Sample::secret(
            "plainsecretBBB",
            "{\"v\":\"plainsecretBBB\"}",
            Category::BareRun,
            true,
        ),
        // a non-secret falsely masked (payload carries the sentinel) → false positive
        Sample::non_secret(
            "CATCHME_nonsecCCC",
            "{\"v\":\"CATCHME_nonsecCCC\"}",
            Category::BareRun,
        ),
        // a non-secret correctly spared → true negative
        Sample::non_secret(
            "plainnonsecDDD",
            "{\"v\":\"plainnonsecDDD\"}",
            Category::BareRun,
        ),
    ];

    let report = measure(&CatchMe, &samples);

    // TP=1, FN=1, FP=1, TN=1
    assert!(
        approx(report.recall().unwrap(), 0.5),
        "recall = 1/2: {:?}",
        report.recall()
    );
    assert!(
        approx(report.precision().unwrap(), 0.5),
        "precision = TP/(TP+FP) = 1/2: {:?}",
        report.precision()
    );
    assert!(
        approx(report.fp_rate().unwrap(), 0.5),
        "FP-rate = 1/2: {:?}",
        report.fp_rate()
    );
}

/// Every metric whose denominator can be zero returns a well-defined `None` (never a panic, a
/// `NaN`, or a silently-wrong `0.0`): no secrets → recall undefined; no positives masked →
/// precision undefined; no non-secrets → FP-rate undefined; a category with no secrets →
/// per-category recall undefined. This harness is reusable §15 measurement infra whose numbers
/// gate a human safety decision — an undefined metric must be EXPLICIT, not corrupt the report.
/// (`ARCHITECTURE.md §15`; orchestrator Step-2.5 ADD.)
#[test]
fn test_harness_metrics_undefined_denominator() {
    // (a) no secrets at all → recall + recall_catchable undefined; the empty category too.
    let only_non = vec![Sample::non_secret(
        "x9",
        "{\"v\":\"x9\"}",
        Category::BareRun,
    )];
    let r = measure(&CatchMe, &only_non);
    assert!(
        r.recall().is_none(),
        "no secrets → recall undefined, not 0.0"
    );
    assert!(
        r.recall_catchable().is_none(),
        "no catchable secrets → recall_catchable undefined"
    );
    assert!(
        r.recall_for_category(Category::KvValue).is_none(),
        "a category with no secrets → per-category recall undefined"
    );

    // (b) no non-secrets at all → FP-rate undefined.
    let only_sec = vec![Sample::secret(
        "CATCHME_a",
        "{\"v\":\"CATCHME_a\"}",
        Category::BareRun,
        true,
    )];
    let r = measure(&CatchMe, &only_sec);
    assert!(
        r.fp_rate().is_none(),
        "no non-secrets → FP-rate undefined, not 0.0"
    );

    // (c) nothing masked (TP=0, FP=0) → precision undefined (no positives to be precise about).
    let none_masked = vec![
        Sample::secret("plain_a", "{\"v\":\"plain_a\"}", Category::BareRun, true),
        Sample::non_secret("plain_b", "{\"v\":\"plain_b\"}", Category::BareRun),
    ];
    let r = measure(&CatchMe, &none_masked);
    assert!(
        r.precision().is_none(),
        "no positives → precision undefined, not 0.0"
    );
}

// ---- Test 2 — the committed corpus is auditably credential-free (rule #5) -------------------

/// Every "secret" sample is synthetic-by-construction — its value carries a documented synthetic
/// sentinel (`FAKE`, `EXAMPLE`, `deadbeef`, …) — so the committed corpus contains NO real
/// credential. A corpus of real secrets in the repo would itself be the §15 violation we defend
/// against (rule #5). (`daemon/CLAUDE.md` forbidden #5 / `ARCHITECTURE.md §15`.)
#[test]
fn test_corpus_contains_no_real_secrets() {
    for s in corpus::all() {
        if s.label == Label::Secret {
            assert!(
                corpus::SYNTHETIC_MARKERS
                    .iter()
                    .any(|m| s.value.contains(m)),
                "secret sample {:?} carries no synthetic marker — is it a real credential?",
                s.value
            );
        }
    }
}

// ---- Test 3 — the catchable-set recall floor (regression pin) -------------------------------

/// `prefix-entropy-v3` achieves ≥ the measured baseline recall on the catchable secret set —
/// recall must not silently drift below the bar (the OQ-SEC-2 mandate, `ARCHITECTURE.md §15`
/// line 360). The floor is a named const set to the measured value; a regression below it fails CI.
#[test]
fn test_baseline_recall_meets_measured_floor() {
    let report = measure(&PrefixRedactor, &corpus::all());
    assert!(
        report.recall_catchable().unwrap() >= BASELINE_RECALL_FLOOR,
        "catchable recall {:?} dropped below the §15 floor {}",
        report.recall_catchable(),
        BASELINE_RECALL_FLOOR
    );
}

// ---- Test 4 — the FP-rate ceiling on the non-secret set (regression pin) --------------------

/// The FP-rate on the non-secret corpus (prefixed-ULIDs, git-SHAs, UUIDs, URLs, paths, config,
/// log lines) stays ≤ the measured ceiling — IDs/SHAs/URLs must stay clear (the precision half of
/// the envelope; a false mask is the cost). (`ARCHITECTURE.md §15` / LESSON §13.)
#[test]
fn test_baseline_false_positive_rate_within_ceiling() {
    let report = measure(&PrefixRedactor, &corpus::all());
    assert!(
        report.fp_rate().unwrap() <= BASELINE_FP_CEILING,
        "FP-rate {:?} rose above the §15 ceiling {}",
        report.fp_rate(),
        BASELINE_FP_CEILING
    );
}

// ---- Test 5 — the per-category envelope, measured not asserted ------------------------------

/// The report quantifies recall per payload-context category, so the accepted residual categories
/// (a) short JSON-value / (b) hex≈git-SHA / (c) adversarial-split are MEASURED, not assumed. Turns
/// the qualitative line-360 residual list into the measured numbers the human rules on. Also pins
/// that EVERY catchable secret — in every category — is caught (`recall_catchable` == 1.0).
/// (`ARCHITECTURE.md §15` line 360.)
#[test]
fn test_envelope_measured_by_category() {
    let report = measure(&PrefixRedactor, &corpus::all());

    // all four payload-context categories are represented among the secrets.
    for cat in [
        Category::Prefixed,
        Category::KvValue,
        Category::JsonValue,
        Category::BareRun,
    ] {
        assert!(
            report.recall_for_category(cat).is_some(),
            "category {cat:?} has no secret samples — the breakdown is incomplete"
        );
    }

    // the measured per-category recall = the documented residual map.
    assert!(approx(
        report.recall_for_category(Category::Prefixed).unwrap(),
        RECALL_PREFIXED
    ));
    assert!(approx(
        report.recall_for_category(Category::KvValue).unwrap(),
        RECALL_KV_VALUE
    ));
    assert!(approx(
        report.recall_for_category(Category::BareRun).unwrap(),
        RECALL_BARE_RUN
    ));
    assert!(approx(
        report.recall_for_category(Category::JsonValue).unwrap(),
        RECALL_JSON_VALUE
    ));

    // every CATCHABLE secret is caught regardless of category — the residual is the uncatchable set.
    assert!(
        approx(report.recall_catchable().unwrap(), 1.0),
        "a catchable secret was missed: {:?}",
        report.recall_catchable()
    );
}

/// Emits the full measured recall/precision/FP envelope of `prefix-entropy-v3` (run with
/// `--nocapture` to read it) — the regenerable artifact the §15 extend-detection human gate is
/// ruled on, and a corpus-completeness guard (a non-trivial corpus across both labels). Re-run
/// this after any future re-tune to regenerate the human-facing report (the harness is the
/// reusable mechanism; the corpus is a living artifact).
#[test]
fn report_measured_envelope() {
    let samples = corpus::all();
    let secrets = samples.iter().filter(|s| s.label == Label::Secret).count();
    let non_secrets = samples
        .iter()
        .filter(|s| s.label == Label::NonSecret)
        .count();
    assert!(
        secrets >= 10 && non_secrets >= 10,
        "corpus too small to be representative: {secrets} secrets / {non_secrets} non-secrets"
    );

    let r = measure(&PrefixRedactor, &samples);
    println!("\n=== prefix-entropy-v3 measured recall envelope (2.0-SEC) ===");
    println!("corpus: {secrets} secrets / {non_secrets} non-secrets");
    println!("overall recall     = {:?}", r.recall());
    println!("recall (catchable) = {:?}", r.recall_catchable());
    println!("precision          = {:?}", r.precision());
    println!("FP-rate            = {:?}", r.fp_rate());
    println!("-- per payload-context category: recall / FP-rate --");
    for cat in [
        Category::Prefixed,
        Category::KvValue,
        Category::JsonValue,
        Category::BareRun,
    ] {
        println!(
            "  {cat:?}: recall={:?}  fp_rate={:?}",
            r.recall_for_category(cat),
            r.fp_rate_for_category(cat)
        );
    }
    println!("===============================================================\n");
}

// ---- L2 — threshold tune-or-confirm (the bar is set by data, not the 1.7 first cut) ----------
//
// The L1 measurement shows the current thresholds are precision-optimal for the catchable set
// (recall_catchable 1.0 / FP 0.0), and residual (b) hex≈git-SHA is irreducible by tuning (lowering
// the bare bar to catch hex secrets would also mask real git-SHAs). So L2 = CONFIRM, with the
// measurement as the justification; the recall bar does NOT move → ENGINE_VERSION is NOT bumped.

use nexusopsd::eventstore::redaction::{
    BARE_ENTROPY_BITS, BARE_MIN_LEN, KV_ENTROPY_BITS, KV_MIN_LEN,
};

/// The named entropy thresholds are at their CONFIRMED measured-optimal values, and the L1
/// floor/ceiling envelope still holds at those values. The measurement IS the justification
/// (`ARCHITECTURE.md §15` / OQ-SEC-2 line 360): a silent edit to a load-bearing threshold is
/// caught, and the confirmed bar is proven to still meet the measured envelope.
#[test]
fn test_tuned_thresholds_named_and_measured() {
    // confirmed measured-optimal — see redaction.rs for the per-const measured justification.
    assert_eq!(KV_ENTROPY_BITS, 4.0, "KV entropy bar");
    assert_eq!(KV_MIN_LEN, 20, "KV min length");
    assert_eq!(BARE_ENTROPY_BITS, 4.5, "bare-run entropy bar");
    assert_eq!(BARE_MIN_LEN, 40, "bare-run min length");

    // the L1 envelope still holds AT these confirmed thresholds (the pin travels with the bar).
    let r = measure(&PrefixRedactor, &corpus::all());
    assert!(
        r.recall_catchable().unwrap() >= BASELINE_RECALL_FLOOR,
        "confirmed thresholds must still meet the §15 recall floor"
    );
    assert!(
        r.fp_rate().unwrap() <= BASELINE_FP_CEILING,
        "confirmed thresholds must still meet the §15 FP ceiling"
    );
}

/// L3 EXTENDED detection (the JSON-value pass) → the recall bar MOVED → `ENGINE_VERSION` is bumped
/// `prefix-entropy-v2` → `prefix-entropy-v3`. The engine version is the provenance contract of
/// WHICH bar produced a persisted row (the `ENGINE_VERSION` const in `redaction.rs`); bump iff the bar moves (brief
/// test 7). (L2 confirmed-no-move kept v2; L3's JSON-value detection is the bar-move that bumps.)
#[test]
fn test_engine_version_reflects_bar() {
    let out = PrefixRedactor.redact("{\"x\":1}");
    assert_eq!(out.engine_version, "prefix-entropy-v3");
}

// ---- L3 — JSON-value detection + value-shape ID-allowlist (residual a closed; HUMAN Option B) -
//
// A new pass masks the high-entropy value of a JSON `"key":"value"` pair IN-PLACE at the
// KV-confidence bar (the `"key":` context raises confidence like `=`), guarded by a value-shape
// ID-allowlist (git-SHA / ULID / UUID) so the measured 0% FP-rate is preserved. The recall bar
// moves → ENGINE_VERSION bumps v2→v3. C (quarantine-bias) is DEFERRED (no test 9).

/// A JSON `"key":"<≥20char/≥4.0bit secret>"` value is masked IN-PLACE while ID-shaped values
/// (git-SHA, ULID, UUID, prefixed-ULID) under JSON keys are spared by the value-shape ID-allowlist.
/// Closes residual (a) without re-introducing the ID-over-mask the 1.7 `KEY=value` scoping avoided
/// (`ARCHITECTURE.md §15` line 360; human-ruled Option B 2026-06-11).
#[test]
fn test_json_value_secret_caught_without_masking_ids() {
    // a ≥20-char non-prefixed JSON-value secret → masked in place (value gone, key + structure kept).
    let secret = "FAKEq7Lm2Zx9Kp4Rn1Vc6Bt3"; // 24 char, high entropy, no `=`/prefix
    let out = PrefixRedactor.redact(&format!("{{\"token\":\"{secret}\"}}"));
    assert!(
        !out.payload_json.contains(secret),
        "JSON-value secret must be masked: {}",
        out.payload_json
    );
    assert!(out.payload_json.contains("[REDACTED]"));
    assert!(
        out.payload_json.contains("\"token\":"),
        "the key + structure survive: {}",
        out.payload_json
    );

    // ID-shaped JSON values are SPARED (the value-shape FP guard).
    let ids = [
        "da39a3ee5e6b4b0d3255bfef95601890afd80709", // git SHA (40 hex)
        "01ARZ3NDEKTSV4RRFFQ69G5FAV",               // ULID (26 Crockford base32)
        "550e8400-e29b-41d4-a716-446655440000",     // UUID
        "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV",          // prefixed-ULID id
    ];
    for id in ids {
        let out = PrefixRedactor.redact(&format!("{{\"ref\":\"{id}\"}}"));
        assert!(
            out.payload_json.contains(id),
            "ID-shaped value must NOT be masked: {id} in {}",
            out.payload_json
        );
    }
}

/// A JSON value containing an ESCAPED quote (`\"`) still has its high-entropy secret masked — the
/// value capture treats `\"` as an interior char (skip-2), not the closing quote, so the secret
/// after/around it cannot leak. Without this, the capture would terminate early at the `\"` and a
/// <40-char secret (below the bare-run floor, no prefix) would survive every pass — a §15 hole.
#[test]
fn test_json_value_escaped_quote_does_not_leak_secret() {
    let secret = "FAKEq7Lm2Zx9Kp4Rn1Vc6Bt3"; // 24 char — below the bare-run floor; only the JSON pass catches it
    for payload in [
        format!("{{\"k\":\"{secret}\\\"x\"}}"), // escaped quote AFTER the secret
        format!("{{\"k\":\"pre\\\"{secret}\"}}"), // escaped quote BEFORE the secret
    ] {
        let out = PrefixRedactor.redact(&payload);
        assert!(
            !out.payload_json.contains(secret),
            "a secret around an escaped quote must not leak: {} → {}",
            payload,
            out.payload_json
        );
    }
}

/// With the L3 JSON-value pass active, the FP-rate on the full non-secret corpus stays ≤ the
/// measured ceiling (0.0) — Option B closes residual (a) WITHOUT spending precision; the measured
/// 0% FP is the guard rail (`ARCHITECTURE.md §15` / LESSON §13).
#[test]
fn test_json_value_pass_preserves_measured_fp_ceiling() {
    let r = measure(&PrefixRedactor, &corpus::all());
    assert!(
        r.fp_rate().unwrap() <= BASELINE_FP_CEILING,
        "the JSON-value pass must not newly mask any non-secret: FP-rate {:?}",
        r.fp_rate()
    );
}

/// The ratchet is HONEST: the catchable set grows (the ≥20-char (a) sample flips false→true) so
/// `recall_catchable` holds 1.0 over the larger set + ENGINE_VERSION bumps (bar moved) — while
/// JsonValue per-category recall ratchets above the L1 0.5 but stays BELOW 1.0, because the
/// <20-char (a) sub-residual is retained in the corpus. So the measured envelope reads "(a) closed
/// for ≥20ch, <20ch residual remains," not "JsonValue fully closed" (`ARCHITECTURE.md §15` line 360).
#[test]
fn test_json_value_recall_ratchets_up() {
    // both a ≥20-char catchable (a) sample AND a <20-char retained residual exist (the honest dual).
    let json_secrets: Vec<Sample> = corpus::all()
        .into_iter()
        .filter(|s| s.label == Label::Secret && s.category == Category::JsonValue)
        .collect();
    assert!(
        json_secrets.iter().any(|s| s.catchable),
        "a ≥20-char catchable (a) sample must exist (demonstrates closure)"
    );
    assert!(
        json_secrets.iter().any(|s| !s.catchable),
        "a <20-char residual (a) sample must be retained (the accepted sub-limit, measured)"
    );

    let r = measure(&PrefixRedactor, &corpus::all());
    let json_recall = r.recall_for_category(Category::JsonValue).unwrap();
    assert!(
        json_recall > 0.5,
        "JsonValue recall must ratchet above the L1 0.5: {json_recall}"
    );
    assert!(
        json_recall < 1.0,
        "JsonValue is NOT fully closed — the <20ch residual is retained"
    );
    assert!(
        approx(json_recall, RECALL_JSON_VALUE),
        "JsonValue recall = the new measured map"
    );
    assert!(
        approx(r.recall_catchable().unwrap(), 1.0),
        "the catchable set grew yet every catchable secret is still caught: {:?}",
        r.recall_catchable()
    );
    assert_eq!(
        PrefixRedactor.redact("{}").engine_version,
        "prefix-entropy-v3",
        "the bar moved → ENGINE_VERSION bumped"
    );
}

// ============================================================================================
// harness — a pure, deterministic measurement harness over a labeled corpus.
// ============================================================================================

mod harness {
    use nexusopsd::eventstore::Redactor;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Label {
        Secret,
        NonSecret,
    }

    /// The payload-context category a sample's value sits in — the four shapes the §15 redactor
    /// reasons about. The per-category recall breakdown turns the line-360 residual list into
    /// measured numbers.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Category {
        /// a known secret-token prefix (`ghp_`, `eyJ`, `AKIA`, …).
        Prefixed,
        /// an env-style `KEY=value` value (pass-1 entropy path).
        KvValue,
        /// a JSON `"key":"value"` value (no `=` — only the bare-run pass can reach it).
        JsonValue,
        /// a bare run with no `KEY=`/prefix context (the stricter bare-run bar).
        BareRun,
    }

    /// One labeled corpus sample: a `value` (the substring whose survival/masking is measured),
    /// embedded in a representative `payload`, with its intrinsic `label`, payload-context
    /// `category`, and — for secrets — whether it is in the **catchable** set the redactor claims
    /// to catch. `catchable` is fixed at corpus-definition from the sample's *designed*
    /// entropy/length/context vs the §15 bar — NEVER derived from redactor output (so the floor
    /// pin is a genuine regression guard, not circular).
    pub struct Sample {
        pub value: String,
        pub payload: String,
        pub label: Label,
        pub category: Category,
        pub catchable: bool,
    }

    impl Sample {
        pub fn secret(value: &str, payload: &str, category: Category, catchable: bool) -> Self {
            Sample {
                value: value.to_string(),
                payload: payload.to_string(),
                label: Label::Secret,
                category,
                catchable,
            }
        }
        pub fn non_secret(value: &str, payload: &str, category: Category) -> Self {
            Sample {
                value: value.to_string(),
                payload: payload.to_string(),
                label: Label::NonSecret,
                category,
                catchable: false,
            }
        }
    }

    /// One sample's measured outcome — whether the redactor MASKED its value (the value no longer
    /// survives verbatim in the redacted payload).
    struct Outcome {
        label: Label,
        category: Category,
        catchable: bool,
        masked: bool,
    }

    /// The measured report over a sample set. Holds per-sample outcomes; metrics derive from them.
    /// Every metric whose denominator can be zero returns `Option<f64>` — an undefined cell is
    /// explicit (`None`), never a silently-wrong `0.0` (this is §15 measurement infra gating a
    /// human safety decision).
    pub struct Report {
        outcomes: Vec<Outcome>,
    }

    impl Report {
        /// recall = caught secrets / all secrets. `None` when there are no secrets.
        pub fn recall(&self) -> Option<f64> {
            self.ratio(
                |o| o.label == Label::Secret && o.masked,
                |o| o.label == Label::Secret,
            )
        }

        /// recall over the **catchable** secret set only — the regression floor. `None` when
        /// there are no catchable secrets.
        pub fn recall_catchable(&self) -> Option<f64> {
            self.ratio(
                |o| o.label == Label::Secret && o.catchable && o.masked,
                |o| o.label == Label::Secret && o.catchable,
            )
        }

        /// precision = caught secrets / all masked samples (TP / (TP+FP)). `None` when nothing was
        /// masked (no positives to be precise about).
        pub fn precision(&self) -> Option<f64> {
            self.ratio(|o| o.label == Label::Secret && o.masked, |o| o.masked)
        }

        /// FP-rate = falsely-masked non-secrets / all non-secrets. `None` when there are no
        /// non-secrets.
        pub fn fp_rate(&self) -> Option<f64> {
            self.ratio(
                |o| o.label == Label::NonSecret && o.masked,
                |o| o.label == Label::NonSecret,
            )
        }

        /// recall for the secrets in one payload-context category. `None` when the category has no
        /// secret samples (the empty-cell case).
        pub fn recall_for_category(&self, cat: Category) -> Option<f64> {
            self.ratio(
                |o| o.label == Label::Secret && o.category == cat && o.masked,
                |o| o.label == Label::Secret && o.category == cat,
            )
        }

        /// FP-rate for the non-secrets in one payload-context category — the per-category false-mask
        /// cost. `None` when the category has no non-secret samples.
        pub fn fp_rate_for_category(&self, cat: Category) -> Option<f64> {
            self.ratio(
                |o| o.label == Label::NonSecret && o.category == cat && o.masked,
                |o| o.label == Label::NonSecret && o.category == cat,
            )
        }

        /// numerator/denominator over the outcomes; `None` on a zero denominator (undefined,
        /// never a silently-wrong 0.0).
        fn ratio(
            &self,
            num: impl Fn(&Outcome) -> bool,
            den: impl Fn(&Outcome) -> bool,
        ) -> Option<f64> {
            let d = self.outcomes.iter().filter(|o| den(o)).count();
            if d == 0 {
                return None;
            }
            let n = self.outcomes.iter().filter(|o| num(o)).count();
            Some(n as f64 / d as f64)
        }
    }

    /// Run `redactor` over every sample; a value is "masked" iff it no longer appears verbatim in
    /// the redacted payload. Pure + deterministic (the redactor is a pure function — §14).
    pub fn measure(redactor: &dyn Redactor, samples: &[Sample]) -> Report {
        let outcomes = samples
            .iter()
            .map(|s| Outcome {
                label: s.label,
                category: s.category,
                catchable: s.catchable,
                masked: !redactor.redact(&s.payload).payload_json.contains(&s.value),
            })
            .collect();
        Report { outcomes }
    }
}

// ============================================================================================
// corpus — a labeled SYNTHETIC corpus: correctly-shaped fake secrets + realistic non-secrets.
// Fake *format*, fake *value* — NEVER a real credential (rule #5).
// ============================================================================================

mod corpus {
    use super::harness::{Category, Sample};

    /// Documented synthetic sentinels — every secret sample's value contains at least one, so the
    /// committed corpus is auditably credential-free (`test_corpus_contains_no_real_secrets`).
    pub const SYNTHETIC_MARKERS: &[&str] = &["FAKE", "EXAMPLE", "deadbeef", "0123456789", "SAMPLE"];

    /// The labeled synthetic corpus across the four payload-context categories (11 secrets / 11
    /// non-secrets). Secrets span the catchable shapes (prefix · KV-high-entropy · bare-≥40 ·
    /// long-JSON-value) AND the line-360 residuals (a short-JSON-value · b hex≈git-SHA · c
    /// adversarial-split). Non-secrets are the realistic IDs/SHAs/UUIDs/URLs/paths/config/log
    /// lines that MUST stay clear (the precision half).
    pub fn all() -> Vec<Sample> {
        vec![
            // ===== SECRETS =====
            // --- Prefixed: known token prefixes (caught regardless of entropy; all catchable) ---
            Sample::secret(
                "ghp_FAKE0123456789abcdef0123456789abcdef00",
                "{\"k\":\"ghp_FAKE0123456789abcdef0123456789abcdef00\"}",
                Category::Prefixed,
                true,
            ),
            Sample::secret(
                "eyJhbGciFAKEiJIUzI1NiwidHlwIjoiSldUIn0", // JWT-shaped, eyJ prefix
                "{\"jwt\":\"eyJhbGciFAKEiJIUzI1NiwidHlwIjoiSldUIn0\"}",
                Category::Prefixed,
                true,
            ),
            Sample::secret(
                "AKIAFAKE0123456789ABCD", // AWS access-key id, AKIA prefix
                "{\"k\":\"AKIAFAKE0123456789ABCD\"}",
                Category::Prefixed,
                true,
            ),
            // --- KvValue: KEY=value high-entropy (entropy fallback; catchable except split) ---
            Sample::secret(
                "FAKEZx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss", // base64-ish 39-char value
                "{\"env\":\"API_SECRET=FAKEZx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss\"}",
                Category::KvValue,
                true,
            ),
            Sample::secret(
                "wJalrXUtFAKEI/K7MDENG/bPxRfiCYEXAMPLEKEY", // AWS secret access key (`/` base64)
                "{\"env\":\"AWS_SECRET_ACCESS_KEY=wJalrXUtFAKEI/K7MDENG/bPxRfiCYEXAMPLEKEY\"}",
                Category::KvValue,
                true,
            ),
            Sample::secret(
                "FAKEZx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8", // the secret sub-run inside a padding-diluted value
                "{\"e\":\"SECRET=AAAAAAAAAAAAAAAAAAAA=FAKEZx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8\"}",
                Category::KvValue,
                true,
            ),
            Sample::secret(
                "FAKEa1b2c3.d4e5f6g7.h8i9j0k1", // adversarial split into <20-char pieces (residual c)
                "{\"e\":\"K=FAKEa1b2c3.d4e5f6g7.h8i9j0k1\"}",
                Category::KvValue,
                false,
            ),
            // --- BareRun: no KEY=/prefix (stricter bar; ≥40 blob catchable, hex≈SHA residual b) ---
            Sample::secret(
                "FAKEQz7Z9pX2mK4vL8nR6wT3yB5cF1dG0hJ7aS4eD2", // ≥40 high-entropy blob
                "{\"data\":\"FAKEQz7Z9pX2mK4vL8nR6wT3yB5cF1dG0hJ7aS4eD2\"}",
                Category::BareRun,
                true,
            ),
            Sample::secret(
                "deadbeefdeadbeef0123456789abcdef01234567", // 40-char hex ≈ git-SHA (~low bits, residual b)
                "{\"digest\":\"deadbeefdeadbeef0123456789abcdef01234567\"}",
                Category::BareRun,
                false,
            ),
            // --- JsonValue: "key":"value" (≥20ch caught by the L3 JSON-value pass; <20ch residual a) ---
            Sample::secret(
                "FAKEMp8Zr2Kt6Wd0Hn3Cf7Ms1Ej5Gb9UuQw3Er5Ty", // ≥40 high-entropy JSON value
                "{\"token\":\"FAKEMp8Zr2Kt6Wd0Hn3Cf7Ms1Ej5Gb9UuQw3Er5Ty\"}",
                Category::JsonValue,
                true,
            ),
            Sample::secret(
                // ≥20-char non-prefixed JSON value — was residual (a) in L1; CLOSED by the L3
                // JSON-value pass (mask-in-place at the KV bar).
                "FAKEq7Lm2Zx9Kp4Rn1Vc6Bt3",
                "{\"apikey\":\"FAKEq7Lm2Zx9Kp4Rn1Vc6Bt3\"}",
                Category::JsonValue,
                true,
            ),
            Sample::secret(
                // <20-char non-prefixed JSON value — a RETAINED accepted sub-residual: below the KV
                // bar (20 char), it can't be distinguished from a short ID/token without FP. Kept in
                // the corpus so the measured envelope reads "(a) closed for ≥20ch, <20ch residual"
                // — encoded in the measurement, not only the prose (TWEAK 2026-06-11).
                "FAKEsk0Lm2Qp7Xy9",
                "{\"apikey\":\"FAKEsk0Lm2Qp7Xy9\"}",
                Category::JsonValue,
                false,
            ),
            // ===== NON-SECRETS (must SURVIVE — the precision half) =====
            // --- Prefixed ---
            Sample::non_secret(
                "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV", // prefixed-ULID id (31 char)
                "{\"sid\":\"sess_01ARZ3NDEKTSV4RRFFQ69G5FAV\"}",
                Category::Prefixed,
            ),
            // --- BareRun ---
            Sample::non_secret(
                "da39a3ee5e6b4b0d3255bfef95601890afd80709", // git SHA (40-char hex, ~4.0 bits)
                "{\"commit\":\"da39a3ee5e6b4b0d3255bfef95601890afd80709\"}",
                Category::BareRun,
            ),
            Sample::non_secret(
                "550e8400-e29b-41d4-a716-446655440000", // UUID v4 (len 36 < BARE_MIN_LEN=40 → spared by length; `-` is in the token alphabet)
                "{\"uuid\":\"550e8400-e29b-41d4-a716-446655440000\"}",
                Category::BareRun,
            ),
            // --- KvValue: low-entropy config / URL / path ---
            Sample::non_secret("info", "{\"env\":\"LOG_LEVEL=info\"}", Category::KvValue),
            Sample::non_secret("8080", "{\"env\":\"PORT=8080\"}", Category::KvValue),
            Sample::non_secret("true", "{\"env\":\"DEBUG=true\"}", Category::KvValue),
            Sample::non_secret(
                "https://example.com/v1/path",
                "{\"env\":\"ENDPOINT=https://example.com/v1/path\"}",
                Category::KvValue,
            ),
            Sample::non_secret(
                "/usr/local/share/app",
                "{\"env\":\"DATA_DIR=/usr/local/share/app\"}",
                Category::KvValue,
            ),
            // --- JsonValue: ids / shas / log lines as JSON values ---
            Sample::non_secret(
                "01ARZ3NDEKTSV4RRFFQ69G5FAV", // bare ULID (26 char)
                "{\"id\":\"01ARZ3NDEKTSV4RRFFQ69G5FAV\"}",
                Category::JsonValue,
            ),
            Sample::non_secret(
                "da39a3ee5e6b4b0d3255bfef95601890afd80709", // git SHA as a JSON value
                "{\"sha\":\"da39a3ee5e6b4b0d3255bfef95601890afd80709\"}",
                Category::JsonValue,
            ),
            Sample::non_secret(
                "2026-06-10T12:00:00Z INFO request handled in 42ms", // a structured log line
                "{\"log\":\"2026-06-10T12:00:00Z INFO request handled in 42ms\"}",
                Category::JsonValue,
            ),
        ]
    }
}
