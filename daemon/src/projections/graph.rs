//! `proj_graph_node` / `proj_graph_edge` projector (§2.3) — the ProjectGraph read
//! model, folded from the `object_refs` rows the [`super::object_refs`] projector
//! wrote earlier in the SAME txn (object_refs is registered first). A node per ref;
//! for a `SessionStarted`, a `project --owns--> session` edge. Upserts/IGNOREs keep
//! it idempotent under replay/rebuild (§7.2).

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;

use super::{ProjectionError, Projector};

pub struct GraphProjector;

impl Projector for GraphProjector {
    fn name(&self) -> &'static str {
        "graph"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        let project_id = env.project_id.as_ref().map(|p| p.as_str());

        // read this event's object_refs (written earlier in-txn), collecting before
        // any write so the prepared statement's borrow is released first.
        let refs: Vec<(String, String)> = {
            let mut stmt =
                tx.prepare("SELECT object_type, object_id FROM object_refs WHERE event_id = ?1")?;
            let rows = stmt.query_map(params![env.event_id.as_str()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<_, _>>()?
        };

        for (node_type, node_id) in &refs {
            tx.execute(
                "INSERT INTO proj_graph_node (node_type, node_id, project_id, label) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(node_type, node_id) DO UPDATE SET \
                   project_id=excluded.project_id, label=excluded.label",
                params![node_type, node_id, project_id, node_id],
            )?;
        }

        if env.event_type == "SessionStarted" {
            if let (Some(p), Some(s)) = (&env.project_id, &env.session_id) {
                tx.execute(
                    "INSERT OR IGNORE INTO proj_graph_edge \
                     (src_type, src_id, dst_type, dst_id, edge_type, project_id) \
                     VALUES ('project', ?1, 'session', ?2, 'owns', ?1)",
                    params![p.as_str(), s.as_str()],
                )?;
            }
        }
        Ok(())
    }
}
