//! `proj_audit_trail` projector (§2.3/§14) + FTS population (§2.11).
//!
//! Every event yields one rendered, **redaction-safe** audit row (a human headline,
//! never the raw `payload_json`), and that headline is indexed into the `fts_events`
//! scaffold for the Command-Center search box. Indexing the rendered headline (not
//! raw payload) keeps the event log from becoming a secret-searchable dump (§9/§4.5).
//!
//! NOTE (Q3 deviation, arch-noted at Step 9): DATA_MODEL §2.11 specifies a contentless
//! `events_fts content='proj_audit_trail'`; 1.2 reuses the existing standalone
//! `fts_events(event_id, body)` scaffold (lowest churn, no migration reshape).

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::{AuditIntegrityViolation, SensitiveOutputRedacted};

use super::{wire_value, ProjectionError, Projector};

pub struct AuditProjector;

impl Projector for AuditProjector {
    fn name(&self) -> &'static str {
        "audit_trail"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        let headline = headline_for(&env.event_type);
        let actor_label = wire_value(&env.actor_type)?;
        let sensitivity = wire_value(&env.sensitivity)?;
        let project_id = env.project_id.as_ref().map(|p| p.as_str());

        tx.execute(
            "INSERT INTO proj_audit_trail \
             (event_id, seq, project_id, occurred_at, headline, actor_label, sensitivity, event_type) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT(event_id) DO UPDATE SET \
               headline=excluded.headline, actor_label=excluded.actor_label, \
               event_type=excluded.event_type",
            params![
                env.event_id.as_str(),
                env.seq,
                project_id,
                env.occurred_at,
                headline,
                actor_label,
                sensitivity,
                // W2-audit/097 — the raw machine event_type (the namespace-filter + icons). In the ON
                // CONFLICT DO UPDATE SET so the MIGRATION_20 offset-reset BACKFILLS pre-migration rows on a
                // catch-up upgrade (not only a truncating rebuild) — LESSON §52.
                env.event_type.as_str(),
            ],
        )?;

        // idempotent FTS upsert (fts5 has no PK): clear this event's row, then index.
        tx.execute(
            "DELETE FROM fts_events WHERE event_id = ?1",
            params![env.event_id.as_str()],
        )?;
        tx.execute(
            "INSERT INTO fts_events (event_id, body) VALUES (?1, ?2)",
            params![env.event_id.as_str(), headline],
        )?;
        Ok(())
    }
}

/// A human, redaction-safe headline for an event type. The registry grows per phase;
/// an unmapped type falls back to its type string (still redaction-safe).
fn headline_for(event_type: &str) -> String {
    match event_type {
        "SessionStarted" => "Session started".to_string(),
        "DeviceRegistered" => "Device registered".to_string(),
        "LocalRunnerRegistered" => "Local runner registered".to_string(),
        // the new type's name has ONE home (the const) — the existing literals consolidate later.
        t if t == AuditIntegrityViolation::EVENT_TYPE => "Audit-integrity violation".to_string(),
        t if t == SensitiveOutputRedacted::EVENT_TYPE => "Sensitive output redacted".to_string(),
        other => other.to_string(),
    }
}
