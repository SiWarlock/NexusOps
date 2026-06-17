//! CodexLaunchSpec (§9.1 / §15) — the pure `codex exec` argv (the no-bypass enforcement surface, the
//! `ClaudeLaunchSpec` analog) + the auth-pinning env-hygiene + the resume-handle.
//!
//! **No-bypass by construction (§15):** `--sandbox` + `--ask-for-approval` are ALWAYS emitted (from the
//! bound profile's REQUIRED fields); there is NO code path that emits `--yolo` /
//! `--dangerously-bypass-approvals-and-sandbox` / `--full-auto`. The spec is the no-bypass half of the
//! INV-SEC-1 enforcement surface; the sandbox-as-OS-containment PROOF + the value choice are 3.3c. This
//! module **spawns NOTHING** (the binding condition — the live codex + its `PreToolUse`→Gateway
//! interception land together at 3.3c, the 042→043/4.0b-1→4.0b-2 precedent).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

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

// ---- CodexLaunchConfig — the §15 DEFENSE-IN-DEPTH containment + the PreToolUse hook wiring (3.3c) ---

/// The Codex `PreToolUse` hook command's timeout (seconds) in the generated config — MUST exceed the
/// daemon's ~5-min `APPROVAL_WAIT` so a legitimately-pending human approval is NOT cut short (research
/// Open-Q #1: "a generous/disabled hook timeout"); matches the hook client's `HOOK_READ_TIMEOUT`. The
/// exact live value (and whether Codex honors a per-hook `timeout`) is HITL-tuned.
pub const CODEX_HOOK_TIMEOUT_SECS: u64 = 360;

/// The `--sandbox workspace-write` DEFENSE-IN-DEPTH containment + the `PreToolUse` hook wiring (brief
/// 066, the cat-1 pin). Codex's `PreToolUse` is a GUARDRAIL, not a boundary (research §4.3) — so the
/// hook is the adjudication+audit channel and THIS is the OS-enforcement boundary: workspace-write
/// scoped to {the worktree + per-profile user-approved extra WRITE paths}, network OFF (USER-CONFIRMED
/// 2026-06-15). **PURE** (no I/O — the on-disk write of `config.toml`/`hooks.json` under `codex_home` +
/// the `CODEX_HOME` wiring + the live trusted-hash tamper handshake are the HITL live-spawn follow-on,
/// the no-spawn binding condition). Grammar (Context7, codex rust-v0.75.0): `sandbox_mode`,
/// `[sandbox_workspace_write] writable_roots`/`network_access`. **Read-scope nuance:** workspace-write
/// reads ALL files; `writable_roots` is a WRITE boundary ONLY — which is exactly what INV-SEC-1 (a
/// MUTATION invariant) requires.
pub struct CodexLaunchConfig {
    /// the DAEMON-resolved codex home (the 3.3b security NIT — never an agent-supplied path); the
    /// generated config + the rollout store live here, and `CODEX_HOME` points the child at it.
    codex_home: PathBuf,
    /// the §15 WRITE boundary: the worktree (cwd) + the per-profile user-approved extra WRITE paths —
    /// never arbitrary (the containment guarantee).
    writable_roots: Vec<String>,
    /// the full `PreToolUse` hook command (`<daemon> hook --harness codex PreToolUse`).
    hook_command: String,
    /// the `[hooks.state]` trust hash over the registered command (Codex re-prompts if it changes — a
    /// tamper check the adapter satisfies by registering once + keeping it stable, research §4.1).
    trusted_hash: String,
}

impl CodexLaunchConfig {
    /// Build the containment + hook-config from the DAEMON-resolved `codex_home`, the worktree `cwd`,
    /// the per-profile user-approved `extra_writable_roots`, and the daemon hook `receiver` (e.g.
    /// `"<daemon> hook --harness codex"`). The hook command appends the `PreToolUse` event; the
    /// trusted-hash is a stable SHA-256 over that command.
    pub fn build(
        codex_home: &Path,
        cwd: &Path,
        extra_writable_roots: &[String],
        receiver: &str,
    ) -> Self {
        let mut writable_roots = vec![cwd.to_string_lossy().into_owned()];
        writable_roots.extend(extra_writable_roots.iter().cloned());
        let hook_command = format!("{receiver} PreToolUse");
        let trusted_hash = trusted_hash_of(&hook_command);
        Self {
            codex_home: codex_home.to_path_buf(),
            writable_roots,
            hook_command,
            trusted_hash,
        }
    }

