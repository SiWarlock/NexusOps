//! Redaction-before-persist seam (§15). The single-writer routes every payload
//! through a `Redactor` before INSERT; the writer GATE refuses to persist anything
//! not `redacted`. 1.1 shipped the high-recall token-prefix Redactor; 1.7 added the
//! Shannon-entropy fallback (OQ-SEC-2) so secret-detection recall doesn't drift below
//! the §15 bar when a secret lacks a recognized token prefix; 2.0-SEC L3 adds JSON-value
//! detection (Option B) to close the measured residual (a) — short non-prefixed secrets
//! under JSON `"key":"value"` keys.
//!
//! **Three passes, all masking in-place** (a masked secret never persists; structure stays):
//! 1. [`mask_kv_values`] — env-style **`KEY=value`** lines (OQ-SEC-2 "KEY=value lines"):
//!    mask a high-entropy value (whitespace/quote-tolerant around `=`; base64-aware value
//!    span incl. `+`/`/`/`=` so standard-base64 secrets like an AWS secret access key are
//!    caught whole). Scoped to `=` so the env-key prefix (`API_SECRET=`) survives.
//! 2. [`mask_json_values`] (2.0-SEC L3) — JSON `"key":"value"` high-entropy values, masked at
//!    the **KV-confidence bar** (the `"key":` context raises confidence like `=`), guarded by a
//!    value-shape **ID-allowlist** ([`is_id_shape`]) so IDs/SHAs/UUIDs stay clear. Sub-run-scored
//!    so prose / paths / URLs (short segments) are spared. Closes residual (a) for values ≥20 char.
//! 3. [`mask_tokens`] — known secret token **prefixes** (`ghp_`, `sk-`, …), PEM blocks (1.1),
//!    and **bare high-entropy runs** with no `KEY=` context (a stricter bar — ≥40 char /
//!    ≥4.5 bits/char — so ≤31-char IDs and 4.0-bit hex hashes / git SHAs are NOT masked).
//!
//! Masking is a **pure function** of the payload (golden-log-safe, §14 / LESSON §3).

use nexusops_shared::event_envelope::RedactionStatus;

/// Result of redacting one payload: the (possibly masked) payload + the status
/// the writer gate checks + the engine provenance recorded on the event.
pub struct RedactionOutcome {
    pub status: RedactionStatus,
    pub payload_json: String,
    pub engine_version: String,
    /// `Some` when the Redactor detected a high-confidence secret it CANNOT safely redact in
    /// place (§15) → the writer DIVERTS: it does not persist the original; it records a
    /// `SensitiveOutputRedacted` instead. `None` is the normal persist/refuse path. Quarantine
    /// is a writer-path decision — `RedactionStatus` (the persisted column) stays at its frozen
    /// two values. The MVP `PrefixRedactor` masks in place and never sets this (Q2).
    pub quarantine: Option<QuarantineSignal>,
}

/// The content-free signal a Redactor raises to DIVERT an event it can't safely redact (§15).
/// Carries forensic metadata ONLY — a redaction-safe **structural** reason + the detector that
/// fired — NEVER the secret or any byte of the payload (mirrors the 1.6c content-free record).
#[derive(Debug, Clone)]
pub struct QuarantineSignal {
    pub reason: String,
    pub detector: String,
}

/// Routes a payload through redaction before persist (§15). Owned by `policy`
/// long-term; the eventstore holds a `&dyn Redactor` and fails closed without one.
pub trait Redactor: Send + Sync {
    fn redact(&self, payload_json: &str) -> RedactionOutcome;
}

/// MVP secret Redactor (§15): masks known token prefixes, PEM blocks, and — 1.7
/// (OQ-SEC-2) — high-entropy `KEY=value` values + bare high-entropy runs the prefix
/// set misses. Masks in place; never quarantines (the quarantine/divert path is a
/// writer-side decision, exercised by an alternate Redactor — see `eventstore::mod`).
pub struct PrefixRedactor;

/// secret token prefixes (§15): GitHub PATs, OpenAI keys, Slack, AWS, JWT.
const SECRET_PREFIXES: &[&str] = &["ghp_", "github_pat_", "sk-", "xox", "AKIA", "eyJ"];

