//! Projection table names for the full-rebuild truncation path.
//!
//! The migration DDL itself lives in `eventstore/schema.rs` (co-located with M1/M2
//! so the migration registry doesn't import `projections/` — layer-direction). This
//! is the set a full rebuild clears before replaying the log. The raw `events` table
//! is deliberately ABSENT: a rebuild must never touch the spine (§7.2 — "projection
//! corruption must not corrupt raw events").

/// Every derived table a full rebuild truncates: `object_refs` + the FTS index + the
/// 11 physical `proj_*` tables (ProjectGraph = node + edge). All are FK children of,
/// or independent from, `events`, so truncating them in any order is FK-safe.
pub(crate) const REBUILD_TABLES: &[&str] = &[
    "object_refs",
    "fts_events",
    "proj_project_activity",
    "proj_session",
    "proj_approval_queue",
    "proj_worktree",
    "proj_pull_request",
    "proj_plan_progress",
    "proj_graph_node",
    "proj_graph_edge",
    "proj_audit_trail",
    "proj_agent_team",
    "proj_usage_ledger",
];
