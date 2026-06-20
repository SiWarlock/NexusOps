//! `proj_review` projector (D5b-1) — the structured-review read model (§7.2 / §11.2). Folds the
//! `ReviewSynced` event into a `proj_review` row, the exact `PullRequestProjector` (edges-025) precedent.
//!
//! The event-sourced columns come from the payload (`review_id`/`pr_number`/`reviewer`/`state`/
//! `submitted_at`/`body`) + the envelope (`project_id`); the `repo_id` is the LESSON-17 IMMUTABLE
//! sibling-read of the action's Repository resource_ref from the `action_requests` row (keyed by
//! `env.action_request_id`) — resource_refs never change post-submit, so a rebuild is deterministic.
//! `review_synced_at` is INTENTIONALLY NOT projected (the brief's row omits it): `submitted_at` is the
//! user-meaningful display timestamp; `review_synced_at` is event-bookkeeping (when the daemon synced it),
//! not row state — additive-later if a "last synced" indicator is ever needed.
//!
//! **`review_id` = the GitHub-native review id (the PK, NOT a minted ULID):** GitHub review ids are
//! globally unique, so `review_id` alone is a valid rebuild-safe key (no `{repo_id}#…` composite — that's
//! only needed when the natural id isn't globally unique, like `pr_number`). `proj_review` is in
//! `REBUILD_TABLES`. `state` binds the frozen `ReviewState` value enum via `wire_value` (the layer-correct
//! serde producer — no fork). `body` is free-form user review text, §15-redacted at the event (the
//! projector reads the persisted/redacted payload).
//!
//! Failure taxonomy (the edges-022 / pull_request precedent — three distinct cases):
//!  * **Healthy SKIP (no-op):** no `env.project_id`, no `env.action_request_id`, or no Repository
//!    resource_ref → a non-projectable event is skipped.
//!  * **Fail-closed `Db` (the MISSING-sibling integrity break):** the link is set but the durable
//!    `action_requests` row is GONE → the `?` propagates `QueryReturnedNoRows` → the txn aborts (LESSON 17).
//!  * **`Decode` → degrade (UNBINDABLE data):** a present-but-unparseable `resource_refs_json` OR a
//!    `ReviewSynced` payload that won't bind (e.g. an unknown `state`) → the projector degrades + skips;
//!    the generic reason NEVER echoes payload bytes (§15).

use rusqlite::{params, Transaction};

use nexusops_shared::actions::{ResourceRef, ResourceType};
use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::{ReviewSubmitted, ReviewSynced};
use nexusops_shared::status::ReviewState;
use nexusops_shared::time::Timestamp;

use super::{wire_value, ProjectionError, Projector};

pub struct ReviewProjector;

impl Projector for ReviewProjector {
    fn name(&self) -> &'static str {
        "review"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        let is_synced = env.event_type == ReviewSynced::EVENT_TYPE;
        let is_submitted = env.event_type == ReviewSubmitted::EVENT_TYPE;
        if !is_synced && !is_submitted {
            // folds ReviewSynced (the read sync, D5b-1) + ReviewSubmitted (the D10 write counterpart) —
            // both upsert by review_id into proj_review.
            return Ok(());
        }
        // identity-less → healthy skip: project_id is the envelope's; action_request_id is the link to
        // the sibling row that carries repo_id (a review keyed to no repo can't carry repo_id).
        let (Some(project_id), Some(action_request_id)) = (&env.project_id, &env.action_request_id)
        else {
            return Ok(());
        };

        // repo_id = the IMMUTABLE sibling read of the action's Repository resource_ref (LESSON 17). A
        // MISSING `action_requests` row is an integrity break → fail-closed `Db` (the `?` propagates
        // `QueryReturnedNoRows`; the pull_request precedent). resource_refs_json is §15-redacted, but the
        // repo_id (a ULID, redaction-allowlisted — LESSON 13) + the resource_type survive (low entropy).
        let resource_refs_json: String = tx.query_row(
            "SELECT resource_refs_json FROM action_requests WHERE action_request_id = ?1",
            params![action_request_id.as_str()],
            |r| r.get(0),
        )?;
        let refs: Vec<ResourceRef> = serde_json::from_str(&resource_refs_json).map_err(|_| {
            ProjectionError::Decode("action_requests.resource_refs did not bind".into())
        })?;
        // no Repository resource_ref → the review can't be keyed to a repo → healthy skip.
        let Some(repo_id) = refs
            .iter()
            .find(|r| r.resource_type == ResourceType::Repo)
            .map(|r| r.id.clone())
        else {
            return Ok(());
        };

        // reject-unknown on the payload — the reason MUST NOT echo (possibly sensitive) payload bytes (§15).
        // Both events carry the same six proj_review fields; extract them by event type (the D10 write
        // event ReviewSubmitted has a `commit_id` the row does NOT project — like ReviewSynced's
        // `review_synced_at`; row-irrelevant, additive-later). `state` is the frozen ReviewState (both).
        let (review_id, pr_number, reviewer, state_enum, submitted_at, body): (
            u64,
            u64,
            String,
            ReviewState,
            Option<Timestamp>,
            Option<String>,
        ) = if is_submitted {
            let p: ReviewSubmitted = serde_json::from_str(&env.payload_json).map_err(|_| {
                ProjectionError::Decode("ReviewSubmitted payload did not bind".into())
            })?;
            (
                p.review_id,
                p.pr_number,
                p.reviewer,
                p.state,
                p.submitted_at,
                p.body,
            )
        } else {
            let p: ReviewSynced = serde_json::from_str(&env.payload_json)
                .map_err(|_| ProjectionError::Decode("ReviewSynced payload did not bind".into()))?;
            (
                p.review_id,
                p.pr_number,
                p.reviewer,
                p.state,
                p.submitted_at,
                p.body,
            )
        };

        // state binds the frozen ReviewState value enum via `wire_value` (the canonical snake_case wire
        // string; the layer-correct serde producer — no fork; derived from the event, rebuild-safe LESSON 17).
        let state = wire_value(&state_enum)?;

        // review_id/pr_number → i64 for the INTEGER columns (a GitHub review/PR id is a non-negative
        // natural; u64→i64 is lossless for any real value). submitted_at → its RFC3339 string (None→NULL).
        tx.execute(
            "INSERT INTO proj_review \
             (review_id, pr_number, project_id, repo_id, reviewer, state, submitted_at, body, \
              updated_at_seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
             ON CONFLICT(review_id) DO UPDATE SET \
               pr_number=excluded.pr_number, project_id=excluded.project_id, repo_id=excluded.repo_id, \
               reviewer=excluded.reviewer, state=excluded.state, submitted_at=excluded.submitted_at, \
               body=excluded.body, updated_at_seq=excluded.updated_at_seq",
            params![
                review_id as i64,
                pr_number as i64,
                project_id.as_str(),
                repo_id,
                reviewer,
                state,
                submitted_at.as_ref().map(|t| t.as_str()),
                body,
                env.seq,
            ],
        )?;
        Ok(())
    }
}
