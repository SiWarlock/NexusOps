//! `integrations/` — external connectors (GitHub via octocrab, Linear) + the §17 integration-failure
//! classifier. Adapters are edges: they submit intents to the Gateway and drain the outbox; they
//! NEVER write the DB directly. So far this module lands two deterministic cores — the `classifier`
//! (edges-003) and the §5.1 `pull_request` status-derivation (edges-004); the octocrab/Linear
//! `Destination` adapters + auth bootstrap + the §17 `SyncFailed`/`auth_expired` wiring are gated/later.

pub mod classifier;
pub mod pull_request;