    /// The sandbox mode — always `workspace-write` (the USER-CONFIRMED containment; never
    /// `danger-full-access`, never a bypass).
    pub fn sandbox_mode(&self) -> &str {
        "workspace-write"
    }

    /// Network access inside the sandbox — always `false` (pinned explicitly, not relied-on-default).
    pub fn network_access(&self) -> bool {
        false
    }

    /// The §15 WRITE boundary — {worktree + per-profile approved extra paths}, never arbitrary.
    pub fn writable_roots(&self) -> &[String] {
        &self.writable_roots
    }

    /// The DAEMON-resolved codex home (never agent-supplied).
    pub fn codex_home(&self) -> &Path {
        &self.codex_home
    }

    /// The registered `[hooks.state]` trust hash (non-empty; Codex's tamper check).
    pub fn trusted_hash(&self) -> &str {
        &self.trusted_hash
    }

    /// Whether NO sandbox bypass is configured — ALWAYS true (workspace-write is never
    /// `danger-full-access`; the spec carries no `--yolo`/`--dangerously-bypass`/`--full-auto`). A
    /// constant by construction (the no-bypass enforcement surface, the `CodexLaunchSpec` precedent).
    pub fn has_bypass(&self) -> bool {
        false
    }

    /// Whether the generated hook-config references `receiver` as the PreToolUse hook command PROGRAM —
    /// an exact first-token match (`<receiver>` or `<receiver> …`), not a loose substring (the
    /// `ClaudeSettings::references_receiver` precedent).
    pub fn references_receiver(&self, receiver: &str) -> bool {
        let prefix = format!("{receiver} ");
        self.hook_command == receiver || self.hook_command.starts_with(&prefix)
    }

    /// The generated Codex config document (format-agnostic — the on-disk `config.toml`/`hooks.json`
    /// split + write under `codex_home` is the HITL live-spawn step): the sandbox containment
    /// (`sandbox_mode` + `[sandbox_workspace_write]`) AND the `[hooks] PreToolUse` adjudication wiring +
    /// `[hooks.state] trusted_hash` — the two INV-SEC-1 layers in one document.
    pub fn config_document(&self) -> serde_json::Value {
        serde_json::json!({
            "sandbox_mode": self.sandbox_mode(),
            "sandbox_workspace_write": {
                "writable_roots": self.writable_roots,
                "network_access": self.network_access(),
            },
            "hooks": {
                "PreToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": self.hook_command,
                        "timeout": CODEX_HOOK_TIMEOUT_SECS,
                    }],
                }],
            },
            "hooks.state": { "trusted_hash": { "hooks.json:pre_tool_use:0:0": self.trusted_hash } },
        })
    }
}

/// A STABLE full SHA-256 (64 lowercase hex chars) over the registered hook command — a PLACEHOLDER for
/// the `[hooks.state]` tamper check (research §4.1 [CONFIRMED-LOCAL] notes the trust-hash table exists,
/// but NOT the exact algorithm Codex computes). The mechanism this slice builds is "register a stable
/// hash + keep it stable so Codex doesn't re-prompt"; if the LIVE format differs (full vs truncated, a
/// different preimage), it is corrected at the HITL live-spawn validation — the no-spawn binding
/// condition. Full digest (not the idempotency-key 128-bit truncation) so it reads as a plain SHA-256
/// and maximizes the chance of a direct match against a real sha256 tamper-check.
fn trusted_hash_of(hook_command: &str) -> String {
    let digest = Sha256::digest(hook_command.as_bytes());
    let mut hash = String::with_capacity(64);
    for byte in digest.iter() {
        let _ = write!(hash, "{byte:02x}");
    }
    hash
}
