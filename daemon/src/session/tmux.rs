//! P4.1b-2 (brief 060) — the tmux detachable-terminal broker subsystem (the §8.1 B2-strict survival
//! realization). USER-ruled mechanism A (tmux, over abduco/dtach + the custom zero-dep holder C).
//!
//! Drops into the FROZEN `Broker`/`SessionLauncher` seams (4.1b-1 / 4.0a) — **daemon-internal, NO
//! `shared/` contract change.** New sessions launch INTO a detached tmux session (so the agent can
//! outlive the daemon); on restart the [`TmuxBroker`] detects survivors via `tmux list-sessions` so
//! `decide_resume` can reach `ReattachedLive`. **tmux is an OPTIONAL upgrade** — absent →
//! [`tmux_probe`] is false → [`select_survival_backend`] degrades to the non-surviving `PtyLauncher` +
//! `NoSurvivorBroker` (= B2-achievable: resume/replay, NEVER `ReattachedLive`).
//!
//! The deterministic surface (command construction · probe · `list-sessions` parsing · backend
//! selection · the [`TmuxLauncher`] command + fail-closed/env-hygiene) is test-first. The LIVE survival
//! (the agent actually outliving the daemon + the lossy alt-screen VT-reattach) is the labelled
//! 0.1/0.3-HITL verify-only follow-on, NOT in this slice's deterministic tests.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use nexusops_shared::ids::SessionId;

use crate::harness::claude::telemetry::TelemetrySinkFactory;
use crate::harness::claude::{ClaudeAdapter, ClaudeLaunchSpec};
use crate::session::broker::{Broker, BrokerReattach, NoSurvivorBroker};
use crate::session::launcher::{terminal_id_for, LaunchedSession, NullTerminalSink, COLS, ROWS};
use crate::session::{PtyLauncher, SessionLauncher};
use crate::terminal::{EnvMutation, PtySpawner, TerminalSession};

/// Our tmux session-name prefix — `nexusops-<sess_ULID>`. tmux-safe (a ULID is alphanumeric, no
/// `:`/`.` which tmux treats specially); distinguishes OUR detached sessions from any foreign ones.
const TMUX_SESSION_PREFIX: &str = "nexusops-";

/// The tmux session name for a daemon session — `nexusops-<sess_…>` (the reattach key, §8.1).
pub fn tmux_session_name(session_id: &SessionId) -> String {
    format!("{TMUX_SESSION_PREFIX}{}", session_id.as_str())
}

/// Parse a tmux session name back to its [`SessionId`] — `None` for a foreign (non-`nexusops-`) name or
/// a malformed id after the prefix (fail-closed parse; a foreign tmux session is never mistaken for ours).
pub fn parse_tmux_session_name(name: &str) -> Option<SessionId> {
    name.strip_prefix(TMUX_SESSION_PREFIX)
        .and_then(|rest| SessionId::parse(rest).ok())
}

// ---- command builders (pure argv; no I/O) -------------------------------------------------------

/// `list-sessions -F '#{session_name}'` — structured output so the survivor set parses deterministically
/// (never scraped free-form).
pub fn list_sessions_argv() -> Vec<String> {
    vec![
        "list-sessions".to_string(),
        "-F".to_string(),
        "#{session_name}".to_string(),
    ]
}

/// `attach -t NAME` — the daemon's DISPLAY PTY attaches to the detached session (the display child).
pub fn attach_argv(name: &str) -> Vec<String> {
    vec!["attach".to_string(), "-t".to_string(), name.to_string()]
}

/// `kill-session -t NAME` — terminate a detached session (orphan-reaping wires with the HITL drive).
pub fn kill_session_argv(name: &str) -> Vec<String> {
    vec![
        "kill-session".to_string(),
        "-t".to_string(),
        name.to_string(),
    ]
}

// ---- the CommandRunner seam (injectable, the PtySpawner/Clock/UsageSource precedent) -------------

/// The captured result of a tmux CLI invocation — a trimmed [`std::process::Output`] (exit success +
/// stdout; stderr is not load-bearing for the probe / list parse).
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
}

/// Runs a one-shot CLI command (tmux). `Send + Sync` — the production [`SystemCommandRunner`] is a ZST;
/// the broker (recovery thread) + the launcher (write-actor thread) each own one. The injectable seam
/// makes the probe / list / `reattach_outcome` deterministic over a [`FakeCommandRunner`].
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[String]) -> io::Result<CommandOutput>;
}

