//! The Claude Code `HarnessAdapter` (ARCHITECTURE §9.1, LOCKED ADR-006; 3.2 — **PTY-primary**).
//!
//! Drive via an **interactive PTY** (the 3.4 `PortablePtyHost`, spawned through the injected
//! `PtySpawner`) + `PreToolUse` hooks + a statusLine heartbeat — NEVER `-p`/SDK-drive (the cat-4
//! ruling = PTY-primary, Branch A). **Safety #10 / O-13:** `launch()` builds a spec that is
//! `default` permission mode ONLY (never acceptEdits/bypass/plan), no background subagents, no
//! `--dangerously-*` — the spec IS the enforcement surface. **Safety #9:** status derives from
//! STRUCTURED signals (hook events + transcript + the 3.4 exit event), NEVER from PTY output bytes.
//!
//! **This is part 1 — the OBSERVE path** (launch / status / transcript). The safety-critical
//! `MutationIntercept`→Gateway (INV-SEC-1) routing + the daemon hook-receiver ingress + telemetry
//! emission are **brief 043**; `resume()` (survival) is **Phase 4**. Each is a named stub here.
//! The production caller of `launch()`/`stream_status()` is the **Phase-4** session-lifecycle drive
//! loop; this module is built + fixture/FakePty-tested now.

use std::path::{Path, PathBuf};

use nexusops_shared::events::TelemetrySampled;
use nexusops_shared::harness::{
    HarnessCapabilities, NormalizedStatus, TelemetrySample, TranscriptRef,
};
use nexusops_shared::status::Session;

use crate::harness::{HarnessAdapter, MutationIntercept, ResumeResult};
use crate::terminal::{Pty, PtySpawner};

pub mod intercept;
mod status;
pub mod telemetry;
pub use status::{derive_status, ClaudeSignal, NotificationKind};
use telemetry::{telemetry_sample, TelemetryEventSink, UsageReading};

// ---- the 10 Claude HarnessCapabilities (Q1, lead-confirmed) -------------------------------------

/// The Claude Code capability matrix (§9.1 / PRD HARN-5; drives §11.4 per-capability UI degradation).
/// The 7 clear-true + the 3 Q1-confirmed: `command_injection=true` (the PTY accepts stdin — the
/// daemon CAN inject; the §6.4 injection-*action* wiring is separate/later), `subagents=true` (Task
/// subagents EXIST; the `MUTATION_COVERAGE_MATRIX` governs interception *reliability*, a separate
/// axis), `cloud_tasks=false` (not in the local MVP).
pub const CLAUDE_CAPABILITIES: HarnessCapabilities = HarnessCapabilities {
    supports_terminal: true,
    supports_resume: true,
    supports_transcript_read: true,
    supports_tool_call_parsing: true,
    supports_usage_metadata: true,
    supports_context_metadata: true,
    supports_command_injection: true,
    supports_subagents: true,
    supports_hooks: true,
    supports_cloud_tasks: false,
};

// ---- ClaudeSettings — the generated per-session hooks + statusLine (Q4: NEVER the user's global) -

/// The hook-command timeout (seconds) — 043's cheap-closure: SHORT (not the 600s default) so a hung
/// PreToolUse adjudication hook fails closed FAST instead of stalling the session for 10 minutes.
const HOOK_TIMEOUT_SECS: u64 = 5;

/// The generated per-session Claude `settings.json` content — the `PreToolUse`/`PostToolUse`/
/// `SessionStart`/`Notification`/`Stop` hooks + the statusLine, all wired to the daemon's hook
/// receiver. **Q4 (lead-confirmed):** this is a per-session file passed via `--settings`; the daemon
/// NEVER mutates the user's `~/.claude/settings.json`. The receiver ENDPOINT itself is **brief 043**;
/// 042 only wires the spec to point at it.
pub struct ClaudeSettings {
    json: serde_json::Value,
}

