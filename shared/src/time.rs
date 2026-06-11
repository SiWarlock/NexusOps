//! `Timestamp` — an RFC3339 date-time string newtype (ARCHITECTURE §5.0; Carry-forward 1.1 L1).
//!
//! `#[serde(transparent)]` over `String`, so the wire value is the **bare RFC3339 string**
//! (unchanged), but the manual `JsonSchema` impl emits `{ "type": "string", "format":
//! "date-time" }` — so the generated Zod/Pydantic consumers carry the format hint. `parse`
//! validates the RFC3339 shape + component ranges (fail-closed, §15) as a defense-in-depth
//! boundary check; the daemon's injectable `Clock` (§14) is the authoritative source of
//! well-formed values. No date-library dependency — a focused structural validator keeps
//! `shared/` minimal (serde / serde_json / schemars / ulid only).

use std::borrow::Cow;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};

/// An RFC3339 date-time, wire-transparent over its string form.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(String);

impl Timestamp {
    /// Parse + validate an RFC3339 date-time (fail-closed, §15). Accepts `YYYY-MM-DDThh:mm:ss`
    /// with an optional `.fraction` and a `Z`/`±hh:mm` offset (`T`/`Z` case-insensitive).
    pub fn parse(s: &str) -> Result<Self, TimeError> {
        if is_rfc3339(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(TimeError::NotRfc3339)
        }
    }

    /// The full RFC3339 string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl JsonSchema for Timestamp {
    fn schema_name() -> Cow<'static, str> {
        "Timestamp".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::Timestamp").into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "format": "date-time"
        })
    }
}

/// A timestamp parse failure (fail-closed, §15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeError {
    /// the string was not a well-formed RFC3339 date-time
    NotRfc3339,
}

impl std::fmt::Display for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeError::NotRfc3339 => f.write_str("not a valid RFC3339 date-time"),
        }
    }
}

impl std::error::Error for TimeError {}

/// `full-date "T" full-time` (RFC3339 §5.6), with component-range checks.
fn is_rfc3339(s: &str) -> bool {
    let b = s.as_bytes();
    // full-date = YYYY-MM-DD (10 bytes) + a 'T'/'t' separator → at least 11 bytes
    if b.len() < 11 {
        return false;
    }
    if !(b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit())
    {
        return false;
    }
    if b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !(b[5].is_ascii_digit()
        && b[6].is_ascii_digit()
        && b[8].is_ascii_digit()
        && b[9].is_ascii_digit())
    {
        return false;
    }
    let month = (b[5] - b'0') * 10 + (b[6] - b'0');
    let day = (b[8] - b'0') * 10 + (b[9] - b'0');
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return false;
    }
    if b[10] != b'T' && b[10] != b't' {
        return false;
    }
    is_rfc3339_time(&b[11..])
}

/// `partial-time time-offset` — `hh:mm:ss[.frac](Z|±hh:mm)`.
fn is_rfc3339_time(t: &[u8]) -> bool {
    // hh:mm:ss is 8 bytes; the offset is at least 1 ('Z')
    if t.len() < 9 {
        return false;
    }
    if !(t[0].is_ascii_digit() && t[1].is_ascii_digit())
        || t[2] != b':'
        || !(t[3].is_ascii_digit() && t[4].is_ascii_digit())
        || t[5] != b':'
        || !(t[6].is_ascii_digit() && t[7].is_ascii_digit())
    {
        return false;
    }
    let hh = (t[0] - b'0') * 10 + (t[1] - b'0');
    let mm = (t[3] - b'0') * 10 + (t[4] - b'0');
    let ss = (t[6] - b'0') * 10 + (t[7] - b'0');
    // 60 permits a leap second (RFC3339 §5.6)
    if hh > 23 || mm > 59 || ss > 60 {
        return false;
    }
    let mut rest = &t[8..];
    // optional fraction: "." 1*DIGIT
    if rest.first() == Some(&b'.') {
        rest = &rest[1..];
        let digits = rest.iter().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 {
            return false;
        }
        rest = &rest[digits..];
    }
    is_rfc3339_offset(rest)
}

/// `Z` | `(+|-)hh:mm` (RFC3339 §5.6, `Z` case-insensitive).
fn is_rfc3339_offset(o: &[u8]) -> bool {
    if o.len() == 1 {
        return o[0] == b'Z' || o[0] == b'z';
    }
    if o.len() == 6
        && (o[0] == b'+' || o[0] == b'-')
        && o[1].is_ascii_digit()
        && o[2].is_ascii_digit()
        && o[3] == b':'
        && o[4].is_ascii_digit()
        && o[5].is_ascii_digit()
    {
        let oh = (o[1] - b'0') * 10 + (o[2] - b'0');
        let om = (o[4] - b'0') * 10 + (o[5] - b'0');
        return oh <= 23 && om <= 59;
    }
    false
}