/// The production [`CommandRunner`] — `std::process::Command` (a real one-shot tmux invocation).
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &str, args: &[String]) -> io::Result<CommandOutput> {
        let out = std::process::Command::new(program).args(args).output()?;
        Ok(CommandOutput {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        })
    }
}

/// A deterministic [`CommandRunner`] for tests — canned outputs keyed by the FIRST arg (the tmux
/// subcommand/flag, e.g. `-V` / `list-sessions`); `with_missing` makes every run an ENOENT (the binary
/// absent → the graceful-degrade probe). `test-support`-gated (the `FakeBroker`/`FakeLauncher` precedent).
/// The recorded argv of every `FakeCommandRunner::run` — `Arc<Mutex<…>>` so a `.clone()` handed to a
/// launcher still reflects its calls (the `RecordingSpawner` pattern).
#[cfg(any(test, feature = "test-support"))]
type RecordedCalls = std::sync::Arc<std::sync::Mutex<Vec<Vec<String>>>>;

#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Default)]
pub struct FakeCommandRunner {
    responses: std::collections::HashMap<String, (bool, String)>,
    missing: bool,
    /// lets the launcher test assert the new-session argv (see [`RecordedCalls`]).
    calls: RecordedCalls,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeCommandRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stub the response for the subcommand keyed by `key` (= the first arg): `success` + `stdout`.
    pub fn with_ok(mut self, key: &str, success: bool, stdout: &str) -> Self {
        self.responses
            .insert(key.to_string(), (success, stdout.to_string()));
        self
    }

    /// The tmux binary is absent → every run returns ENOENT (the probe degrades, never panics).
    pub fn with_missing(mut self) -> Self {
        self.missing = true;
        self
    }

