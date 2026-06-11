//! §6.3 idempotency-key derivation (2.3 L1) — the key is **catalog-authoritative, never requester-
//! supplied** (the same recorded-not-trusted posture as 2.2's risk: a proposer-chosen key could force
//! a collision to suppress a victim's action, or evade dedup). The Gateway derives the key from the
//! catalog `idempotency_formula` at submit and overwrites `req.idempotency_key`.
//!
//! # §15 precedent — one-way hash of RAW inputs (lead-ruled Option A, 2026-06-11)
//!
//! The key MUST distinguish actions by their **raw** (pre-redaction) inputs — hashing the *redacted*
//! inputs would falsely dedup two actions that differ only in a secret (both redact to the same
//! `[REDACTED]`). But the derived key persists **unredacted** into the indexed `idempotency_key`
//! column. So the key is a **one-way SHA-256 of the raw preimage** — a *fingerprint, not the secret*
//! (rule #4: secrets never persist in rows; a one-way hash reveals nothing recoverable).
//!
//! - **What makes A safe is the keychain_ref convention:** correct usage keeps real secrets in the OS
//!   keychain (`inputs` carries `keychain_ref` pointers, never raw secret material), so the hash
//!   fingerprints only non-secret structure.
//! - **Why A and not HMAC (Option C):** A is chosen *because* it gives stable, deterministic-FOREVER
//!   dedup — which is the feature's whole purpose (prevent a duplicate mutation on retry / crash-
//!   recovery). An HMAC keyed on a daemon secret would break every fingerprint on a key reset →
//!   dedup misses → **duplicate execution**, a safety regression for a mutation-dedup feature.
//! - **Bounded residual + the C-upgrade trigger:** a low-entropy secret WRONGLY placed inline could be
//!   brute-forced from its hash — bounded by (rule-#4 misuse AND low entropy AND a local-DB read,
//!   which is already past the trust boundary). **Upgrade to HMAC (C) if** a real low-entropy-inline-
//!   secret threat surfaces, **OR if `idempotency_key` ever leaves the local trust boundary**
//!   (off-machine backup/sync) — at which point "the DB holds no recoverable secrets" becomes
//!   load-bearing against an off-machine attacker.

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use nexusops_shared::actions::ActionRequest;
use nexusops_shared::catalog::{ActionTypeCatalogEntry, IdempotencyFormula};

/// the separator between hash-preimage fields AND between list elements (NUL — cannot appear in an
/// `action_type` / prefixed-ULID id / JSON token, so distinct tuples cannot collide by concatenation).
const SEP: &str = "\0";

/// Derive the §6.3 idempotency key for `req` from its catalog `entry` (authoritative, **never**
/// `req.idempotency_key`). `None` → not idempotent (never deduped). `FromInputs` → a one-way hash over
/// `{action_type, project_id, canonical-JSON(inputs)}`. `NaturalResourceRef` → a one-way hash over
/// `{action_type, SORTED resource_ref ids}`. The hash is **SHA-256 of the RAW preimage** — see the
/// module §15 precedent: a fingerprint, not the secret.
pub fn derive_key(req: &ActionRequest, entry: &ActionTypeCatalogEntry) -> Option<String> {
    let preimage = match entry.idempotency_formula {
        IdempotencyFormula::None => return None,
        IdempotencyFormula::FromInputs => {
            // canonical-JSON: serde_json's default `Value::Object` is a BTreeMap (no `preserve_order`
            // feature), so `to_string` emits keys in sorted order at every level → the same logical
            // inputs hash identically regardless of construction order.
            let project = req.project_id.as_ref().map(|p| p.as_str()).unwrap_or("");
            // a `serde_json::Value` is structurally always serializable; `expect` makes that invariant
            // explicit + fails LOUD (vs an empty-string fallback that would silently collide two
            // distinct actions onto one key → a false dedup that suppresses a mutation).
            let inputs =
                serde_json::to_string(&req.inputs).expect("serde_json::Value always serializes");
            format!("{}{SEP}{}{SEP}{}", req.action_type, project, inputs)
        }
        IdempotencyFormula::NaturalResourceRef => {
            // the targeted resources ARE the natural key: action_type + the sorted resource ids
            // (ids are prefixed-ULIDs, not secrets; sorting makes ref order irrelevant).
            let mut ids: Vec<&str> = req.resource_refs.iter().map(|r| r.id.as_str()).collect();
            ids.sort_unstable();
            format!("{}{SEP}{}", req.action_type, ids.join(SEP))
        }
    };
    Some(hash_key(&preimage))
}

/// SHA-256 the preimage → `idem_` + the first 128 bits as 32 lowercase hex chars (bounded key length;
/// 128 bits is ample collision resistance for the dedup index).
fn hash_key(preimage: &str) -> String {
    let digest = Sha256::digest(preimage.as_bytes());
    let mut key = String::with_capacity(5 + 32);
    key.push_str("idem_");
    for byte in &digest[..16] {
        let _ = write!(key, "{byte:02x}");
    }
    key
}
