//! Frozen projection-row contracts (§7 / §5.0) — the typed read-model rows the UI/Brain consume.
//!
//! The FIRST frozen projection-row is [`ApprovalQueueRow`] (P4.0b-ui2, the ②-mini): the
//! `proj_approval_queue` read model the §11.5 human-approval card renders, typed in `shared/` so the
//! safety-critical approval path is served as a contract, not loose JSON (Fork B, pin #2). The other
//! projection-row reconciles (SessionRow / ProjectActivityRow / PullRequestRow / AuditEventRow) follow
//! this pattern. `schemars` → versioned JSON-Schema → generated Zod/Pydantic (§5.0); reject-unknown.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::actions::{PolicyDecision, RequesterType, RiskLevel};
use crate::harness::ResumeMode;
use crate::status::{Approval, PullRequest, ReviewState, Session};

/// One open row of the `proj_approval_queue` read model (§7 / §11.5) — an approval awaiting a human
/// decision. The bookkeeping `sort_key`/`updated_at_seq` columns are NOT on the wire row (internal —
/// the UI sorts off `risk_level` + `requested_at`). The load-bearing typed fields: `risk_level`
/// (the authoritative §6.3 risk) + `policy_decision` (the frozen §6.2 decision, persisted §15-redacted
/// at approval-open in C1; `None` for a plan-level approve-all). `status`/`requester_type` are the
/// frozen §5.1/§6.2 enums (reject-unknown — no loose JSON on the approval path). `action_request_id`
/// is `None` for a plan-level approve-all approval. Optionals serialize as explicit `null` (no
/// `skip_serializing_if`) so the §2.5-seam field-name snapshot is stable (LESSON §15 trap 3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ApprovalQueueRow {
    pub approval_id: String,
    pub action_request_id: Option<String>,
    pub plan_id: Option<String>,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_team_id: Option<String>,
    pub risk_level: RiskLevel,
    pub status: Approval,
    pub requester_type: RequesterType,
    pub requester_id: String,
    pub preview_summary: Option<String>,
    pub requested_at: String,
    pub expires_at: Option<String>,
    pub policy_decision: Option<PolicyDecision>,
}

/// One row of the `proj_pull_request` read model (§7.2 / §11.2) — the GitHub-authoritative PR cache the
/// ui PR Review Workspace renders. The 2nd frozen projection-row (after [`ApprovalQueueRow`], P7.2). The
/// BASIC columns the edges-P7.1 `PullRequestProjector` folds: `status` is the frozen §5.1 [`PullRequest`]
/// enum (reject-unknown — no loose status string). `mergeable`/`checks_summary` are the **D5a enrichment**
/// (the basic-now + SPREAD consumed) — folded from `PullRequestSynced.mergeable?`/`checks_summary?` into
/// the 2 columns the row now carries (`mergeable` is the FIRST bool projection column → stored as a SQLite
/// INTEGER 0/1; the daemon read layer coerces it to this JSON bool, so the contract stays a pure
/// `Option<bool>`). The internal `updated_at_seq` is NOT a wire field. **Nullability matches the DDL**
/// (a display read model tolerates a NULL over failing the whole typed serve closed — contrast the
/// safety-critical `ApprovalQueueRow`): `pr_id` (PK) + `status` (NOT NULL) are non-Option; the rest are
/// `Option`. `pr_number` is the GitHub-native PR number (a non-negative external natural → `u64`, a
/// bounded integer in schemars — LESSON §15 trap 2). Optionals serialize as explicit `null` (no
/// `skip_serializing_if`) so the §2.5-seam field-name snapshot is stable (LESSON §15 trap 3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct PullRequestRow {
    pub pr_id: String,
    pub project_id: Option<String>,
    pub repo_id: Option<String>,
    pub pr_number: Option<u64>,
    pub title: Option<String>,
    pub status: PullRequest,
    pub head_branch: Option<String>,
    pub base_branch: Option<String>,
    pub pr_checked_at: Option<String>,
    pub mergeable: Option<bool>,
    pub checks_summary: Option<String>,
    /// D6 — the diff-stats the §11.2 PR card renders (folded from `PullRequestSynced.additions?`/…).
    /// INTEGER projection columns surfacing as JSON numbers → bind directly to `Option<u64>` (no
    /// bool-coercion, unlike `mergeable`); `None`/NULL where GitHub omitted them or pre-D6 rows.
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub changed_files: Option<u64>,
    pub commits: Option<u64>,
    /// P4.7 — the PR head commit SHA the §11.2 PR Workspace reads to FORM the cat-1 merge/review SHA-pin
    /// (folded from `PullRequestSynced.head_sha`). A TEXT column → JSON string → `Option<String>` direct
    /// passthrough (no coercion, unlike `mergeable`); `None`/NULL where GitHub omitted it or pre-P4.7 rows.
    /// Display/pin-FORMATION source only — the daemon's anti-race is the LIVE GitHub 409, not this field.
    pub head_sha: Option<String>,
}

