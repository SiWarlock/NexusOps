//! P3.2 part 1 — the Claude Code `HarnessAdapter` OBSERVE path (launch / status / transcript).
//!
//! PTY-primary (cat-4 resolved): drive via an interactive PTY + `PreToolUse` hooks + statusLine,
//! NEVER `-p`/SDK-drive, default permission mode only, no background subagents (O-13 / safety #10).
//! Status derives from STRUCTURED signals (hook events + transcript + the 3.4 exit event), NEVER
//! from PTY output bytes (safety #9). Interception (INV-SEC-1) + telemetry = brief 043; resume = P4.

use std::path::PathBuf;

use nexusops_shared::status::Session;
use nexusopsd::harness::claude::{ClaudeAdapter, ClaudeLaunchSpec, CLAUDE_CAPABILITIES};
use nexusopsd::harness::HarnessAdapter;

/// a live `ClaudeAdapter` for the observe-path tests (capability/status/transcript/telemetry). Option A
/// (P4.0b-2): the adapter no longer spawns or holds a PTY — the `PtyLauncher` owns the live-claude
/// spawn site (pinned by `tests/session_live.rs`); `new(cwd, session_id)` is the status/transcript-only
/// constructor (the launch/#10 tests moved to `session_live.rs`).
fn live_adapter() -> ClaudeAdapter {
    ClaudeAdapter::new(
        PathBuf::from("/Users/x/proj"),
        "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
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
    let a = live_adapter();
    assert_eq!(a.capabilities(), CLAUDE_CAPABILITIES);
}

// ---- 3.2 L1 RED #2 — the launch spec is O-13-compliant (safety #10 — the enforcement surface) ----

#[test]
fn test_launch_spec_is_o13_compliant() {
    // safety #10 / O-13 — the launch spec is the default-mode-only / no-SDK-drive / no-bg-subagent
    // enforcement surface (PTY-primary). The argv + the generated settings ARE the contract.
    let spec = ClaudeLaunchSpec::build(
        &PathBuf::from("/Users/x/proj"),
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

// ---- launch() = the Creating→Starting marker only (the launcher owns the spawn — Option A) --------
//
// (The former `test_launch_spawns_via_injected_pty_returns_starting` + `test_launch_spawn_failure_*`
// MOVED to `tests/session_live.rs` — the launcher is now the single live-claude spawn site + the O-13
// #10 enforcement surface, P4.0b-2 Option A. `ClaudeAdapter::launch` no longer spawns; it holds no
// spawner — structurally cannot.)

#[test]
fn test_launch_is_the_lifecycle_marker_no_spawn() {
    // spec(§9.1 / P4.0b-2 Option A) — the adapter's `launch()` is the daemon-lifecycle Creating→Starting
    // marker ONLY (it holds no spawner, no PTY). The live spawn + the O-13 #10 spec live at the launcher
    // (pinned by `tests/session_live.rs`); status then derives from hook signals (safety #9).
    let mut adapter = live_adapter();
    assert_eq!(
        adapter.launch(),
        Session::Starting,
        "launch() = the Creating→Starting marker (no spawn — the launcher owns the spawn site)"
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
    let mut adapter = live_adapter();
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

// ==== L3 — read_transcript + the observe-path completion + the §14 conformance fixture ==========

// ---- 3.2 L3 RED #8 — read_transcript locates the session JSONL (§9.1) ----

#[test]
fn test_read_transcript_locates_jsonl() {
    // §9.1 — Claude stores the session transcript at ~/.claude/projects/<slug(cwd)>/<session_id>.jsonl.
    // The path is derived from cwd + session_id (043's live hook input supplies the authoritative
    // path; this is the best-effort default). Never None for Claude (supports_transcript_read=true).
    assert!(
        std::env::var_os("HOME").is_some(),
        "this test derives ~/.claude/… and needs HOME set"
    );
    let adapter = live_adapter(); // cwd=/Users/x/proj, sess_01ARZ…
    let t = adapter
        .read_transcript()
        .expect("Claude supports transcript read");
    assert!(t.is_in_place, "the Claude transcript is in-place");
    assert!(
        t.path.contains("/.claude/projects/"),
        "under ~/.claude/projects: {}",
        t.path
    );
    assert!(
        t.path.contains("-Users-x-proj"),
        "the cwd-derived project slug: {}",
        t.path
    );
    assert!(
        t.path.ends_with("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV.jsonl"),
        "the session JSONL: {}",
        t.path
    );
    // Q3: an HONEST EMPTY placeholder hash now — real content-addressing is P4; NEVER a fake sha256:.
    assert!(
        t.hash.is_empty(),
        "empty placeholder hash until P4 content-addressing: {:?}",
        t.hash
    );

    // the project slug replaces path separators AND dots → '-' (best-effort; 043 overwrites). A
    // dotted cwd exercises the non-obvious dot-replacement.
    let dotted = ClaudeAdapter::new(
        PathBuf::from("/Users/x/v1.2.3/proj"),
        "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
    );
    let dt = dotted.read_transcript().expect("transcript ref");
    assert!(
        dt.path.contains("-Users-x-v1-2-3-proj"),
        "dots + separators slugged to '-': {}",
        dt.path
    );
}

// ---- 3.2 RED #9 — the still-deferred observe-path surfaces are honest stubs (named-not-silent) ----

#[test]
fn test_observe_path_stubs_marked() {
    use nexusopsd::harness::claude::telemetry::UsageReading;

    // intercept_mutation→ None (the live PreToolUse→Gateway hook wiring is P4), resume→ P4; the
    // adapter stays object-safe (Box<dyn HarnessAdapter>).
    let mut adapter: Box<dyn HarnessAdapter> = Box::new(live_adapter());
    adapter.launch();
    assert!(
        adapter.intercept_mutation().is_none(),
        "intercept_mutation → None (the live hook wiring is P4)"
    );
    let r = adapter.resume();
    assert!(
        !r.resumed_live,
        "resume → minimal, not re-attached live (survival is P4)"
    );

    // telemetry_heartbeat is NO LONGER an always-None stub (044): None before any reading, Some
    // after a usage reading is pushed (the emission landed this slice).
    let mut concrete = live_adapter();
    assert!(
        concrete.telemetry_heartbeat().is_none(),
        "telemetry_heartbeat → None before any usage reading"
    );
    concrete.push_usage(UsageReading {
        tokens_in: 100,
        tokens_out: 20,
        context_pct: Some(30.0),
        cost: 0.01,
        model: None,
    });
    assert!(
        concrete.telemetry_heartbeat().is_some(),
        "telemetry_heartbeat → Some after a reading (044 emission landed)"
    );
}

// ---- 3.2 L3 RED #10 — the §14 conformance fixture: a signal sequence → the Session trajectory ----

#[test]
fn test_claude_status_conformance_fixture() {
    use nexusopsd::harness::claude::{derive_status, ClaudeSignal, NotificationKind};
    use Session::*;
    // a golden ClaudeSignal sequence folded through derive_status → the expected §5.1 trajectory (the
    // §14 conformance scaffold; the both-harness conformance suite completes at 3.3 Codex).
    let pre = |tool: &str| ClaudeSignal::PreToolUse {
        tool: tool.to_string(),
    };
    let fixture: Vec<(ClaudeSignal, Session)> = vec![
        (
            ClaudeSignal::SessionStart {
                source: "startup".to_string(),
                model: Some("claude-opus-4-8".to_string()),
            },
            Starting,
        ),
        (pre("Read"), Active),
        (pre("Edit"), EditingFiles),
        (ClaudeSignal::PostToolUse, Active),
        (pre("Bash"), RunningCommand),
        (
            ClaudeSignal::Notification(NotificationKind::Permission),
            WaitingOnPermission,
        ),
        (ClaudeSignal::PostToolUse, Active),
        (ClaudeSignal::Stop, Idle),
        (
            ClaudeSignal::ProcessExited {
                exit_code: Some(0),
                signal: None,
            },
            Completed,
        ),
        // the terminal sink — a post-exit signal does NOT move it (R-9)
        (pre("Bash"), Completed),
    ];
    let mut state = Creating;
    for (sig, expected) in &fixture {
        state = derive_status(state, sig);
        assert_eq!(state, *expected, "after {sig:?}");
    }
}
