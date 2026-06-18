//! P4.1b-2 (brief 060) — the tmux detachable-terminal broker: the deterministic surface (command
//! construction · availability-probe · `list-sessions` parsing · `reattach_outcome` · backend-selection
//! · the `TmuxLauncher` command + fail-closed/env-hygiene). LIVE survival (the agent outliving the
//! daemon + the lossy VT-reattach) is the labelled 0.1/0.3-HITL follow-on, NOT here.
//!
//! L1+L2 (this file's first block) = non-safety primitives + the `TmuxBroker`. L3 (the launcher +
//! selection) = the INVARIANT-touching block (its own commit + security-reviewer).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nexusops_shared::harness::ResumeMode;
use nexusops_shared::ids::SessionId;

use nexusopsd::harness::resume::{decide_resume, ResumeInputs};
use nexusopsd::session::broker::Broker;
use nexusopsd::session::tmux::{
    attach_argv, env_wrapper_args, kill_session_argv, list_sessions_argv, new_session_argv,
    parse_live_sessions, parse_tmux_session_name, select_survival_backend, tmux_probe,
    tmux_session_name, FakeCommandRunner, SurvivalKind, TmuxBroker, TmuxLauncher,
};
use nexusopsd::session::SessionLauncher;
use nexusopsd::terminal::{EnvMutation, ExitStatus, FakePty, Pty, PtyRead, PtySpawner};

// =================================================================================================
// L1+L2 — non-safety primitives + the TmuxBroker (deterministic over the FakeCommandRunner)
// =================================================================================================

// ---- RED #1 — the session-name mapping round-trips; foreign names are ignored ---------------------

#[test]
fn test_session_name_roundtrips() {
    // spec(§8.1) — reattach keys survivors to the ORIGINAL SessionId (identity-preserving recovery,
    // Q1=(a)). `nexusops-<sess_ULID>` is tmux-safe (ULID is alphanumeric — no `:`/`.`). A non-`nexusops-`
    // name parses to None (a foreign tmux session is never mistaken for one of ours).
    let id = SessionId::new();
    let name = tmux_session_name(&id);
    assert!(name.starts_with("nexusops-"));
    assert_eq!(parse_tmux_session_name(&name), Some(id));
    assert_eq!(
        parse_tmux_session_name("scratch"),
        None,
        "foreign name ignored"
    );
    assert_eq!(
        parse_tmux_session_name("nexusops-not-a-ulid"),
        None,
        "a malformed id after the prefix is rejected (fail-closed parse)"
    );
}

// ---- RED #3 — exact argv for list-sessions / attach / kill-session --------------------------------

#[test]
fn test_list_sessions_argv() {
    // spec(§8.1) — structured output (`-F '#{session_name}'`) so the survivor set parses deterministically
    // (never scraped free-form). `-F` with the session_name format only.
    assert_eq!(
        list_sessions_argv(),
        vec!["list-sessions", "-F", "#{session_name}"]
    );
}

#[test]
fn test_attach_argv() {
    let id = SessionId::new();
    let name = tmux_session_name(&id);
    assert_eq!(attach_argv(&name), vec!["attach", "-t", name.as_str()]);
}

#[test]
fn test_kill_session_argv() {
    let id = SessionId::new();
    let name = tmux_session_name(&id);
    assert_eq!(
        kill_session_argv(&name),
        vec!["kill-session", "-t", name.as_str()]
    );
}

// ---- RED #4 — the probe drives graceful-degrade; a missing binary never panics --------------------

#[test]
fn test_probe_present_absent() {
    // spec(pin #3) — `tmux -V` exit-0 → available; a non-zero exit OR ENOENT (binary absent) → unavailable.
    // Deterministic via the fake runner; a missing binary must NEVER panic (graceful-degrade).
    let present = FakeCommandRunner::new().with_ok("-V", true, "tmux 3.4\n");
    assert!(tmux_probe(&present), "tmux -V exit-0 → available");

    let nonzero = FakeCommandRunner::new().with_ok("-V", false, "");
    assert!(!tmux_probe(&nonzero), "non-zero exit → unavailable");

    let missing = FakeCommandRunner::new().with_missing();
    assert!(
        !tmux_probe(&missing),
        "ENOENT (binary absent) → unavailable, no panic"
    );
}

// ---- RED #5 — list-sessions output → our survivor set; foreign sessions filtered ------------------

