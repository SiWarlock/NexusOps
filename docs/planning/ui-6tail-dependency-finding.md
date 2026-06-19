# ui-track 6-tail — dependency Finding + re-sequence decision (orchestrator → lead, away-authority)

> **Orchestrator-authored Finding** (escalation cat-2) at a CLEAN boundary (slice 044 landed,
> nothing in flight). The lead's forward tail was **6.3e → 7.2/7.3-UI → 8.2-UI**; the next item
> (6.3e) is **BLOCKED on an unfrozen daemon contract**, and the rest is likely cross-track-gated.
> I do NOT author a blocked slice or paper over the gap. **The lead rules the re-sequence / pause**
> (and routes anything needing the user). Canonical context: `/context-check` → **`OK — max ctx 67%
> (ui-implementer)`**.

## What just landed (context)
- **Slice 044 (P6.3d slice 2) — `GatewayModal`-real, the cat-1 daemon-driven approval card.** L1 `259801c` (PolicyDecision shadow) + L2 `0a05b27` (modal-real + the net-new `precondition_stale` re-approvable card). 245/245 green; security-reviewer PASS; visual gate PASS; `/wired` real Shell→seam path. Hot-routing done (Lesson §17 + cross-doc consumer row + carry-forward) — sits unstaged for `/orchestrate-end`. The intent seam now has BOTH a foundation (043) and a live consumer (044).

## The Finding — 6.3e is BLOCKED (Explore-verified, cited)
6.3e = "Code/Diff review **with per-hunk actions**." Its core deliverable (the per-hunk **actions**) consumes daemon contracts that are **NOT frozen at 0.23.0**:

1. **No per-hunk MUTATION action types in the frozen §6.3 catalog.** The 22 MVP `action_type`s (`shared/src/catalog.rs:80-103`) include only read-only `git.diff` (risk-0), `git.status`, `git.create_worktree`, `git.create_branch` — **no `git.stage_hunk` / `apply_hunk` / `discard_hunk` / `revert_hunk`.** An unknown `action_type` → catalog `lookup()` returns `None` → policy **denies, fail-closed** (§15). So per-hunk actions submitted through the (real, built) intent seam would be **rejected daemon-side**.
2. **No frozen diff-CONTENT source.** `ProjectionName` (frozen, closed) has **no `Diff`** projection; no diff-read RPC in `daemon/src/ipc/methods.rs`; the git2 diff-read (task **5.2**, Phase 5) is on the **`edges` track — NOT landed on `track/ui`**. Only a UI **fixture** (`ui/src/views/display-fixtures.ts:diffFixture`) exists.
3. **The diff RENDERING already exists.** `ui/src/views/code/DiffReview.tsx` already renders the fixture with kit `DiffHunk` + **disabled-but-present** per-hunk buttons ("PR approval is a Gateway mutation"). So 6.3e's *only new* work — the per-hunk **actions** — is exactly the blocked part. There is **no clean buildable increment** here right now.

**Why not "build 6.3e fixture-only and wire the buttons":** that means submitting `action_type`s the daemon doesn't have = building a **mutation surface against an unfrozen contract** — the precise anti-pattern **Decision C** forbade ("build the first mutation seam ONCE against the real frozen contract — never provisionally"). The seam + the 044 card are the disciplined version of that rule; a fixture-only per-hunk action path would regress it.

## The rest of the forward tail — likely also cross-track-gated (lead, please confirm)
- **7.2 — Full PR Review Workspace (O-6):** depends on **Phase-7 PR-data integration** (octocrab GitHub / Linear read+link) — Phase 7 is not landed on `track/ui`. Likely gated (needs confirmation).
- **8.2 — Brain drawer (Phase 8):** depends on the **Project Brain stdio-MCP sidecar + §11.5 Brain contracts** — Phase 8 not landed. Likely gated.
- *(I confirmed 6.3e directly; 7.2/8.2 I'm flagging as probable, not verified — they share the cross-track-contract-gated shape.)*

## What IS genuinely buildable now on `track/ui`
- **§11.7 accessible-names on shell controls + kit closed-props** (IMPLEMENTATION_PLAN line 556) — UI-only; a real (smallish) slice (kit-contract addition for `HTMLAttributes`/`aria-*` passthrough, or keep the wrapper pattern). **TDD-able now.**
- **6.6 — §18 Project-Graph render benchmark** (line 580) — "Depends on: none"; fixture-driven, runnable now. But it's a **bench at its own cadence** (NOT a per-slice RED/GREEN loop) — a `/phase-exit`/nightly task, not a TDD slice.
- *(Daemon-1.5-gated, NOT buildable: the checking-banner trigger [line 558], real Repair UI [559], ExecutionProfile descriptors [557, 0.5b-gated].)*

## Decision options for the lead (→ user where noted)
- **(A) Re-sequence to the buildable in-lane work.** Do the **§11.7 accessible-names** slice now (+ optionally the **6.6** graph bench at cadence); **defer 6.3e/7.2/8.2** to their cross-track unblocks. *Pro:* keeps shipping real in-lane value. *Con:* a thin runway — after §11.7 the high-value tail is still daemon-gated.
- **(B) Clean PAUSE the ui track at this boundary** (the **edges-track R4 pattern** — in-lane runway largely exhausted; the high-value tail is daemon-gated). **The unblock = the user routes a cross-track packet**: the daemon freezes (i) the per-hunk git action-catalog extension (`git.stage_hunk`/…) + (ii) a diff-CONTENT source (a `Diff` projection or the Phase-5 git2 read), and (for 7.2/8.2) the PR-data + Brain contracts. *Pro:* honest; avoids provisional mutation surfaces. *Con:* stops the track.
- **(C) 6.3e fixture-only now — NOT recommended** (regresses Decision C; wires a mutation path to non-existent action types).
- **Plus — a cycle is reasonable here regardless:** `ui-implementer` is at **67%** (OK but climbing) and this is a clean boundary; if you choose (B), cycling now is natural; if (A), the §11.7 slice fits before a cycle.

## Orchestrator recommendation
**(A) then (B):** author **§11.7 accessible-names** as the next slice (genuinely buildable, in-lane), and in parallel surface to the user that **6.3e/7.2/8.2 need a cross-track daemon packet** (per-hunk action catalog + diff-content source + PR-data + Brain) — so the user can decide whether to route that packet now or pause the ui track after §11.7. I'll hold for your ruling before dispatching (this re-sequence + the implicit 6.3e deferment are yours/the user's call, not agent-only).
