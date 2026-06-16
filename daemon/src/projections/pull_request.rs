//! `proj_pull_request` projector (P7.1, edges-025) — the GitHub-authoritative PR read cache (§7.2);
//! CLOSES the github read vertical (mutator → event → projection → IPC). The exact edges-022
//! `proj_worktree` precedent.
//!
//! Folds `PullRequestSynced` (the edges-023 `github.create_pr`/`_draft` emitter) into a
//! `proj_pull_request` row. The event-sourced columns come from the payload (`pr_number`/`branch`/
//! `base`/`status`/`pr_checked_at`) + the envelope (`project_id`); the `repo_id` is the LESSON-17
//! IMMUTABLE sibling-read of the action's Repository resource_ref from the `action_requests` row (keyed
//! by `env.action_request_id`) — resource_refs never change post-submit, so a rebuild (which reads
//! `action_requests` at its FINAL state) is deterministic.
//!
//! **`pr_id` = the deterministic `{repo_id}#{pr_number}` composite (NOT a minted ULID):**
//! `proj_pull_request` is in `REBUILD_TABLES`, so a minted key would change per replay → break
//! rebuild-equivalence. The `#` delimiter can appear in neither a ULID repo_id nor a numeric pr_number,
//! so the composite is unambiguous + unique per repo+PR. `status` binds the frozen §5.1 `PullRequest`
//! machine via `wire_value` (the layer-correct serde producer — no fork). `title` is NULL (the event
//! carries none); `mergeable`/`checks_summary` are NOT projected (no column — they already fed the
//! derived `status` in the edges-023 executor).
//!
//! Failure taxonomy (three distinct cases — the edges-022 precedent):
//!  * **Healthy SKIP (no-op, not a degrade):** no `env.project_id`, no `env.action_request_id`, or no
//!    Repository resource_ref → a non-projectable event is skipped.
//!  * **Fail-closed `Db` (the MISSING-sibling integrity break):** the link is set but the durable
//!    `action_requests` row is GONE → the `?` propagates `QueryReturnedNoRows` → the append/replay txn
//!    aborts (LESSON 17 — the sibling is durable from submit + survives rebuild, so its absence is
//!    corruption, never a silent default).
//!  * **`Decode` → degrade (UNBINDABLE data):** a present-but-unparseable `resource_refs_json` OR a
//!    `PullRequestSynced` payload that won't bind → the projector degrades + skips this event; the
//!    generic reason NEVER echoes payload bytes (§15).

use rusqlite::{params, Transaction};

use nexusops_shared::actions::{ResourceRef, ResourceType};
use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::PullRequestSynced;

use super::{wire_value, ProjectionError, Projector};

pub struct PullRequestProjector;

impl Projector for PullRequestProjector {
    fn name(&self) -> &'static str {
        "pull_request"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        if env.event_type != PullRequestSynced::EVENT_TYPE {
            // folds ONLY PullRequestSynced.
            return Ok(());
        }
        // identity-less → healthy skip: project_id is the envelope's; action_request_id is the link to
        // the sibling row that carries repo_id (a PR keyed to no repo can't be projected).
        let (Some(project_id), Some(action_request_id)) = (&env.project_id, &env.action_request_id)
        else {
            return Ok(());
        };

        // repo_id = the IMMUTABLE sibling read of the action's Repository resource_ref (LESSON 17). A
        // MISSING `action_requests` row is an integrity break → fail-closed `Db` (the `?` propagates
        // `QueryReturnedNoRows`; the worktree/approval_queue precedent). resource_refs_json is
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
        // no Repository resource_ref → the PR can't be keyed to a repo → healthy skip.
        let Some(repo_id) = refs
            .iter()
            .find(|r| r.resource_type == ResourceType::Repo)
            .map(|r| r.id.clone())
        else {
            return Ok(());
        };

        // reject-unknown on the payload — the reason MUST NOT echo (possibly sensitive) payload bytes (§15).
        let payload: PullRequestSynced = serde_json::from_str(&env.payload_json).map_err(|_| {
            ProjectionError::Decode("PullRequestSynced payload did not bind".into())
        })?;

        // status binds the frozen §5.1 PullRequest machine via `wire_value` (the canonical snake_case
        // wire string; the layer-correct serde producer — no fork).
        let status = wire_value(&payload.status)?;

        // pr_id = the deterministic `{repo_id}#{pr_number}` composite (rebuild-safe; NOT a minted ULID —
        // proj_pull_request is in REBUILD_TABLES). The `#` delimiter can't appear in a ULID repo_id or a
        // numeric pr_number → unambiguous + unique per repo+PR.
        let pr_id = format!("{repo_id}#{}", payload.pr_number);

        // `title` ← NULL (the event carries none) + `mergeable`/`checks_summary` are NOT projected (no
        // column — they already fed the derived `status`), so both are ABSENT from the INSERT (→ NULL for
        // title) AND from the DO UPDATE set. `pr_number` → i64 for the INTEGER column (a GitHub PR number
        // is a small positive natural; u64→i64 is lossless for any real value).
        tx.execute(
            "INSERT INTO proj_pull_request \
             (pr_id, project_id, repo_id, pr_number, status, head_branch, base_branch, pr_checked_at, \
              updated_at_seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
             ON CONFLICT(pr_id) DO UPDATE SET \
               project_id=excluded.project_id, repo_id=excluded.repo_id, pr_number=excluded.pr_number, \
               status=excluded.status, head_branch=excluded.head_branch, base_branch=excluded.base_branch, \
               pr_checked_at=excluded.pr_checked_at, updated_at_seq=excluded.updated_at_seq",
            params![
                pr_id,
                project_id.as_str(),
                repo_id,
                payload.pr_number as i64,
                status,
                payload.branch,
                payload.base,
                payload.pr_checked_at.as_str(),
                env.seq,
            ],
        )?;
        Ok(())
    }
}
