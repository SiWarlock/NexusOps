//! CodexLaunchSpec (§9.1 / §15) — the pure `codex exec` argv (the no-bypass enforcement surface, the
//! `ClaudeLaunchSpec` analog) + the auth-pinning env-hygiene + the resume-handle.
//!
//! **No-bypass by construction (§15):** `--sandbox` + `--ask-for-approval` are ALWAYS emitted (from the
//! bound profile's REQUIRED fields); there is NO code path that emits `--yolo` /
//! `--dangerously-bypass-approvals-and-sandbox` / `--full-auto`. The spec is the no-bypass half of the
//! INV-SEC-1 enforcement surface; the sandbox-as-OS-containment PROOF + the value choice are 3.3c. This
//! module **spawns NOTHING** (the binding condition — the live codex + its `PreToolUse`→Gateway
//! interception land together at 3.3c, the 042→043/4.0b-1→4.0b-2 precedent).

use std::path::{Path, PathBuf};

use crate::terminal::EnvMutation;

use super::auth::{auth_env_mutations, CodexExecutionProfile};
use super::perms::CODEX_CHILD_UMASK;

/// The O-13-style Codex launch spec: the `codex exec [resume <UUID>] --json --sandbox <s>
/// --ask-for-approval <a> --model <m> [--profile <p>]` argv + the auth-pinning env-hygiene + the child
/// umask. PURE (no I/O — the real spawn + the `harden_codex_dirs`/umask application are 3.3c). A bypass
/// is impossible by construction.
pub struct CodexLaunchSpec {
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    /// the daemon session id → the `NEXUSOPS_SESSION_ID` correlation key the child/hook inherits.
    session_id: String,
    /// the resolved auth env-var to KEEP (env-hygiene strips the rest); `None` = file-based ChatGPT.
    auth_source_env: Option<String>,
}

impl CodexLaunchSpec {
    /// A fresh-session spec (`codex exec …`).
    pub fn build(cwd: &Path, session_id: &str, profile: &CodexExecutionProfile) -> Self {
        Self::build_inner(cwd, session_id, profile, None)
    }

    /// A resume spec (`codex exec resume <UUID> …`) — keyed off the rollout UUIDv7 (the CONFIRMED-LOCAL
    /// CLI form; the app-server `thread/resume`/`thr_` path is HITL Open-Q #3/#4).
    pub fn build_resume(
        cwd: &Path,
        rollout_uuid: &str,
        session_id: &str,
        profile: &CodexExecutionProfile,
    ) -> Self {
        Self::build_inner(cwd, session_id, profile, Some(rollout_uuid))
    }

    fn build_inner(
        cwd: &Path,
        session_id: &str,
        profile: &CodexExecutionProfile,
        resume_uuid: Option<&str>,
    ) -> Self {
        let mut args = vec!["exec".to_string()];
        if let Some(uuid) = resume_uuid {
            args.push("resume".to_string());
            args.push(uuid.to_string());
        }
        args.push("--json".to_string());
        // --sandbox + --ask-for-approval ALWAYS present (from the profile's REQUIRED fields) — the
        // no-bypass enforcement surface (§15). No --yolo/--dangerously-bypass/--full-auto path exists.
        args.push("--sandbox".to_string());
        args.push(profile.sandbox.clone());
        args.push("--ask-for-approval".to_string());
        args.push(profile.approval_policy.clone());
        args.push("--model".to_string());
        args.push(profile.model.clone());
        if let Some(p) = &profile.profile {
            args.push("--profile".to_string());
            args.push(p.clone());
        }
        Self {
            program: "codex".to_string(),
            args,
            cwd: cwd.to_path_buf(),
            session_id: session_id.to_string(),
            auth_source_env: profile.auth_source_env.clone(),
        }
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// The full argv (program + args).
    pub fn argv(&self) -> Vec<String> {
        let mut v = Vec::with_capacity(self.args.len() + 1);
        v.push(self.program.clone());
        v.extend(self.args.iter().cloned());
        v
    }

    /// The §15 #8 env-hygiene: strip every non-chosen auth env var (exactly one source reaches the
    /// child) + SET the `NEXUSOPS_SESSION_ID` correlation key (the child/hook inherits it).
    pub fn env_mutations(&self) -> Vec<EnvMutation> {
        let mut muts = auth_env_mutations(self.auth_source_env.as_deref());
        muts.push(EnvMutation::set("NEXUSOPS_SESSION_ID", &self.session_id));
        muts
    }

    /// The child umask (§15 #11) the spawner applies pre-exec at the real spawn (3.3c).
    pub fn umask(&self) -> u32 {
        CODEX_CHILD_UMASK
    }
}

/// Whether a resumable rollout handle exists (the rollout UUIDv7) — true iff `Some` (so the
/// harness-agnostic `decide_resume` picks `Resumed` for Codex vs falls through). The app-server
/// `thr_`/`thread/resume` path + the UUID↔`thr_` interconversion are HITL (research Open-Q #3/#4).
pub fn has_resume_handle(rollout_uuid: Option<&str>) -> bool {
    rollout_uuid.is_some()
}
