//! The `ActionExecutor` seam (§6.2/§6.3). 2.3 ships the executor FRAMEWORK — the trait (validate /
//! execute / preview / optional rollback) + [`CatalogExecutor`] dispatching by the catalog
//! `ExecutorKind` — but the per-namespace executor BODIES stay **side-effect-free structured stubs**
//! (the real git2 / octocrab / session-host / Brain adapters land in their owning phase: 3/5/7/8,
//! whose modules don't exist yet). 2.1b's `StubExecutor` stays test-only.
//!
//! **The executor runs OFF the write-actor txn** — the Gateway invokes [`ActionExecutor::execute`]
//! BETWEEN the decision/approval txn and the `ActionStarted`/`ActionSucceeded` txn, never inside a
//! `gateway_txn` (2.3's slow/blocking git-CLI/octocrab executors must not starve the single writer).
//!
//! **§7.2 read-source split:** the risk-0 auto-execute path runs the executor off the **in-memory
//! reconciled `ActionRequest`** (raw inputs — a §15-redaction FP must not break a legit execution);
//! the approve path runs off the **durable row** (`request::load`), which §7.2 deems canonical (the
//! in-memory inputs are gone at approve-time, possibly post-restart). Wired at the Gateway call
//! sites; pinned by `daemon/tests/executor.rs` #12/#13.

use nexusops_shared::actions::{ActionPreview, ActionRequest, ResourceRef};
use nexusops_shared::catalog::{self, ActionTypeCatalogEntry};
use nexusops_shared::time::Timestamp;

use crate::gateway::preview;

/// Pre-execution validation failures (§6.3). 2.3 enforces only the catalog `requires_resource_refs`
/// precondition; the structured execute-error taxonomy is 2.4 (`Failed` stays a `String` for now).
#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("action_type '{0}' requires at least one resource_ref but carries none")]
    MissingResourceRef(String),
    #[error("action_type '{0}' is not in the §6.3 catalog")]
    Uncatalogued(String),
}

/// The outcome of running an action's executor (§6.2 ActionResult, simplified for 2.3).
pub enum ExecutionOutcome {
    /// the action executed. `changed_resources` = the resources it changed (Q7; feeds a future §6.2
    /// `ActionResult`, 2.4 — the 2.3 stubs report the req's `resource_refs`, no real change).
    /// `detail` = a human-readable execution summary; the 2.3 stubs record "would execute via
    /// <namespace> (Phase N)" — which makes the per-namespace dispatch observable + records the stub
    /// provenance (the real adapters return a real summary). Daemon-internal (no `shared/` surface).
    Succeeded {
        changed_resources: Vec<ResourceRef>,
        detail: String,
    },
    /// the executor (or its `validate`) failed; the message is recorded on `ActionFailed`
    /// (structured taxonomy → 2.4).
    Failed(String),
}

/// Runs (or previews) an action's side effect (§6.2/§6.3). The Gateway invokes `execute` BETWEEN the
/// approval and the completion transitions — never inside a write-actor txn.
pub trait ActionExecutor: Send + Sync {
    /// pre-execution validation (Q5 — executor-owned; runs as the first step of `execute`). A failure
    /// surfaces as `ActionFailed` (never a silent skip).
    fn validate(&self, req: &ActionRequest) -> Result<(), ExecError>;
    /// run the action's side effect (2.3 stubs: NONE) and report the outcome.
    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome;
    /// a dry-run preview envelope (§6.2/§6.3). NOTE: the live `preview_action` path renders via
    /// `preview::generate_preview` directly; this trait method is reserved for real per-executor
    /// dry-runs (Phase 3+) and is not on the 2.3 production path.
    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview;
    /// roll back a completed action (§6.2 — optional). **Fail-closed default:** an executor that has
    /// not implemented rollback returns `Failed` (NEVER `Succeeded` — a caller must not infer a real
    /// rollback happened from the default; that could mark an action rolled-back when nothing ran).
    /// Real rollback lands with the executors + 2.4's rollback edges.
    fn rollback(&self, _req: &ActionRequest) -> ExecutionOutcome {
        ExecutionOutcome::Failed(
            "rollback not implemented for this executor (2.3 — lands with the real adapters / 2.4)"
                .to_string(),
        )
    }
}

