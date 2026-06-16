//! The Codex `HarnessAdapter` (ARCHITECTURE §9.1, LOCKED ADR-006; 3.3 — the SECOND harness).
//!
//! The CodexAdapter implements the SAME frozen §9.1 `HarnessAdapter` trait as the Claude adapter — no
//! new trait (LESSON §25). The divergence is the PARSER: Codex persists/streams two vocabularies (the
//! rollout-JSONL `event_msg`/`response_item` [`parse`] and the `exec --json` `ThreadEvent` [`stream`]),
//! both normalized to ONE `CodexSignal` stream that ONE pure [`status::derive_status`] folds — vs
//! Claude's hook signals. Tool calls classify BY SEMANTICS not vendor names (the OSS-vs-desktop
//! divergence). **Safety #9:** status derives from the structured event stream, NEVER PTY bytes.
//!
//! **This is 3.3a — the OBSERVE core** (parse / normalize / `capabilities`/`stream_status`/
//! `read_transcript`). Launch + auth/umask (3.3b), the CAT-1 `PreToolUse`→Gateway interception +
//! `--sandbox` (3.3c), telemetry emission (3.3d), and the LIVE drive loop (HITL) are later slices —
//! each a named stub here. No production launch caller yet (the 042 "observe path built, live caller
//! later" precedent); the parser/normalizer feed the observe methods via [`CodexAdapter::push_signal`].

use std::path::PathBuf;

use nexusops_shared::harness::{
    HarnessCapabilities, NormalizedStatus, TelemetrySample, TranscriptRef,
};
use nexusops_shared::status::Session;

use crate::harness::{HarnessAdapter, MutationIntercept, ResumeMode, ResumeResult};

pub mod auth;
pub mod launch;
pub mod parse;
pub mod perms;
pub mod status;
pub mod stream;

pub use status::{derive_status, CodexSignal, CodexToolKind};

use crate::harness::resume::ResumeInputs;

// ---- the 10 Codex HarnessCapabilities (research §5/§6; 3.3a) ------------------------------------

/// The Codex capability matrix (§9.1 / PRD HARN-5; drives §11.4 per-capability UI degradation).
/// `context_metadata=true` — Codex reports `model_context_window` (the research corrected the earlier
/// "unknown"). `subagents=false` — Codex has NO subagent concept (the `MUTATION_COVERAGE_MATRIX` marks
/// it n/a). `hooks=true` (the Codex hooks system). `cloud_tasks=false` (not in the local MVP).
pub const CODEX_CAPABILITIES: HarnessCapabilities = HarnessCapabilities {
    supports_terminal: true,
    supports_resume: true,
    supports_transcript_read: true,
    supports_tool_call_parsing: true,
    supports_usage_metadata: true,
    supports_context_metadata: true,
    supports_command_injection: true,
    supports_subagents: false,
    supports_hooks: true,
    supports_cloud_tasks: false,
};

// ---- CodexAdapter — the §9.1 adapter (observe path) --------------------------------------------

/// The Codex `HarnessAdapter` (observe path). Holds the rollout path (the transcript) + the daemon
/// session id + the current derived status. **Status-from-the-event-stream ONLY** (safety #9): the
/// `stream_status` is advanced by [`push_signal`](Self::push_signal) over structured `CodexSignal`s —
/// the seam the 3.3c live `exec --json` / app-server stream will feed (the parser/normalizer produce
/// those signals). It does NOT own/spawn the PTY (3.3b's launcher does). **Send, not Sync** — owned by
/// ONE drive-loop thread (the Claude adapter precedent).
pub struct CodexAdapter {
    /// the rollout JSONL path (`~/.codex/sessions/.../rollout-*.jsonl`) — the transcript we read.
    rollout_path: PathBuf,
    session_id: String,
    /// the current derived status (advanced by `push_signal` via `derive_status`).
    status: Session,
}

impl CodexAdapter {
    pub fn new(rollout_path: PathBuf, session_id: String) -> Self {
        Self {
            rollout_path,
            // pre-launch daemon-created state (`Creating` is a daemon-lifecycle state, not
            // adapter-derived — `launch()` transitions it to `Starting`).
            status: Session::Creating,
            session_id,
        }
    }