impl ClaudeSettings {
    fn build(hook_receiver: &str) -> Self {
        // each hook invokes the daemon receiver, tagged with its event (the receiver also sees
        // `hook_event_name` on stdin; the arg is belt-and-suspenders for the receiver's dispatch).
        // **043 (the cheap-closure):** every hook command carries a SHORT timeout (`HOOK_TIMEOUT_SECS`,
        // not the 600s default) so a hung/slow hook fails closed FAST — combined with no
        // `permissions.allow` + the non-interactive PTY, a PreToolUse hook-miss has no cached allow to
        // fall through to → it BLOCKS (fail-closed), never silently proceeds.
        let cmd = |event: &str| {
            serde_json::json!({
                "type": "command",
                "command": format!("{hook_receiver} {event}"),
                "timeout": HOOK_TIMEOUT_SECS,
            })
        };
        let json = serde_json::json!({
            "hooks": {
                "SessionStart": [{ "matcher": "startup|resume", "hooks": [cmd("SessionStart")] }],
                "PreToolUse":   [{ "matcher": "*",              "hooks": [cmd("PreToolUse")] }],
                "PostToolUse":  [{ "matcher": "*",              "hooks": [cmd("PostToolUse")] }],
                "Notification": [{ "hooks": [cmd("Notification")] }],
                "Stop":         [{ "hooks": [cmd("Stop")] }],
            },
            // **043 (the O-13 coverage-gap compensation + the cheap-closure):** the hook-INDEPENDENT
            // permission baseline. `deny` HARD-BLOCKS the un-interceptable channels (MCP best-effort /
            // `Task` subagent) regardless of any hook decision (a Claude deny-rule is evaluated even
            // when a hook returns allow). EMPTY `allow` (+ empty `ask`) makes the daemon hook the SOLE
            // allow-authority — nothing is pre-allowed or cached, so a hook-MISS on a mutating tool, in
            // the non-interactive daemon PTY, has no cached allow + no human to prompt → it BLOCKS
            // (fail-closed). NOTE: the exact Claude rule syntax for the deny-baseline is validated at
            // the P4 live drive loop; the receiver-side deny (intercept.rs) is the PRIMARY control —
            // this is the defense-in-depth second layer.
            "permissions": {
                "defaultMode": "default",
                "deny": ["mcp__*", "Task"],
                "allow": [],
                "ask": [],
            },
            "statusLine": { "type": "command", "command": format!("{hook_receiver} statusline") },
        });
        Self { json }
    }

    /// Whether a hook is wired for the given event.
    pub fn has_hook(&self, event: &str) -> bool {
        self.json.get("hooks").and_then(|h| h.get(event)).is_some()
    }

    /// Whether the statusLine is wired.
    pub fn has_status_line(&self) -> bool {
        self.json.get("statusLine").is_some()
    }

    /// Whether the generated settings reference the given daemon-receiver as the PROGRAM of a hook /
    /// statusLine command — an exact first-token match (`<receiver>` or `<receiver> …`), NOT a loose
    /// substring, so `/x/hook` does not falsely match `/x/hook-evil`.
    pub fn references_receiver(&self, receiver: &str) -> bool {
        let prefix = format!("{receiver} ");
        self.command_strings()
            .iter()
            .any(|c| c == receiver || c.starts_with(&prefix))
    }

    /// Every hook + statusLine `command` string in the settings (walks the structure).
    fn command_strings(&self) -> Vec<String> {
        let mut cmds = Vec::new();
        if let Some(hooks) = self.json.get("hooks").and_then(|h| h.as_object()) {
            for groups in hooks.values() {
                for group in groups.as_array().into_iter().flatten() {
                    for hook in group
                        .get("hooks")
                        .and_then(|h| h.as_array())
                        .into_iter()
                        .flatten()
                    {
                        if let Some(c) = hook.get("command").and_then(|c| c.as_str()) {
                            cmds.push(c.to_string());
                        }
                    }
                }
            }
        }
        if let Some(c) = self
            .json
            .get("statusLine")
            .and_then(|s| s.get("command"))
            .and_then(|c| c.as_str())
        {
            cmds.push(c.to_string());
        }
        cmds
    }

    /// Whether the generated `permissions.deny` carries the EXACT rule `pattern` (the 043
    /// hook-INDEPENDENT compensation for an un-interceptable channel — `mcp__*` / `Task`). This is an
    /// exact-string presence check (NOT glob tool-matching — Claude itself interprets the rule glob);
    /// the receiver-side `CoverageGap` deny is the PRIMARY control, this baseline is defense-in-depth.
    /// NOTE: the exact Claude rule grammar for the deny-baseline is validated at the P4 live drive loop.
    pub fn has_deny_rule(&self, pattern: &str) -> bool {
        self.permission_list("deny").iter().any(|d| d == pattern)
    }

