//! P4.0b-2 (Option A, lead-ruled) — the **launcher-owns-PTY** safe-floor pins: the live-claude spawn
//! site + the O-13 #10 enforcement surface live at `PtyLauncher` (NOT the adapter). These pin
//! hard-condition 1 ("only `PtyLauncher` spawns a live claude PTY; `ClaudeAdapter::launch` no longer
//! spawns", with the #10 spec verified at its new home) for the dedicated L2 security pass.
//!
//! **Safe floor:** these test the launcher in ISOLATION over a fake spawner — there is NO reachable
//! production caller (no IPC `session.create`, no `register(Session, …)`); the live co-land (the
//! interception wiring) is the fresh impl's L2. (The migrated home of the former
//! `claude_adapter.rs::test_launch_spawns_*` tests, now that the adapter no longer spawns.)

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nexusops_shared::status::Session;
use nexusopsd::session::{PtyLauncher, SessionLauncher};
use nexusopsd::terminal::{ExitStatus, FakePty, Pty, PtyRead, PtySpawner};

/// a recording `PtySpawner` test double — records the spawn call (program/args) + returns a scripted
/// `FakePty` (no real process). The launcher's ONE spawn is the only thing that should ever call it.
#[derive(Clone)]
struct SpawnCall {
    program: String,
    args: Vec<String>,
}

#[derive(Clone, Default)]
struct RecordingSpawner {
    calls: Arc<Mutex<Vec<SpawnCall>>>,
}

impl PtySpawner for RecordingSpawner {
    fn spawn(
        &self,
        program: &str,
        args: &[String],
        _cwd: &Path,
        _rows: u16,
        _cols: u16,
    ) -> std::io::Result<Box<dyn Pty>> {
        self.calls.lock().unwrap().push(SpawnCall {
            program: program.to_string(),
            args: args.to_vec(),
        });
        Ok(Box::new(FakePty::new(
            vec![PtyRead::Eof],
            ExitStatus {
                exit_code: Some(0),
                signal: None,
            },
        )))
    }
}

/// a spawner that always fails — pins the launcher's fail-closed (no-phantom-session) error path.
struct FailingSpawner;

impl PtySpawner for FailingSpawner {
    fn spawn(
        &self,
        _program: &str,
        _args: &[String],
        _cwd: &Path,
        _rows: u16,
        _cols: u16,
    ) -> std::io::Result<Box<dyn Pty>> {
        Err(std::io::Error::other("spawn refused (test)"))
    }
}

// ---- condition-1 — the launcher is the SINGLE live-claude spawn site (#10 at its new home) --------

#[test]
fn test_only_launcher_spawns_live_claude_pty() {
    // spec(§9.1 / O-13 #10 / P4.0b-2 Option A) — the LAUNCHER (NOT the adapter) spawns the ONE live
    // claude PTY, with the O-13 #10 argv (default mode · no `-p` · no `--dangerously-*`). The
    // `ClaudeAdapter` holds no spawner → it structurally CANNOT spawn; `launch()` is the daemon-
    // lifecycle `Creating→Starting` marker only (status then derives from hook signals, safety #9).
    let spawner = RecordingSpawner::default();
    let launcher = PtyLauncher::new(
        Box::new(spawner.clone()),
        PathBuf::from("/tmp/proj"),
        "/usr/local/bin/nexusops-hook",
    );
    let launched = launcher.launch_session().expect("launch");
    {
        let calls = spawner.calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            1,
            "the LAUNCHER spawns the ONE live claude PTY (single spawn site)"
        );
        assert_eq!(calls[0].program, "claude", "the live program is `claude`");
        assert!(
            calls[0]
                .args
                .windows(2)
                .any(|w| w[0] == "--permission-mode" && w[1] == "default"),
            "O-13 #10 default mode, verified at the launcher's new home: {:?}",
            calls[0].args
        );
        assert!(
            !calls[0]
                .args
                .iter()
                .any(|a| a == "-p" || a.starts_with("--dangerously")),
            "no `-p` / no `--dangerously-*` (the #10 spec, content-preserved at the launcher): {:?}",
            calls[0].args
        );
    }
    // the adapter does NOT spawn — `launch()` is the lifecycle marker; the spawn count STAYS 1.
    let mut adapter = launched.adapter;
    assert_eq!(
        adapter.launch(),
        Session::Starting,
        "ClaudeAdapter::launch is the `Creating→Starting` marker only (no spawn)"
    );
    assert_eq!(
        spawner.calls.lock().unwrap().len(),
        1,
        "the adapter did NOT spawn — still exactly 1 (the launcher is the sole spawn site)"
    );
}

// ---- the launcher fail-closes (no phantom session) on a spawn failure -----------------------------

#[test]
fn test_launcher_spawn_failure_is_fail_closed() {
    // spec(§15 / O-13) — a spawn failure → NO session (the launcher returns `Err`): never a live agent
    // without its `PreToolUse` hook (INV-SEC-1). The #10 settings-write fail-closed shares this contract.
    let launcher = PtyLauncher::new(
        Box::new(FailingSpawner),
        PathBuf::from("/tmp/proj"),
        "/usr/local/bin/nexusops-hook",
    );
    assert!(
        launcher.launch_session().is_err(),
        "a spawn failure → no session (fail-closed, never an un-hooked live agent)"
    );
}