    /// Feed one structured [`CodexSignal`] into the status derivation (the §14 ingestion seam — the
    /// live `exec --json`/app-server stream PUSHES these typed signals; the I/O wires at 3.3c).
    /// Advances [`stream_status`](HarnessAdapter::stream_status) via [`derive_status`] — NEVER from PTY
    /// output bytes (safety #9). The same seam `parse`/`stream` feed the parsed signals through.
    pub fn push_signal(&mut self, signal: CodexSignal) {
        self.status = derive_status(self.status, &signal);
    }

    /// The daemon session id this adapter tracks (the rollout UUIDv7).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// The §8.1 survival inputs this Codex session feeds the harness-agnostic `decide_resume` (3.3b):
    /// Codex `supports_resume` + a resume handle iff a resumable rollout exists. The production consumer
    /// is the bootstrap restart path (`recover_sessions_on_restart` builds `ResumeInputs` from the
    /// recovered session); this is the per-adapter seam that bit feeds.
    pub fn resume_inputs(&self) -> ResumeInputs {
        // a resume handle exists iff a PRIOR rollout file is present — a fresh, never-run session has
        // none, so `decide_resume` falls through to `Relaunched`, NOT a spurious `Resumed`. The Codex
        // resume id IS the rollout UUIDv7 (== session_id); the file's existence is the resumability
        // signal (NOT merely a non-empty daemon session id, which is always set).
        let resumable = self.rollout_path.exists() && !self.session_id.is_empty();
        let rollout_uuid = resumable.then_some(self.session_id.as_str());
        ResumeInputs {
            supports_resume: CODEX_CAPABILITIES.supports_resume,
            has_resume_handle: launch::has_resume_handle(rollout_uuid),
            ..ResumeInputs::default()
        }
    }
}

impl HarnessAdapter for CodexAdapter {
    fn capabilities(&self) -> HarnessCapabilities {
        CODEX_CAPABILITIES
    }

    fn launch(&mut self) -> NormalizedStatus {
        // 3.3a (the Claude Option-A precedent): the adapter does NOT spawn — 3.3b's launcher owns the
        // single live-codex spawn site (the umask-0077 + auth/profile binding). `launch()` is the
        // daemon-lifecycle `Creating→Starting` marker; the real status then derives from the live
        // stream via `push_signal` (safety #9), never the PTY.
        self.status = Session::Starting;
        self.status
    }

    fn stream_status(&self) -> NormalizedStatus {
        // the current derived status — advanced by `push_signal` over structured signals via
        // `derive_status`, NEVER from PTY output bytes (safety #9).
        self.status
    }

    fn intercept_mutation(&self) -> Option<MutationIntercept> {
        None // → 3.3c (the CAT-1 PreToolUse→Gateway INV-SEC-1 routing + --sandbox; named, not silent)
    }

    fn read_transcript(&self) -> Option<TranscriptRef> {
        // Codex writes its own rollout JSONL we read in place. The capability gate honors the shape so
        // a future no-transcript build surfaces None (never a fabricated ref). `hash` is an honest
        // empty placeholder — content-addressing the live-appended JSONL is a recovery follow-on
        // (the Claude precedent), never a fake `sha256:`.
        if !CODEX_CAPABILITIES.supports_transcript_read {
            return None;
        }
        Some(TranscriptRef {
            path: self.rollout_path.to_string_lossy().into_owned(),
            hash: String::new(), // → recovery/3.3 content-addressing
            is_in_place: true,
        })
    }

    fn telemetry_heartbeat(&self) -> Option<TelemetrySample> {
        None // → 3.3d (the telemetry emission path; the parsed `token_count` samples feed it)
    }

    fn resume(&self) -> ResumeResult {
        // A minimal non-live stub (the 4.1a Q4 uniform rule: `Replayed`, the neutral non-live default).
        // The real strategy is the daemon-internal `decide_resume` ladder; Codex resume (`codex resume`
        // / app-server `thread/resume`) wires its live source at 3.3b/c.
        ResumeResult {
            mode: ResumeMode::Replayed,
            replayed_event_count: 0,
        }
    }
}
