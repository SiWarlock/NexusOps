//! The `project.rescan` executor (P5.1, edges-019) — `ExecutorKind::Project`.
//!
//! Runs the read-only detection engine (`detect_git` + `detect_workflow`) over the scan path carried
//! in `req.inputs["path"]` and emits a `ProjectRescanned` event through the Gateway's in-txn §15
//! redaction gate (`ExecutionOutcome.emitted_events` → the pipeline appends it in txn-B ATOMIC with
//! `ActionSucceeded`; the `SessionExecutor`/`SessionStarted` precedent, LESSON §28). The git
//! `remote_url` credential userinfo is stripped AT THE EMIT SOURCE ([`strip_userinfo`], §15 rule #5);
//! the Redactor is the backstop only (a generic URL password has no detectable prefix — LESSON §13).
//!
//! **Read-only (`side_effect_applied: false`):** detection is git2 reads + filesystem presence checks
//! — no FS/git/network mutation. The emitted event IS the audit record but not a durable EXTERNAL side
//! effect, so a txn-B append failure rolls back cleanly (the action stays `executing` → L5), never
//! `ActionPartiallySucceeded` (§17). This is the FIRST real edges Gateway mutator-path Action;
//! INV-SEC-1 holds — the executor runs only via the Gateway's gated execute seam and emits only via
//! the in-txn append.

use std::path::Path;

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::events::ProjectRescanned;
use nexusops_shared::time::Timestamp;

use crate::clock::Clock;
use crate::gateway::executor::{ActionExecutor, EmittedEvent, ExecError, ExecutionOutcome};
use crate::git::detect::detect_git;
use crate::workflow::detect::detect_workflow;

/// The action type this leaf executor handles (`ExecutorKind::Project`).
const PROJECT_RESCAN: &str = "project.rescan";

/// Strip credential userinfo from a git remote URL **at the emit source** (§15 rule #5).
///
/// Only a **scheme-bearing** URL (`scheme://authority/…`) carries credential userinfo in its
/// authority. The userinfo is the span up to and including the **LAST `@` IN THE AUTHORITY** (a
/// password may itself contain `@`; an `@` in the PATH is never userinfo). We strip the WHOLE userinfo
/// — both the `user` and any `:password` — because a token can ride the bare-username slot
/// (`https://ghp_xxx@host`) and we cannot safely distinguish it from a benign ssh user without a
/// brittle allowlist (Q2). A **scp-style** ssh remote (`git@host:path`, no `://`) carries no scheme/
/// authority split — `git@` is the ssh username, not a secret — so it is left intact.
///
/// Deterministic + prefix-free (the Redactor's entropy/prefix net is the backstop only, LESSON §13).
pub fn strip_userinfo(url: &str) -> String {
    // no scheme delimiter → scp-style or schemeless → no authority userinfo to strip.
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };
    let authority_start = scheme_end + "://".len();
    // the authority runs until the first '/', '?', or '#' after it (RFC 3986 authority terminators);
    // absent any, the authority is the rest of the string.
    let rest_offset = url[authority_start..]
        .find(['/', '?', '#'])
        .map(|i| authority_start + i)
        .unwrap_or(url.len());
    let authority = &url[authority_start..rest_offset];
    // userinfo is delimited by the LAST '@' within the authority (authority-scoped + correct
    // delimiter — a naive first-'@' strip would leak a password containing '@').
    let Some(at) = authority.rfind('@') else {
        return url.to_string(); // no userinfo
    };
    let host_and_port = &authority[at + 1..];
    format!(
        "{}{}{}",
        &url[..authority_start], // "scheme://"
        host_and_port,           // "host[:port]"
        &url[rest_offset..],     // "/path?query#frag"
    )
}

/// Runs project detection + emits `ProjectRescanned`. Holds an injected [`Clock`] for the
/// deterministic `scanned_at` stamp (golden-log replay, LESSON §3).
pub struct ProjectExecutor {
    clock: Box<dyn Clock>,
}

impl ProjectExecutor {
    pub fn new(clock: Box<dyn Clock>) -> Self {
        Self { clock }
    }

