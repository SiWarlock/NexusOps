//! The Action Gateway — the single, audited mutator (INV-SEC-1 chokepoint, §6/§6.1/§6.2/§15).
//!
//! Every state mutation flows through here as a typed, risk-classified, approved `ActionRequest`
//! recorded as an immutable event (forbidden #2). The foundation (L1) is the durable registry
//! schema (`action_requests`/`approvals`, MIGRATION_7) + the **R-9 transition guards** for the
//! frozen §5.1 ActionRequest(15)/Approval(10) machines — an illegal edge is a typed error, never
//! silently applied. The `submit_action`/`approve`/`deny`/`preview_action` pipeline + the
//! `ActionExecution*` event family + the write-actor integration build on it (L2/L3).

pub mod approval;
pub mod request;