    /// Whether `permissions.allow` is EMPTY (043's cheap-closure: the daemon hook is the SOLE
    /// allow-authority, so a hook-MISS has no cached allow → it fails closed in the non-interactive PTY).
    pub fn permission_allow_is_empty(&self) -> bool {
        self.permission_list("allow").is_empty()
    }

    /// The `permissions.<key>` array's string entries (empty if absent).
    fn permission_list(&self, key: &str) -> Vec<String> {
        self.json
            .get("permissions")
            .and_then(|p| p.get(key))
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The `PreToolUse` hook's timeout in seconds (043's cheap-closure — short so a hung adjudication
    /// hook fails closed fast). `None` if absent (a missing timeout would default to 600s).
    pub fn pre_tool_use_timeout_secs(&self) -> Option<u64> {
        self.json
            .get("hooks")?
            .get("PreToolUse")?
            .as_array()?
            .iter()
            .flat_map(|g| {
                g.get("hooks")
                    .and_then(|h| h.as_array())
                    .into_iter()
                    .flatten()
            })
            .find_map(|h| h.get("timeout").and_then(|t| t.as_u64()))
    }

    /// The settings JSON serialized for the `--settings` file. **Fallible** so a (latent) serialize
    /// failure fails CLOSED — `launch()` maps it to `Failed` rather than writing an empty, hook-less
    /// settings file (a hook-less session would silently skip the 043 INV-SEC-1 interception ingress).
    pub fn to_json_string(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&self.json)
    }
}

// ---- ClaudeLaunchSpec — the O-13 enforcement surface (safety #10; PURE construction) ------------

/// The O-13-compliant `claude` launch spec: the argv + the generated per-session settings. **Pure**
/// (no I/O — `launch()` does the settings-file write + the spawn). Constructed so a non-`default`
/// permission mode / `-p` / `--dangerously-*` / background-subagent enablement is impossible by
/// construction (safety #10 — the spec is the enforcement surface).
pub struct ClaudeLaunchSpec {
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    settings: ClaudeSettings,
    settings_path: PathBuf,
}