/// The engine provenance recorded on every event this Redactor masks; a contract of WHICH recall
/// bar produced a row. Bumped `prefix-v1` (1.1) → `-v2` (1.7 entropy fallback, OQ-SEC-2) → `-v3`
/// (2.0-SEC L3 JSON-value detection moved the bar). A future re-tune bumps it again.
const ENGINE_VERSION: &str = "prefix-entropy-v3";

/// `KEY=value` high-entropy value bar (OQ-SEC-2). The `=` assignment context raises confidence,
/// so the bar is lower than a bare run. **Confirmed sufficient on the 2.0-SEC measured corpus**
/// (`daemon/tests/redaction_recall.rs`, pinned by `test_tuned_thresholds_named_and_measured`): at
/// 4.0 bits / 20 char the catchable KV secrets (base64 · `+`/`/` AWS · padding-diluted) all mask
/// while `LOG_LEVEL=info`/`PORT=8080` and ordinary paths/URLs do not — recall_catchable 1.0 /
/// FP 0.0. (Sufficient on the corpus, not a proven unique global optimum.) `pub` so the bar is a
/// named, measured, regression-guarded contract.
pub const KV_ENTROPY_BITS: f64 = 4.0;
pub const KV_MIN_LEN: usize = 20;

/// Bare high-entropy run bar — a high-entropy token NOT in a `KEY=value` position. Stricter than
/// the KV bar because there is no `=` context: ≥40 char (spares ≤31-char prefixed-ULID IDs) AND
/// ≥4.5 bits/char (spares ~4.0-bit hex hashes / git SHAs). **Confirmed sufficient on the 2.0-SEC
/// measured corpus**: catches a raw ≥40-char base64 blob with zero false masks. The 4.5 bar is
/// load-bearing for precision — lowering it to catch hex≈git-SHA secrets (residual b, ~4.0 bits)
/// would also mask real git-SHAs, since entropy can't discriminate them; that recall/precision
/// trade is irreducible by tuning (→ an extend-detection decision, not a threshold one).
pub const BARE_ENTROPY_BITS: f64 = 4.5;
pub const BARE_MIN_LEN: usize = 40;

impl Redactor for PrefixRedactor {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        // pass 1: env-style KEY=value high-entropy values (base64-aware, ws/quote-tolerant).
        let kv_masked = mask_kv_values(payload_json);
        // pass 2 (2.0-SEC L3): JSON `"key":"value"` high-entropy values, mask-in-place at the KV
        // bar, ID-allowlist-guarded. AFTER pass 1 so a `KEY=value` inside a JSON value keeps its
        // env-key (pass 1 already masked that value); pass 2 handles bare quoted values.
        let json_masked = mask_json_values(&kv_masked);
        // pass 3: known prefixes + bare high-entropy runs (URL/path-safe token alphabet).
        let mut masked = mask_tokens(&json_masked);
        // PEM blocks (multi-line private keys) — mask the whole payload region.
        if masked.contains("BEGIN") && masked.contains("PRIVATE KEY") {
            masked = "\"[REDACTED-PEM]\"".to_string();
        }
        RedactionOutcome {
            status: RedactionStatus::Redacted,
            payload_json: masked,
            engine_version: ENGINE_VERSION.to_string(),
            // the MVP redactor masks in place — it never diverts (Q2). The quarantine/divert
            // path stays wired in the writer for an alternate Redactor (§15 safety net).
            quarantine: None,
        }
    }
}

const REDACTED: &str = "[REDACTED]";

/// A char that can be part of a base64-ish secret VALUE (the RHS of `KEY=value`). Wider than
/// the token alphabet — includes `+`/`/`/`=` — so a standard-base64 secret is one value span,
/// not fragments. (`:`/`.` stay OUT so a URL value breaks at `://` → short, low-entropy span.)
fn is_value_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '+' | '/' | '=')
}

/// `[A-Za-z0-9_-]` — the token alphabet for pass 2. Excludes `:`/`/`/`.` so URLs + dotted
/// paths split into short low-entropy sub-tokens (never masked); base64url (`-_`) stays one
/// token. (`+`/`/` base64 in a bare run fragments here — accepted residual, see module doc.)
fn is_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