    /// The recorded argv of every `run()` (for asserting a launcher's command construction).
    pub fn calls(&self) -> Vec<Vec<String>> {
        self.calls.lock().unwrap().clone()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl CommandRunner for FakeCommandRunner {
    fn run(&self, _program: &str, args: &[String]) -> io::Result<CommandOutput> {
        self.calls.lock().unwrap().push(args.to_vec());
        if self.missing {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        let key = args.first().map(String::as_str).unwrap_or("");
        match self.responses.get(key) {
            Some((success, stdout)) => Ok(CommandOutput {
                success: *success,
                stdout: stdout.clone(),
            }),
            // an unstubbed subcommand → benign empty success (tests stub exactly what they assert).
            None => Ok(CommandOutput {
                success: true,
                stdout: String::new(),
            }),
        }
    }
}

// ---- the availability probe (drives graceful-degrade, pin #3) ------------------------------------

/// `tmux -V` exit-0 → tmux available; a non-zero exit OR ENOENT (binary absent) → unavailable. NEVER
/// panics on a missing binary — the result drives the backend selection (graceful-degrade).
pub fn tmux_probe(runner: &dyn CommandRunner) -> bool {
    matches!(runner.run("tmux", &["-V".to_string()]), Ok(out) if out.success)
}

// ---- the survivor-set parse ----------------------------------------------------------------------

/// Parse `list-sessions -F '#{session_name}'` stdout → the set of OUR live `nexusops-*` survivors;
/// foreign sessions are filtered; empty / "no server running" stdout → an empty set (never an error).
pub fn parse_live_sessions(stdout: &str) -> HashSet<SessionId> {
    stdout.lines().filter_map(parse_tmux_session_name).collect()
}

// ---- the TmuxBroker (the §8.1 ladder's reattach INPUT) -------------------------------------------

/// The detachable-terminal [`Broker`] — `reattach_outcome` runs `list-sessions`, parses the survivor
/// set, and reports `has_live_session = set.contains(session_id)`. **Fails SAFE toward "no survivor":**
/// a runner error, a non-zero exit ("no server running"), or empty output → no survivor (a tmux glitch
/// degrades recovery to resume/replay/relaunch, it NEVER errors the recovery path).
pub struct TmuxBroker {
    runner: Box<dyn CommandRunner>,
}

impl TmuxBroker {
    pub fn new(runner: Box<dyn CommandRunner>) -> Self {
        Self { runner }
    }
}

impl Broker for TmuxBroker {
    fn reattach_outcome(&self, session_id: &SessionId) -> BrokerReattach {
        let survivors = match self.runner.run("tmux", &list_sessions_argv()) {
            Ok(out) if out.success => parse_live_sessions(&out.stdout),
            // any failure / non-zero exit → fail-safe to no survivor (never errors recovery).
            _ => HashSet::new(),
        };
        BrokerReattach {
            has_live_session: survivors.contains(session_id),
        }
    }
}

// =================================================================================================
// L3 (INVARIANT-touching) — the new-session command (env-strip wrapper) + the TmuxLauncher + selection
// =================================================================================================

/// Map a spec's [`EnvMutation`]s to `env`-command args — the §15 #8 strip+set applied at EXEC time
/// inside tmux: `Remove(k)` → `["-u", k]`, `Set(k,v)` → `["k=v"]`. **Q1=(b), lead/user-ruled:** tmux's
/// `-e` can only SET (never UNSET), and a pre-existing tmux server injects its own env — so `-e` cannot
/// structurally strip `ANTHROPIC_API_KEY`. Wrapping the agent in `env` strips/sets at exec time
/// REGARDLESS of tmux's server env (`env` is universal). DERIVED from `env_mutations()` (one source of
/// truth with the direct `PtyLauncher` path — anti-drift if the mutation set ever changes).
pub fn env_wrapper_args(env: &[EnvMutation]) -> Vec<String> {
    let mut out = Vec::new();
    for m in env {
        match &m.value {
            None => {
                out.push("-u".to_string());
                out.push(m.key.clone());
            }
            Some(v) => out.push(format!("{}={}", m.key, v)),
        }
    }
    out
}

/// `tmux new-session -d -s NAME -c CWD -- env <env-args> PROGRAM ARGS…` — launch the agent DETACHED
/// (`-d`, survivable) wrapped in `env` (the §15 #8 strip+set). `--` ends tmux's option parsing so the
/// spec's flags aren't mis-read as tmux's. The agent is the UNCHANGED [`ClaudeLaunchSpec`] (O-13
/// default-mode / no-`-p` / no-bg by construction), just inside tmux behind the `env` wrapper.
pub fn new_session_argv(
    name: &str,
    cwd: &Path,
    env: &[EnvMutation],
    program: &str,
    args: &[String],
) -> Vec<String> {
    let mut argv = vec![
        "new-session".to_string(),
        "-d".to_string(),
        "-s".to_string(),
        name.to_string(),
        "-c".to_string(),
        cwd.to_string_lossy().into_owned(),
        "--".to_string(),
        "env".to_string(),
    ];
    argv.extend(env_wrapper_args(env));
    argv.push(program.to_string());
    argv.extend(args.iter().cloned());
    argv
}

/// A [`SessionLauncher`] that launches the live `claude` PTY INSIDE a detached tmux session (the §8.1
/// survivable holder). It builds the SAME [`ClaudeLaunchSpec`] as [`PtyLauncher`] (the O-13 #10 surface
/// preserved — default mode, no `-p`, the generated 0600 settings, fail-closed settings write), wraps it
/// in `tmux new-session -d … -- env <strip/set> <spec>`, then opens the daemon's DISPLAY PTY via
/// `tmux attach`. The agent survives a daemon exit; the display PTY is re-openable on restart. **The
/// LIVE survival (the agent outliving the daemon + the lossy VT-reattach) is HITL — this constructs the
/// COMMAND (deterministically pinned) + fail-closes identically to `PtyLauncher`.**
pub struct TmuxLauncher {
    runner: Box<dyn CommandRunner>,
    spawner: Box<dyn PtySpawner>,
    cwd: PathBuf,
    hook_receiver: String,
    telemetry_sink_factory: Option<Box<dyn TelemetrySinkFactory>>,
}

impl TmuxLauncher {
    pub fn new(
        runner: Box<dyn CommandRunner>,
        spawner: Box<dyn PtySpawner>,
        cwd: PathBuf,
        hook_receiver: impl Into<String>,
    ) -> Self {
        Self {
            runner,
            spawner,
            cwd,
            hook_receiver: hook_receiver.into(),
            telemetry_sink_factory: None,
        }
    }

    /// Inject the production telemetry-sink factory (the `PtyLauncher::with_telemetry_sink_factory`
    /// precedent) — each launched session emits its `TelemetrySampled` observations via the write-actor.
    pub fn with_telemetry_sink_factory(mut self, factory: Box<dyn TelemetrySinkFactory>) -> Self {
        self.telemetry_sink_factory = Some(factory);
        self
    }
}

impl SessionLauncher for TmuxLauncher {
    fn launch_session(&self) -> io::Result<LaunchedSession> {
        let session_id = SessionId::new();
        let name = tmux_session_name(&session_id);
        // the SAME O-13 spec as PtyLauncher — write the 0600 settings FAIL-CLOSED (a write error → no
        // session, never a hook-less live agent, INV-SEC-1; NOT soft-degrade).
        let spec = ClaudeLaunchSpec::build(&self.cwd, session_id.as_str(), &self.hook_receiver);
        spec.write_settings()?;
        // launch the agent DETACHED into tmux, wrapped in `env` (the §15 #8 strip+set at exec time).
        let new_session = new_session_argv(
            &name,
            &self.cwd,
            &spec.env_mutations(),
            spec.program(),
            spec.args(),
        );
        let out = self.runner.run("tmux", &new_session)?;
        if !out.success {
            // fail-closed: a failed `new-session` → no session (never a half-launched agent).
            return Err(io::Error::other("tmux new-session failed"));
        }
        // open the daemon's DISPLAY PTY by attaching to the detached session (env-irrelevant — the
        // agent's env was set at new-session time; this child is just the display).
        let pty = self
            .spawner
            .spawn("tmux", &attach_argv(&name), &self.cwd, ROWS, COLS, &[])?;
        let terminal = TerminalSession::new(
            terminal_id_for(&session_id),
            pty,
            Box::new(NullTerminalSink),
        );
        // the LIVE ClaudeAdapter — it does NOT spawn (tmux did); status derives from hook signals (#9).
        let mut adapter = ClaudeAdapter::new(self.cwd.clone(), session_id.as_str().to_string());
        if let Some(factory) = &self.telemetry_sink_factory {
            adapter = adapter.with_telemetry_sink(factory.make_sink(&session_id, None));
        }
        Ok(LaunchedSession {
            session_id,
            adapter: Box::new(adapter),
            terminal,
        })
    }
}

/// Which survival mechanism the daemon selected at startup (the consistency marker — set in the SAME
/// arm as both seams, so a `Tmux` kind always pairs a `TmuxLauncher` with a `TmuxBroker`, `Pty` always
/// pairs `PtyLauncher` with `NoSurvivorBroker`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurvivalKind {
    /// tmux available → survivable sessions + survivor detection (`ReattachedLive` reachable).
    Tmux,
    /// tmux absent → graceful-degrade to the non-surviving launcher (B2-achievable, never reattach).
    Pty,
}

/// A consistent launcher+broker pair (pin #3/#7 — structural). Built by [`select_survival_backend`];
/// the `launcher` is consumed into the `SessionExecutor`, the `broker` into `run_restart_recovery`.
pub struct SurvivalBackend {
    pub kind: SurvivalKind,
    pub launcher: Box<dyn SessionLauncher>,
    pub broker: Box<dyn Broker>,
}

/// Select the survival backend from the [`tmux_probe`] result — a CONSISTENT pair (you can never get a
/// `TmuxLauncher` with a `NoSurvivorBroker`, or vice-versa: both seams + the `kind` come from ONE arm).
/// tmux → `TmuxLauncher` + `TmuxBroker` (`ReattachedLive` reachable); else → `PtyLauncher` +
/// `NoSurvivorBroker` (graceful-degrade, B2-achievable). The `spawner`/`cwd`/`hook_receiver`/telemetry
/// thread into whichever launcher is selected.
pub fn select_survival_backend(
    tmux_available: bool,
    spawner: Box<dyn PtySpawner>,
    cwd: PathBuf,
    hook_receiver: String,
    telemetry: Option<Box<dyn TelemetrySinkFactory>>,
) -> SurvivalBackend {
    if tmux_available {
        let mut launcher =
            TmuxLauncher::new(Box::new(SystemCommandRunner), spawner, cwd, hook_receiver);
        if let Some(f) = telemetry {
            launcher = launcher.with_telemetry_sink_factory(f);
        }
        SurvivalBackend {
            kind: SurvivalKind::Tmux,
            launcher: Box::new(launcher),
            broker: Box::new(TmuxBroker::new(Box::new(SystemCommandRunner))),
        }
    } else {
        let mut launcher = PtyLauncher::new(spawner, cwd, hook_receiver);
        if let Some(f) = telemetry {
            launcher = launcher.with_telemetry_sink_factory(f);
        }
        SurvivalBackend {
            kind: SurvivalKind::Pty,
            launcher: Box::new(launcher),
            broker: Box::new(NoSurvivorBroker),
        }
    }
}
