//! §6.3 ActionTypeCatalog — the per-`action_type` binding contract (ARCHITECTURE §6.3, AG §28.2).
//!
//! For each of the LOCKED MVP action types ([`MVP_ACTION_TYPES`], 22), a binding entry pins the
//! **locked risk 0-4** (authoritative — the policy engine resolves risk from HERE, never the
//! requester-supplied `risk_level`), the required **preview class** (the §6.2 typed previews are
//! 2.3; the catalog NAMES the class now), the **idempotency-key formula** (NAMED here; the real key
//! derivation + dedup store are 2.3), the **executor** binding (the real adapters are 2.3), and the
//! `requires_resource_refs` / `params_schema_present` flags. [`lookup`] is a CLOSED table — an
//! `action_type` not in the MVP set returns `None` (fail-closed, §15; the policy denies it, never
//! default-allows). `workflow.command.invoke` carries `params_schema_present=false` — the §6.3 /
//! OQ-WP-5 "cannot be standing-granted" floor, which is locked at **risk-4 (critical)** since
//! arbitrary pack-command execution has unbounded blast radius (lead-ruled 2026-06-11).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::actions::RiskLevel;

/// A frozen, snake_case-wire, reject-unknown value enum + its `ALL` (mirrors `actions::wire_enum!`).
macro_rules! catalog_enum {
    ($(#[$m:meta])* $name:ident { $($v:ident),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($v),+ }
        impl $name {
            /// Every variant, declaration order.
            pub const ALL: &'static [Self] = &[ $(Self::$v),+ ];
        }
    };
}

catalog_enum! {
    /// The required dry-run preview class for an action (§6.2/§6.3). The 6 typed per-class previews
    /// (command/diff/git/api/session/workflow/rollback) are 2.3's deliverable; the catalog names the
    /// class each `action_type` must produce.
    PreviewClass { Command, Diff, Git, Api, Session, Workflow, Rollback }
}

catalog_enum! {
    /// Which executor adapter handles an action (§6.3). A NAME binding — the real per-namespace
    /// adapters (git CLI / octocrab / session host / …) land in 2.3.
    ExecutorKind { Brain, Project, Workflow, Plan, Session, Git, Github, Linear, Code, Review }
}

catalog_enum! {
    /// How an action's idempotency key derives (§6.3). NAMED here; the real key derivation + the
    /// dedup store are 2.3. `None` = not idempotent; `FromInputs` = derived from the input payload;
    /// `NaturalResourceRef` = the targeted resource is the natural key.
    IdempotencyFormula { None, FromInputs, NaturalResourceRef }
}

/// The binding per-`action_type` catalog entry (§6.3; Appendix A row `ActionTypeCatalog`). The
/// **`locked_risk` is authoritative** — the policy engine resolves risk from here. Reject-unknown
/// end-to-end (§5.0/§15).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionTypeCatalogEntry {
    /// the LOCKED risk 0-4 (authoritative; NOT the requester-supplied `risk_level`)
    pub locked_risk: RiskLevel,
    /// the required dry-run preview class (the typed previews are 2.3)
    pub preview_class: PreviewClass,
    /// how the idempotency key derives (NAMED; realized 2.3)
    pub idempotency_formula: IdempotencyFormula,
    /// which executor adapter runs it (NAMED; realized 2.3)
    pub executor: ExecutorKind,
    /// whether the action requires ≥1 `resource_ref`
    pub requires_resource_refs: bool,
    /// whether the action's params carry a typed schema (`false` = the §6.3/OQ-WP-5 null-schema
    /// floor — `workflow.command.invoke`)
    pub params_schema_present: bool,
}

/// The LOCKED §6.3 MVP action-type set (22; AG §28.2). The closed lookup domain — a type not here
/// fails closed in [`lookup`].
pub const MVP_ACTION_TYPES: &[&str] = &[
    "brain.ask",
    "brain.sync",
    "brain.summarize_session",
    "project.rescan",
    "workflow.detect",
    "workflow.command.invoke",
    "plan.link_task",
    "session.create",
    "session.attach_terminal",
    "session.send_message",
    "session.pause",
    "session.resume",
    "git.status",
    "git.diff",
    "git.create_worktree",
    "git.create_branch",
    "github.create_pr_draft",
    "github.create_pr",
    "linear.link_issue",
    "linear.create_issue",
    "code.open_file",
    "review.request_agent_fix",
];

