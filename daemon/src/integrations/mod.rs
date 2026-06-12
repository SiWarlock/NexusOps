//! `integrations/` — external connectors (GitHub via octocrab, Linear) + the §17 integration-failure
//! classifier. Adapters are edges: they submit intents to the Gateway and drain the outbox; they
//! NEVER write the DB directly. Landed deterministic cores: the `classifier` (edges-003), the §5.1
//! `pull_request` status-derivation + GitHub-response decode (edges-004/006/008), and the `github`
//! PR read client (edges-009 — trait + fake + injected-octocrab live fetch). The `github`/Linear
//! executor arms + auth bootstrap + the §17 `SyncFailed`/`auth_expired` wiring are gated/later.

pub mod classifier;
pub mod github;
pub mod pull_request;
