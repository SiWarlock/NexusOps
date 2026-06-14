//! Phase 3.3b — CodexLaunchSpec (the pure `codex exec` argv — the no-bypass enforcement surface) +
//! env-hygiene + the resume-handle. RED-first.
//! ARCHITECTURE §9.1 (the launch spec; the ClaudeLaunchSpec precedent), §15 (the no-bypass argv +
//! the umask the spec carries), §8.1 (the resume-handle feeding the harness-agnostic decide_resume).
//! Foundation: docs/planning/0.3-codex-schema-research.md §3.2 (resume) / §4.3 (sandbox).
//!
//! NON-cat-1 + NO SPAWN (the binding condition — the live codex + its PreToolUse→Gateway interception
//! land together at 3.3c). #12 structurally pins the no-spawn.

use std::path::Path;

use nexusopsd::harness::codex::auth::{
    bind_codex_profile, resolve_codex_auth, AuthSources, CodexExecutionProfile,
};
use nexusopsd::harness::codex::launch::{has_resume_handle, CodexLaunchSpec};
use nexusopsd::harness::codex::perms::CODEX_CHILD_UMASK;

const UUID: &str = "019e75d8-6e9a-7893-839c-da1256d36380";

fn profile() -> CodexExecutionProfile {
    // a resolved Chatgpt (auth.json) profile — source_env=None → env-hygiene strips ALL auth env vars.
    let resolved = resolve_codex_auth(&AuthSources {
        auth_json_chatgpt: true,
        ..AuthSources::default()
    })
    .unwrap();
    bind_codex_profile(
        resolved,
        "keychain://codex/sess-x",
        "gpt-5.5",
        "openai",
        "on-request",
        "workspace-write",
        Some("default"),
    )
}

// ---- #1 — the exact `codex exec` argv (§9.1) ---------------------------------------------------

#[test]
fn test_codex_launch_spec_argv() {
    // spec(§9.1) — CodexLaunchSpec::build → the exact ordered `codex exec --json --sandbox <s>
    // --ask-for-approval <a> --model <m> --profile <p>` token sequence (the ClaudeLaunchSpec::build
    // precedent). program == codex.
    let spec = CodexLaunchSpec::build(Path::new("/work/proj"), "sess_x", &profile());
    assert_eq!(spec.program(), "codex");
    assert_eq!(
        spec.argv(),
        vec![
            "codex",
            "exec",
            "--json",
            "--sandbox",
            "workspace-write",
            "--ask-for-approval",
            "on-request",
            "--model",
            "gpt-5.5",
            "--profile",
            "default",
        ]
    );
}

// ---- #2 — no bypass by construction (§15 enforcement surface) -----------------------------------

#[test]
fn test_launch_spec_no_bypass_by_construction() {
    // spec(§15) — `--sandbox` is ALWAYS present (never silently omitted); NO constructor path emits
    // `--yolo` / `--dangerously-bypass-approvals-and-sandbox` / `--full-auto`. The spec is the
    // no-bypass half of the INV-SEC-1 enforcement surface (3.3c proves the sandbox containment).
    for spec in [
        CodexLaunchSpec::build(Path::new("/w"), "s", &profile()),
        CodexLaunchSpec::build_resume(Path::new("/w"), UUID, "s", &profile()),
    ] {
        let argv = spec.argv();
        assert!(
            argv.iter().any(|a| a == "--sandbox"),
            "--sandbox always present"
        );
        for bypass in [
            "--yolo",
            "--dangerously-bypass-approvals-and-sandbox",
            "--full-auto",
        ] {
            assert!(
                !argv.iter().any(|a| a == bypass),
                "no bypass flag ({bypass})"
            );
        }
    }
    // structural: no bypass flag is EMITTED as a string LITERAL in the launch source. Grep the quoted
    // form (`"--yolo"`) so the doc-comments that DOCUMENT the forbidden flags (bare/backtick prose)
    // don't false-alarm — only an actual `push("--yolo")`-style emission would.
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/harness/codex/launch.rs"
    ))
    .unwrap();
    for bypass in ["\"--yolo\"", "\"--dangerously-bypass", "\"--full-auto\""] {
        assert!(
            !src.contains(bypass),
            "launch source emits no {bypass} literal"
        );
    }
}

// ---- #5b — env hygiene sets the correlation key + pins auth (§15 #8) ----------------------------

#[test]
fn test_launch_env_mutations_pin_and_correlate() {
    // spec(§15) — the spec's env_mutations() compose the auth-pinning strips (auth.rs) + SET
    // NEXUSOPS_SESSION_ID (the correlation key the child/hook inherits). For a Chatgpt method the
    // API-key env vars are stripped; the session id is set.
    let spec = CodexLaunchSpec::build(Path::new("/w"), "sess_corr", &profile());
    let muts = spec.env_mutations();
    let set: Vec<_> = muts.iter().filter(|e| e.value.is_some()).collect();
    assert!(
        set.iter()
            .any(|e| e.key == "NEXUSOPS_SESSION_ID" && e.value.as_deref() == Some("sess_corr")),
        "sets the session correlation key"
    );
    let removed: Vec<_> = muts
        .iter()
        .filter(|e| e.value.is_none())
        .map(|e| &e.key)
        .collect();
    assert!(
        removed.iter().any(|k| *k == "OPENAI_API_KEY"),
        "Chatgpt method strips the API-key env"
    );
}

