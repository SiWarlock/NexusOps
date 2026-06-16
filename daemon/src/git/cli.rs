//! The git-CLI seam — git **mutations** run through the git CLI (forbidden #6: NEVER git2 for
//! mutations).
//!
//! `git2` in this module's siblings (`detect`/`reads`/`precedence`) is READ-ONLY introspection; every
//! worktree/branch/commit/merge MUTATION runs the real `git` binary as an audited Gateway action. The
//! [`GitCli`] trait is the injectable seam (the `SessionLauncher`/`PtySpawner` precedent, LESSON
//! 25/28) so the non-deterministic spawn is fakeable; production uses [`SystemGitCli`].

use std::path::Path;

/// The captured result of a git CLI invocation.
pub struct GitCliOutput {
    /// whether the process exited with a success (zero) status.
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// A failure to even SPAWN the git binary — distinct from a non-zero exit (which is a
/// `GitCliOutput { success: false, .. }`). The `Display` is STRUCTURAL (no path/payload content, §15).
#[derive(Debug, thiserror::Error)]
pub enum GitCliError {
    #[error("failed to spawn the git binary")]
    Spawn,
}

/// Runs the git CLI — the seam the worktree/branch mutators drive (forbidden #6). `Send + Sync` for
/// the `ActionExecutor` bound.
pub trait GitCli: Send + Sync {
    /// Run `git <args>` with `cwd` as the working directory. A spawn failure is `Err`; a non-zero exit
    /// is `Ok(GitCliOutput { success: false, .. })` (the caller decides how to treat it).
    fn run(&self, args: &[String], cwd: &Path) -> Result<GitCliOutput, GitCliError>;
}

/// Production — the real `git` binary via `std::process::Command`.
pub struct SystemGitCli;

impl GitCli for SystemGitCli {
    fn run(&self, args: &[String], cwd: &Path) -> Result<GitCliOutput, GitCliError> {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|_| GitCliError::Spawn)?;
        Ok(GitCliOutput {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

/// The recorded `(args, cwd)` invocations of a [`FakeGitCli`], shared with the test that inspects them.
#[cfg(feature = "test-support")]
pub type FakeGitInvocations =
    std::sync::Arc<std::sync::Mutex<Vec<(Vec<String>, std::path::PathBuf)>>>;

/// Test double — records every invocation `(args, cwd)` + returns canned output (or a spawn error).
/// Gated behind the `test-support` feature from creation (the post-merge feature; the daemon dev-dep
/// self-enables it for test/bench targets — LESSON 21).
#[cfg(feature = "test-support")]
pub struct FakeGitCli {
    invocations: FakeGitInvocations,
    mode: FakeMode,
}

#[cfg(feature = "test-support")]
enum FakeMode {
    Succeed,
    Fail,
    SpawnError,
}

#[cfg(feature = "test-support")]
impl FakeGitCli {
    /// records invocations; returns a zero-exit success.
    pub fn succeeding() -> Self {
        Self {
            invocations: Default::default(),
            mode: FakeMode::Succeed,
        }
    }
    /// records invocations; returns a non-zero exit (a CLI failure).
    pub fn failing() -> Self {
        Self {
            invocations: Default::default(),
            mode: FakeMode::Fail,
        }
    }
    /// records invocations; returns a spawn error (git not runnable).
    pub fn spawn_error() -> Self {
        Self {
            invocations: Default::default(),
            mode: FakeMode::SpawnError,
        }
    }
    /// a handle to the recorded `(args, cwd)` invocations — clone it BEFORE the cli is boxed.
    pub fn invocations(&self) -> FakeGitInvocations {
        self.invocations.clone()
    }
}

#[cfg(feature = "test-support")]
impl GitCli for FakeGitCli {
    fn run(&self, args: &[String], cwd: &Path) -> Result<GitCliOutput, GitCliError> {
        // `.unwrap()` (daemon no-bare-unwrap convention): test-only double; the Mutex is uncontended
        // (single-threaded test use) and only poisons if a test already panicked while holding it — no
        // new failure mode. (The `SessionExecutor` Mutex-lock precedent.)
        self.invocations
            .lock()
            .unwrap()
            .push((args.to_vec(), cwd.to_path_buf()));
        match self.mode {
            FakeMode::Succeed => Ok(GitCliOutput {
                success: true,
                stdout: String::new(),
                stderr: String::new(),
            }),
            FakeMode::Fail => Ok(GitCliOutput {
                success: false,
                stdout: String::new(),
                stderr: "fatal: simulated git failure".to_string(),
            }),
            FakeMode::SpawnError => Err(GitCliError::Spawn),
        }
    }
}
