//! nexusops-shared — the native Rust contract authority (Option A, ARCHITECTURE §5.0).
//!
//! Frozen in Phase 0.5 (OQ-DATA-SPIKE-5): the status state machines (§5.1), the
//! shared IDs + ULID-prefix format (§5.2), the actor enum (§7.1/R-2), and the
//! desktop-addendum objects (§5.3). `schemars` emits the versioned JSON Schema
//! consumed (generated) by the TS UI (Zod) and the Python Brain (Pydantic).

pub mod actions;
pub mod actor;
pub mod catalog;
pub mod event_envelope;
pub mod events;
pub mod gateway_ids;
pub mod harness;
pub mod ids;
pub mod ipc;
pub mod objects;
pub mod schema;
pub mod status;
pub mod time;

/// The frozen-contract version, stamped into the emitted JSON Schema and asserted
/// to agree across Rust / schema / Zod / Pydantic (the §5.0 propagation contract).
/// 0.9.0 added the IPC `GatewayPort` wire contract (§6.4: HelloFrame/HelloAck/
/// VersionSkewError/Capabilities/WireError + the IpcErrorCode + ProjectionName enums,
/// 1.5 L2). 0.10.0 (1.5 L3) adds the §6.1 RPC method envelopes (RpcRequest/RpcResponse/
/// GetProjectionParams/ProjectionScope) + the `protocol_error` code (the lead-ratified §6.4
/// gap resolution). 0.11.0 (1.5 L4) adds the frame-type multiplexing envelope (ServerFrame) +
/// subscribe streaming (ProjectionDelta/DeltaKind/SubscribeParams) — additive.
/// 0.12.0 (1.6a L3) adds the Device/LocalRunner registration event payloads
/// (DeviceRegistered/LocalRunnerRegistered + the DeviceId/LocalRunnerId newtypes,
/// §5.3/§16 bootstrap self-registration) — additive EventTypeRegistry rows.
/// 0.13.0 (1.6c L2) adds the §17 AuditIntegrityViolation event payload (Option C —
/// the loud, consumer-visible record emitted when startup replay quarantines a row).
/// 0.14.0 (1.7 L2) adds the §15 SensitiveOutputRedacted event payload — the
/// redaction "can't safely redact → divert the event + record this instead" net.
/// 0.15.0 (2.1a) freezes the §6.2 Gateway core data model (`shared/src/actions.rs`): the 10
/// ActionRequest/ActionPlan/Approval/ActionResult-family models + 9 new enums (RequesterType,
/// RiskLevel[bounded-int], ApprovalScope/Mode, PolicyDecisionStatus, ActionResultStatus,
/// ResourceType/EvidenceType/EvidenceConfidence) + the non-cross-product gateway IDs
/// (ApprovalId/ActionPlanId, off GatewayObjectKind) + the Timestamp newtype — additive, no
/// frozen type reshaped. Pure contract freeze; the Gateway pipeline/rows/events are 2.1b.
/// 0.16.0 (2.1b) adds the Gateway `ActionExecution*` EventTypeRegistry payloads (ActionRequested,
/// ActionApprovalRequested, ActionApproved, ActionDenied, ActionExpired, ActionStarted,
/// ActionSucceeded, ActionFailed) + the §6.1 `ActionAck` submit-result wire type — additive (the
/// chokepoint's authoritative event family + the ui/Brain intent-seam ack).
/// 0.17.0 (2.1c) adds the §6.1 `submit_action_plan` result wire types `PlanAck`/`PlanStepAck` (the
/// O-3 bundled-plan ack: the plan handle + per-step minted ids/status) — additive. The plan grouping
/// itself is daemon-internal (`action_plans` table + `plan_id` FK), NOT a new `shared/` type / event.
/// 0.18.0 (2.2) adds the §6.3 `ActionTypeCatalog` contract (`ActionTypeCatalogEntry` + the
/// `PreviewClass`/`ExecutorKind`/`IdempotencyFormula` enums, `shared/src/catalog.rs`) + the
/// `PolicyDecision` extension (`required_approvals`/`constraints`/`safer_alt`) — additive (the
/// catalog-driven policy engine's authoritative per-type risk + the richer decision payload).
/// 0.19.0 (2.4 L1) adds the §17 failure-mode contract: the `ActionPartiallySucceeded` event (the
/// side-effect-applied-but-terminal-event-unwritable record) + the structured `ActionError`
/// taxonomy now carried on `ActionFailed` (replacing the 2.1b free-string `error`) — additive.
/// 0.20.0 (3.1) freezes the §9.1 HarnessAdapter normalized return types (`shared/src/harness.rs`):
/// `TelemetrySample` + `MetricQuality` + `TranscriptRef` + `HarnessCapabilities` (10 PRD-HARN-5
/// fields) + the §7.1 `TelemetrySampled` telemetry-observation event — additive, no frozen type
/// reshaped. `NormalizedStatus` re-exports the frozen §5.1 `Session` (not a new type/$def). The
/// `HarnessAdapter` trait + `MutationIntercept` + the mutation-coverage matrix + `ResumeResult` are
/// DAEMON-INTERNAL (not a `shared/` wire contract); `ResumeResult`/survival freezes in Phase 4 (§8/§17).
/// 0.21.0 (3.4) freezes the §6.4 Terminal Channel wire contract (`shared/src/ipc.rs`): the 3 terminal
/// frames (`TerminalOutputFrame`/`TerminalInputFrame`/`TerminalControlFrame`) + the `TerminalControlKind`
/// (pause|resume) flow-control enum + the §7.1 `TerminalProcessExited` PTY-death observation event.
/// `ServerFrame` gains the `TerminalOutput` variant — the reserved Terminal slot filled with the
/// JSON-base64 MVP (raw PTY bytes base64 over the unchanged codec, LESSON §7); the binary fast-path is
/// a deferred 3.5 decision (additive — a future variant + bump, not a reshape). Additive, no frozen
/// type reshaped (§5.0). `terminal_id` = an opaque daemon runtime handle (`String` wire), NOT a 23rd
/// `IdKind`; the PTY host + backpressure pump are DAEMON-INTERNAL (`daemon/src/terminal/`).
pub const CONTRACT_VERSION: &str = "0.21.0";

/// **ExecutionProfile's status machine (the 10th §5.1 machine) is intentionally
/// HELD, not frozen, in 0.5.** Its runtime states (`rate_limited`/`auth_expired`,
/// plus a possible SDK-credit-exhaustion value) are the one surface the cat-4
/// SDK-vs-PTY decision and the ≥2026-06-15 credit-pool drain (guardrail 1) could
/// reshape. Re-frozen in **0.5b** once cat-4 resolves. This marker makes the
/// absence deliberate, not forgotten.
pub const EXECUTION_PROFILE_STATUS_HELD: &str =
    "ExecutionProfile status machine held for 0.5b pending cat-4 SDK-vs-PTY + \
     the >=2026-06-15 credit-pool drain (guardrail 1)";