#[test]
fn test_parse_live_sessions() {
    // spec(§8.1) — only OUR detached `nexusops-*` sessions are survivors; foreign tmux sessions are
    // filtered; an empty / "no server running" stdout → an empty set (never an error).
    let a = SessionId::new();
    let b = SessionId::new();
    let stdout = format!(
        "{}\nwork\n{}\n0\n",
        tmux_session_name(&a),
        tmux_session_name(&b)
    );
    let set = parse_live_sessions(&stdout);
    assert_eq!(set.len(), 2, "exactly our two sessions");
    assert!(set.contains(&a) && set.contains(&b));

    assert!(
        parse_live_sessions("").is_empty(),
        "empty stdout → empty set"
    );
    assert!(
        parse_live_sessions("no server running on /tmp/tmux-501/default\n").is_empty(),
        "a foreign/no-server line is not a nexusops session"
    );
}

// ---- RED #6 — TmuxBroker.reattach_outcome via list-sessions; fail-safe to "no survivor" -----------

#[test]
fn test_tmux_broker_reattach_outcome() {
    // spec(§8.1 ladder input) — a seeded survivor → has_live_session=true; an absent session → false; a
    // runner error / non-zero exit ("no server") → false (the broker NEVER errors the recovery path —
    // it fails SAFE toward "no survivor", so a tmux glitch degrades to replay/relaunch, never a crash).
    let live = SessionId::new();
    let absent = SessionId::new();
    let runner = FakeCommandRunner::new().with_ok(
        "list-sessions",
        true,
        &format!("{}\n", tmux_session_name(&live)),
    );
    let broker = TmuxBroker::new(Box::new(runner));
    assert!(
        broker.reattach_outcome(&live).has_live_session,
        "seeded survivor"
    );
    assert!(
        !broker.reattach_outcome(&absent).has_live_session,
        "a session tmux does not hold → no survivor"
    );

    // a runner error (tmux vanished mid-run) → no survivor, never an error out of the broker.
    let broken = TmuxBroker::new(Box::new(FakeCommandRunner::new().with_missing()));
    assert!(
        !broken.reattach_outcome(&live).has_live_session,
        "runner error → fail-safe to no survivor (recovery never crashes)"
    );

    // "no server running" (non-zero exit) → no survivor.
    let no_server = TmuxBroker::new(Box::new(FakeCommandRunner::new().with_ok(
        "list-sessions",
        false,
        "no server running",
    )));
    assert!(
        !no_server.reattach_outcome(&live).has_live_session,
        "no server → no survivor"
    );
}

// =================================================================================================
// L3 — the TmuxLauncher (INVARIANT-touching) + backend selection (Q1=(b) env-wrapper, lead-ruled)
// =================================================================================================

/// recorded `(program, args)` of each display spawn.
type SpawnCalls = Arc<Mutex<Vec<(String, Vec<String>)>>>;

/// a recording `PtySpawner` double (the session_live.rs precedent) — records the display `tmux attach`
/// spawn + returns a scripted FakePty (no real process).
#[derive(Clone, Default)]
struct RecordingSpawner {
    calls: SpawnCalls,
}

impl PtySpawner for RecordingSpawner {
    fn spawn(
        &self,
        program: &str,
        args: &[String],
        _cwd: &Path,
        _rows: u16,
        _cols: u16,
        _env: &[EnvMutation],
    ) -> std::io::Result<Box<dyn Pty>> {
        self.calls
            .lock()
            .unwrap()
            .push((program.to_string(), args.to_vec()));
        Ok(Box::new(FakePty::new(
            vec![PtyRead::Eof],
            ExitStatus {
                exit_code: Some(0),
                signal: None,
            },
        )))
    }
}

/// a spawner that always fails — used to prove the display spawn is NEVER reached on a failed new-session.
struct NeverSpawner;

impl PtySpawner for NeverSpawner {
    fn spawn(
        &self,
        _program: &str,
        _args: &[String],
        _cwd: &Path,
        _rows: u16,
        _cols: u16,
        _env: &[EnvMutation],
    ) -> std::io::Result<Box<dyn Pty>> {
        panic!("the display spawn must NOT be reached when tmux new-session fails (fail-closed)");
    }
}

// ---- RED #2 — exact new-session argv: detached, env-wrapped, `--` before the spec ----------------

