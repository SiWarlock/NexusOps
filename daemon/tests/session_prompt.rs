//! P4.0b-2-smoke (brief 053) — the Option-G `initial_prompt` thread. The `SessionExecutor` writes an
//! optional `initial_prompt` (+ the submit terminator) to the **launched** session's PTY exactly once,
//! **post-launch**, **best-effort** (a PTY write error degrades — it NEVER fail-closes the session).
//!
//! The drive mechanism rides the existing `TerminalSession::write` seam (§9.1 live drive loop); it does
//! NOT touch the interception / the O-13 #10 argv (asserted by the unchanged `tests/claude_intercept.rs`
//! plus the inverted guard `tests/session_executor.rs::test_live_session_create_has_interception`).
//! Safety holds: the prompt only makes claude *act* — every tool call still routes through the
//! unchanged `intercept` adjudication (the chokepoint, regardless of how claude was prompted).

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::harness::HarnessCapabilities;
use nexusops_shared::ids::{ActionRequestId, ExecutionProfileId, SessionId};
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::gateway::session_executor::SessionExecutor;
use nexusopsd::gateway::Gateway;
use nexusopsd::harness::FakeHarness;
use nexusopsd::idgen::UlidGen;
use nexusopsd::session::{
    spawn_supervisor_task, LaunchedSession, NullTerminalSink, SessionLauncher, SupervisorHandle,
};
use nexusopsd::terminal::{
    ExitStatus, FakePty, Pty, PtyKiller, PtyRead, PtyWriter, TerminalId, TerminalSession,
};

/// the input sink of a launched [`FakePty`] (Arc-shared so a test can assert the bytes written).
type CapturedSink = Arc<Mutex<Vec<u8>>>;
/// the launcher's slot holding the most-recent launch's [`CapturedSink`] (None until launch / erroring).
type CapturedSinkSlot = Arc<Mutex<Option<CapturedSink>>>;

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-13T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

fn full_caps() -> HarnessCapabilities {
    HarnessCapabilities {
        supports_terminal: true,
        supports_resume: true,
        supports_transcript_read: true,
        supports_tool_call_parsing: true,
        supports_usage_metadata: true,
        supports_context_metadata: true,
        supports_command_injection: true,
        supports_subagents: true,
        supports_hooks: true,
        supports_cloud_tasks: true,
    }
}

/// A test launcher that builds either a recordable [`FakePty`] (capturing its `input_sink` so a test
/// can assert the bytes the executor wrote) OR a write-erroring PTY (the degradation test), then yields
/// a [`LaunchedSession`]. Mirrors `session/launcher.rs::FakeLauncher` but exposes the captured sink +
/// (for the erroring PTY) a write-attempt counter, so the degradation test is a true RED (0 attempts
/// before the executor writes anything).
struct PromptLauncher {
    /// the `input_sink` of the most-recently-launched [`FakePty`] (None until launch / when erroring).
    captured_sink: CapturedSinkSlot,
    /// write attempts on the erroring PTY (proves the executor actually tried to write the prompt).
    write_attempts: Arc<AtomicUsize>,
    write_errors: bool,
    launches: Arc<AtomicUsize>,
}

impl PromptLauncher {
    fn new(write_errors: bool) -> Self {
        Self {
            captured_sink: Arc::new(Mutex::new(None)),
            write_attempts: Arc::new(AtomicUsize::new(0)),
            write_errors,
            launches: Arc::new(AtomicUsize::new(0)),
        }
    }
    fn captured_sink(&self) -> CapturedSinkSlot {
        Arc::clone(&self.captured_sink)
    }
    fn write_attempts(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.write_attempts)
    }
    fn launches(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.launches)
    }
}