/// One row of the `proj_session` read model (§7.2 / §11.4) — the derived current state of a session, the
/// 3rd frozen projection-row (after [`ApprovalQueueRow`]/[`PullRequestRow`], D2). Carries the
/// user-meaningful columns + `status: Session` (the frozen §5.1 enum, reject-unknown) + the §8.1/§11.4
/// survival-recovery fields (`resume_mode`/`replayed_event_count`/`recovered_at` — the
/// resumed-vs-replayed-vs-reattached banner source, folded from the now-consumed `SessionRecovered`).
/// The not-yet-consumed `proj_session` columns (worktree/linked_*/token_usage/pending_approvals/…) are a
/// later SPREAD; the internal `updated_at_seq` is NOT a wire field. **Nullability matches the DDL:**
/// `session_id` (PK) / `project_id` (NOT NULL — the fold guarantees it) / `status` (NOT NULL) are
/// non-Option; the rest are `Option`. `display_name` is the daemon-canonical name (the ui provisional's
/// `title` maps to it on regen). Optionals serialize as explicit `null` (no `skip_serializing_if`) so the
/// §2.5-seam field-name snapshot is stable (LESSON §15 trap 3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct SessionRow {
    pub session_id: String,
    pub project_id: String,
    pub status: Session,
    pub display_name: Option<String>,
    pub harness: Option<String>,
    pub model: Option<String>,
    pub execution_profile_id: Option<String>,
    pub resume_mode: Option<ResumeMode>,
    pub replayed_event_count: Option<u64>,
    pub recovered_at: Option<String>,
}

/// One row of the `proj_review` read model (§7.2 / §11.2) — a single structured PR review the ui PR Review
/// Workspace renders. The 4th frozen projection-row (after ApprovalQueue/PullRequest/Session, D5b-1). Folded
/// from the `ReviewSynced` event (the live GitHub producer is D5b-2). `review_id` is the globally-unique
/// GitHub-native review id (the PK → non-Option `u64`, a bounded integer in schemars — LESSON §15 trap 2);
/// `state` is the frozen [`ReviewState`] value enum (reject-unknown — no loose state string). `repo_id` is
/// sibling-read from the action's Repo resource_ref (the `PullRequestRow` precedent). `body` is FREE-FORM
/// user review text, §15-redacted at the event (the row serves the redacted value). The internal
/// `updated_at_seq` is NOT a wire field. **Nullability matches the DDL** (a display read model tolerates a
/// NULL over failing the whole typed serve closed): `review_id` (PK) + `state` (NOT NULL) are non-Option;
/// the rest are `Option`. Optionals serialize as explicit `null` (no `skip_serializing_if`) so the
/// §2.5-seam field-name snapshot is stable (LESSON §15 trap 3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ReviewRow {
    pub review_id: u64,
    pub pr_number: Option<u64>,
    pub project_id: Option<String>,
    pub repo_id: Option<String>,
    pub reviewer: Option<String>,
    pub state: ReviewState,
    pub submitted_at: Option<String>,
    pub body: Option<String>,
}
