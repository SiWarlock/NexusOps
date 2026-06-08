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
             (event_id, seq, project_id, occurred_at, headline, actor_label, sensitivity) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(event_id) DO UPDATE SET \
               headline=excluded.headline, actor_label=excluded.actor_label",
            params![
                env.event_id.as_str(),
                env.seq,
                project_id,
                env.occurred_at,
                headline,
                actor_label,
                sensitivity,
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
        other => other.to_string(),
    }
}
