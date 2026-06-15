//! `proj_project` + `proj_repository` registry projector (P5.1, edges-028) — the gated P5.1 remainder;
//! the event-fed project registry read model.
//!
//! Folds `ProjectRescanned` (the edges-019 `project.rescan` emitter) into TWO rows split by axis:
//! `proj_project` (the project-IDENTITY fields — `workflow_pack`/`cc_crew`/`plan_file`/`brain`/
//! `scanned_at`) + `proj_repository` (the git-DETECTION fields — `is_git`/`repo_root`/`remote_url`/
//! `branch`/`detached`/`is_dirty`/`scanned_at`), both keyed by `env.project_id` (1:1 MVP), closing the
//! P5.1 read vertical (rescan executor → event → projection).
//!
//! SIMPLER than the edges-022 `WorktreeProjector`: `ProjectRescanned` is self-contained — its detection
//! fields are in the payload and the project IDENTITY (`project_id`) is on the envelope, so there is NO
//! LESSON-17 sibling-read (`project.rescan` is `requires_resource_refs=false`).
//!
//! These are **event-fed projections** (rebuildable, in `REBUILD_TABLES`), NOT the DATA_MODEL §2.8
//! durable-registry direct-write rows (the canonical `name`/`workspace_id`/`policy_json`/`created_at` +
//! a `register_project` mutator are the deferred fuller model — lead-ruled at the R1b `ProjectRescanned`
//! freeze). A re-rescan is coarse-atomic: it UPSERTs (REPLACES the detection snapshot, advances the seq).
//!
//! Failure taxonomy (two cases — no integrity-break case: no sibling read):
//!  * **Healthy SKIP (no-op, not a degrade):** no `env.project_id` → `proj_project.project_id` is NOT
//!    NULL, so a non-projectable event is skipped (the `session.rs`/`worktree.rs` precedent).
//!  * **`Decode` → degrade (UNBINDABLE data):** a `ProjectRescanned` payload that won't bind → the
//!    projector degrades + skips this event (the §7.2 reject-unknown contained-failure norm); the generic
//!    reason never echoes payload bytes (§15 — the `map_err` discards the serde error).
//!
//! §15: `remote_url` is read from the **already-committed, already-redacted** event (userinfo was
//! stripped at source in edges-019 + the redactor backstop) and written through to the column — the
//! projector never re-handles a token and never logs the value (no new redaction surface).

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::ProjectRescanned;

use super::{ProjectionError, Projector};

pub struct ProjectRegistryProjector;

impl Projector for ProjectRegistryProjector {
    fn name(&self) -> &'static str {
        "project_registry"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        if env.event_type != ProjectRescanned::EVENT_TYPE {
            // folds ONLY ProjectRescanned.
            return Ok(());
        }
        // identity-less → healthy skip (proj_project.project_id is NOT NULL): project_id is the envelope's.
        let Some(project_id) = &env.project_id else {
            return Ok(());
        };

        // reject-unknown on the payload — the reason MUST NOT echo (possibly sensitive) payload bytes (§15).
        let p: ProjectRescanned = serde_json::from_str(&env.payload_json)
            .map_err(|_| ProjectionError::Decode("ProjectRescanned payload did not bind".into()))?;

        // the project-IDENTITY axis.
        tx.execute(
            "INSERT INTO proj_project \
             (project_id, workflow_pack, cc_crew, plan_file, brain, scanned_at, updated_at_seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(project_id) DO UPDATE SET \
               workflow_pack=excluded.workflow_pack, cc_crew=excluded.cc_crew, \
               plan_file=excluded.plan_file, brain=excluded.brain, \
               scanned_at=excluded.scanned_at, updated_at_seq=excluded.updated_at_seq",
            params![
                project_id.as_str(),
                p.workflow_pack,
                p.cc_crew,
                p.plan_file,
                p.brain,
                p.scanned_at.as_str(),
                env.seq,
            ],
        )?;

        // the git-DETECTION axis (1:1 by project_id; remote_url = the already-stripped committed value).
        tx.execute(
            "INSERT INTO proj_repository \
             (project_id, is_git, repo_root, remote_url, branch, detached, is_dirty, scanned_at, \
              updated_at_seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
             ON CONFLICT(project_id) DO UPDATE SET \
               is_git=excluded.is_git, repo_root=excluded.repo_root, remote_url=excluded.remote_url, \
               branch=excluded.branch, detached=excluded.detached, is_dirty=excluded.is_dirty, \
               scanned_at=excluded.scanned_at, updated_at_seq=excluded.updated_at_seq",
            params![
                project_id.as_str(),
                p.is_git,
                p.repo_root,
                p.remote_url,
                p.branch,
                p.detached,
                p.is_dirty,
                p.scanned_at.as_str(),
                env.seq,
            ],
        )?;
        Ok(())
    }
}
