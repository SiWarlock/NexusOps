//! `proj_usage_ledger` projector (§2.3/§7.2/§18) — per-day usage rollups.
//!
//! Folds the §9.1 `TelemetrySampled` telemetry-OBSERVATION event (written via the single
//! write-actor, NOT the Gateway — INV-SEC-1 governs mutations, telemetry is an observation; Q1)
//! into rollup rows keyed by `ledger_id = (project, session, profile, model, bucket_day)`.
//! `tokens_in`/`tokens_out`/`cost_estimate` ACCUMULATE (SUM); `context_pct_max` takes the MAX (a
//! gauge, not a sum); `metric_quality` is worst-quality-wins across the bucket (§11.7 — the
//! UsageMeter must never show "exact" over partially-estimated data). Idempotent under replay:
//! the engine folds each event exactly once (strict `seq > last_seq` + truncate-on-rebuild), so a
//! full `rebuild()` reproduces identical SUM/MAX rollups.
//!
//! The feeding `TelemetrySampled` emission lands in 3.2/3.3 (the real adapters' `telemetry_heartbeat`);
//! this projector is wired + reachable NOW (registered in `projectors()` → runs in `apply_all`), its
//! `TelemetrySampled` branch firing once the adapters emit — the `proj_session`-before-lifecycle precedent.

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::TelemetrySampled;

use super::{wire_value, ProjectionError, Projector};

pub struct UsageLedgerProjector;

impl Projector for UsageLedgerProjector {
    fn name(&self) -> &'static str {
        "usage_ledger"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        if env.event_type != TelemetrySampled::EVENT_TYPE {
            return Ok(()); // a non-telemetry event is a healthy no-op (the session.rs precedent)
        }

        // reject-unknown: the reason MUST NOT echo payload bytes (§15) — generic structural text.
        let payload: TelemetrySampled = serde_json::from_str(&env.payload_json)
            .map_err(|_| ProjectionError::Decode("TelemetrySampled payload did not bind".into()))?;
        let bucket_day = utc_bucket_day(&env.occurred_at)?;

        // rollup dims: project/session from the envelope COLUMNS; model/profile from the PAYLOAD
        // (the dims the envelope lacks). `metric_quality` → its canonical §9.1 wire string.
        let project_id = env.project_id.as_ref().map(|p| p.as_str());
        let session_id = env.session_id.as_ref().map(|s| s.as_str());
        let profile = payload.execution_profile_id.as_deref();
        let model = payload.model.as_deref();
        let ledger_id = ledger_id(project_id, session_id, profile, &bucket_day, model);
        let quality = wire_value(&payload.sample.metric_quality)?;
        let context_pct = payload.sample.context_pct.map(f64::from);
        // tokens are u64 per-sample deltas; bind as i64 (a delta never exceeds i64::MAX).
        let tokens_in = payload.sample.tokens_in as i64;
        let tokens_out = payload.sample.tokens_out as i64;

        tx.execute(
            "INSERT INTO proj_usage_ledger \
               (ledger_id, project_id, session_id, execution_profile_id, model, bucket_day, \
                tokens_in, tokens_out, context_pct_max, cost_estimate, metric_quality, updated_at_seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) \
             ON CONFLICT(ledger_id) DO UPDATE SET \
               tokens_in       = COALESCE(tokens_in, 0) + excluded.tokens_in, \
               tokens_out      = COALESCE(tokens_out, 0) + excluded.tokens_out, \
               cost_estimate   = COALESCE(cost_estimate, 0) + excluded.cost_estimate, \
               context_pct_max = CASE \
                 WHEN excluded.context_pct_max IS NULL THEN context_pct_max \
                 WHEN context_pct_max IS NULL THEN excluded.context_pct_max \
                 ELSE MAX(context_pct_max, excluded.context_pct_max) END, \
               metric_quality  = CASE \
                 WHEN metric_quality = 'unavailable' OR excluded.metric_quality = 'unavailable' THEN 'unavailable' \
                 WHEN metric_quality = 'estimated'   OR excluded.metric_quality = 'estimated'   THEN 'estimated' \
                 ELSE 'exact' END, \
               updated_at_seq  = excluded.updated_at_seq",
            params![
                ledger_id,
                project_id,
                session_id,
                profile,
                model,
                bucket_day,
                tokens_in,
                tokens_out,
                context_pct,
                payload.sample.cost_estimate,
                quality,
                env.seq,
            ],
        )?;
        Ok(())
    }
}

/// The deterministic composite rollup key. The four LEADING dims are delimiter-free by contract —
/// `project_id`/`session_id` are envelope ULIDs, `execution_profile_id` is a `prof_` ULID (§5.2;
/// the adapter resolves it to the ExecutionProfile id), and `bucket_day` is `YYYY-MM-DD`. So
/// **model LAST** (the one genuinely free-form dim — `claude-opus-4-8` etc.) means a `|` in a model
/// name cannot shift any leading dim boundary → collision-free for well-formed inputs (Step-2.5 Q3).
/// A `None` dim → the `~` sentinel (outside the ULID alphabet `[0-9A-Z]` and not a date), so a
/// missing dim and a literal value never collide. (A `|` in a `profile` would be an off-contract
/// non-ULID value; flagged at Step 9 — if a free-form profile dim is ever needed, switch to a hash key.)
fn ledger_id(
    project: Option<&str>,
    session: Option<&str>,
    profile: Option<&str>,
    bucket_day: &str,
    model: Option<&str>,
) -> String {
    const NONE: &str = "~";
    format!(
        "{}|{}|{}|{}|{}",
        project.unwrap_or(NONE),
        session.unwrap_or(NONE),
        profile.unwrap_or(NONE),
        bucket_day,
        model.unwrap_or(NONE),
    )
}

/// The UTC date (`YYYY-MM-DD`) of an `occurred_at`. `occurred_at` is daemon-`Clock`-stamped UTC-`Z`
/// (LESSON §5), so the date prefix IS the UTC date. **Fail-closed on a non-`Z` source** (a future
/// regression / non-Clock writer) → `Decode` (the projector degrades LOUDLY, visible) rather than
/// silently mis-bucketing into a local day (Flag-4 addendum). The reason is structural — it never
/// echoes the timestamp value (§15).
fn utc_bucket_day(occurred_at: &str) -> Result<String, ProjectionError> {
    let is_z = occurred_at.ends_with('Z') || occurred_at.ends_with('z');
    match (is_z, occurred_at.get(..10)) {
        (true, Some(day)) => Ok(day.to_string()),
        _ => Err(ProjectionError::Decode(
            "TelemetrySampled occurred_at is not UTC-Z normalized (bucket_day requires Z)".into(),
        )),
    }
}
