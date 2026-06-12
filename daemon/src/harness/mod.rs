//! The `HarnessAdapter` layer (ARCHITECTURE §9.1, LOCKED ADR-006).
//!
//! ONE trait both the Claude (3.2) and Codex (3.3) adapters implement, over the frozen
//! normalized return types in `shared/src/harness.rs` (TelemetrySample, MetricQuality,
//! TranscriptRef, HarnessCapabilities) and the §5.1 `Session` machine as `NormalizedStatus`.
//! The trait, `MutationIntercept` (carrying the non-serializable daemon-side `decision_sink`),
//! the per-harness mutation-coverage matrix, and `ResumeResult` are all **daemon-only** (not a
//! `shared/` wire contract — Q5): the UI degrades off `HarnessCapabilities`, not the matrix,
//! and the conformance suite and docs are the matrix's consumers. `ResumeResult`/survival is
//! daemon-internal here and freezes in `shared/` at Phase 4 (§8/§17) to avoid a P4 reshape.
//!
//! **Safety #9** — status is derived from SDK/app-server streams, NEVER scraped from the PTY
//! (`stream_status` returns the structured `NormalizedStatus`; the PTY is display-only).
//! **Safety #10** — the coverage matrix encodes the O-13 default-mode-only + no-background-subagent
//! posture as the CONTRACT; its enforcement is 3.2/3.3 behavior. This slice freezes the shape.
//!
//! 3.1 ships the trait + types + the matrix + a `FakeHarness` test double (proving the trait is
//! satisfiable + object-safe); the real adapters + their async drive loop land in 3.2/3.3.

use nexusops_shared::harness::{
    HarnessCapabilities, MetricQuality, NormalizedStatus, TelemetrySample, TranscriptRef,
};

// ---- mutation interception (the can_use_tool / app-server approval surface, §9.1) ------------

/// The verdict the daemon returns through a [`MutationIntercept`]'s sink — allow the agent's
/// tool-use or deny it (mapped from the Gateway's policy/approval decision in 3.2/3.3).
#[derive(Debug)]
pub enum MutationVerdict {
    Allow,
    Deny { reason: String },
}

/// A daemon-side callback the Gateway invokes once with the [`MutationVerdict`]. Non-serializable
/// (a live closure) → this and [`MutationIntercept`] live DAEMON-side, never in `shared/` (Q5).
pub type MutationDecisionSink = Box<dyn FnOnce(MutationVerdict) + Send>;

/// An intercepted agent mutation awaiting a decision (§9.1). Both Claude `can_use_tool` and Codex
/// `app-server` approval produce one: the requested `tool` + its `params`, plus the `decision_sink`
/// the daemon calls back with the verdict (after routing the intent through the Gateway, 3.2/3.3).
pub struct MutationIntercept {
    pub tool: String,
    pub params: serde_json::Value,
    pub decision_sink: MutationDecisionSink,
}

// ---- the per-harness mutation-coverage matrix (§9.1, binding; daemon-internal — Q5) ----------

/// The two MVP harnesses (§9.1 / O-1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Harness {
    Claude,
    Codex,
}

/// The tool-use categories the §9.1 coverage matrix classifies (the "Tool category" rows).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MutationChannel {
    /// direct bash / file-edit
    DirectToolUse,
    /// subagent (Task)
    Subagent,
    /// background subagent
    BackgroundSubagent,
    /// MCP tools (`mcp__*`)
    McpTool,
}

/// How reliably a (harness, channel) mutation is intercepted (§9.1). Encodes the table's cells
/// VERBATIM — incl. Claude's "default-mode-only" (O-13 #10) and "not guaranteed" qualifiers, so the
/// contract carries the real safety posture, not a flattened guaranteed/best-effort/unsupported.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MutationCoverage {
    /// reliably intercepted (Codex direct/mcp)
    Guaranteed,
    /// intercepted ONLY in `default` permission mode (Claude direct; O-13 #10)
    GuaranteedInDefaultMode,
    /// falls through to mode+callback, direct-only (Claude mcp)
    BestEffort,
    /// inherits parent mode; bypassed under acceptEdits/bypass (Claude subagent)
    NotGuaranteed,
    /// bypasses the callback — forbidden until fixed (Claude background subagent, bug #27203)
    Unsupported,
    /// the harness has no such concept (Codex subagent / background subagent)
    NotApplicable,
}