/// 2.1b STUB — **NO real side effect**, **NO precondition check** (test-only; the production path
/// uses [`CatalogExecutor`]). `execute` records a "would-execute"; `preview` a minimal envelope.
pub struct StubExecutor;

impl ActionExecutor for StubExecutor {
    fn validate(&self, _req: &ActionRequest) -> Result<(), ExecError> {
        Ok(()) // the stub validates nothing — production validation is CatalogExecutor's.
    }

    fn execute(&self, _req: &ActionRequest) -> ExecutionOutcome {
        // would-execute: no FS/git/network side effect — the lifecycle completes so the chokepoint +
        // its events stay test-first.
        ExecutionOutcome::Succeeded {
            changed_resources: vec![],
            detail: "stub: no side effect".to_string(),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: format!("(stub preview) {} — test-only executor", req.action_type),
            changed_resources: req.resource_refs.clone(),
            cannot_preview_reason: None,
        }
    }
}

/// 2.3 — the **production** executor framework. `execute` validates (Q5) then dispatches by the
/// catalog `ExecutorKind` to a per-namespace handler; in 2.3 every handler is a **structured stub**
/// (NO FS/git/network side effect — the owning phase swaps its handler for the real adapter:
/// git2/octocrab/session-host/Brain, Phase 3/5/7/8). `validate` enforces the catalog
/// `requires_resource_refs` precondition (fail-closed → `ActionFailed`).
pub struct CatalogExecutor;

impl CatalogExecutor {
    /// Resolve the catalog entry AND enforce the `requires_resource_refs` precondition — the single
    /// source of both the lookup and the validation, shared by [`CatalogExecutor::validate`] (which
    /// discards the entry) and [`CatalogExecutor::execute`] (which dispatches on it). One lookup, one
    /// check — no redundant re-lookup, and the Phase-3 real arms reuse this resolved entry.
    fn resolve(&self, req: &ActionRequest) -> Result<ActionTypeCatalogEntry, ExecError> {
        let entry = catalog::lookup(&req.action_type)
            .ok_or_else(|| ExecError::Uncatalogued(req.action_type.clone()))?;
        if entry.requires_resource_refs && req.resource_refs.is_empty() {
            return Err(ExecError::MissingResourceRef(req.action_type.clone()));
        }
        Ok(entry)
    }
}

impl ActionExecutor for CatalogExecutor {
    fn validate(&self, req: &ActionRequest) -> Result<(), ExecError> {
        self.resolve(req).map(|_entry| ())
    }

    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        // Q5: validate FIRST (via resolve) — a failed precondition is an execution failure (→
        // ActionFailed), never a silent skip, never a panic in the write-actor path.
        let entry = match self.resolve(req) {
            Ok(entry) => entry,
            Err(e) => return ExecutionOutcome::Failed(e.to_string()),
        };
        // dispatch by ExecutorKind → a side-effect-free per-namespace stub. The `detail` names the
        // namespace + its owning phase, so the dispatch is observable + the stub provenance is
        // recorded. Each phase REPLACES its namespace's arm with the real adapter (which runs the
        // actual side effect + returns the true changed_resources + a real detail).
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: format!(
                "would execute `{}` via the {} adapter — no side effect in 2.3 (real adapter lands {})",
                req.action_type,
                preview::namespace_label(entry.executor),
                preview::owning_phase(entry.executor),
            ),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        // the catalog-driven render (the same `preview_action` uses). An uncatalogued type never
        // reaches a real row; a minimal envelope is the fail-closed fallback if ever called directly.
        match catalog::lookup(&req.action_type) {
            Some(entry) => preview::generate_preview(req, &entry, generated_at),
            None => ActionPreview {
                action_request_id: req.action_request_id.clone(),
                generated_at,
                risk_level: req.risk_level,
                risk_reasons: vec!["uncatalogued action_type".to_string()],
                summary: format!(
                    "no preview: '{}' is not in the §6.3 catalog",
                    req.action_type
                ),
                changed_resources: vec![],
                cannot_preview_reason: Some("uncatalogued action_type".to_string()),
            },
        }
    }
}
