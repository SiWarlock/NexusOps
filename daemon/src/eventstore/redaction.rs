//! Redaction-before-persist seam (§15). The single-writer routes every payload
//! through a `Redactor` before INSERT; the writer GATE refuses to persist anything
//! not `redacted`. 1.1 ships the high-recall token-prefix Redactor; the
//! Shannon-entropy fallback is the separate blocking task 1.7 (OQ-SEC-2).

use nexusops_shared::event_envelope::RedactionStatus;

/// Result of redacting one payload: the (possibly masked) payload + the status
/// the writer gate checks + the engine provenance recorded on the event.
pub struct RedactionOutcome {
    pub status: RedactionStatus,
    pub payload_json: String,
    pub engine_version: String,
}

/// Routes a payload through redaction before persist (§15). Owned by `policy`
/// long-term; the eventstore holds a `&dyn Redactor` and fails closed without one.
pub trait Redactor: Send + Sync {
    fn redact(&self, payload_json: &str) -> RedactionOutcome;
}

/// MVP high-recall token-prefix Redactor (§15): masks tokens whose prefix marks a
/// known secret shape. Redacts-or-passes (never quarantines) — the entropy
/// fallback that quarantines unredactable secrets is 1.7.
pub struct PrefixRedactor;

/// secret token prefixes (§15): GitHub PATs, OpenAI keys, Slack, AWS, JWT.
const SECRET_PREFIXES: &[&str] = &["ghp_", "github_pat_", "sk-", "xox", "AKIA", "eyJ"];

impl Redactor for PrefixRedactor {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        let mut masked = mask_prefixed_tokens(payload_json);
        // PEM blocks (multi-line private keys) — mask the whole payload region
        if masked.contains("BEGIN") && masked.contains("PRIVATE KEY") {
            masked = "\"[REDACTED-PEM]\"".to_string();
        }
        RedactionOutcome {
            status: RedactionStatus::Redacted,
            payload_json: masked,
            engine_version: "prefix-v1".to_string(),
        }
    }
}

/// Replace every `[A-Za-z0-9_-]+` token that starts with a known secret prefix
/// with `[REDACTED]`, leaving structure (JSON punctuation) intact.
fn mask_prefixed_tokens(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut token = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
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
        if SECRET_PREFIXES.iter().any(|p| token.starts_with(p)) {
            out.push_str("[REDACTED]");
        } else {
            out.push_str(token);
        }
        token.clear();
    }
}
