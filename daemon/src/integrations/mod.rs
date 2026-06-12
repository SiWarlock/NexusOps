//! `integrations/` — external connectors (GitHub via octocrab, Linear) + the §17 integration-failure
//! classifier. Adapters are edges: they submit intents to the Gateway and drain the outbox; they
//! NEVER write the DB directly. This slice (edges-003) lands the deterministic `classifier` only; the
//! octocrab/Linear `Destination` adapters + auth bootstrap + the §17 `SyncFailed`/`auth_expired`
//! wiring are gated/later.

pub mod classifier;
