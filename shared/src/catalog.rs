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
    /// adapters (git CLI / octocrab / session host / …) land in 2.3. **`Adjudication` (3.2-part-2,
    /// brief 043) is the ODD ONE OUT**: it marks the agent-mutation family as **adjudication-only** —
    /// the `ActionRequest` TERMINATES at the policy/approval verdict and **no daemon executor runs the
    /// tool** (the agent executes the allowed tool itself; the daemon only adjudicates + audits, the
    /// INV-SEC-1 chokepoint). It is NOT a real-executor namespace and never reaches the executor seam.
    ExecutorKind { Brain, Project, Workflow, Plan, Session, Git, Github, Linear, Code, Review, Adjudication, Integration }
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
    /// whether a standing grant (plan-level approve-all) may cover this action (§6.2; P4.0b-ui1).
    /// `false` = NON-standing-grantable: the §6.2 floor refuses to fold it into an approve-all — it
    /// ALWAYS gets a per-action human approval (the destructive `git.discard_hunk` + the risk-4
    /// `workflow.command.invoke` floor — unified to ONE mechanism). The approve-all exclusion keeps
    /// BOTH disjuncts (`risk==Level4 OR !standing_grant_eligible`, defense-in-depth).
    pub standing_grant_eligible: bool,
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
    "session.kill",
    "session.profile_change",
    "session.attach_terminal",
    "session.send_message",
    "session.pause",
    "session.resume",
    "git.status",
    "git.diff",
    "git.create_worktree",
    "git.create_branch",
    "git.stage_hunk",
    "git.unstage_hunk",
    "git.discard_hunk",
    "github.create_pr_draft",
    "github.create_pr",
    "linear.link_issue",
    "linear.create_issue",
    "integration.connect",
    "code.open_file",
    "review.request_agent_fix",
];

/// The **agent-mutation** action-type family (3.2-part-2, brief 043) — the §9.1 Claude/Codex tool
/// categories the harness adapter intercepts and routes through the EXISTING Gateway as
/// **adjudication-only** `ActionRequest`s (`ExecutorKind::Adjudication`; Option A — one chokepoint).
/// SEPARATE from [`MVP_ACTION_TYPES`]: these are machine-internal (minted by the hook-receiver from a
/// `PreToolUse` payload), not human/Brain-submitted MVP actions — so they DON'T inflate the locked
/// MVP-22 count. Still fully catalogued ([`lookup`] resolves them) so they ride the same risk/policy/
/// audit pipeline. The `tool_name → agent.*` mapping + the params-sensitive deny-rules live in the
/// daemon (`harness::claude::intercept`); the catalog pins only the base risk + the adjudication class.
pub const AGENT_MUTATION_ACTION_TYPES: &[&str] = &[
    "agent.bash",
    "agent.file_edit",
    "agent.file_read",
    "agent.mcp_tool",
    // P4.0b-2 (the live drive loop, user-ruled d.2 split tool-policy): the catalog IS the explicit
    // enumerated allowlist. `agent.todo_write` = the LONE benign-internal auto-allow (risk-0; provably
    // no FS/git/external/exfil surface — the agent's own scratch TODO list). `agent.web_fetch` /
    // `agent.web_search` = the network-EGRESS tools (risk-2 approval-gated — a data-exfil dimension,
    // secrets-in-a-URL past the trust boundary; NOT benign). Everything unclassified stays fail-closed
    // (the receiver denies an unmapped tool — `CoverageGap`/`UnmappedTool`). Additive (MVP-22 untouched).
    "agent.todo_write",
    "agent.web_fetch",
    "agent.web_search",
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
        // the default: most actions ARE standing-grant-eligible. The NON-eligible ones (the §6.2
        // floor — destructive/critical) use [`entry_no_standing_grant`] (P4.0b-ui1).
        standing_grant_eligible: true,
    }
}

