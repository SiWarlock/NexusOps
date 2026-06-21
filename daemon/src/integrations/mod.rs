//! `integrations/` — external connectors (GitHub via octocrab, Linear) + the §17 integration-failure
//! classifier. Adapters are edges: they submit intents to the Gateway and drain the outbox; they
//! NEVER write the DB directly. Landed deterministic cores: the `classifier` (edges-003); the §5.1
//! `pull_request` status-derivation + GitHub-response decode (edges-004/006/008) + the `github` PR
//! read client (edges-009/010 — trait + fake + injected-octocrab REST/GraphQL fetch); and the
//! `linear` issue-state derivation (edges-013 — `WorkflowState.type` → §5.1 `Task`). The `github`/
//! Linear executor arms + auth bootstrap + the §17 `SyncFailed`/`auth_expired` wiring are gated/later.

pub mod auth;
pub mod classifier;
pub mod connect;
pub mod connections;
pub mod executor;
pub mod github;
pub mod github_write;
pub mod keychain;
pub mod linear;
pub mod linear_write;
pub mod pull_request;
pub mod repo_resolve;
