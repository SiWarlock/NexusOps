# /tdd brief — status_model_and_attention_rank

## Feature
The canonical **status model**: a descriptor table mapping every frozen §5.1 status — keyed by **(machine, status)** — to an **attention-rank (0–5)**, a kit visual-kind, and a display label; plus the attention derivations (**needs-my-attention** membership, **sort order**) and the **Worktree two-axis precedence** function. This is the pure-logic heart of 6.2; the StatusPill/AttentionMarker **rendering** + Approval/ActionRequest two-surface are the separate **6.2b** slice. Drift-pinned against the 6.1a generated enums so no status can silently fall through to idle.

## Use case + traceability
- **Task ID:** P6.2a (decomposition of 6.2: **6.2a status model** → 6.2b status rendering)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.3` (status rendering binding — keys == §5.1 verbatim; **one canonical status→attention-rank table; no silent fall-through to idle**; ordering per PRD §5.2), `§5.1` (the 10 status machines; Worktree derived two-axis precedence), `§11.1` (Attention Ladder 0–5; never-color-alone is 6.2b), `§7.2` (Worktree derived precedence; stale is time-derived).
- **Related context:**
  - 6.1a generated enums (`ui/src/contracts/`) are the **source of truth** for the status keys — the descriptor table is keyed off them and a completeness test pins coverage.
  - Kit (verified): `StatusPill` exposes a **~20-key visual `StatusKind`** (`active/running/waiting-human/waiting-perm/approval/failed/blocked/conflict/stale/degraded/completed/pr-open/critical/…`, kebab-case) + a `STATUS` descriptor map; `AttentionMarker` takes `level: 0–5`. So §5.1 canonical (snake_case, ~100 states) → kit visual-kind is a **mapping** this slice builds; the descriptor's `visualKind` field names the kit kind (6.2b renders it).
  - Kit Attention Ladder (`tokens/status.css`): **5** waiting-on-human · **4** failed/blocked/conflict/critical + waiting-on-permission/approval · **3** degraded/high-capacity · **2** running/testing · **1** active/dirty/PR-open · **0** idle/done/archived.
  - **ExecutionProfile (the 10th §5.1 machine) is HELD (0.5b) — NOT frozen, NOT in the generated layer. 6.2a covers the 9 frozen machines only; ExecutionProfile's descriptors are deferred to its freeze (surfaces at 6.4 Settings).** Do not hard-bind it.

## Acceptance criteria (what "done" means)
- [ ] A **status descriptor table** keyed by **(machine, status)** — `{ machine, status, attentionRank: 0–5, visualKind, label }` — covering **every** value of all 9 frozen status enums (Session 17, Task 17, PullRequest 11, WorkflowInstance 12, ProjectBrain 10, Approval 10, ActionRequest 15, AgentTeam 9, + Worktree via its two axes).
- [ ] **Completeness, drift-pinned:** a test iterates every frozen enum value (from the generated layer `.options`) and asserts a descriptor exists — so a future daemon-added status **fails the test** until the table covers it. No `(machine, status)` falls through to a default idle/0.
- [ ] **No silent fall-through to idle (§11.3 fix):** `waiting_on_permission`, `conflict`, `blocked`, `stale` (and the other attention-bearing states) map to their proper rank (≥3/4), **never 0** — they enter "Needs my attention".
- [ ] **Attention derivations:** `needsMyAttention(rank)` (queue membership) + the triage bucket (`needs-attention` / `working` / `settled`) + `compareByAttention` (sort: higher rank first, stable tiebreak).
- [ ] **Worktree two-axis precedence:** `deriveWorktreeStatus(gitAxis, overlayAxis)` → one derived status per §5.1/§7.2 precedence (overlay terminal states win; conflicts surface loud; clean+no-overlay → clean), which then resolves to a descriptor.
- [ ] **Unknown status is visible, not idle:** an out-of-table status (defensive) resolves to a visible "unknown" descriptor (a non-zero, visibly-flagged rank), not a silent idle (§11.3 "unknown → visible, not idle").
- [ ] `attention.ts`/status model lives in **`ui/src/status/`** (NOT `shared/` — see Q4). `/preflight` clean.

## Wiring / entry point (Step 7.5)
The status model is **consumed-by-6.2b** (StatusPill/AttentionMarker rendering) and by the shell's sidebar weight / queue / sort (6.2b wires those into the 6.1b shell). Within 6.2a it's pure logic + a completeness test against the generated enums. Name 6.2b as the consumer at Step 7.5; this is reachable-by-next-slice (the rendering), not silently unreachable — and the completeness test is its own production-relevant guard. (`deriveProjectSwitcherCounts`/`deriveActivityFeed` in 6.1b's shell may later be refactored to reuse `needsMyAttention` — note, don't force it here.)

## Files expected to touch
**New:**
- `ui/src/status/attention.ts` — the rank ladder constants + `needsMyAttention` + triage bucket + `compareByAttention`.
- `ui/src/status/descriptors.ts` — the (machine, status) → descriptor table + lookup (`describeStatus(machine, status)`).
- `ui/src/status/worktree.ts` — `deriveWorktreeStatus(gitAxis, overlayAxis)`.
- `ui/src/status/{attention,descriptors,worktree}.test.ts`.

**Modified:** none expected (consumes the generated layer read-only). Flag any beyond this at Step 2.5.

## RED test outline (Step 2)
**`status/descriptors.test.ts`:**
1. **`descriptor_covers_every_frozen_status`** — for each frozen status enum, every `.options` value has a `(machine,status)` descriptor with a valid rank 0–5. Asserts completeness, drift-pinned to the generated layer. Why §11.3 (render every state); §5.0 drift discipline. **[load-bearing]**
2. **`attention_states_not_floored_to_idle`** — `waiting_on_permission`→4, `conflict`→4, `blocked`→4, `stale`→3 (representative across machines), all > 0. Asserts the §11.3 fix. **[load-bearing correctness]**
3. **`attention_ranks_match_ladder`** — representative states map to the §11.1 ladder: `waiting_on_human_input`→5; `failed`→4; `running_command`/`running_tests`→2; `active`→1; `idle`/`completed`/`archived`→0.
4. **`unknown_status_is_visible_not_idle`** — `describeStatus('Session','bogus')` → a visible "unknown" descriptor (non-zero/flagged rank), not idle. Why §11.3.

**`status/attention.test.ts`:**
5. **`needs_my_attention_membership`** — `needsMyAttention(rank)` true for the attention threshold (Q3), false below; triage buckets partition all ranks.
6. **`sort_by_attention_desc`** — `compareByAttention` orders higher rank first with a stable tiebreak.

**`status/worktree.test.ts`:**
7. **`worktree_overlay_terminal_precedence`** — overlay `deleted`/`merged` wins over git-axis; `pr_open` overlay surfaces; clean git + no overlay → clean. Why §5.1/§7.2 precedence.
8. **`worktree_conflicts_surface_loud`** — git-axis `conflicts` → a high-attention derived status (not masked by a benign overlay). Why §5.1 (conflicts is loud).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none frozen. The descriptor table consumes the frozen enums (already cross-doc-rowed in 6.1a).
- **Orchestrator doc rows to write hot:** flag a **`ui/CLAUDE.md` cross-doc row** for the canonical status→attention-rank table (§11.3) — the UI-canonical the sidebar/queue/sort bind to. I write it. (The 6.2 task line's `Cross-doc invariant: extended — the status enums` is satisfied by this + the 6.1a row.)
- **MVP_TASKS 6.2 Files-line correction:** `shared/contracts/attention.ts` → `ui/src/status/` — I correct it (orchestrator territory) per Q4.

## Things to flag at Step 2.5
1. **Descriptor key = (machine, status) tuple.** Status strings aren't globally unique (`active` is in Session/AgentTeam/WorkflowInstance; `archived` in several; `failed`/`completed` shared). Default vote: **key by (machine, status)**. Confirm.
2. **The attention-rank ladder assignment (load-bearing — this IS the Step-2.5 review).** Produce the FULL (machine,status)→rank table and send it in the write-up; I review the assignments against §11.1/§5.2. Rubric to apply:
   - **5** waiting-on-human (`waiting_on_human_input`; AgentTeam `waiting_on_human`).
   - **4** `failed`/`killed`-pending?/`blocked`/`conflict`/`waiting_on_permission`/Approval `awaiting_approval`+`requested`+`escalated`/ActionRequest `awaiting_approval`+`rollback_failed`+`partially_succeeded`/PR `conflict`+`checks_failing`+`changes_requested`/Brain `error`/Task `needs_clarification`+`requested_changes`/**`changes_ready`** (review-needed) — the "needs a human now" band.
   - **3** degraded/stale band: `stale`/Brain `graph_degraded`+`reindex_required`/WorkflowInstance `degraded`+`drift_detected`/PR `checks_pending`/ExecutionProfile rate_limited+auth_expired (deferred — 0.5b).
   - **2** working: `thinking`/`running_command`/`running_tests`/`editing_files`/ActionRequest `executing`+`queued`/Brain `indexing`+`transcript_ingestion_active`/WorkflowInstance `personalization_in_progress`.
   - **1** active/in-progress-low: `active`/`starting`/`creating`/Task `in_progress`+`assigned`/PR `open`+`draft`+`mergeable`+`approved`/worktree `dirty`+`pr_open`+`ahead/behind_base`/AgentTeam `active`+`reconciling_outputs`.
   - **0** settled: `idle`/`completed`/`archived`/`killed`/`done`/`merged`/`closed`/`abandoned`/`deferred`/Brain `ready`/`not_configured`/`not_detected`/worktree `clean`.
   - Disagree case-by-case — `changes_ready` (rank 4, review-needed) and the "no-fall-through" four (`waiting_on_permission`/`conflict`/`blocked`/`stale`) are the ones to get right.
3. **needs-my-attention threshold + triage buckets.** Default vote: **needs-attention = ranks {4,5}; working = {2,3}; settled = {0,1}**. Confirm (esp. whether rank-3 degraded belongs in needs-attention).
4. **`attention.ts` location.** Default vote: **`ui/src/status/`, NOT `shared/contracts/`.** The attention-rank is UI render policy (drives sidebar weight/queue/sort — §11.3), not a cross-language contract the daemon/Brain consume; `shared/` is frozen daemon territory + the cross-track merge surface. If the notifier (§10) ever needs a canonical attention-rank it gets promoted to a shared contract via the daemon track — not written by the ui track into frozen `shared/`. Confirm; I'll correct the MVP_TASKS Files line.
5. **ExecutionProfile deferral.** Default vote: **9 frozen machines only; ExecutionProfile (held 0.5b) deferred** — its descriptors land when its enum freezes (6.4 Settings). The completeness test iterates only the 9 frozen enums, so it won't fail on ExecutionProfile's absence. Confirm.

## Dependencies + sequencing
- **Depends on:** 6.1a (generated enums) `fd9738b`; 6.1b shell `39a87c6` (the sidebar/queue 6.2b will wire into).
- **Blocks:** **6.2b** (StatusPill/AttentionMarker rendering for all states + four-channel never-color-alone + Approval/ActionRequest two-surface — consumes this table); 6.3 screens (Command Center triage uses the buckets + sort).
- **Deferred:** ExecutionProfile descriptors ← 0.5b.

## Estimated commit count
**1–2.** The descriptor table + attention derivations + worktree precedence are one cohesive model. Implementer may split the worktree-precedence commit if it helps. No safety **invariant** touched (pure UI render policy) → security-reviewer NOT required; code-quality every-slice.

## Lessons-logged candidates anticipated
- **Convention candidate** — the status→attention-rank descriptor table is keyed by (machine, status), drift-pinned to the generated enums (completeness test), and is the single source for sidebar weight / queue / sort; no fall-through to idle (§11.3). Likely `ui/LESSONS.md §5`.
- **Architecture-doc note candidate** — if the §5.2 PRD attention ordering needs a tiebreak rule §11.3 doesn't pin, flag it (don't invent silently).

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd`.
1. **Read this brief end-to-end** — Q2 (the full rank table) is the load-bearing Step-2.5 review; produce the complete (machine,status)→rank assignment in the write-up.
2. **Run `/tdd status_model_and_attention_rank`.**
3. **Step 2.5** — send the test design + the FULL rank table + answers to the 5 questions. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
4. **Step 7.5** — name 6.2b as the consumer; the completeness test is the production-relevant guard.
5. **Step 9** — flag the cross-doc row (attention-rank table) + the attention.ts-location/MVP_TASKS-correction; commit-message-first.
