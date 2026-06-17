# Daemon-Track Work Order — fully unblock the UI track

> Authored by `ui-team-lead` 2026-06-16, from this session's verified findings (UI orchestrator's
> surface-verification passes + lead git checks against `main`). Each item is the **same shape** the
> daemon already executed twice: fold/emit an event into a projection + serve it typed (the
> `ApprovalQueueRow` ②-mini precedent, LESSON §37; the `SessionFailed`→`proj_session.status` fold,
> LESSON §40). The daemon orchestrator scopes each into a `/tdd` slice; this is the WHAT + WHERE.
>
> **Merge gate:** the UI lead HOLDS the `main→ui` merge until the daemon lands the REQUIRED items
> (D2–D4) so it's ONE merge that unblocks everything — not a partial.

---

## ✅ D1 — `PullRequestRow` freeze (P7.2) — **ALREADY DONE**
`main@fb9f6d3`, CONTRACT **0.34.0** (`e748874` "freeze the typed PullRequestRow + typed serve").
Unblocks **Phase 7-UI (basic)**: the row carries `pr_number / status / head_branch / base_branch /
pr_checked_at`. No further daemon work needed for the basic PR list. *(Rich workspace = D5, optional.)*

---

## 🔴 D2 — Survival: fold `SessionRecovered` → `proj_session` + serve the recovery state typed  **[REQUIRED — unblocks survival UI]**

**The gap:** `SessionRecovered` is emitted but **no projector folds it** — `SessionProjector`
(`daemon/src/projections/session.rs`) folds only `SessionStarted` + `SessionFailed`. So
`get_projection("Session")` returns rows with **no recovery state**, and the UI's recovery banner
(`ui/src/recovery/model.ts`) is fixture-driven. The UI *already anticipates* the data —
`ui/src/contracts/provisional.ts:147` has `resume_mode: ResumeMode.optional()` on `SessionRow`.

**The event payload to read** (`shared/src/events.rs:301`, already frozen):
```rust
pub struct SessionRecovered {
    pub mode: ResumeMode,                                  // the §8.1 recovery outcome
    pub replayed_event_count: u64,                         // scrollback replayed (non-zero only on Replayed)
    pub execution_profile_id: Option<ExecutionProfileId>,  // §15 #8 profile preserved
}
```

**The slice:**
1. **Fold** — add a `SessionRecovered` arm to `SessionProjector` (mirror the existing `SessionFailed`
   arm; status/state derived from the EVENT TYPE+payload, never the row's current value → rebuild-safe,
   LESSON §17/§40). `UPDATE proj_session SET resume_mode=?, replayed_event_count=?, recovered_at=? WHERE session_id=?`.
2. **Columns + migration** — add `resume_mode` (nullable; the `ResumeMode` wire value), `replayed_event_count`
   (nullable/0), `recovered_at` (from the envelope) to `proj_session`. New `MIGRATION_N` + bump `SUPPORTED_USER_VERSION`.
3. **Freeze + serve** — surface these on the served Session projection row, and freeze the typed
   `SessionRow` `$def` in `shared/src/projections.rs` with `resume_mode` (+ `replayed_event_count`,
   `recovered_at`) — the `ApprovalQueueRow` precedent (the row is currently only a "follow" comment).
4. **CONTRACT** — additive bump **0.34 → 0.35** (the SessionRow gains the recovery fields).

**Unblocks:** (a) per-session recovery state (the UI banner reads real `resume_mode` via
`get_projection("Session")`); (b) the daemon-wide `RecoveryState` banner, which the UI re-derives from
per-session recovery data (per the `daemon/src/runtime/recovery.rs:100` design comment).

---

## 🟠 D3 — Session live-delta emission: nudge on status changes, not just `SessionStarted`  **[REQUIRED — "Session stays live" vs the REAL daemon]**

**The gap (a confirmed Finding):** `deltas_for_append` (`daemon/src/runtime/writer.rs:720`) emits a
Session delta **only** `if intent.event_type == "SessionStarted"`. So `SessionFailed` / `SessionRecovered`
(and any status change) emit **no nudge** → the UI's Session list only refreshes on reconnect, not
per-change. (The L1/052 "cockpit stays live" was Mock-validated, not real-daemon.)

