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

// ==== L2 — derive_status: structured signals → the §5.1 Session machine ==========================

// ---- 3.2 L2 RED #4 — the hook → Session mapping table (§5.1 R-4) ----

#[test]
fn test_derive_status_hook_to_session_mapping() {
    use nexusopsd::harness::claude::{derive_status, ClaudeSignal, NotificationKind};
    use Session::*;

    let start = ClaudeSignal::SessionStart {
        source: "startup".to_string(),
        model: Some("claude-opus-4-8".to_string()),
    };
    assert_eq!(
        derive_status(Creating, &start),
        Starting,
        "SessionStart → Starting"
    );

    let pre = |tool: &str| ClaudeSignal::PreToolUse {
        tool: tool.to_string(),
    };
    assert_eq!(derive_status(Active, &pre("Bash")), RunningCommand);
    assert_eq!(derive_status(Active, &pre("Write")), EditingFiles);
    assert_eq!(derive_status(Active, &pre("Edit")), EditingFiles);
    assert_eq!(derive_status(Active, &pre("MultiEdit")), EditingFiles);
    assert_eq!(derive_status(Active, &pre("NotebookEdit")), EditingFiles);
    assert_eq!(
        derive_status(Idle, &pre("Read")),
        Active,
        "other tool → Active"
    );

    assert_eq!(
        derive_status(RunningCommand, &ClaudeSignal::PostToolUse),
        Active
    );

    assert_eq!(
        derive_status(
            Active,
            &ClaudeSignal::Notification(NotificationKind::Permission)
        ),
        WaitingOnPermission
    );
    assert_eq!(
        derive_status(
            Active,
            &ClaudeSignal::Notification(NotificationKind::InputNeeded)
        ),
        WaitingOnHumanInput
    );
    // an unclassified notification holds the prior state (no spurious transition).
    assert_eq!(
        derive_status(
            RunningCommand,
            &ClaudeSignal::Notification(NotificationKind::Other)
        ),
        RunningCommand
    );

    assert_eq!(
        derive_status(Active, &ClaudeSignal::Stop),
        Idle,
        "Stop → Idle"
    );
}

// ---- 3.2 L2 RED #5 — process exit → the terminal Session states (§17) ----

#[test]
fn test_derive_status_process_exit_terminal() {
    use nexusopsd::harness::claude::{derive_status, ClaudeSignal};
    use Session::*;
    let exit = |code: Option<i32>, signal: Option<&str>| ClaudeSignal::ProcessExited {
        exit_code: code,
        signal: signal.map(|s| s.to_string()),
    };
    assert_eq!(
        derive_status(Active, &exit(Some(0), None)),
        Completed,
        "exit 0 → Completed"
    );
    assert_eq!(
        derive_status(Active, &exit(Some(1), None)),
        Failed,
        "exit ≠0 → Failed"
    );
    assert_eq!(
        derive_status(RunningCommand, &exit(None, Some("SIGKILL"))),
        Failed,
        "signal → Failed"
    );
    // the unknown `(None, None)` exit (no code, no signal) is fail-closed → Failed, never a false Completed.
    assert_eq!(
        derive_status(Active, &exit(None, None)),
        Failed,
        "unknown exit → Failed (fail-closed)"
    );
}

// ---- 3.2 L2 RED #6 — an illegal (terminal-sink) transition holds the prior state (R-9) ----

#[test]
fn test_derive_status_illegal_transition_holds_state() {
    use nexusopsd::harness::claude::{derive_status, ClaudeSignal};
    use Session::*;
    // a terminal state is a SINK — no signal resurrects it (the adapter-level R-9 guard; the full
    // §5.1 legal-edge set is a §5.1-machine concern, NOT the adapter's). Held, never corrupted.
    for prev in [Completed, Failed, Archived, Killed] {
        assert_eq!(
            derive_status(
                prev,
                &ClaudeSignal::PreToolUse {
                    tool: "Bash".to_string()
                }
            ),
            prev,
            "{prev:?} is terminal — held, not transitioned"
        );
        assert_eq!(
            derive_status(
                prev,
                &ClaudeSignal::ProcessExited {
                    exit_code: Some(0),
                    signal: None
                }
            ),
            prev,
            "{prev:?} is terminal — a second exit does not change it"
        );
        assert_eq!(
            derive_status(prev, &ClaudeSignal::Stop),
            prev,
            "{prev:?} is terminal — a Stop signal does not move it (guard precedes every arm)"
        );
    }
}

// ---- 3.2 L2 RED #7 — status derives from STRUCTURED signals, never PTY bytes (#9 / forbidden #4) ----

#[test]
fn test_status_derivation_takes_structured_signals_not_pty_bytes() {
    use nexusopsd::harness::claude::{derive_status, ClaudeSignal};
    // compile-time: derive_status's input is the STRUCTURED `ClaudeSignal` — it has NO access to PTY
    // output bytes (safety #9); this call proves the signature.
    let _ = derive_status(Session::Active, &ClaudeSignal::Stop);
    // structural grep-pin (the 3.4 #9 pattern, applied to the status derivation): the status path
    // reads NO PTY/TerminalSession output to derive status. Scoped to `claude/status.rs` (the pure
    // derivation) so the future hook-receiver/transcript-reader I/O (043/P4) in sibling modules can
    // NOT false-alarm it. (The adapter owns a `Box<dyn Pty>` for launch, but never reads it.)
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/harness/claude/status.rs"
    ))
    .unwrap();
    for tok in [
        "TerminalSession",
        "read_step",
        "PtyRead",
        ".pump(",
        "pty.read",
    ] {
        assert!(
            !src.contains(tok),
            "claude status path must not read PTY output (`{tok}`) — #9 display-only"
        );
    }
}

// ---- 3.2 L2 RED #7b — push_signal advances the adapter's stream_status via derive_status ----

#[test]
fn test_push_signal_updates_stream_status() {
    use nexusopsd::harness::claude::ClaudeSignal;
    let mut adapter = adapter_with(RecordingSpawner::default());
    assert_eq!(adapter.launch(), Session::Starting);
    assert_eq!(adapter.stream_status(), Session::Starting);
    adapter.push_signal(ClaudeSignal::PreToolUse {
        tool: "Bash".to_string(),
    });
    assert_eq!(
        adapter.stream_status(),
        Session::RunningCommand,
        "the pushed signal advanced status"
    );
    adapter.push_signal(ClaudeSignal::Stop);
    assert_eq!(adapter.stream_status(), Session::Idle);
}