/// One cell of the §9.1 coverage matrix.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CoverageCell {
    pub harness: Harness,
    pub channel: MutationChannel,
    pub coverage: MutationCoverage,
}

/// The §9.1 per-harness mutation-coverage matrix, VERBATIM (binding; the §14 conformance suite
/// asserts per category per harness). The CONTRACT shape; its enforcement is 3.2/3.3 behavior.
pub const MUTATION_COVERAGE_MATRIX: &[CoverageCell] = &[
    // Claude (can_use_tool): default-mode-only direct; subagent not guaranteed; background
    // subagent bypassed (#27203); MCP best-effort (falls through to mode+callback, direct only).
    CoverageCell {
        harness: Harness::Claude,
        channel: MutationChannel::DirectToolUse,
        coverage: MutationCoverage::GuaranteedInDefaultMode,
    },
    CoverageCell {
        harness: Harness::Claude,
        channel: MutationChannel::Subagent,
        coverage: MutationCoverage::NotGuaranteed,
    },
    CoverageCell {
        harness: Harness::Claude,
        channel: MutationChannel::BackgroundSubagent,
        coverage: MutationCoverage::Unsupported,
    },
    CoverageCell {
        harness: Harness::Claude,
        channel: MutationChannel::McpTool,
        coverage: MutationCoverage::BestEffort,
    },
    // Codex (app-server approval): direct (commandExecution/applyPatch) + MCP (mcp_tool_call
    // elicitation) covered; no subagent concept → n/a.
    CoverageCell {
        harness: Harness::Codex,
        channel: MutationChannel::DirectToolUse,
        coverage: MutationCoverage::Guaranteed,
    },
    CoverageCell {
        harness: Harness::Codex,
        channel: MutationChannel::Subagent,
        coverage: MutationCoverage::NotApplicable,
    },
    CoverageCell {
        harness: Harness::Codex,
        channel: MutationChannel::BackgroundSubagent,
        coverage: MutationCoverage::NotApplicable,
    },
    CoverageCell {
        harness: Harness::Codex,
        channel: MutationChannel::McpTool,
        coverage: MutationCoverage::Guaranteed,
    },
];

/// The coverage for a (harness, channel) pair, per [`MUTATION_COVERAGE_MATRIX`] (None if absent).
pub fn coverage_of(harness: Harness, channel: MutationChannel) -> Option<MutationCoverage> {
    MUTATION_COVERAGE_MATRIX
        .iter()
        .find(|c| c.harness == harness && c.channel == channel)
        .map(|c| c.coverage)
}

// ---- ResumeResult (DAEMON-INTERNAL — survival/resume freezes in shared/ at Phase 4, §8/§17) --

/// The outcome of resuming a session (§9.1 `resume`). **Daemon-internal**, NOT a `shared/` frozen
/// type: resume/survival is a Phase-4 §8/§17 design (the ui's provisional already pins a `ResumeMode`
/// enum) — freezing a wire shape now would collide at P4. `resumed_live` = re-attached to a live
/// process; `false` = replayed/relaunched (the §11.4 "resumed-(live) vs replayed-(relaunched)" bit).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResumeResult {
    pub resumed_live: bool,
    pub replayed_event_count: u64,
}

// ---- the HarnessAdapter trait (DAEMON-INTERNAL + UNFROZEN — only the data types are §2.5-seam) -

