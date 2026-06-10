//! nexusops-shared — the native Rust contract authority (Option A, ARCHITECTURE §5.0).
//!
//! Frozen in Phase 0.5 (OQ-DATA-SPIKE-5): the status state machines (§5.1), the
//! shared IDs + ULID-prefix format (§5.2), the actor enum (§7.1/R-2), and the
//! desktop-addendum objects (§5.3). `schemars` emits the versioned JSON Schema
//! consumed (generated) by the TS UI (Zod) and the Python Brain (Pydantic).

pub mod actor;
pub mod event_envelope;
pub mod events;
pub mod ids;
pub mod ipc;
pub mod objects;
pub mod schema;
pub mod status;

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
pub const CONTRACT_VERSION: &str = "0.14.0";

/// **ExecutionProfile's status machine (the 10th §5.1 machine) is intentionally
/// HELD, not frozen, in 0.5.** Its runtime states (`rate_limited`/`auth_expired`,
/// plus a possible SDK-credit-exhaustion value) are the one surface the cat-4
/// SDK-vs-PTY decision and the ≥2026-06-15 credit-pool drain (guardrail 1) could
/// reshape. Re-frozen in **0.5b** once cat-4 resolves. This marker makes the
/// absence deliberate, not forgotten.
pub const EXECUTION_PROFILE_STATUS_HELD: &str =
    "ExecutionProfile status machine held for 0.5b pending cat-4 SDK-vs-PTY + \
     the >=2026-06-15 credit-pool drain (guardrail 1)";
