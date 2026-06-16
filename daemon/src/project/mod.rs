//! The `project` namespace (`ExecutorKind::Project`) — project registry + detection wiring.
//!
//! P5.1 (edges-019) lands the [`executor::ProjectExecutor`]: the `project.rescan` Gateway action's
//! executor, which composes the read-only detection engine (`git::detect` + `workflow::detect`) and
//! emits a `ProjectRescanned` event through the Gateway's in-txn §15 redaction gate. The
//! `projects`/`repositories` read-model registry projector that consumes `ProjectRescanned` is
//! DEFERRED to the registry-migration slice (it needs MIGRATION_9).

pub mod executor;