/// The one adapter contract both the Claude (3.2) and Codex (3.3) harnesses implement, over the
/// frozen `shared/` normalized types. **Object-safe + `Send + Sync`** (so `Box<dyn HarnessAdapter>`),
/// **synchronous** — mirroring the established `ActionExecutor`/`PolicyEngine`/`PreconditionOracle`
/// `Box<dyn>` seams (the async drive I/O lives in the runtime/tasks, LESSON §9; no speculative
/// async-trait dep). The trait + signatures are daemon-internal + UNFROZEN — 3.2/3.3 reshape the
/// drive loop freely; only the normalized DATA types are the §2.5-seam freeze. **Safety #9** —
/// `stream_status` returns the structured `NormalizedStatus`, NEVER PTY-scraped (PTY is display-only).
pub trait HarnessAdapter: Send + Sync {
    /// The harness's capability matrix (drives per-capability UI degradation, §9.1/§11.4).
    fn capabilities(&self) -> HarnessCapabilities;
    /// Launch the agent process/session → its initial normalized status.
    fn launch(&self) -> NormalizedStatus;
    /// The current normalized status, derived from the SDK/app-server stream (safety #9).
    fn stream_status(&self) -> NormalizedStatus;
    /// A pending mutation interception (the can_use_tool / app-server approval surface), if any.
    fn intercept_mutation(&self) -> Option<MutationIntercept>;
    /// A reference to the harness transcript (None when `supports_transcript_read=false`).
    fn read_transcript(&self) -> Option<TranscriptRef>;
    /// The latest telemetry sample (None when `supports_usage_metadata=false`).
    fn telemetry_heartbeat(&self) -> Option<TelemetrySample>;
    /// Resume an existing session (resume-or-replay; §8/§17).
    fn resume(&self) -> ResumeResult;
}

// ---- FakeHarness — the §14 test double (proves the trait is satisfiable + object-safe) --------

/// A capability-respecting test double for the [`HarnessAdapter`] trait (the real adapters land
/// 3.2/3.3). It surfaces capability-gated degradation HONESTLY — an unsupported capability returns
/// `None`, never a fabricated value (§9.1/§11.4) — so the conformance scaffold exercises the contract.
pub struct FakeHarness {
    caps: HarnessCapabilities,
}

impl FakeHarness {
    pub fn new(caps: HarnessCapabilities) -> Self {
        Self { caps }
    }
}

impl HarnessAdapter for FakeHarness {
    fn capabilities(&self) -> HarnessCapabilities {
        self.caps.clone()
    }

    fn launch(&self) -> NormalizedStatus {
        NormalizedStatus::Starting
    }

    fn stream_status(&self) -> NormalizedStatus {
        NormalizedStatus::Active
    }

    fn intercept_mutation(&self) -> Option<MutationIntercept> {
        None // no pending interception in the bare double; real interception is 3.2/3.3
    }

    fn read_transcript(&self) -> Option<TranscriptRef> {
        // an unsupported capability is surfaced as None, NEVER a fabricated TranscriptRef (§9.1).
        if self.caps.supports_transcript_read {
            Some(TranscriptRef {
                path: "/fake/transcript.jsonl".to_string(),
                hash: "sha256:fake".to_string(),
                is_in_place: true,
            })
        } else {
            None
        }
    }

    fn telemetry_heartbeat(&self) -> Option<TelemetrySample> {
        if !self.caps.supports_usage_metadata {
            return None;
        }
        // §11.4: context-% is None ("unknown") when the harness can't report it (e.g. Codex),
        // NEVER a faked 0%. metric_quality degrades to Estimated when the context gauge is missing.
        let context_pct = if self.caps.supports_context_metadata {
            Some(50.0)
        } else {
            None
        };
        let metric_quality = if context_pct.is_some() {
            MetricQuality::Exact
        } else {
            MetricQuality::Estimated
        };
        Some(TelemetrySample {
            tokens_in: 100,
            tokens_out: 20,
            context_pct,
            cost_estimate: 0.01,
            metric_quality,
        })
    }

