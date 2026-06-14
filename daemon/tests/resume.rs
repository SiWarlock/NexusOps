//! P4.1a commit 2 (brief 057) — the pure resume/replay/reattach DECISION logic (`harness/resume.rs`).
//!
//! `decide_resume(&ResumeInputs) -> ResumeResult` is a pure, TOTAL classifier over availability inputs
//! (the §8 survival ladder), harness-agnostic (Claude + Codex differ only in how the caller populates
//! the inputs — the function is the same). The LESSONS §9/§12/§25 pure-classifier family applied to
//! survival; the failed-resume-attempt fallback is the CALLER's loop, not this function (Q3). The
//! production caller (the bootstrap restart path) lands in 4.1b (the 4.0a "mechanism built test-first,
//! driven by its production caller next slice" precedent, LESSON §28).

use nexusops_shared::harness::ResumeMode;
use nexusopsd::harness::resume::{decide_resume, ResumeInputs};

// ---- RED #6 — broker-live reattach is the TOP of the ladder (§8 B2-strict EXTENSION) -------------

#[test]
fn test_broker_live_session_reattaches() {
    // spec(§8) — B2-strict: a surviving in-flight turn (the detachable-terminal broker still holds the
    // live PTY) reattaches → ReattachedLive, count 0. Precedence over EVERY other available option.
    let r = decide_resume(&ResumeInputs {
        broker_has_live_session: true,
        // everything else also available — reattach-live still wins (strict precedence).
        supports_resume: true,
        has_resume_handle: true,
        has_scrollback: true,
        replayed_event_count: 99,
    });
    assert_eq!(r.mode, ResumeMode::ReattachedLive);
    assert_eq!(r.replayed_event_count, 0, "a live reattach replays nothing");
}

// ---- RED #7 — native resume when supported + a handle (§8 --resume/thread/resume) ----------------

#[test]
fn test_native_resume_when_supported_with_handle() {
    // spec(§8) — no broker; the harness supports --resume/thread/resume AND a resume handle exists →
    // Resumed (a NEW process resumed from the transcript), count 0.
    let r = decide_resume(&ResumeInputs {
        supports_resume: true,
        has_resume_handle: true,
        ..Default::default()
    });
    assert_eq!(r.mode, ResumeMode::Resumed);
    assert_eq!(r.replayed_event_count, 0);
}

// ---- RED #8 — replay when not resume-able but scrollback exists (§8 "else …replay+relaunch") ------

#[test]
fn test_replay_when_no_resume_but_scrollback() {
    // spec(§8) — not resume-able but serialized scrollback exists → Replayed, count == the scrollback
    // event count (the ONLY ladder path that carries a non-zero count).
    let r = decide_resume(&ResumeInputs {
        has_scrollback: true,
        replayed_event_count: 17,
        ..Default::default()
    });
    assert_eq!(r.mode, ResumeMode::Replayed);
    assert_eq!(r.replayed_event_count, 17);
}

// ---- RED #9 — relaunch when nothing available (§8/§17 "restart session" affordance) --------------

#[test]
fn test_relaunch_when_nothing_available() {
    // spec(§8/§17) — nothing available → Relaunched (fresh; the "restart session" affordance tail),
    // count 0.
    let r = decide_resume(&ResumeInputs::default());
    assert_eq!(r.mode, ResumeMode::Relaunched);
    assert_eq!(r.replayed_event_count, 0);
}

// ---- RED #10 — strict ladder ordering: resume beats scrollback -----------------------------------

#[test]
fn test_precedence_resume_over_scrollback() {
    // spec(§8) — strict ladder ordering: resume-able AND scrollback present → Resumed wins (resume is
    // a higher rung than replay); count 0, NOT the scrollback count.
    let r = decide_resume(&ResumeInputs {
        supports_resume: true,
        has_resume_handle: true,
        has_scrollback: true,
        replayed_event_count: 50,
        ..Default::default()
    });
    assert_eq!(r.mode, ResumeMode::Resumed);
    assert_eq!(
        r.replayed_event_count, 0,
        "resume replays nothing; the scrollback count is ignored when resume wins"
    );
}

// ---- RED #11 — the Resumed rung is a CONJUNCTION: capability WITHOUT a handle falls through -------

#[test]
fn test_resume_requires_handle_not_just_capability() {
    // spec(§8) — the Resumed rung requires `supports_resume` AND `has_resume_handle`. A capability
    // without a handle does NOT resume: supports_resume=true · has_resume_handle=false · scrollback
    // · !broker → Replayed (count carried), NOT Resumed. Pins that the handle is required and the rung
    // falls THROUGH to scrollback when it's absent (guards a bug that checks only the capability).
    let r = decide_resume(&ResumeInputs {
        supports_resume: true,
        has_resume_handle: false,
        has_scrollback: true,
        replayed_event_count: 9,
        ..Default::default()
    });
    assert_eq!(
        r.mode,
        ResumeMode::Replayed,
        "a capability without a handle does not resume"
    );
    assert_eq!(
        r.replayed_event_count, 9,
        "falls through to the scrollback rung, carrying its count"
    );
}

// ---- RED #12 — harness-agnostic: same function for Claude + Codex shapes -------------------------

#[test]
fn test_harness_agnostic_both_shapes() {
    // spec(§8) — the function is the SAME for both harnesses; only the caller-populated inputs differ.
    // A Codex-shaped input (supports_resume=true, handle present) and a Claude-shaped input both reach
    // Resumed; a no-resume-CAPABILITY shape falls through the resume rung (handle present is moot).
    let codex = decide_resume(&ResumeInputs {
        supports_resume: true,
        has_resume_handle: true,
        ..Default::default()
    });
    let claude = decide_resume(&ResumeInputs {
        supports_resume: true,
        has_resume_handle: true,
        has_scrollback: true,
        ..Default::default()
    });
    assert_eq!(codex.mode, ResumeMode::Resumed);
    assert_eq!(
        claude.mode,
        ResumeMode::Resumed,
        "resume wins over the scrollback it also has"
    );
    // supports_resume=false → NOT Resumed even with a handle; falls through to the scrollback rung.
    let no_capability = decide_resume(&ResumeInputs {
        supports_resume: false,
        has_resume_handle: true,
        has_scrollback: true,
        replayed_event_count: 3,
        ..Default::default()
    });
    assert_eq!(
        no_capability.mode,
        ResumeMode::Replayed,
        "supports_resume=false falls through to replay (capability gates the rung)"
    );
    assert_eq!(no_capability.replayed_event_count, 3);
}
