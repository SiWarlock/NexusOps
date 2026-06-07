//! nexusops-shared — the native Rust contract authority (Option A, ARCHITECTURE §5.0).
//!
//! Frozen in Phase 0.5 (OQ-DATA-SPIKE-5): the status state machines (§5.1), the
//! shared IDs + ULID-prefix format (§5.2), the actor enum (§7.1/R-2), and the
//! desktop-addendum objects (§5.3). `schemars` emits the versioned JSON Schema
//! consumed (generated) by the TS UI (Zod) and the Python Brain (Pydantic).

pub mod actor;
pub mod ids;
pub mod objects;
pub mod schema;
pub mod status;

/// The frozen-contract version, stamped into the emitted JSON Schema and asserted
/// to agree across Rust / schema / Zod / Pydantic (the §5.0 propagation contract).
pub const CONTRACT_VERSION: &str = "0.5.0";

/// **ExecutionProfile's status machine (the 10th §5.1 machine) is intentionally
/// HELD, not frozen, in 0.5.** Its runtime states (`rate_limited`/`auth_expired`,
/// plus a possible SDK-credit-exhaustion value) are the one surface the cat-4
/// SDK-vs-PTY decision and the ≥2026-06-15 credit-pool drain (guardrail 1) could
/// reshape. Re-frozen in **0.5b** once cat-4 resolves. This marker makes the
/// absence deliberate, not forgotten.
pub const EXECUTION_PROFILE_STATUS_HELD: &str =
    "ExecutionProfile status machine held for 0.5b pending cat-4 SDK-vs-PTY + \
     the >=2026-06-15 credit-pool drain (guardrail 1)";