    fn resume(&self) -> ResumeResult {
        ResumeResult {
            resumed_live: true,
            replayed_event_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexusops_shared::harness::{HarnessCapabilities, MetricQuality, NormalizedStatus};

    // a fully-capable FakeHarness (every capability true) for the satisfiability test.
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

    // ---- 3.1 L1 RED #8 — the per-harness coverage matrix == §9.1 table verbatim ----

    #[test]
    fn test_coverage_matrix_matches_spec() {
        // spec(§9.1) — the binding coverage matrix the §14 conformance suite asserts. O-13 (safety #10):
        // Claude is default-mode-only + background-subagents forbidden (#27203). Codex has no subagent
        // concept (n/a). The matrix is the CONTRACT; 3.2/3.3 enforce it.
        use Harness::*;
        use MutationChannel::*;
        use MutationCoverage::*;

        // exactly the 8 cells (2 harnesses × 4 tool categories) — no more, no fewer.
        assert_eq!(
            MUTATION_COVERAGE_MATRIX.len(),
            8,
            "2 harnesses × 4 categories"
        );

        // Claude (can_use_tool): intercepted in DEFAULT mode only; subagent NOT guaranteed;
        // background subagent bypassed (#27203); MCP falls through to mode+callback (best-effort).
        assert_eq!(
            coverage_of(Claude, DirectToolUse),
            Some(GuaranteedInDefaultMode)
        );
        assert_eq!(coverage_of(Claude, Subagent), Some(NotGuaranteed));
        assert_eq!(coverage_of(Claude, BackgroundSubagent), Some(Unsupported));
        assert_eq!(coverage_of(Claude, McpTool), Some(BestEffort));

        // Codex (app-server approval): direct + MCP covered; no subagent concept → n/a.
        assert_eq!(coverage_of(Codex, DirectToolUse), Some(Guaranteed));
        assert_eq!(coverage_of(Codex, Subagent), Some(NotApplicable));
        assert_eq!(coverage_of(Codex, BackgroundSubagent), Some(NotApplicable));
        assert_eq!(coverage_of(Codex, McpTool), Some(Guaranteed));
    }

    // ---- 3.1 L1 RED #9 — FakeHarness satisfies the trait + object-safety ----

    #[test]
    fn test_fake_harness_satisfies_trait() {
        // spec(§9.1) — the trait must be object-safe (`Box<dyn HarnessAdapter>`) and satisfiable; each
        // method returns its normalized type. The real adapters (3.2/3.3) bind to this same surface.
        let adapter: Box<dyn HarnessAdapter> = Box::new(FakeHarness::new(full_caps()));

        // capabilities() → the frozen HarnessCapabilities
        assert!(adapter.capabilities().supports_terminal);
        // launch()/stream_status() → the §5.1 NormalizedStatus (never PTY-scraped, safety #9)
        let _launch: NormalizedStatus = adapter.launch();
        let _status: NormalizedStatus = adapter.stream_status();
        // read_transcript()/telemetry_heartbeat() → their normalized types (Some when supported)
        let _t: Option<nexusops_shared::harness::TranscriptRef> = adapter.read_transcript();
        assert!(adapter.read_transcript().is_some(), "transcript supported");
        let sample = adapter
            .telemetry_heartbeat()
            .expect("usage metadata supported");
        assert_eq!(sample.metric_quality, MetricQuality::Exact);
        // resume() → ResumeResult (DAEMON-INTERNAL — not a shared/ frozen type; survival freezes P4)
        let _r: ResumeResult = adapter.resume();
        // intercept_mutation() → Option<MutationIntercept> (None when nothing pending)
        assert!(
            adapter.intercept_mutation().is_none(),
            "no pending interception"
        );

        // a MutationIntercept constructs with its daemon-side decision_sink (the non-serializable
        // callback the Gateway invokes with the verdict — proves the sink type works + is callable).
        let intercept = MutationIntercept {
            tool: "Bash".to_string(),
            params: serde_json::json!({ "command": "ls" }),
            decision_sink: Box::new(|v| assert!(matches!(v, MutationVerdict::Allow))),
        };
        assert_eq!(intercept.tool, "Bash");
        (intercept.decision_sink)(MutationVerdict::Allow);
    }

    // ---- 3.1 L1 RED #9b — capability-gated degradation surfaced honestly (not faked) ----

    #[test]
    fn test_capabilities_gate_degradation_not_faked() {
        // spec(§9.1) — §11.4: Codex `supportsContextMetadata=false` → context-% is None ("unknown"),
        // NEVER a faked number/0%; an unsupported capability is surfaced (None), not fabricated.
        let mut caps = full_caps();
        caps.supports_context_metadata = false; // the Codex edge
        caps.supports_transcript_read = false; // an unsupported capability
        let codex_like: Box<dyn HarnessAdapter> = Box::new(FakeHarness::new(caps));

        let sample = codex_like
            .telemetry_heartbeat()
            .expect("usage metadata still supported");
        assert_eq!(
            sample.context_pct, None,
            "context-% is None (unknown) when supports_context_metadata=false — never faked"
        );
        assert_eq!(
            codex_like.read_transcript(),
            None,
            "an unsupported capability is surfaced as None, not a fabricated TranscriptRef"
        );
    }
}
