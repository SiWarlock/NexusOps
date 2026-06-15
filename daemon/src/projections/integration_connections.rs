//! `proj_integration_connection` projector (P7.1 Wave-C, edges-030) — the connection-registry read model.
//!
//! Folds `IntegrationConnectionRegistered` (the edges-029 `integration.connect` emitter) into a
//! `proj_integration_connection` row, closing the Wave-C connection vertical (mutator → event →
//! projection). Self-contained like edges-028 (NO LESSON-17 sibling-read). **Key difference from
//! edges-028:** the identity `connection_id` is on the PAYLOAD (the mutator minted it), NOT the
//! envelope → the row is keyed by `payload.connection_id`.
//!
//! `provider` binds via [`wire_value`] (the frozen `Provider` enum → its canonical snake_case wire
//! string, the edges-022 layer-correct producer). `status` is a plain TEXT resting state (`connected`)
//! — there is no frozen §5.1 Connection status machine; a future disconnect/expire event flips it
//! (mutable-from-event-type, LESSON 17). `keychain_ref` is the §15 #4 NON-SECRET pointer, written
//! through from the already-redacted committed event (the edges-029 mutator guarantees it is a pointer);
//! the projector never re-handles a token and never logs the value — no new secret surface.
//!
//! Failure taxonomy (no integrity-break case — no sibling read; no envelope-identity skip — the
//! identity is on the payload):
//!  * **`Decode` → degrade (UNBINDABLE data):** an `IntegrationConnectionRegistered` payload that won't
//!    bind, OR one that binds with an empty/whitespace-only `connection_id` (a connection with no identity is malformed
//!    — fail-closed defense-in-depth; the edges-029 mutator never emits empty) → the projector degrades
//!    and skips this event (the §7.2 reject-unknown contained-failure norm); the generic reason never
//!    echoes payload bytes (§15).

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::IntegrationConnectionRegistered;

use super::{wire_value, ProjectionError, Projector};

/// The resting `status` for a freshly-registered connection (a plain literal — no frozen §5.1 machine).
const STATUS_CONNECTED: &str = "connected";

pub struct IntegrationConnectionProjector;

impl Projector for IntegrationConnectionProjector {
    fn name(&self) -> &'static str {
        "integration_connection"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        if env.event_type != IntegrationConnectionRegistered::EVENT_TYPE {
            // folds ONLY IntegrationConnectionRegistered.
            return Ok(());
        }
        // reject-unknown on the payload — the reason MUST NOT echo (possibly sensitive) payload bytes (§15).
        let p: IntegrationConnectionRegistered =
            serde_json::from_str(&env.payload_json).map_err(|_| {
                ProjectionError::Decode(
                    "IntegrationConnectionRegistered payload did not bind".into(),
                )
            })?;
        // the identity is on the payload (the mutator minted conn_); an empty id is a malformed
        // connection → Decode-degrade (fail-closed; the mutator never emits empty).
        if p.connection_id.trim().is_empty() {
            return Err(ProjectionError::Decode(
                "IntegrationConnectionRegistered carried an empty connection_id".into(),
            ));
        }
        // provider → the canonical wire value (frozen-enum contract, LESSON 2); status = the resting
        // literal; keychain_ref written through (the already-redacted §15 #4 pointer).
        let provider = wire_value(&p.provider)?;
        tx.execute(
            "INSERT INTO proj_integration_connection \
             (connection_id, provider, keychain_ref, account, status, updated_at_seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(connection_id) DO UPDATE SET \
               provider=excluded.provider, keychain_ref=excluded.keychain_ref, \
               account=excluded.account, status=excluded.status, updated_at_seq=excluded.updated_at_seq",
            params![
                p.connection_id,
                provider,
                p.keychain_ref,
                p.account,
                STATUS_CONNECTED,
                env.seq,
            ],
        )?;
        Ok(())
    }
}
