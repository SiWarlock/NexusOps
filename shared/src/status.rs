//! Status state machines (ARCHITECTURE §5.1, LOCKED — R-4..R-9).
//!
//! Value sets are the frozen contract: stored as `TEXT status`, serialized as the
//! exact snake_case wire string (serde `rename_all`). Each machine declares its
//! terminal states. The 10th machine, **ExecutionProfile, is intentionally HELD**
//! for 0.5b (see [`crate::EXECUTION_PROFILE_STATUS_HELD`]). Worktree is two axes
//! (git + overlay); the precedence fn that combines them is Phase-1 projection
//! logic, out of scope for the freeze.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Generates a frozen status enum: snake_case wire serialization, `ALL` (every
/// variant), and `is_terminal()` (the §5.1 bold set).
macro_rules! status_machine {
    (
        $(#[$meta:meta])*
        $name:ident { $($variant:ident),+ $(,)? }
        terminal { $($term:ident),* $(,)? }
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "snake_case")]
        // meta (doc comments + any `#[schemars(...)]` helper, e.g. a schema rename) emitted AFTER
        // the derive so a `schemars` helper attribute is introduced before use (rustc #79202).
        $(#[$meta])*
        pub enum $name { $($variant),+ }

        impl $name {
            /// Every variant, declaration order.
            pub const ALL: &'static [Self] = &[ $(Self::$variant),+ ];

            /// True for the machine's terminal states (§5.1 bold).
            pub fn is_terminal(self) -> bool {
                #[allow(unreachable_patterns)]
                match self {
                    $( Self::$term => true, )*
                    _ => false,
                }
            }
        }
    };
}

status_machine! {
    /// Session lifecycle (17, R-4) — the driver-agnostic normalized vocabulary the
    /// §9.1 harness adapters map INTO. `stale` is time-derived (§5.1).
    Session {
        Creating, Starting, Active, Thinking, RunningCommand, EditingFiles,
        RunningTests, WaitingOnPermission, WaitingOnHumanInput,
        WaitingOnExternalService, Idle, Stale, ChangesReady,
        Failed, Completed, Archived, Killed,
    }
    terminal { Failed, Completed, Archived, Killed }
}

status_machine! {
    /// Task / PlanTask superset (17, R-8).
    Task {
        Unassigned, Queued, Assigned, Ready, InProgress, Blocked,
        NeedsClarification, InReview, ChangesReady, PrOpened, NeedsReview,
        RequestedChanges, Done, Deferred, Merged, Closed, Abandoned,
    }
    terminal { Merged, Closed, Abandoned }
}

status_machine! {
    /// Worktree git-sync axis (R-7). No terminal states (the overlay axis carries
    /// the terminal `deleted`).
    WorktreeGit { Clean, Dirty, UntrackedFiles, Conflicts, BehindBase, AheadOfBase }
    terminal {}
}

status_machine! {
    /// Worktree lifecycle-overlay axis (R-7).
    WorktreeOverlay { Creating, Locked, PrOpen, Merged, Prunable, Deleted }
    terminal { Deleted }
}

status_machine! {
    /// PullRequest (11) — GitHub-authoritative cache (§7.2).
    PullRequest {
        Draft, Open, ChecksPending, ChecksFailing, NeedsReview, ChangesRequested,
        Approved, Mergeable, Conflict, Merged, Closed,
    }
    terminal { Merged, Closed }
}

status_machine! {
    /// WorkflowInstance (12, R-7).
    WorkflowInstance {
        NotDetected, PackAvailable, NeedsPersonalization, PersonalizationInProgress,
        GeneratedReviewRequired, Active, ReadyForTeamRun, Degraded, DriftDetected,
        UpgradeAvailable, Archived, Detached,
    }
    terminal { Archived, Detached }
}

status_machine! {
    /// ProjectBrain (10) — daemon-cached last-reported sidecar status (§7.2).
    ProjectBrain {
        NotConfigured, Indexing, Ready, PartialIndex, Stale, GraphDegraded,
        TranscriptIngestionOff, TranscriptIngestionActive, ReindexRequired, Error,
    }
    terminal { Error }
}

status_machine! {
    /// Approval (10, R-5) — split from ActionRequest; the human/policy decision axis.
    /// Schema name = `ApprovalStatus` (its Rust alias) so the §6.2 `actions::Approval` MODEL
    /// takes the canonical bare `Approval` $def (2.1a Option-B rename; value-set unchanged).
    #[schemars(rename = "ApprovalStatus")]
    Approval {
        Requested, Previewed, AwaitingApproval, Approved, Denied, Edited,
        AutoApprovedByPolicy, Expired, Cancelled, Escalated,
    }
    terminal { Approved, Denied, Edited, AutoApprovedByPolicy, Expired, Cancelled, Escalated }
}

status_machine! {
    /// ActionRequest (15, R-5, NEW) — the execution-lifecycle axis.
    /// Schema name = `ActionRequestStatus` (its Rust alias) so the §6.2 `actions::ActionRequest`
    /// MODEL takes the canonical bare `ActionRequest` $def (2.1a Option-B rename; value-set
    /// unchanged).
    #[schemars(rename = "ActionRequestStatus")]
    ActionRequest {
        Submitted, Previewed, PolicyDecided, AwaitingApproval, Approved, Denied,
        Queued, Executing, Succeeded, Failed, PartiallySucceeded, RolledBack,
        RollbackFailed, Cancelled, Expired,
    }
    terminal { Succeeded, Failed, PartiallySucceeded, RolledBack, RollbackFailed, Cancelled, Expired }
}

status_machine! {
    /// AgentTeam (9, R-6).
    AgentTeam {
        Draft, Starting, Active, WaitingOnHuman, Blocked, ReconcilingOutputs,
        Completed, Failed, Archived,
    }
    terminal { Completed, Failed, Archived }
}

// Type aliases so the in-language identifiers read like the §5.1 machine names
// (the wire value is the contract; these are idiomatic Rust handles).
pub type SessionStatus = Session;
pub type TaskStatus = Task;
pub type WorktreeGitStatus = WorktreeGit;
pub type WorktreeOverlayStatus = WorktreeOverlay;
pub type PullRequestStatus = PullRequest;
pub type WorkflowInstanceStatus = WorkflowInstance;
pub type ProjectBrainStatus = ProjectBrain;
pub type ApprovalStatus = Approval;
pub type ActionRequestStatus = ActionRequest;
pub type AgentTeamStatus = AgentTeam;