impl SessionLauncher for PromptLauncher {
    fn launch_session(&self) -> io::Result<LaunchedSession> {
        self.launches.fetch_add(1, Ordering::SeqCst);
        let session_id = SessionId::new();
        let adapter = Box::new(FakeHarness::new(full_caps()));
        let terminal = if self.write_errors {
            TerminalSession::new(
                TerminalId::from_raw("term_smoke_test"),
                Box::new(WriteErrPty {
                    attempts: Arc::clone(&self.write_attempts),
                }),
                Box::new(NullTerminalSink),
            )
        } else {
            // an immediately-EOF FakePty (the actor's read-pump reaps at once); it records writes into
            // `input_sink`, which the test captures BEFORE the session moves into the supervisor.
            let pty = FakePty::new(
                vec![PtyRead::Eof],
                ExitStatus {
                    exit_code: Some(0),
                    signal: None,
                },
            );
            *self.captured_sink.lock().unwrap() = Some(pty.input_sink());
            TerminalSession::new(
                TerminalId::from_raw("term_smoke_test"),
                Box::new(pty),
                Box::new(NullTerminalSink),
            )
        };
        Ok(LaunchedSession {
            session_id,
            adapter,
            terminal,
        })
    }
}

/// A PTY whose `write` ALWAYS errors (the prompt-feed degradation test); its `read` EOFs immediately.
/// Counts attempts so the test can prove the executor actually tried to write (and then degraded).
struct WriteErrPty {
    attempts: Arc<AtomicUsize>,
}
impl Pty for WriteErrPty {
    fn read(&mut self) -> io::Result<PtyRead> {
        Ok(PtyRead::Eof)
    }
    fn write(&mut self, _bytes: &[u8]) -> io::Result<()> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(io::Error::other("simulated PTY write failure"))
    }
    fn resize(&mut self, _rows: u16, _cols: u16) -> io::Result<()> {
        Ok(())
    }
    fn exit_status(&mut self) -> ExitStatus {
        ExitStatus {
            exit_code: Some(0),
            signal: None,
        }
    }
    fn kill(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn killer(&self) -> Box<dyn PtyKiller> {
        Box::new(NoopKiller)
    }
    fn writer(&self) -> Box<dyn PtyWriter> {
        // consistent with this PTY's always-erroring write — the cross-thread writer also errors. (This
        // test exercises the initial_prompt feed via `terminal.write()`, not the cross-thread writer.)
        Box::new(ErrWriter)
    }
}
struct NoopKiller;
impl PtyKiller for NoopKiller {
    fn kill(&self) {}
}
struct ErrWriter;
impl PtyWriter for ErrWriter {
    fn write(&self, _bytes: &[u8]) -> io::Result<()> {
        Err(io::Error::other("simulated PTY write failure"))
    }
}

/// A permissive [`ProfileLookup`](nexusopsd::profiles::ProfileLookup) for the prompt tests: every id
/// "exists" + a fixed default — profile resolution always succeeds (these tests are about the
/// initial_prompt feed, not §15 #8 fail-closed resolution, which `session_executor.rs` covers).
struct AnyProfileLookup(ExecutionProfileId);

impl nexusopsd::profiles::ProfileLookup for AnyProfileLookup {
    fn default_id(&self) -> Result<ExecutionProfileId, nexusopsd::profiles::ProfileError> {
        Ok(self.0.clone())
    }
    fn exists(&self, _id: &ExecutionProfileId) -> Result<bool, nexusopsd::profiles::ProfileError> {
        Ok(true)
    }
}

/// The production policy (catalog-driven) + a [`SessionExecutor`] over the given launcher + a real
/// (unbounded) supervisor handle. Returns the gateway + the shutdown/join keep-alives (drop them only
/// at the end so the supervisor task stays up). Grab the launcher's sink/attempts/launch handles
/// BEFORE passing it in.
fn gateway_with(
    launcher: PromptLauncher,
) -> (
    Gateway,
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
) {
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let (join, handle): (_, SupervisorHandle) = spawn_supervisor_task(
        shutdown_rx,
        Arc::new(nexusopsd::decisions::DecisionRegistry::new()),
        Box::new(nexusopsd::session::NullSessionDeathSink),
        Arc::new(nexusopsd::terminal::NoopScrollbackStore),
    );
    // P5.3a — a permissive profile lookup: these tests exercise the initial_prompt feed, not §15 #8
    // resolution, so any requested id resolves + None → a fixed default (keeps the resolve fail-open here).
    let profile_lookup = Box::new(AnyProfileLookup(ExecutionProfileId::new()));
    let executor = SessionExecutor::new(Box::new(launcher), handle, profile_lookup);
    let gateway = Gateway::new(
        Box::new(nexusopsd::gateway::policy::CatalogPolicy),
        Box::new(executor),
    );
    (gateway, shutdown_tx, join)
}