    fn execute_rescan(&self, req: &ActionRequest) -> ExecutionOutcome {
        // the scan path is REQUIRED in inputs["path"] (project.rescan is requires_resource_refs=false
        // — the path is the target, not a resource_ref). A missing/blank path FAILS CLOSED — never a
        // silent skip, never a scan of the daemon's own cwd (§15/fail-closed).
        let path = match req.inputs.get("path").and_then(|v| v.as_str()) {
            Some(p) if !p.trim().is_empty() => p,
            _ => {
                return ExecutionOutcome::Failed(
                    "project.rescan requires a non-empty inputs[\"path\"]".to_string(),
                )
            }
        };
        let scan_path = Path::new(path);

        // read-only detection (git2 reads + FS presence) — both degrade to a valid default on a
        // non-git / missing path, never an Err/panic.
        let git = detect_git(scan_path);
        let wf = detect_workflow(scan_path);

        // the Clock contract guarantees valid RFC3339-Z (clock.rs) — an invalid stamp is
        // contract-impossible, but fail CLOSED rather than emit a malformed audit event.
        let scanned_at = match Timestamp::parse(&self.clock.now_rfc3339()) {
            Ok(ts) => ts,
            Err(e) => return ExecutionOutcome::Failed(format!("invalid clock timestamp: {e}")),
        };

        // detail is read BEFORE the partial moves of git's fields into the payload below.
        let detail = format!(
            "project.rescan — scanned `{path}` (is_git={}, dirty={})",
            git.is_git, git.is_dirty
        );

        // map the two detection structs into the frozen ProjectRescanned field set. §15 rule #5 —
        // strip the remote_url credential userinfo AT SOURCE. PathBuf → String is lossy (never drop a
        // path on a non-UTF-8 component — a degraded-but-present root beats a silently-absent one).
        let payload = ProjectRescanned {
            is_git: git.is_git,
            repo_root: git.repo_root.map(|p| p.to_string_lossy().into_owned()),
            remote_url: git.remote_url.map(|u| strip_userinfo(&u)),
            branch: git.branch,
            detached: git.detached,
            is_dirty: git.is_dirty,
            workflow_pack: wf.workflow_pack,
            cc_crew: wf.cc_crew,
            plan_file: wf.plan_file.map(|p| p.to_string_lossy().into_owned()),
            brain: wf.brain,
            scanned_at,
        };

        // the executor serializes its own FROZEN event struct (Q1=B — the generic Namespaced bridge);
        // a serialize fault fails CLOSED (no malformed event ever reaches the in-txn append).
        let payload_json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => return ExecutionOutcome::Failed(format!("serialize ProjectRescanned: {e}")),
        };

        ExecutionOutcome::Succeeded {
            changed_resources: vec![],
            detail,
            // read-only detection: no durable EXTERNAL side effect (the event is the audit record, not
            // an external mutation) → a txn-B fault rolls back cleanly, never ActionPartiallySucceeded.
            side_effect_applied: false,
            emitted_events: vec![EmittedEvent::Namespaced {
                event_type: ProjectRescanned::EVENT_TYPE,
                payload_json,
            }],
        }
    }
}

impl ActionExecutor for ProjectExecutor {
    fn validate(&self, _req: &ActionRequest) -> Result<(), ExecError> {
        // ProjectExecutor is a LEAF handler registered for ExecutorKind::Project — the outer
        // CatalogExecutor.resolve() already enforced the catalog `requires_resource_refs` precondition
        // BEFORE dispatching here (and project.rescan is requires_resource_refs=false anyway), so there
        // is nothing left to validate at this leaf.
        Ok(())
    }

    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        match req.action_type.as_str() {
            PROJECT_RESCAN => self.execute_rescan(req),
            // a non-rescan action should never reach this leaf (the outer CatalogExecutor dispatches by
            // ExecutorKind::Project, and project.rescan is the only Project-kind type today) — but fail
            // CLOSED rather than silently succeed if a future Project-kind type is catalogued before its
            // arm lands here.
            other => ExecutionOutcome::Failed(format!(
                "ProjectExecutor has no handler for action_type `{other}`"
            )),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        // never on the production path (the outer CatalogExecutor renders previews via the catalog) — a
        // minimal envelope, mirroring the StubExecutor pattern.
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: format!("project executor: {}", req.action_type),
            changed_resources: vec![],
            cannot_preview_reason: None,
        }
    }
}
