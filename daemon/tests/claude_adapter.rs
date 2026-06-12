//! P3.2 part 1 — the Claude Code `HarnessAdapter` OBSERVE path (launch / status / transcript).
//!
//! PTY-primary (cat-4 resolved): drive via an interactive PTY + `PreToolUse` hooks + statusLine,
//! NEVER `-p`/SDK-drive, default permission mode only, no background subagents (O-13 / safety #10).
//! Status derives from STRUCTURED signals (hook events + transcript + the 3.4 exit event), NEVER
//! from PTY output bytes (safety #9). Interception (INV-SEC-1) + telemetry = brief 043; resume = P4.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nexusops_shared::status::Session;
use nexusopsd::harness::claude::{ClaudeAdapter, ClaudeLaunchSpec, CLAUDE_CAPABILITIES};
use nexusopsd::harness::HarnessAdapter;
use nexusopsd::terminal::{ExitStatus, FakePty, Pty, PtyRead, PtySpawner};

// ---- a recording PtySpawner test double: records the spawn call + returns a scripted FakePty ----

#[derive(Clone)]
struct SpawnCall {
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
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
        cwd: &Path,
        _rows: u16,
        _cols: u16,
    ) -> std::io::Result<Box<dyn Pty>> {
        self.calls.lock().unwrap().push(SpawnCall {
            program: program.to_string(),
            args: args.to_vec(),
            cwd: cwd.to_path_buf(),
        });
        // a minimal live child for the adapter to own (launch only needs a spawned Pty).
        Ok(Box::new(FakePty::new(
            vec![PtyRead::Eof],
            ExitStatus {
                exit_code: Some(0),
                signal: None,
            },
        )))
    }
}

/// a spawner that always fails — pins the launch error path.
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

fn adapter_with(spawner: RecordingSpawner) -> ClaudeAdapter {
    ClaudeAdapter::new(
        Box::new(spawner),
        PathBuf::from("/Users/x/proj"),
        "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
        // the hook-receiver command the generated settings reference — the real daemon receiver
        // endpoint is built in 043; 042 only wires the spec to point at it.
        "/usr/local/bin/nexusops-hook".to_string(),
    )
}

// ---- 3.2 L1 RED #1 — the 10-field Claude HarnessCapabilities const (§9.1/§11.4) ----

#[test]
fn test_claude_capabilities_const() {
    // spec(§9.1) — the per-capability UI degradation source (§11.4). The 7 clear-true + the 3 Q1 votes.
    let caps = &CLAUDE_CAPABILITIES;
    assert!(caps.supports_terminal);
    assert!(caps.supports_resume);
    assert!(caps.supports_transcript_read);
    assert!(caps.supports_tool_call_parsing);
    assert!(caps.supports_usage_metadata);
    assert!(caps.supports_context_metadata);
    assert!(caps.supports_hooks);
    // Q1 votes: the daemon CAN inject stdin into the live PTY → true; Task subagents EXIST → true;
    // cloud tasks are not in the local MVP → false.
    assert!(caps.supports_command_injection);
    assert!(caps.supports_subagents);
    assert!(!caps.supports_cloud_tasks);
    // and the adapter surfaces exactly this const.
    let a = adapter_with(RecordingSpawner::default());
    assert_eq!(a.capabilities(), CLAUDE_CAPABILITIES);
}

// ---- 3.2 L1 RED #2 — the launch spec is O-13-compliant (safety #10 — the enforcement surface) ----

#[test]
fn test_launch_spec_is_o13_compliant() {
    // safety #10 / O-13 — the launch spec is the default-mode-only / no-SDK-drive / no-bg-subagent
    // enforcement surface (PTY-primary). The argv + the generated settings ARE the contract.
    let spec = ClaudeLaunchSpec::build(
        Path::new("/Users/x/proj"),
        "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "/usr/local/bin/nexusops-hook",
    );
    let argv = spec.argv(); // program + args

    // default permission mode, explicit — NEVER acceptEdits/bypassPermissions/plan
    assert_eq!(spec.permission_mode(), "default");
    assert!(
        argv.windows(2)
            .any(|w| w[0] == "--permission-mode" && w[1] == "default"),
        "explicit --permission-mode default: {argv:?}"
    );
    for forbidden in ["acceptEdits", "bypassPermissions", "plan"] {
        assert!(
            !argv.iter().any(|a| a == forbidden),
            "no non-default permission mode ({forbidden})"
        );
    }
    // PTY-primary: NO -p/--print (that is SDK/headless drive), NO --dangerously-skip-permissions
    for forbidden in ["-p", "--print", "--dangerously-skip-permissions"] {
        assert!(
            !argv.iter().any(|a| a == forbidden),
            "PTY-primary: no {forbidden}"
        );
    }
    assert!(
        !argv.iter().any(|a| a.contains("dangerously")),
        "no --dangerously-* flag of any form"
    );

    // the hooks + statusLine are wired to the daemon (the generated settings, never the user's global).
    let settings = spec.settings();
    for hook in [
        "SessionStart",
        "PreToolUse",
        "PostToolUse",
        "Notification",
        "Stop",
    ] {
        assert!(settings.has_hook(hook), "the {hook} hook is wired");
    }
    assert!(settings.has_status_line(), "the statusLine is wired");
    // the hooks point at the injected daemon receiver, NOT a mutation of ~/.claude/settings.json.
    assert!(
        settings.references_receiver("/usr/local/bin/nexusops-hook"),
        "hooks reference the daemon receiver command"
    );
}

// ---- 3.2 L1 RED #3 — launch() spawns via the injected Pty seam → Starting (§9.1 + 3.4) ----

#[test]
fn test_launch_spawns_via_injected_pty_returns_starting() {
    // §9.1 launch + the 3.4 Pty seam — launch() builds the O-13 spec + spawns it via the injected
    // PtySpawner (FakePty here, PortablePtyHost in prod) exactly once, with the spec's argv + cwd,
    // and returns the normalized Starting status (the live spawn is the §14 non-deterministic surface).
    let spawner = RecordingSpawner::default();
    let mut adapter = adapter_with(spawner.clone());

    let status = adapter.launch();
    assert_eq!(
        status,
        Session::Starting,
        "launch → NormalizedStatus::Starting"
    );

    let calls = spawner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "spawned exactly once");
    assert_eq!(calls[0].program, "claude");
    assert_eq!(calls[0].cwd, PathBuf::from("/Users/x/proj"));
    assert!(
        calls[0]
            .args
            .windows(2)
            .any(|w| w[0] == "--permission-mode" && w[1] == "default"),
        "spawned with the O-13 argv (default mode): {:?}",
        calls[0].args
    );
}

// ---- 3.2 L1 RED #3b — a spawn failure yields Failed (the launch error path) ----

#[test]
fn test_launch_spawn_failure_yields_failed() {
    // launch() maps a spawn failure to Session::Failed (fail-closed — no phantom Starting). The
    // write-fail→Failed arm shares this contract (harder to inject deterministically; the spawn-fail
    // arm pins the error-handling shape).
    let mut adapter = ClaudeAdapter::new(
        Box::new(FailingSpawner),
        PathBuf::from("/Users/x/proj"),
        "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
        "/usr/local/bin/nexusops-hook".to_string(),
    );
    assert_eq!(
        adapter.launch(),
        Session::Failed,
        "a spawn failure → Failed (never a phantom Starting)"
    );
}