**The slice:** extend `deltas_for_append` to also emit a `ProjectionName::Session` Upsert delta (the
row-less "nudge" — subscriber re-reads via `get_projection`) for `SessionFailed`, `SessionRecovered`,
and any other event that mutates a `proj_session` row. **CONTRACT-neutral** (runtime delta behavior).
Pairs with the UI's Session refetch-on-nudge (a UI carry-forward — the UI half).

---

## 🟡 D4 — Live-delta emission for the other 4 cockpit projections  **[REQUIRED for "whole cockpit live"; lower priority]**

**The gap:** only **Session** (`writer.rs:720`) and **ApprovalQueue** (`gateway/pipeline.rs:79`) emit
deltas today. The other 4 the UI shows — **ProjectActivity, PullRequest, AuditTrail, UsageLedger** —
emit **none**, so they can't be made live (they only update on reload/reconnect).

**The slice:** emit a row-less Upsert "nudge" delta for each of these 4 when its underlying rows change
(on the relevant event appends / post-commit), so the UI can subscribe + refetch-on-nudge per
projection. **CONTRACT-neutral.** Lower priority — the action-relevant surfaces (Session, ApprovalQueue)
are already covered; this is completeness so the whole cockpit reflects daemon mutations live.

---

## ⚪ D5 — (OPTIONAL) Rich PR row: `mergeable` / `checks_summary` / reviews  **[only for a rich PR Workspace beyond status+branches]**

The D1 `PullRequestRow` froze the **minimal** columns. `mergeable` + `checks_summary` are in the
`PullRequestSynced` payload but **not projected** (they fed the derived `status` in the edges executor).
A structured **reviews** list doesn't exist in the payload (review-decision is daemon-side derivation).

**If you want a rich PR Workspace** (mergeable badge, individual checks, reviews): extend
`proj_pull_request` to project `mergeable` + `checks_summary` as columns + add them to the frozen
`PullRequestRow`; reviews would need the daemon to capture/project a structured list first. Additive
CONTRACT bump. **Skip if the basic PR list (status + branches) is enough for now.**

---

## D6 — PR card diff-stats (Option A, user-chosen 2026-06-17) — **NEW daemon ask for the PR Workspace**

The prototype's PR cards show `+additions / -deletions · N files · M commits`. **VERIFIED these are NOT in `PullRequestSynced`** (which carries only pr_number/status/branch/base/mergeable/checks_summary/pr_checked_at) — so they're not capturable by a projection add; the **edges GitHub-sync must FETCH them from the API**.
**Slice:** extend the edges PR producer to capture `additions / deletions / changed_files / commits` from the GitHub PR API → add to the `PullRequestSynced` payload → project onto `PullRequestRow` → typed serve. Additive CONTRACT bump. (Bigger than D1's freeze — it's a producer-side capture.)

## D7 (verify-first) — the PR-diff data path for the Review tab

The prototype "Review" tab shows **PR #84's code diff** (changed files + hunks). The existing `get_diff` is **worktree-scoped** (`wt_` id, the 6.3e per-hunk *worktree* review) — NOT a remote-PR diff. If reviewing a PR's diff needs the PR's changeset (vs its base), that's a **new daemon data path** (the orch verifies whether a worktree already backs each PR diff, or this is a real gap). Flagged, not yet scoped.

## Future arcs (NOT this read-only Phase-7) — surfaced by the prototype
- **PR-review mutations** — the prototype's `Merge` (board), `Approve PR` + per-hunk `Accept / Reject / Request fix` (Review tab) are **risk≥3 Gateway mutations** = a **future cat-1 arc** (own checkpoint, like the L2 go-live). Render disabled in the read-only Phase-7.
- **Brain controls** — `Ask Brain` / `Ask why` are Brain-backed → the deferred sibling `brain/` product. Render disabled.

## Out of scope (not a daemon-unblock item)
- **Brain drawer (8.2-UI)** — the sibling `brain/` product, integrated later, not started. Deferred.

## Net for the merge gate
- **REQUIRED before the `main→ui` merge:** D2 (survival), D3 (Session live), D4 (other-4 live).
- **Optional:** D5 (rich PR).
- **Ending CONTRACT:** D2 = 0.35 (D3/D4 contract-neutral); +D5 = 0.36.
- After they land on `main` (idle, stable), the UI lead runs **one** `main→ui` merge + regen → the UI
  builds: Phase 7-UI (D1✓), survival UI (D2), whole-cockpit-live (D3+D4), rich PR (D5 if done).
