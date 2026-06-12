//! P7.1 — Linear issue-state derivation (edges-013). The deterministic foundation of the Linear read
//! vertical (the analog of edges-004's GitHub PR-status derivation). A Linear issue is an
//! **external_task** (§5.1 R-8): its status derives from the issue's `WorkflowState.type` — the 6-value
//! closed set Linear's API exposes — NOT the team's custom state NAME. Two stages, mirroring the GitHub
//! `derive_pull_request_status` pattern:
//!   - `parse_linear_state_type` decodes the raw GraphQL `WorkflowState.type` string → the daemon enum
//!     (conservative floor + case-insensitive, the edges-008 convention);
//!   - `derive_task_status_from_linear` maps the state-type → the frozen §5.1 `Task` status (the
//!     external-task subset), an **exhaustive** match so a new state-type forces a reconcile (LESSON-2).
//!
//! The Linear → `Task` mapping is DAEMON-DEFINED (the architecture lists the Task machine + the R-8
//! subset rule but does not pin the Linear mapping — recorded as a §5.1/§9 arch-note, like the §7.2
//! PR-derivation precedence). Pure — no network (the Linear read client that fetches the issue is the
//! next slice; review-flavored Task states like NeedsReview/PrOpened come from a PR, not an issue's
//! state-type, so they are correctly unreachable here).

use nexusops_shared::status::Task;

/// Linear's `WorkflowState.type` — the closed 6-value lifecycle set (confirmed against the Linear
/// GraphQL API: `state: { type: { eq: "started" } }` etc.). The daemon-internal decode of the raw
/// API string; NOT the `agentSession.plan` status enum (a different, unrelated field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinearStateType {
    Triage,
    /// The conservative parked floor (also `parse_linear_state_type`'s unknown default) — matches the
    /// `Mergeability::Unknown` / `ReviewDecision::None` floor-as-`#[default]` convention so the gated
    /// Linear read client can put this in a `#[derive(Default)]` signals struct.
    #[default]
    Backlog,
    Unstarted,
    Started,
    Completed,
    Canceled,
}

/// Decode Linear's GraphQL `WorkflowState.type` string → `LinearStateType` (case-insensitive, the
/// edges-008 convention). None / null / unrecognized → `Backlog` — the conservative parked floor:
/// Linear's 6 types are a stable closed set, so an unknown is a parse-miss API anomaly that parks
/// (never fabricating human-attention from a miss).
pub fn parse_linear_state_type(state_type: Option<&str>) -> LinearStateType {
    match state_type.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("triage") => LinearStateType::Triage,
        Some("backlog") => LinearStateType::Backlog,
        Some("unstarted") => LinearStateType::Unstarted,
        Some("started") => LinearStateType::Started,
        Some("completed") => LinearStateType::Completed,
        Some("canceled") => LinearStateType::Canceled,
        _ => LinearStateType::Backlog,
    }
}

/// Map a Linear `WorkflowState.type` → the frozen §5.1 `Task` status (the external-task subset). The
/// daemon-defined mapping (§5.1/§9 arch-note): `Triage→NeedsClarification` (untriaged → needs a human
/// look), `Backlog→Queued`, `Unstarted→Ready` (todo), `Started→InProgress`, `Completed→Done` (work
/// complete — `Done` is non-terminal in the Task machine, so a completed issue can reopen; the terminal
/// set is Merged/Closed/Abandoned, which are PR/issue-close outcomes), `Canceled→Abandoned` (terminal
/// won't-do). **Exhaustive** (no `_`) — a new `LinearStateType` variant forces a reconcile here (LESSON-2).
pub fn derive_task_status_from_linear(state_type: LinearStateType) -> Task {
    match state_type {
        LinearStateType::Triage => Task::NeedsClarification,
        LinearStateType::Backlog => Task::Queued,
        LinearStateType::Unstarted => Task::Ready,
        LinearStateType::Started => Task::InProgress,
        LinearStateType::Completed => Task::Done,
        LinearStateType::Canceled => Task::Abandoned,
    }
}