/// A UI session.create with the project resource_ref (catalog `requires_resource_refs`) + an optional
/// `initial_prompt` in the inputs (the Option-G dev-drive).
fn session_create_req(prompt: Option<&str>) -> ActionRequest {
    let mut inputs = serde_json::Map::new();
    if let Some(p) = prompt {
        inputs.insert(
            "initial_prompt".to_string(),
            serde_json::Value::String(p.to_string()),
        );
    }
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "session.create".to_string(),
        requester_type: RequesterType::User, // UI/IPC (PIN e)
        requester_id: "u_local".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::Project,
            id: "proj_smoke_ctx".to_string(),
            uri: None,
        }],
        inputs: serde_json::Value::Object(inputs),
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-13T00:00:00Z").unwrap(),
    }
}

#[tokio::test]
async fn test_session_create_initial_prompt_written_to_pty() {
    // spec(§9.1) — the Option-G drive mechanism: the executor writes the initial_prompt (+ the `\r`
    // submit terminator) to the LAUNCHED session's PTY exactly once. Because the captured sink is the
    // launched FakePty's own input sink, a non-empty sink ALSO proves the write targeted the launched
    // terminal (not a pre-launch no-op) — subsuming the brief's "after-launch" ordering check.
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("nexusops.db"));
    let launcher = PromptLauncher::new(false);
    let sink = launcher.captured_sink();
    let (gw, _sd, _join) = gateway_with(launcher);

    gw.submit_action(
        &mut store,
        session_create_req(Some("read CLAUDE.md then stop")),
    )
    .expect("session.create auto-executes");

    let captured = sink
        .lock()
        .unwrap()
        .clone()
        .expect("a FakePty was launched + its input sink captured");
    let written = captured.lock().unwrap().clone();
    assert_eq!(
        written,
        b"read CLAUDE.md then stop\r".to_vec(),
        "the prompt + the `\\r` submit terminator reaches the launched PTY exactly once"
    );
}

#[tokio::test]
async fn test_session_create_no_prompt_writes_nothing() {
    // spec(§9.1) — additive/opt-in: no initial_prompt → nothing written (the 4.0b-1/4.0b-2 no-prompt
    // path is byte-unchanged; back-compat regression guard — green before AND after by design).
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("nexusops.db"));
    let launcher = PromptLauncher::new(false);
    let sink = launcher.captured_sink();
    let (gw, _sd, _join) = gateway_with(launcher);

    gw.submit_action(&mut store, session_create_req(None))
        .expect("session.create auto-executes");

    let captured = sink
        .lock()
        .unwrap()
        .clone()
        .expect("a FakePty was launched + its input sink captured");
    assert!(
        captured.lock().unwrap().is_empty(),
        "no initial_prompt → the launched PTY input sink stays empty (opt-in)"
    );
}

#[tokio::test]
async fn test_prompt_write_error_does_not_fail_the_session() {
    // spec(§17) — the prompt-feed is best-effort dev convenience, NOT a safety I/O: a PTY write error
    // DEGRADES (the session still launches + auto-executes to Succeeded), never fail-closes the session
    // (don't collapse a live-agent launch on a convenience write). Asserting the write was ATTEMPTED
    // makes this a true RED (0 attempts before the executor writes anything).
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("nexusops.db"));
    let launcher = PromptLauncher::new(true); // write-erroring PTY
    let attempts = launcher.write_attempts();
    let launches = launcher.launches();
    let (gw, _sd, _join) = gateway_with(launcher);

    let ack = gw
        .submit_action(
            &mut store,
            session_create_req(Some("this prompt write will error")),
        )
        .expect("session.create still succeeds despite the prompt-write error");
    assert_eq!(
        ack.status,
        ActionRequestStatus::Succeeded,
        "a prompt-write error degrades — the session still launches + auto-executes (fail-soft)"
    );
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        1,
        "the prompt write WAS attempted exactly once (and errored) — then degraded soft"
    );
    assert_eq!(
        launches.load(Ordering::SeqCst),
        1,
        "the session WAS launched (the prompt-write error did not abort the launch)"
    );
}