fn entry(
    locked_risk: RiskLevel,
    preview_class: PreviewClass,
    idempotency_formula: IdempotencyFormula,
    executor: ExecutorKind,
    requires_resource_refs: bool,
    params_schema_present: bool,
) -> ActionTypeCatalogEntry {
    ActionTypeCatalogEntry {
        locked_risk,
        preview_class,
        idempotency_formula,
        executor,
        requires_resource_refs,
        params_schema_present,
    }
}

/// The CLOSED catalog lookup (§6.3) — the binding per-type contract. An `action_type` not in the
/// MVP set returns `None` (fail-closed, §15 — the policy denies it, never default-allows; deferred
/// high-risk types like `git.force_push`/`merge`/`delete_worktree`, AG §28.3, are simply absent).
pub fn lookup(action_type: &str) -> Option<ActionTypeCatalogEntry> {
    use ExecutorKind as X;
    use IdempotencyFormula as I;
    use PreviewClass as P;
    use RiskLevel as R;
    Some(match action_type {
        // risk-0 — read / inspect / propose-only (auto-execute eligible; no FS/git/external mutation)
        "brain.ask" => entry(R::Level0, P::Api, I::None, X::Brain, false, true),
        "project.rescan" => entry(R::Level0, P::Command, I::None, X::Project, false, true),
        "workflow.detect" => entry(R::Level0, P::Workflow, I::None, X::Workflow, false, true),
        "git.status" => entry(R::Level0, P::Git, I::None, X::Git, true, true),
        "git.diff" => entry(R::Level0, P::Diff, I::None, X::Git, true, true),
        "code.open_file" => entry(R::Level0, P::Command, I::None, X::Code, true, true),
        // risk-1
        "session.attach_terminal" => entry(
            R::Level1,
            P::Session,
            I::NaturalResourceRef,
            X::Session,
            true,
            true,
        ),
        "session.pause" => entry(
            R::Level1,
            P::Session,
            I::NaturalResourceRef,
            X::Session,
            true,
            true,
        ),
        // risk-2
        "session.create" => entry(R::Level2, P::Session, I::FromInputs, X::Session, true, true),
        "session.resume" => entry(
            R::Level2,
            P::Session,
            I::NaturalResourceRef,
            X::Session,
            true,
            true,
        ),
        "session.send_message" => {
            entry(R::Level2, P::Session, I::FromInputs, X::Session, true, true)
        }
        "plan.link_task" => entry(
            R::Level2,
            P::Api,
            I::NaturalResourceRef,
            X::Plan,
            true,
            true,
        ),
        "linear.link_issue" => entry(
            R::Level2,
            P::Api,
            I::NaturalResourceRef,
            X::Linear,
            true,
            true,
        ),
        "linear.create_issue" => entry(R::Level2, P::Api, I::FromInputs, X::Linear, false, true),
        "git.create_worktree" => {
            entry(R::Level2, P::Git, I::NaturalResourceRef, X::Git, true, true)
        }
        "git.create_branch" => entry(R::Level2, P::Git, I::NaturalResourceRef, X::Git, true, true),
        "github.create_pr_draft" => entry(
            R::Level2,
            P::Api,
            I::NaturalResourceRef,
            X::Github,
            true,
            true,
        ),
        "brain.sync" => entry(R::Level2, P::Api, I::FromInputs, X::Brain, false, true),
        "brain.summarize_session" => entry(R::Level2, P::Api, I::FromInputs, X::Brain, true, true),
        // risk-3
        "github.create_pr" => entry(
            R::Level3,
            P::Api,
            I::NaturalResourceRef,
            X::Github,
            true,
            true,
        ),
        "review.request_agent_fix" => {
            entry(R::Level3, P::Workflow, I::FromInputs, X::Review, true, true)
        }
        // risk-4 — CRITICAL: arbitrary pack-command execution, unbounded blast radius; the §6.3/
        // OQ-WP-5 "cannot be standing-granted" floor falls out of risk-4 (params_schema_present=false;
        // lead-ruled 2026-06-11). Never approve-all-eligible; never auto-executes.
        "workflow.command.invoke" => entry(
            R::Level4,
            P::Workflow,
            I::FromInputs,
            X::Workflow,
            false,
            false,
        ),
        _ => return None,
    })
}