impl ClaudeLaunchSpec {
    /// Build the O-13 spec for a session. PTY-primary: explicit `--permission-mode default`, the
    /// generated `--settings` file, and NOTHING that drives via the SDK (`-p`) or escalates
    /// permissions (`--dangerously-*` / acceptEdits / bypass / plan).
    pub fn build(cwd: &Path, session_id: &str, hook_receiver: &str) -> Self {
        let settings = ClaudeSettings::build(hook_receiver);
        // a generated per-session settings file (Q4) — under the system temp dir for 042; the
        // production per-session settings dir (the daemon app-support dir) wires at the P4 drive loop.
        let settings_path =
            std::env::temp_dir().join(format!("nexusops-claude-settings-{session_id}.json"));
        let args = vec![
            "--permission-mode".to_string(),
            "default".to_string(),
            "--settings".to_string(),
            settings_path.to_string_lossy().into_owned(),
        ];
        Self {
            program: "claude".to_string(),
            args,
            cwd: cwd.to_path_buf(),
            settings,
            settings_path,
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

    pub fn settings(&self) -> &ClaudeSettings {
        &self.settings
    }

    pub fn settings_path(&self) -> &Path {
        &self.settings_path
    }

    /// The locked permission mode — always `default` (O-13; never escalated).
    pub fn permission_mode(&self) -> &str {
        "default"
    }

    /// The full argv (program + args).
    pub fn argv(&self) -> Vec<String> {
        let mut v = Vec::with_capacity(self.args.len() + 1);
        v.push(self.program.clone());
        v.extend(self.args.iter().cloned());
        v
    }

    /// Write the generated per-session settings file (the I/O part of `launch`). NEVER touches the
    /// user's `~/.claude/settings.json` (Q4). Fails CLOSED on a serialize error (no empty/hook-less
    /// file). Written **0600** (not world-readable) — defense-in-depth; the move to the daemon
    /// app-support dir + `O_EXCL` against the predictable temp name is the 043/P4 hardening (Finding).
    pub fn write_settings(&self) -> std::io::Result<()> {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        let json = self
            .settings
            .to_json_string()
            .map_err(|e| std::io::Error::other(format!("settings serialize: {e}")))?;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&self.settings_path)?;
        f.write_all(json.as_bytes())
    }
}

// ---- ClaudeAdapter — the §9.1 adapter (observe path) --------------------------------------------

/// The default PTY size for a launched session (the real size comes from the client at the P4 drive loop).
const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLS: u16 = 80;

/// The Claude project-dir slug for a cwd: path separators + dots → '-' (so `/Users/x/proj` →
/// `-Users-x-proj`). Best-effort — 043's live hook input supplies the authoritative `transcript_path`.
fn project_slug(cwd: &Path) -> String {
    cwd.to_string_lossy().replace(['/', '.'], "-")
}

/// The Claude Code `HarnessAdapter` (PTY-primary). Owns the spawned `Box<dyn Pty>` (the 3.4 seam) +
/// the derived status state machine. **Send, not Sync** — owned by ONE drive-loop thread.
pub struct ClaudeAdapter {
    spawner: Box<dyn PtySpawner>,
    cwd: PathBuf,
    session_id: String,
    hook_receiver: String,
    /// the live PTY child, set on `launch()` (the daemon owns it; display-only, safety #9).
    pty: Option<Box<dyn Pty>>,
    /// the current derived status (L2 `derive_status` advances it via `push_signal`).
    status: Session,
    /// the last CUMULATIVE usage reading (044) — `push_usage` deltas the next reading against it.
    last_cumulative: Option<UsageReading>,
    /// the latest per-heartbeat DELTA sample (044) — what `telemetry_heartbeat` returns. Updated by
    /// `push_usage` REGARDLESS of whether a sink is bound (delta-tracking is decoupled from emission).
    last_sample: Option<TelemetrySample>,
    /// the ExecutionProfile id for the telemetry rollup dims — resolved at launch/approval time
    /// (safety #8) by the P4 drive loop; `None` in 044's standalone path (the projector keys
    /// `None`→`~` sentinel).
    execution_profile_id: Option<String>,
    /// the injected telemetry emission seam (044; the 3.4 `TerminalEventSink` precedent). The
    /// production write-actor binding + the periodic pump land at the P4 drive loop; `None` =
    /// store-the-delta-but-don't-emit (the sink-less default; P4 always binds one).
    telemetry_sink: Option<Box<dyn TelemetryEventSink>>,
}

impl ClaudeAdapter {
    pub fn new(
        spawner: Box<dyn PtySpawner>,
        cwd: PathBuf,
        session_id: String,
        hook_receiver: String,
    ) -> Self {
        Self {
            spawner,
            cwd,
            session_id,
            hook_receiver,
            pty: None,
            // pre-launch: the daemon-created session state (`Creating` is a daemon-lifecycle state,
            // not adapter-derived — `launch()` transitions it to `Starting`).
            status: Session::Creating,
            last_cumulative: None,
            last_sample: None,
            execution_profile_id: None,
            telemetry_sink: None,
        }
    }

    /// Inject the telemetry emission [`TelemetryEventSink`] (044). A BUILDER (not a `new` parameter) so
    /// the existing 4-arg [`new`](Self::new) call sites don't churn — the `TerminalSession::new(.., sink)`
    /// precedent applied as a builder. The production write-actor binding + the periodic pump land at
    /// the P4 drive loop; tests inject a collecting double.
    pub fn with_telemetry_sink(mut self, sink: Box<dyn TelemetryEventSink>) -> Self {
        self.telemetry_sink = Some(sink);
        self
    }

    /// Feed one structured [`ClaudeSignal`] into the status derivation (the §14 ingestion seam — the
    /// live hook receiver + transcript tailer + statusLine PUSH these typed signals; the I/O wires
    /// at 043/P4). Advances [`stream_status`](HarnessAdapter::stream_status) via [`derive_status`] —
    /// NEVER from PTY output bytes (safety #9).
    pub fn push_signal(&mut self, signal: ClaudeSignal) {
        self.status = derive_status(self.status, &signal);
    }