/// Pass 1: mask the high-entropy value of each env-style `KEY=value` line. On each `=`, skip
/// surrounding whitespace + quotes, capture the base64-aware value span, and replace it with
/// `[REDACTED]` when it clears the KV entropy+length bar. Pure (golden-log-safe, §14).
fn mask_kv_values(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        out.push(ch);
        i += 1;
        if ch != '=' {
            continue;
        }
        // skip whitespace + quotes + JSON-escape backslashes between `=` and the value
        // (so `KEY = value`, `KEY="value"`, and the JSON-escaped `KEY=\"value\"` all reach
        // the value), emitting the skipped chars verbatim.
        while i < chars.len()
            && (chars[i].is_whitespace() || chars[i] == '"' || chars[i] == '\'' || chars[i] == '\\')
        {
            out.push(chars[i]);
            i += 1;
        }
        // capture the value span (base64-aware run).
        let start = i;
        while i < chars.len() && is_value_char(chars[i]) {
            i += 1;
        }
        let value: String = chars[start..i].iter().collect();
        if kv_value_should_mask(&value) {
            out.push_str(REDACTED);
        } else {
            out.push_str(&value);
        }
    }
    out
}

/// Decide whether a captured `KEY=value` value span is a secret to mask. Mask if EITHER the
/// whole span clears the KV bar (catches a `/`/`+`-bearing standard-base64 secret scored as
/// one high-entropy span, e.g. an AWS secret access key) OR any maximal alnum sub-run within
/// it clears the bar (defeats an entropy-DILUTION evasion that `=`/`+`/`/`-glues low-entropy
/// padding to a secret to drag the whole-span average below the bar — security-reviewer (d)).
fn kv_value_should_mask(span: &str) -> bool {
    if span.len() >= KV_MIN_LEN && shannon_bits_per_char(span) >= KV_ENTROPY_BITS {
        return true;
    }
    span.split(|c: char| !is_token_char(c))
        .any(|sub| sub.len() >= KV_MIN_LEN && shannon_bits_per_char(sub) >= KV_ENTROPY_BITS)
}

/// Pass 2 (2.0-SEC L3, Option B): mask the high-entropy VALUE of a JSON `"key":"value"` pair
/// IN-PLACE at the KV-confidence bar (the `"key":` context raises confidence like `=`), guarded by
/// a value-shape ID-allowlist so the measured 0% FP-rate is preserved. Scans for a `:` that opens a
/// quoted string value, captures the value (to the next unescaped `"`), and masks it when it clears
/// the bar and is not an ID shape. Pure (golden-log-safe, §14). Closes §15 residual (a) for
/// JSON-value secrets ≥`KV_MIN_LEN`; shorter values stay below the bar (an accepted sub-residual).
fn mask_json_values(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        out.push(ch);
        i += 1;
        if ch != ':' {
            continue;
        }
        // skip whitespace between `:` and the value, emitting it verbatim.
        while i < chars.len() && chars[i].is_whitespace() {
            out.push(chars[i]);
            i += 1;
        }
        // only a quoted string value is a candidate (`"k":123` / `"k":{…}` are left alone).
        if i >= chars.len() || chars[i] != '"' {
            continue;
        }
        out.push(chars[i]); // opening quote
        i += 1;
        // capture the value to the next UNESCAPED quote (a `\"` inside the value is not the end).
        let start = i;
        while i < chars.len() && chars[i] != '"' {
            if chars[i] == '\\' && i + 1 < chars.len() {
                i += 2;
            } else {
                i += 1;
            }
        }
        let value: String = chars[start..i].iter().collect();
        if json_value_should_mask(&value) {
            out.push_str(REDACTED);
        } else {
            out.push_str(&value);
        }
        // the closing quote (chars[i], if present) is emitted on the next loop iteration.
    }
    out
}

/// Decide whether a captured JSON string value is a secret to mask. Mask iff it is NOT an
/// allowlisted ID shape AND a maximal token sub-run within it clears the KV bar. Sub-run scoring
/// (not whole-value entropy) spares prose / paths / URLs — their words/segments are short sub-runs
/// — while catching a contiguous high-entropy secret token.
fn json_value_should_mask(value: &str) -> bool {
    if is_id_shape(value) {
        return false;
    }
    value
        .split(|c: char| !is_token_char(c))
        .any(|sub| sub.len() >= KV_MIN_LEN && shannon_bits_per_char(sub) >= KV_ENTROPY_BITS)
}