#[test]
fn test_new_session_argv() {
    // spec(§9.1) — `new-session -d -s NAME -c CWD -- env <env-args> PROGRAM ARGS`: detached (`-d`,
    // survivable); `--` ends tmux options so the spec's flags aren't mis-parsed; the agent is wrapped in
    // `env` (the §15 #8 strip+set at exec time, Q1=(b)).
    let env = vec![
        EnvMutation::remove("ANTHROPIC_API_KEY"),
        EnvMutation::set("NEXUSOPS_SESSION_ID", "sess_x"),
    ];
    let argv = new_session_argv(
        "nexusops-sess_x",
        Path::new("/tmp/proj"),
        &env,
        "claude",
        &["--permission-mode".to_string(), "default".to_string()],
    );
    assert_eq!(
        argv,
        vec![
            "new-session",
            "-d",
            "-s",
            "nexusops-sess_x",
            "-c",
            "/tmp/proj",
            "--",
            "env",
            "-u",
            "ANTHROPIC_API_KEY",
            "NEXUSOPS_SESSION_ID=sess_x",
            "claude",
            "--permission-mode",
            "default",
        ]
    );
}

// ---- RED #7 — backend selection is a CONSISTENT pair (never mixed) --------------------------------

#[test]
fn test_select_backend_consistency() {
    // spec(pin #3/#7) — tmux available → kind Tmux; absent → kind Pty (degrade). The kind is set in the
    // SAME arm as both seams, so a mixed pair (e.g. a TmuxLauncher with a NoSurvivorBroker) is
    // structurally impossible. The degrade broker provably reports NO survivor (a non-surviving launcher
    // never creates one → no false ReattachedLive).
    let tmux = select_survival_backend(
        true,
        Box::new(RecordingSpawner::default()),
        PathBuf::from("/tmp/p"),
        "hook".to_string(),
        None,
    );
    assert_eq!(
        tmux.kind,
        SurvivalKind::Tmux,
        "tmux available → Tmux backend"
    );

    let pty = select_survival_backend(
        false,
        Box::new(RecordingSpawner::default()),
        PathBuf::from("/tmp/p"),
        "hook".to_string(),
        None,
    );
    assert_eq!(
        pty.kind,
        SurvivalKind::Pty,
        "tmux absent → Pty backend (graceful degrade)"
    );
    let any = SessionId::new();
    assert!(
        !pty.broker.reattach_outcome(&any).has_live_session,
        "the degrade broker reports no survivor (NoSurvivorBroker) — never a false ReattachedLive"
    );
}

// ---- RED #8 — the launcher wraps the UNCHANGED spec in new-session + attaches the display ----------

#[test]
fn test_tmux_launcher_builds_wrapped_spec() {
    // spec(§9.1 / O-13 #10) — the launcher builds `tmux new-session … -- env … claude --permission-mode
    // default --settings …` (the UNCHANGED O-13 spec, just inside tmux behind the env wrapper) + opens
    // the display via `tmux attach -t NAME`.
    let runner = FakeCommandRunner::new().with_ok("new-session", true, "");
    let spawner = RecordingSpawner::default();
    let launcher = TmuxLauncher::new(
        Box::new(runner.clone()),
        Box::new(spawner.clone()),
        PathBuf::from("/tmp/proj"),
        "/usr/local/bin/nexusops-hook",
    );
    launcher.launch_session().expect("launch");

    let calls = runner.calls();
    let new_session = calls
        .iter()
        .find(|a| a.first().is_some_and(|s| s == "new-session"))
        .expect("a new-session call");
    assert!(
        new_session.contains(&"-d".to_string()),
        "detached: {new_session:?}"
    );
    assert!(
        new_session.contains(&"env".to_string()),
        "env wrapper: {new_session:?}"
    );
    assert!(
        new_session.contains(&"claude".to_string()),
        "the O-13 program: {new_session:?}"
    );
    let dd = new_session
        .iter()
        .position(|s| s == "--")
        .expect("`--` separator");
    let prog = new_session.iter().position(|s| s == "claude").unwrap();
    assert!(
        dd < prog,
        "`--` precedes the spec so its flags aren't mis-parsed"
    );
    assert!(
        new_session
            .windows(2)
            .any(|w| w[0] == "--permission-mode" && w[1] == "default"),
        "the UNCHANGED O-13 default-mode spec: {new_session:?}"
    );

    let spawned = spawner.calls.lock().unwrap();
    assert_eq!(spawned.len(), 1, "exactly one display spawn");
    assert_eq!(spawned[0].0, "tmux");
    assert_eq!(
        spawned[0].1[0], "attach",
        "the display is `tmux attach -t NAME`"
    );
}