    /// Ingest one CUMULATIVE [`UsageReading`] (044; the §14 ingestion seam — the live transcript/
    /// statusLine merge PUSHES these at the P4 drive loop, the `push_signal` precedent). Telemetry is
    /// kept ORTHOGONAL to status derivation (a usage reading can never drive a status transition,
    /// safety #9). Computes the per-heartbeat DELTA against the last reading, STORES it (so
    /// [`telemetry_heartbeat`](HarnessAdapter::telemetry_heartbeat) is correct even sink-less), and
    /// emits exactly one `TelemetrySampled` observation event through the sink if one is bound.
    /// Delta-tracking is updated BEFORE (and regardless of) emission — never coupled to the sink.
    pub fn push_usage(&mut self, reading: UsageReading) {
        let sample = telemetry_sample(self.last_cumulative.as_ref(), &reading);
        let event = TelemetrySampled {
            sample: sample.clone(),
            model: reading.model.clone(),
            execution_profile_id: self.execution_profile_id.clone(),
        };
        // update delta-tracking first — decoupled from emission (the sink-less heartbeat stays correct).
        self.last_cumulative = Some(reading);
        self.last_sample = Some(sample);
        if let Some(sink) = &self.telemetry_sink {
            sink.emit_telemetry(event);
        }
    }
}

impl HarnessAdapter for ClaudeAdapter {
    fn capabilities(&self) -> HarnessCapabilities {
        CLAUDE_CAPABILITIES
    }

    fn launch(&mut self) -> NormalizedStatus {
        let spec = ClaudeLaunchSpec::build(&self.cwd, &self.session_id, &self.hook_receiver);
        // write the generated per-session settings (Q4 — never the user's global config). A write
        // failure leaves the session un-launched → Failed (the richer launch-error handling lands
        // with the P4 drive loop).
        if spec.write_settings().is_err() {
            self.status = Session::Failed;
            return self.status;
        }
        match self.spawner.spawn(
            spec.program(),
            spec.args(),
            &self.cwd,
            DEFAULT_ROWS,
            DEFAULT_COLS,
        ) {
            Ok(pty) => {
                self.pty = Some(pty);
                self.status = Session::Starting;
            }
            Err(_) => self.status = Session::Failed,
        }
        self.status
    }

    fn stream_status(&self) -> NormalizedStatus {
        // the current derived status — advanced by `push_signal` over structured signals via
        // `derive_status`, NEVER from PTY output bytes (safety #9).
        self.status
    }

    fn intercept_mutation(&self) -> Option<MutationIntercept> {
        None // → 043 (the PreToolUse→Gateway INV-SEC-1 routing; named, not silent)
    }

    fn read_transcript(&self) -> Option<TranscriptRef> {
        // Claude stores the session transcript at ~/.claude/projects/<slug>/<session_id>.jsonl, where
        // <slug> is the cwd with path separators → '-'. This is the best-effort default derived from
        // cwd + session_id; 043's live hook input carries the AUTHORITATIVE `transcript_path` (every
        // hook provides it). None only if the capability is off (never for Claude) or HOME is unset.
        // defensive — `supports_transcript_read` is always `true` for Claude; this honors the
        // capability gate so the Codex adapter (3.3) can reuse the shape with `false`.
        if !CLAUDE_CAPABILITIES.supports_transcript_read {
            return None;
        }
        let home = std::env::var_os("HOME")?;
        let path = PathBuf::from(home)
            .join(".claude")
            .join("projects")
            .join(project_slug(&self.cwd))
            .join(format!("{}.jsonl", self.session_id));
        Some(TranscriptRef {
            // lossy is intentional — a best-effort default; 043's hook input overwrites with the
            // authoritative `transcript_path` (a non-UTF-8 HOME is exotic on macOS).
            path: path.to_string_lossy().into_owned(),
            // an HONEST placeholder — real content-addressing of the live-appended JSONL is the P4
            // reattach/dedup story (Q3); never a fake `sha256:` that would mislead that path.
            hash: String::new(), // → P4 content-addressing
            is_in_place: true,
        })
    }

    fn telemetry_heartbeat(&self) -> Option<TelemetrySample> {
        // the latest per-heartbeat DELTA sample (044) — `None` before any reading. Per-heartbeat
        // DELTAS, never cumulative (the proj_usage_ledger SUMs them). The live transcript/statusLine
        // merge that FEEDS `push_usage` + the periodic pump cadence land at the P4 drive loop.
        self.last_sample.clone()
    }

    fn resume(&self) -> ResumeResult {
        // → Phase 4 (survival §8/§17). A minimal "did not re-attach live" stub for now.
        ResumeResult {
            resumed_live: false,
            replayed_event_count: 0,
        }
    }
}
