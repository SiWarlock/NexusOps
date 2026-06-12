//! P7.1 — Linear issue-state derivation (edges-013). The deterministic foundation of the Linear read
//! vertical (analogous to edges-004's GitHub PR-status derivation). Maps Linear's `WorkflowState.type`
//! (the 6-value closed set: triage/backlog/unstarted/started/completed/canceled) → the frozen §5.1
//! `Task` status (the external-task subset, §5.1 R-8). Pure — no network (the Linear read client that
//! fetches the issue is the next slice).
//!
//! Linear's `WorkflowState.type` is confirmed against the GraphQL API (`type: { eq: "started" }` etc.,
//! lowercase). The Linear→Task mapping is daemon-defined (recorded as a §5.1/§9 arch-note, like the
//! §7.2 PR-derivation precedence).

use nexusops_shared::status::Task;
use nexusopsd::integrations::linear::{
    derive_task_status_from_linear, parse_linear_state_type, LinearStateType,
};

#[test]
fn parse_linear_state_type_known_values() {
    // spec(§9): each of Linear's 6 WorkflowState.type strings decodes to its variant.
    assert_eq!(
        parse_linear_state_type(Some("triage")),
        LinearStateType::Triage
    );
    assert_eq!(
        parse_linear_state_type(Some("backlog")),
        LinearStateType::Backlog
    );
    assert_eq!(
        parse_linear_state_type(Some("unstarted")),
        LinearStateType::Unstarted
    );
    assert_eq!(
        parse_linear_state_type(Some("started")),
        LinearStateType::Started
    );
    assert_eq!(
        parse_linear_state_type(Some("completed")),
        LinearStateType::Completed
    );
    assert_eq!(
        parse_linear_state_type(Some("canceled")),
        LinearStateType::Canceled
    );
}

#[test]
fn parse_linear_state_type_none_and_unknown() {
    // spec(§9): None/null/unrecognized/empty → the conservative parked floor Backlog (edges-008 — an
    // API anomaly parks, it does not fabricate human-attention).
    assert_eq!(parse_linear_state_type(None), LinearStateType::Backlog);
    assert_eq!(
        parse_linear_state_type(Some("FOO")),
        LinearStateType::Backlog
    );
    assert_eq!(parse_linear_state_type(Some("")), LinearStateType::Backlog);
}

#[test]
fn parse_linear_state_type_case_insensitive() {
    // spec(§9): case-folded before match (edges-008/009 convention).
    assert_eq!(
        parse_linear_state_type(Some("Started")),
        LinearStateType::Started
    );
    assert_eq!(
        parse_linear_state_type(Some("BACKLOG")),
        LinearStateType::Backlog
    );
    // the doc-comment trim claim: surrounding whitespace + mixed case both fold.
    assert_eq!(
        parse_linear_state_type(Some("  STARTED  ")),
        LinearStateType::Started
    );
}

#[test]
fn derive_task_status_all_state_types() {
    // spec(§5.1 R-8): each Linear state-type → its mapped frozen Task status (the daemon-defined table,
    // exhaustive — a new LinearStateType variant forces a reconcile here, LESSON-2).
    assert_eq!(
        derive_task_status_from_linear(LinearStateType::Triage),
        Task::NeedsClarification
    );
    assert_eq!(
        derive_task_status_from_linear(LinearStateType::Backlog),
        Task::Queued
    );
    assert_eq!(
        derive_task_status_from_linear(LinearStateType::Unstarted),
        Task::Ready
    );
    assert_eq!(
        derive_task_status_from_linear(LinearStateType::Started),
        Task::InProgress
    );
    assert_eq!(
        derive_task_status_from_linear(LinearStateType::Completed),
        Task::Done
    );
    assert_eq!(
        derive_task_status_from_linear(LinearStateType::Canceled),
        Task::Abandoned
    );
}

#[test]
fn derive_terminal_states() {
    // spec(§5.1): the issue-closed mappings + their terminal property — Completed→Done is the
    // work-complete state but NON-terminal (a completed issue can reopen; terminal = Merged/Closed/
    // Abandoned), while Canceled→Abandoned lands on a real terminal (won't-do).
    let completed = derive_task_status_from_linear(LinearStateType::Completed);
    assert_eq!(completed, Task::Done);
    assert!(
        !completed.is_terminal(),
        "Done is non-terminal (reopenable)"
    );
    let canceled = derive_task_status_from_linear(LinearStateType::Canceled);
    assert_eq!(canceled, Task::Abandoned);
    assert!(canceled.is_terminal(), "Abandoned is terminal (won't-do)");
}

#[test]
fn parse_then_derive_chain() {
    // spec(§5.1/§9): the parse→derive composition — "started"→Started→InProgress; "completed"→Completed→Done.
    let started = derive_task_status_from_linear(parse_linear_state_type(Some("started")));
    assert_eq!(started, Task::InProgress);
    let completed = derive_task_status_from_linear(parse_linear_state_type(Some("completed")));
    assert_eq!(completed, Task::Done);
}