/// Like [`entry`] but NON-standing-grantable (§6.2 floor, P4.0b-ui1) — a standing grant / approve-all
/// can never cover it; it ALWAYS gets a per-action human approval. For the destructive
/// `git.discard_hunk` (irreversible content loss) + the risk-4 `workflow.command.invoke` floor.
fn entry_no_standing_grant(
    locked_risk: RiskLevel,
    preview_class: PreviewClass,
    idempotency_formula: IdempotencyFormula,
    executor: ExecutorKind,
    requires_resource_refs: bool,
    params_schema_present: bool,
) -> ActionTypeCatalogEntry {
    ActionTypeCatalogEntry {
        standing_grant_eligible: false,
        ..entry(
            locked_risk,
            preview_class,
            idempotency_formula,
            executor,
            requires_resource_refs,
            params_schema_present,
        )
    }
}

/// The CLOSED catalog lookup (§6.3) — the binding per-type contract over BOTH catalog families:
/// the human/Brain-facing [`MVP_ACTION_TYPES`] AND the machine-internal [`AGENT_MUTATION_ACTION_TYPES`]
/// (3.2-part-2). An `action_type` in NEITHER family returns `None` (fail-closed, §15 — the policy
/// denies it, never default-allows; deferred high-risk types like `git.force_push`/`merge`/
/// `delete_worktree`, AG §28.3, are simply absent — as is any un-mapped agent tool).
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
        // risk-0 — session-lifecycle (P4.0b-1 away-ruled): session.create/kill are MUTATIONS yet
        // risk-0 (audited auto-allow). The relaxation is NARROW to the supervised session lifecycle
        // (NOT a general "mutations may be risk-0") — the danger is downstream-gated by the live
        // per-tool interception (4.0b-2); the policy enforces 5 pins (UI/IPC-only requester, the risk-0
        // allowlist, SessionStarted-audited, profile-recorded-at-start, profile-CHANGE-approval-gated).
        "session.create" => entry(R::Level0, P::Session, I::FromInputs, X::Session, true, true),
        "session.kill" => entry(
            R::Level0,
            P::Session,
            I::NaturalResourceRef,
            X::Session,
            true,
            true,
        ),
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
        // session.profile_change — the §15 #8 no-silent-account-hop APPROVAL gate (PIN c). The TYPE +
        // the risk-2 gate land here (4.0b-1); the executor BODY (the actual profile swap) is a later
        // slice — the SAFETY pin is the approval-gating, testable now.
        "session.profile_change" => {
            entry(R::Level2, P::Session, I::FromInputs, X::Session, true, true)
        }
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
        // P7.1 Wave-C (edges-029) — register an integration connection. risk-2 (approval-gated);
        // REGISTRATION-ONLY (§15 + LESSON 20): inputs carry {provider, keychain_ref POINTER, account?},
        // NEVER a token (a risk-≥1 action executes off the §15-redacted durable row → a secret in inputs
        // is masked by execute-time, so §15 #4 holds by construction). requires_resource_refs=false (the
        // connection identity is the inputs, not a §6.2 resource_ref). FromInputs → re-connecting the
        // same provider+account dedups. **NON-standing-grantable** (`entry_no_standing_grant`, §6.2
        // floor, security-reviewer-ruled): a credential/AUTHORIZATION-ESTABLISHING action must never be
        // folded into a plan-level approve-all — it ALWAYS gets a per-action human approval (the
        // eligibility axis is blast-radius, NOT risk class — LESSON 32; the discard_hunk precedent). The
        // IntegrationExecutor (X::Integration) registers on the live CatalogExecutor; the token→keychain
        // WRITE is a separate deferred non-Gateway mechanism.
        "integration.connect" => {
            entry_no_standing_grant(R::Level2, P::Api, I::FromInputs, X::Integration, false, true)
        }
        "git.create_worktree" => {
            entry(R::Level2, P::Git, I::NaturalResourceRef, X::Git, true, true)
        }
        "git.create_branch" => entry(R::Level2, P::Git, I::NaturalResourceRef, X::Git, true, true),
        // P4.0b-ui1 (the ui-6.3e per-hunk surface; USER-ruled) — stage/unstage = risk-2 (the git.*
        // tier, index ops → preview_class=git); the resource_ref targets the precise hunk
        // (worktree+file+position) so NaturalResourceRef dedups per-hunk, not per-file (read↔mutate
        // consistency). git executor BODY = stub via the R1-A registry seam (real git op = Phase 5).
        "git.stage_hunk" => entry(R::Level2, P::Git, I::NaturalResourceRef, X::Git, true, true),
        "git.unstage_hunk" => entry(R::Level2, P::Git, I::NaturalResourceRef, X::Git, true, true),
        // git.discard_hunk = risk-3 + DESTRUCTIVE (irreversible content loss) → NON-standing-grantable
        // (USER-ruled: always a per-action human approval, never folded into an approve-all) +
        // preview_class=diff (the preview shows EXACTLY the hunk content discarded).
        "git.discard_hunk" => entry_no_standing_grant(
            R::Level3,
            P::Diff,
            I::NaturalResourceRef,
            X::Git,
            true,
            true,
        ),
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
        // lead-ruled 2026-06-11). Never approve-all-eligible; never auto-executes. P4.0b-ui1: also
        // `standing_grant_eligible=false` (entry_no_standing_grant) — UNIFYING the floor onto ONE
        // mechanism (the approve-all exclusion keeps both disjuncts; test_risk4_implies_non_standing_grantable
        // pins they can't drift).
        "workflow.command.invoke" => entry_no_standing_grant(
            R::Level4,
            P::Workflow,
            I::FromInputs,
            X::Workflow,
            false,
            false,
        ),
        // ---- the agent-mutation family (3.2-part-2 / brief 043) — adjudication-only -------------
        // Each routes through the existing pipeline as an `ExecutorKind::Adjudication` action: the
        // verdict is the outcome (the agent runs the tool, the daemon adjudicates + audits) — NO
        // executor runs, so `preview_class` is moot (a placeholder; never rendered) and
        // `idempotency_formula = None` (every tool call is a distinct adjudication — never deduped).
        // `requires_resource_refs = false` (the params ARE the tool_input, not a §6.2 resource_ref).
        // `params_schema_present = true` (the tool_input is a structured payload, NOT the OQ-WP-5
        // null-schema floor). The Q2-CONSERVATIVE base risk: read-only ≠ mutation → risk-0 auto-allow;
        // every MUTATING tool = risk-2 → require_approval by DEFAULT (the L4 params deny-rules raise
        // to a deny; they NEVER lower below this floor).
        "agent.file_read" => entry(R::Level0, P::Command, I::None, X::Adjudication, false, true),
        "agent.bash" => entry(R::Level2, P::Command, I::None, X::Adjudication, false, true),
        "agent.file_edit" => entry(R::Level2, P::Diff, I::None, X::Adjudication, false, true),
        "agent.mcp_tool" => entry(R::Level2, P::Api, I::None, X::Adjudication, false, true),
        // P4.0b-2 — the d.2 split tool-policy. `agent.todo_write` = the LONE benign-internal auto-allow
        // (risk-0). NOT "read-only" — it WRITES the agent's own scratch TODO list — but that write has
        // **no daemon-side side effect**: adjudication-only (no executor runs) + no FS/git/external/exfil
        // surface, so the agent updating its own task list can't harm the trust boundary. `agent.web_fetch`/
        // `agent.web_search` = network EGRESS → risk-2 require_approval (the exfil dimension; the L4 params
        // deny-rules may raise, never lower).
        "agent.todo_write" => entry(R::Level0, P::Command, I::None, X::Adjudication, false, true),
        "agent.web_fetch" => entry(R::Level2, P::Api, I::None, X::Adjudication, false, true),
        "agent.web_search" => entry(R::Level2, P::Api, I::None, X::Adjudication, false, true),
        _ => return None,
    })
}
