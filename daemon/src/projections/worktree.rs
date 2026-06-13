//! `proj_worktree` projector (P5.2, edges-022) — the gated 5.2-remainder; the worktree read model.
//!
//! Folds `WorktreeCreated` (the edges-020 `git.create_worktree` emitter) into a `proj_worktree` row,
//! closing the P5.2 read vertical (mutator → event → projection → IPC). The event-sourced columns come
//! from the payload (`worktree_id`/`path`/`branch_name`/`base_branch`) + the envelope (`project_id`);
//! the `repo_id` is the LESSON-17 IMMUTABLE sibling-read of the action's Repository resource_ref from
//! the `action_requests` row (keyed by `env.action_request_id`) — resource_refs never change
//! post-submit, so a rebuild (which reads `action_requests` at its FINAL state) is deterministic.
//!
//! `status` binds the §5.1 Worktree OVERLAY axis; a just-created worktree's resting overlay is
//! `Creating` (the git-sync axis + the rest of the live-read cache — `dirty_state`/`ahead_count`/
//! `behind_count`/`last_commit_sha`/`pr_status`/`git_checked_at`/`owner_*`/`linked_task_id` — are left
//! NULL; a separate §7.2 live-read refresh populates them, NOT this projector). `status` is bound via
//! `wire_value` (the layer-correct serde producer — `git::precedence::DerivedWorktreeStatus` is an
//! EDGE module the persistence core must not import; both pin to the same serde wire value).
//!
//! Failure taxonomy (three distinct cases):
//!  * **Healthy SKIP (no-op, not a degrade):** no `env.project_id`, no `env.action_request_id`, or no
//!    Repository resource_ref → `proj_worktree.project_id`/`repo_id` are NOT NULL, so a non-projectable
//!    event is skipped (the `session.rs` precedent).
//!  * **Fail-closed `Db` (the MISSING-sibling integrity break):** the link is set but the durable
//!    `action_requests` row is GONE → the `?` propagates `QueryReturnedNoRows` → the append/replay txn
//!    aborts (LESSON 17 / the `approval_queue` precedent — the sibling is durable + survives rebuild, so
//!    its absence is corruption, never a silent default).
//!  * **`Decode` → degrade (UNBINDABLE data):** a present-but-unparseable `resource_refs_json` OR a
//!    `WorktreeCreated` payload that won't bind → the projector degrades + skips this event (the §7.2
//!    reject-unknown contained-failure norm, same as `session.rs`'s payload bind) — distinct from the
//!    missing-row integrity break above; the generic reason never echoes payload bytes (§15).

use rusqlite::{params, Transaction};

use nexusops_shared::actions::{ResourceRef, ResourceType};
use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::WorktreeCreated;
use nexusops_shared::status::WorktreeOverlay;

use super::{wire_value, ProjectionError, Projector};

pub struct WorktreeProjector;

impl Projector for WorktreeProjector {
    fn name(&self) -> &'static str {
        "worktree"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        if env.event_type != WorktreeCreated::EVENT_TYPE {
            // folds ONLY WorktreeCreated (the overlay-lifecycle events have no emitter yet).
            return Ok(());
        }
        // identity-less → healthy skip (proj_worktree.project_id/repo_id are NOT NULL): project_id is
        // the envelope's; action_request_id is the link to the sibling row that carries repo_id.
        let (Some(project_id), Some(action_request_id)) = (&env.project_id, &env.action_request_id)
        else {
            return Ok(());
        };

        // repo_id = the IMMUTABLE sibling read of the action's Repository resource_ref (LESSON 17). A
        // MISSING `action_requests` row is an integrity break → fail-closed `Db` error (the `?`
        // propagates `QueryReturnedNoRows`; the approval_queue precedent). resource_refs_json is
        // §15-redacted, but the repo_id (a ULID, redaction-allowlisted — LESSON 13) + the resource_type
        // survive (low entropy).
        let resource_refs_json: String = tx.query_row(
            "SELECT resource_refs_json FROM action_requests WHERE action_request_id = ?1",
            params![action_request_id.as_str()],
            |r| r.get(0),
        )?;
        let refs: Vec<ResourceRef> = serde_json::from_str(&resource_refs_json).map_err(|_| {
            ProjectionError::Decode("action_requests.resource_refs did not bind".into())
        })?;
        // no Repository resource_ref → the worktree can't be keyed to a repo → healthy skip.
        let Some(repo_id) = refs
            .iter()
            .find(|r| r.resource_type == ResourceType::Repo)
            .map(|r| r.id.clone())
        else {
            return Ok(());
        };

        // reject-unknown on the payload — the reason MUST NOT echo (possibly sensitive) payload bytes (§15).
        let payload: WorktreeCreated = serde_json::from_str(&env.payload_json)
            .map_err(|_| ProjectionError::Decode("WorktreeCreated payload did not bind".into()))?;

        // the §5.1 Worktree OVERLAY resting state for a just-created worktree → "creating". Bound via
        // `wire_value` (the frozen enum → its canonical snake_case wire string); the git-sync axis +
        // the live-read cache columns are NULL (omitted from the INSERT), populated by the §7.2 refresh.
        let status = wire_value(&WorktreeOverlay::Creating)?;

        // the live-read-cache columns are deliberately ABSENT from the column list (→ inserted NULL),
        // and ABSENT from the DO UPDATE set (→ a re-fold/rebuild preserves any §7.2-refresh values).
        tx.execute(
            "INSERT INTO proj_worktree \
             (worktree_id, project_id, repo_id, path, branch_name, base_branch, status, updated_at_seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT(worktree_id) DO UPDATE SET \
               project_id=excluded.project_id, repo_id=excluded.repo_id, path=excluded.path, \
               branch_name=excluded.branch_name, base_branch=excluded.base_branch, \
               status=excluded.status, updated_at_seq=excluded.updated_at_seq",
            params![
                payload.worktree_id.as_str(),
                project_id.as_str(),
                repo_id,
                payload.path,
                payload.branch_name,
                payload.base_branch,
                status,
                env.seq,
            ],
        )?;
        Ok(())
    }
}
