//! Crash-reconcile (§17 daemon-crash-mid-action, 2.4 L5 — the Phase-2 capstone).
//!
//! On restart, [`reconcile_orphans`] scans the action rows a crash stranded mid-pipeline
//! (`status IN ('executing','queued')`) and drives each to a TERMINAL state, emitting the missing
//! terminal event so the audit log is complete + no action is left stuck. Wired into
//! [`crate::bootstrap::cold_start`] (after the event store's catch-up-replay + outbox crash-recovery
//! + the quarantine audit-events — the restart-recovery composition point).
//!
//! **The two orphan classes (the Q6 dedup-key contract):**
//! - **`queued`** — the crash happened AFTER approve+queue but BEFORE execute began, so the executor
//!   never ran → definitively NO side effect. Reconcile → `failed` + `ActionError::ExecutorError`
//!   (the honest crash message) and **CLEARS** the `idempotency_key` (the action never ran → it is
//!   safe to re-submit; clearing the key avoids the 2.3 dedup re-run lockout).
//! - **`executing`** — the crash happened mid-execute, so a side effect MAY have landed. 2.4's stub
//!   executor has no real side-effect re-read, so the outcome is UN-RECONCILABLE → `failed` +
//!   `ActionError::UnknownOutcome` (the loud, consumer-visible audit-integrity record, the L2
//!   `ActionPartiallySucceeded` pattern) and **KEEPS** the `idempotency_key` (dedup must protect
//!   against a double-run of a maybe-applied mutation: a re-submit replays the original, not a 2nd run).
//!
//! The re-derivation INPUTS (`idempotency_key` + `locks::validate_held`) are what the REAL adapters
//! (Phase 5/7) use for a DEFINITE `succeeded`/`failed` re-read; 2.4 lands the scan + the unknown-outcome
//! fallback + the Q6 key contract. A crashed orphan's lease is freed by the **reaper** (reconcile drives
//! the action terminal; it does not release the lease — most crash leases are already TTL-expired at
//! restart; a prompt-release is a Future-TODO). Each orphan is reconciled in ITS OWN atomic gateway txn
//! (fail-closed): if one orphan's terminal write fails, it stays orphaned for the next restart.

use nexusops_shared::actions::ActionError;
use nexusops_shared::status::ActionRequest as ARStatus;

use crate::eventstore::EventStore;
use crate::gateway::{request, GatewayError};

/// Reconcile every orphaned (`executing`/`queued`) action to a terminal state on restart (§17 L5).
/// Returns the count reconciled. **plan_id-agnostic** — scans by status, so a single action and a
/// plan-cascade step (the 2.1c N-orphan carry-forward) reconcile identically.
pub fn reconcile_orphans(store: &mut EventStore) -> Result<usize, GatewayError> {
    let orphans = request::scan_orphans(store.read_conn())?;
    let mut reconciled = 0usize;
    for (act_id, status) in orphans {
        store.gateway_txn(|gtx| -> Result<(), GatewayError> {
            let now = gtx.now_rfc3339();
            let req = request::load(gtx.tx(), &act_id)?;
            match status {
                // queued: never executed → no side effect. Terminal `failed` + CLEAR the dedup key
                // (safe to re-submit). The taxonomy has no "never-ran" variant yet → ExecutorError with
                // an honest message (Future-TODO: a distinct `daemon_crashed`/`abandoned` variant).
                ARStatus::Queued => {
                    request::update_status(gtx.tx(), &act_id, ARStatus::Queued, ARStatus::Failed)?;
                    gtx.append(&request::failed_intent(
                        &req,
                        ActionError::ExecutorError {
                            message: "reconciled crash-orphan: the daemon crashed before execute; \
                                      the action NEVER ran, no side effect — safe to re-submit"
                                .to_string(),
                        },
                        &now,
                    )?)?;
                    request::clear_idempotency_key(gtx.tx(), &act_id)?;
                }
                // executing: a side effect MAY have landed → un-reconcilable in 2.4 → unknown_outcome
                // (the loud audit-integrity record) + KEEP the dedup key (protect against a double-run).
                ARStatus::Executing => {
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::Executing,
                        ARStatus::Failed,
                    )?;
                    gtx.append(&request::failed_intent(
                        &req,
                        ActionError::UnknownOutcome,
                        &now,
                    )?)?;
                }
                // the scan only returns executing/queued; any other status is a logic error.
                other => {
                    return Err(GatewayError::Serialize(format!(
                        "reconcile_orphans scanned a non-orphan status: {other:?}"
                    )))
                }
            }
            Ok(())
        })?;
        reconciled += 1;
    }
    Ok(reconciled)
}