// ---- #10b — the spec carries umask 0o077 (§15 #11) ---------------------------------------------

#[test]
fn test_launch_spec_umask_0077() {
    // spec(§15) — the spec exposes the child umask 0o077 (the spawner applies it pre-exec at 3.3c).
    let spec = CodexLaunchSpec::build(Path::new("/w"), "s", &profile());
    assert_eq!(spec.umask(), 0o077);
    assert_eq!(spec.umask(), CODEX_CHILD_UMASK);
}

// ---- #11 — the resume-handle (§8.1 / research §3.2) --------------------------------------------

#[test]
fn test_resume_handle_uuid_keyed() {
    // spec(§8.1) — the resume variant → `codex exec resume <UUID> --json …` (the CONFIRMED-LOCAL CLI
    // form, keyed off the rollout UUIDv7); has_resume_handle true iff a resumable rollout UUID exists
    // (so the harness-agnostic decide_resume picks Resumed vs falls through). The app-server
    // `thread/resume`/`thr_` path + the UUID↔thr_ interconversion are HITL (Open-Q #3/#4).
    let spec = CodexLaunchSpec::build_resume(Path::new("/w"), UUID, "s", &profile());
    let argv = spec.argv();
    // the resume sub-form: `exec resume <UUID>` in order, then the flags.
    let exec_i = argv.iter().position(|a| a == "exec").expect("exec");
    assert_eq!(argv[exec_i + 1], "resume", "exec resume …");
    assert_eq!(argv[exec_i + 2], UUID, "keyed off the rollout UUID");
    assert!(argv.iter().any(|a| a == "--json"));

    assert!(
        has_resume_handle(Some(UUID)),
        "a resumable UUID → has handle"
    );
    assert!(
        !has_resume_handle(None),
        "no UUID → no handle (decide_resume falls through)"
    );
}

// ---- #11b — CodexAdapter::resume_inputs feeds the harness-agnostic decide_resume (§8.1) ---------

#[test]
fn test_codex_adapter_resume_inputs() {
    // spec(§8.1) — the per-adapter seam (flag 3): resume_inputs feeds the harness-agnostic
    // decide_resume. has_resume_handle reflects the PRIOR rollout file's existence (NOT merely a
    // non-empty session id) — so a recovered session (rollout present) → Resumed, a fresh/never-run
    // session (no rollout) → Relaunched (no spurious Resumed). The production consumer is the bootstrap
    // restart path (recover_sessions_on_restart, 3.3c wiring).
    use nexusops_shared::harness::ResumeMode;
    use nexusopsd::harness::codex::CodexAdapter;
    use nexusopsd::harness::resume::decide_resume;

    // a PRESENT rollout → has_resume_handle → Resumed.
    let dir = tempfile::tempdir().unwrap();
    let rollout = dir.path().join("rollout.jsonl");
    std::fs::write(&rollout, b"{}").unwrap();
    let recovered = CodexAdapter::new(rollout, UUID.to_string());
    let inputs = recovered.resume_inputs();
    assert!(inputs.supports_resume, "Codex supports resume");
    assert!(
        inputs.has_resume_handle,
        "a present rollout → resume handle"
    );
    assert_eq!(decide_resume(&inputs).mode, ResumeMode::Resumed);

    // a MISSING rollout (fresh/never-run session) → no handle → Relaunched (no spurious Resumed).
    let fresh = CodexAdapter::new(dir.path().join("absent.jsonl"), UUID.to_string());
    assert!(
        !fresh.resume_inputs().has_resume_handle,
        "missing rollout → no resume handle"
    );
    assert_eq!(
        decide_resume(&fresh.resume_inputs()).mode,
        ResumeMode::Relaunched
    );
}

// ---- #12 — NO codex spawn in the slice (the cat-1 binding condition) ----------------------------

#[test]
fn test_no_codex_spawn_in_slice() {
    // the binding condition (the 042/4.0a "mechanism built, no live caller" idiom): the codex harness
    // surface spawns NOTHING — no `Command::new("codex")`, no `.spawn(`, no `SessionLauncher` impl. The
    // reachable live codex + its PreToolUse→Gateway interception land TOGETHER at 3.3c.
    let codex_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/codex");
    for entry in std::fs::read_dir(codex_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        for forbidden in ["Command::new", ".spawn(", "impl SessionLauncher"] {
            assert!(
                !src.contains(forbidden),
                "{}: no live-spawn surface (`{forbidden}`) — the 3.3c binding condition",
                path.display()
            );
        }
    }
}
