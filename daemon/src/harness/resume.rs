//! P4.1a commit 2 — the pure resume/replay/reattach DECISION logic (§8 survival ladder).
//!
//! `decide_resume(&ResumeInputs) -> ResumeResult` is a pure, TOTAL classifier over availability
//! inputs — harness-agnostic (Claude + Codex differ only in how the caller populates the inputs). A
//! member of the LESSONS §9/§12/§25 pure-classifier family. The failed-resume-ATTEMPT fallback is the
//! CALLER's loop (4.1b re-calls with `has_resume_handle=false`), NOT this function — keeping it a
//! total, side-effect-free classifier (the daemon-internal `ResumeInputs` is its input record, not a wire
//! type; only `ResumeResult` is the frozen §9.1 return). The production caller (the bootstrap restart
//! path: rebuild → reclaim leases → decide_resume per session → emit the resumed-vs-replayed signal)
//! lands in 4.1b (the 4.0a "mechanism built test-first, driven next slice" precedent, LESSON §28).

use nexusops_shared::harness::{ResumeMode, ResumeResult};

/// The availability inputs the §8 survival decision keys off — harness-agnostic (the caller populates
/// them per harness: broker reattach outcome, the harness `supports_resume` capability, a resume-handle
/// presence, serialized-scrollback presence + its event count). Daemon-internal (not a wire type).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResumeInputs {
    /// the detachable-terminal broker still holds the live in-flight PTY (B2-strict reattach available)
    pub broker_has_live_session: bool,
    /// the harness supports native resume (`--resume` / `thread/resume`)
    pub supports_resume: bool,
    /// a resume handle exists for this session (the transcript / thread id to resume from)
    pub has_resume_handle: bool,
    /// serialized scrollback exists to replay
    pub has_scrollback: bool,
    /// the scrollback event count (carried on the `Replayed` result; ignored on the other rungs)
    pub replayed_event_count: u64,
}

/// The §8 survival ladder, in STRICT precedence (a TOTAL function — every input maps to exactly one
/// mode): broker-live → `ReattachedLive` · `supports_resume && has_resume_handle` → `Resumed` ·
/// `has_scrollback` → `Replayed` (carries the count) · else → `Relaunched`. `replayed_event_count`
/// is non-zero ONLY on the `Replayed` rung (the §11.4 "resumed-(live) vs replayed-(relaunched)"
/// semantics). A failed resume ATTEMPT is the caller's concern (it re-calls with
/// `has_resume_handle=false` → the ladder falls through), not modelled here.
pub fn decide_resume(inputs: &ResumeInputs) -> ResumeResult {
    if inputs.broker_has_live_session {
        ResumeResult {
            mode: ResumeMode::ReattachedLive,
            replayed_event_count: 0,
        }
    } else if inputs.supports_resume && inputs.has_resume_handle {
        ResumeResult {
            mode: ResumeMode::Resumed,
            replayed_event_count: 0,
        }
    } else if inputs.has_scrollback {
        ResumeResult {
            mode: ResumeMode::Replayed,
            replayed_event_count: inputs.replayed_event_count,
        }
    } else {
        ResumeResult {
            mode: ResumeMode::Relaunched,
            replayed_event_count: 0,
        }
    }
}