// ---- RED #9 — fail-closed: a failed new-session → no session, no display spawn --------------------

#[test]
fn test_tmux_launcher_fail_closed_new_session() {
    // spec(INV-SEC-1 / LESSON 25/30) — a failed `tmux new-session` → `launch_session` returns Err and the
    // display spawn is NEVER reached (no orphaned attach to a non-existent session; never a half-launched
    // agent). [The settings-write fail-closed (`write_settings()?` BEFORE new-session) is structurally
    // shared with PtyLauncher — the same `?`-ordering; a settings-write-failure injection isn't feasible
    // without a settings-path override, out of scope, the session_live.rs PtyLauncher precedent.]
    let runner = FakeCommandRunner::new().with_ok("new-session", false, "");
    let launcher = TmuxLauncher::new(
        Box::new(runner),
        Box::new(NeverSpawner), // panics if the display spawn is reached
        PathBuf::from("/tmp/proj"),
        "/usr/local/bin/nexusops-hook",
    );
    assert!(
        launcher.launch_session().is_err(),
        "a failed new-session → no session (fail-closed)"
    );
}

// ---- RED #10 — env-hygiene through tmux: the §15 #8 strip+set, DERIVED from the spec --------------

#[test]
fn test_tmux_launcher_env_hygiene() {
    // spec(§15 #8, Q1=(b)) — the env wrapper is DERIVED generically from `EnvMutation` (one source of
    // truth with PtyLauncher): Remove→`-u k`, Set→`k=v`. So ANTHROPIC_API_KEY is stripped (`-u`) and
    // NEXUSOPS_SESSION_ID carried — NEVER a bare `ANTHROPIC_API_KEY=…` leak — regardless of tmux's
    // server env.
    // (a) the derivation itself (not two hardcoded literals):
    let derived = env_wrapper_args(&[EnvMutation::remove("FOO"), EnvMutation::set("BAR", "baz")]);
    assert_eq!(
        derived,
        vec!["-u", "FOO", "BAR=baz"],
        "derived from the mutations"
    );

    // (b) the live launcher's new-session argv applies the spec's §15 #8 mutations:
    let runner = FakeCommandRunner::new().with_ok("new-session", true, "");
    let launcher = TmuxLauncher::new(
        Box::new(runner.clone()),
        Box::new(RecordingSpawner::default()),
        PathBuf::from("/tmp/proj"),
        "/usr/local/bin/nexusops-hook",
    );
    launcher.launch_session().expect("launch");
    let calls = runner.calls();
    let ns = calls
        .iter()
        .find(|a| a.first().is_some_and(|s| s == "new-session"))
        .unwrap();
    assert!(
        ns.windows(2)
            .any(|w| w[0] == "-u" && w[1] == "ANTHROPIC_API_KEY"),
        "ANTHROPIC_API_KEY is stripped via `env -u` (§15 #8): {ns:?}"
    );
    assert!(
        ns.iter().any(|s| s.starts_with("NEXUSOPS_SESSION_ID=")),
        "NEXUSOPS_SESSION_ID is carried (hook correlation): {ns:?}"
    );
    assert!(
        !ns.iter().any(|s| s.starts_with("ANTHROPIC_API_KEY=")),
        "no bare ANTHROPIC_API_KEY= set (no leak): {ns:?}"
    );
}

// ---- RED #11 — graceful degrade: no tmux → the broker never yields ReattachedLive ----------------

#[test]
fn test_degrade_no_tmux_never_reattaches() {
    // spec(pin #3) — the fallback (no-tmux) backend's broker reports no survivor for every session, so
    // even with every other input available, decide_resume NEVER yields ReattachedLive (degrade =
    // B2-achievable: resume/replay/relaunch, never the top rung).
    let backend = select_survival_backend(
        false,
        Box::new(RecordingSpawner::default()),
        PathBuf::from("/tmp/p"),
        "hook".to_string(),
        None,
    );
    let any = SessionId::new();
    let inputs = ResumeInputs {
        broker_has_live_session: backend.broker.reattach_outcome(&any).has_live_session,
        supports_resume: true,
        has_resume_handle: true,
        has_replayable_snapshot: true,
        replayed_event_count: 5,
    };
    assert!(
        !inputs.broker_has_live_session,
        "degrade broker → no survivor"
    );
    assert_ne!(
        decide_resume(&inputs).mode,
        ResumeMode::ReattachedLive,
        "degrade can never reach the top rung"
    );
}