/// The value-shape ID-allowlist (the §15 FP guard): a value matching a known non-secret ID shape —
/// git-SHA (all-hex), ULID (26-char Crockford base32, optionally `<lowercase>_`-prefixed), UUID
/// (8-4-4-4-12 hex) — is spared. Robust regardless of key-name. Sparing the hex shape also leaves
/// residual (b) hex≈git-SHA accepted (entropy can't discriminate it; consistent with the 2.0-SEC
/// ruling). The cost is a non-ID-shaped high-entropy non-secret may be over-masked (mask-in-place →
/// loses only the blob, fail-closed; LESSON §13).
fn is_id_shape(value: &str) -> bool {
    is_hex_id(value) || is_uuid(value) || is_ulid(value)
}

/// All-hex (git-SHA / hex id). Length-agnostic — any all-hex value is ID-shaped (a hex secret is
/// the accepted residual (b)).
fn is_hex_id(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// UUID canonical form: 8-4-4-4-12 hex groups separated by `-`.
fn is_uuid(value: &str) -> bool {
    let groups: Vec<&str> = value.split('-').collect();
    let widths = [8usize, 4, 4, 4, 12];
    groups.len() == widths.len()
        && groups
            .iter()
            .zip(widths)
            .all(|(g, w)| g.len() == w && g.chars().all(|c| c.is_ascii_hexdigit()))
}

/// ULID: a 26-char Crockford base32 string, optionally carrying a single `<lowercase>_` platform-id
/// prefix (e.g. `sess_<ULID>` — the frozen 22-ID format, exactly one `_`; `rsplit_once` matches the
/// last `_`, so a multi-underscore string is not treated as a prefixed ULID). Crockford excludes I/L/O/U.
fn is_ulid(value: &str) -> bool {
    let core = match value.rsplit_once('_') {
        Some((prefix, core)) => {
            if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_lowercase()) {
                return false;
            }
            core
        }
        None => value,
    };
    core.len() == 26 && core.chars().all(is_crockford_base32)
}

/// Crockford base32 alphabet: 0-9 and A-Z excluding I, L, O, U.
fn is_crockford_base32(c: char) -> bool {
    matches!(c, '0'..='9' | 'A'..='H' | 'J' | 'K' | 'M' | 'N' | 'P'..='T' | 'V'..='Z')
}

/// Pass 3: tokenize on [`is_token_char`] and mask each token that is a known secret prefix or
/// a bare high-entropy run (≥`BARE_MIN_LEN` / ≥`BARE_ENTROPY_BITS`). Pure (golden-log-safe).
fn mask_tokens(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut token = String::new();
    for ch in s.chars() {
        if is_token_char(ch) {
            token.push(ch);
        } else {
            flush_token(&mut out, &mut token);
            out.push(ch);
        }
    }
    flush_token(&mut out, &mut token);
    out
}

fn flush_token(out: &mut String, token: &mut String) {
    if !token.is_empty() {
        if should_mask_token(token) {
            out.push_str(REDACTED);
        } else {
            out.push_str(token);
        }
        token.clear();
    }
}

/// The pass-2 masking decision for one token: a known prefix, or a bare high-entropy run.
/// Pure + total.
fn should_mask_token(token: &str) -> bool {
    // (1) known secret token prefix — 1.1, highest-confidence.
    if SECRET_PREFIXES.iter().any(|p| token.starts_with(p)) {
        return true;
    }
    // (2) bare high-entropy run (no `KEY=` context → stricter bar; spares IDs + hex hashes).
    // token is ASCII (the alphabet is ASCII), so byte-len == char-count.
    token.len() >= BARE_MIN_LEN && shannon_bits_per_char(token) >= BARE_ENTROPY_BITS
}

/// Shannon entropy of `s` in **bits per character** over its empirical byte distribution.
/// `H = -Σ p_i·log2(p_i)`. Pure + deterministic (no float nondeterminism for equal inputs);
/// the masked spans/tokens are ASCII so a byte histogram is exact.
fn shannon_bits_per_char(s: &str) -> f64 {
    let mut counts = [0u32; 256];
    for b in s.bytes() {
        counts[b as usize] += 1;
    }
    let n = s.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mut h = 0.0;
    for &c in counts.iter() {
        if c > 0 {
            let p = c as f64 / n;
            h -= p * p.log2();
        }
    }
    h
}
