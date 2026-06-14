# /tdd brief — per_hunk_enrich_real_risk

## Feature
**L2-prep, the per-hunk half of the 044 [med] resolution (NON-cat-1) — the Q2 split-out of 053.**
053's Layer C swapped the **clean** `enrichApproval(row)` path (the HIQ/existing-approval card, the
`ApprovalQueueRow` in-hand) → real risk/policy. This slice swaps the **harder** per-hunk
`enrichHunkAction` path: the DiffReview per-hunk git action (stage/unstage/discard) has **no row in
hand** — the ack carries an `action_request_id`, not an `approval_id` — so sourcing the REAL
`risk_level`/`policy_decision` needs a **`get_projection("ApprovalQueue")` re-fetch + match by
`action_request_id`** + an **honest "awaiting" treatment** when the row isn't in the queue yet
(timing — the daemon mints the approval row async; the ApprovalQueue isn't subscribed, 052 Q3
spread). After this, the **044 [med] is RESOLVED on BOTH approval-card paths** (no fixture risk
anywhere a human approves). **NON-cat-1** (a read-shape swap — the per-hunk submit stays L2-HELD, so
this is exposed-ahead-of-L2 plumbing), but **`security-reviewer` REQUIRED** (the approval-card
surface + the post-submit timing).

## Use case + traceability
- **Task ID:** P6.8 L2-prep (the 044 [med] per-hunk half; the Q2 split from 053; NON-cat-1; in the closeout regen round)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.5` (the human-approval card renders the daemon's risk/policy), `§6.1` (the `get_projection("ApprovalQueue")` re-fetch), `§11.7` (honest degradation — pending/absent, never fabricated risk), `§5.0`/`§5.1` (the frozen `ApprovalQueueRow`/enums consumed).
- **Reference:**
  - **053 layer C** (`280862d`+C — the clean `enrichApproval(row)` pattern this mirrors; `display-meta.ts`); the frozen `ApprovalQueueRow` (`shared/src/projections.rs`, 14 fields — incl. `action_request_id: Option<String>` + `risk_level`/`policy_decision`) reconciled at 053 B.
  - **The swap site** `ui/src/shell/display-meta.ts` `enrichHunkAction(actionType, ack)` — the daemon-SHAPED fixture side-map (risk + approval_id fixture, extends the 044 [med]); the 048 per-hunk DiffReview flow (`DiffReview.tsx` → seam `submit_action` → `GatewayModal`). The read path = the live `UdsGatewayPort.get_projection` (L1 ✅).
  - The clean-path security note (053 C): the modal renders the risk number from the live `preview_action` `ActionPreview` (so the row's `risk_level` is correct-but-secondary) — the swap's job is to remove the **fixture** from the enrichment, not to be the sole risk source.
  - LESSON 17 (the card renders the daemon's decision, never UI-derived/invented), LESSON 22 (parse-don't-trust the read), forbidden #2 (never invent consequences) / #4.

## Acceptance criteria (what "done" means)
- [ ] `enrichHunkAction` sources the per-hunk approval's `risk_level` + `policy_decision` from the **real `ApprovalQueueRow`** via `get_projection("ApprovalQueue")` → **match the row by `action_request_id`** (the ack's id == `ApprovalQueueRow.action_request_id`), NOT the `gatewayApprovalEnrichment` fixture side-map. The card renders the daemon's authoritative risk/decision (LESSON 17).
- [ ] **Absent-row → honest "awaiting"** (the row not yet in the queue at the ack — timing): a pending/awaiting treatment (the real risk preserved via the live `preview_action`, never a fabricated risk — forbidden #2). Pin both the matched and the absent cases.
- [ ] **Parse-don't-trust:** the re-fetched `ApprovalQueue` page parses through `boundary.ts` (`parseProjectionPage`) before the match (a malformed payload → `BoundaryValidationError`, never a fabricated row).
- [ ] No fixture risk/policy reaches the per-hunk card on the real path; `gatewayApprovalEnrichment` stays test-only (zero new production refs). The **mutation submit stays L2-HELD** (exposed-ahead-of-L2 — the per-hunk submit throws not-wired until L2; the swap is the read plumbing, TDD'd against a fake projection).
- [ ] **`security-reviewer` REQUIRED:** real risk from the matched row (not fixture/UI-derived); the absent-row honest-pending (no fabrication); parse-don't-trust on the re-fetch; no mutation reach; the `action_request_id` match can't surface the WRONG approval's risk (match exact, never a fuzzy/first-row fallback).
- [ ] Whole suite green (337 + the per-hunk pins); `/preflight` clean; cross-doc flagged at Step 9. **Completes the 044 [med] resolution (both paths).**

## Wiring / entry point (Step 7.5)
**REAL plumbing (live at L2).** DiffReview per-hunk action → seam `submit_action` (L2-HELD — throws
not-wired until L2) → on the (future-L2) ack → `enrichHunkAction(actionType, ack)` →
`get_projection("ApprovalQueue")` → match by `action_request_id` → the card's risk/policy from the
real row (absent → pending). `/wired`: `enrichHunkAction` now traces to the live projection re-fetch,
not the `display-meta.ts` fixture. Exposed-ahead-of-L2 (like the 048 per-hunk wiring); TDD'd against
a fake `get_projection` + fake ack.

## Files expected to touch
**Modified:**
- `ui/src/shell/display-meta.ts` (`enrichHunkAction` — the fixture→real re-fetch/match/absent-pending) + `display-meta.test.ts` (the per-hunk pins).
- Possibly `ui/src/views/code/DiffReview.tsx` (if the enrich call needs the gateway/`get_projection` threaded) + its test.

If beyond this list, **flag at Step 2.5**.

## RED test outline (Step 2)
1. `enrich_hunk_action_sources_real_risk_by_action_request_id` — a fake `ApprovalQueue` page with a row matching the ack's `action_request_id` → the card reads `row.risk_level`/`row.policy_decision`, not the fixture. — Asserts: real risk from the matched row (§11.5 / LESSON 17).
2. `enrich_hunk_action_absent_row_is_honest_pending` — no matching row → an awaiting/pending treatment, never a fabricated risk. — Asserts: §11.7 honest degradation / forbidden #2.
3. `enrich_hunk_action_match_is_exact_not_first_row` — a page with a non-matching row present → does NOT surface that row's risk (exact `action_request_id` match, no fuzzy/first-row fallback). — Asserts: the security match-correctness.
4. `enrich_hunk_re_fetch_parses_at_boundary` — a malformed `ApprovalQueue` payload → `BoundaryValidationError`, never a fabricated row. — Asserts: parse-don't-trust (LESSON 22).
5. `per_hunk_submit_stays_l2_held` — the per-hunk submit still throws not-wired (the swap adds no mutation reach). — Asserts: INV-SEC-1 / L2-HELD.
Each carries `Asserts: <invariant> (§anchor)`; the coverage map ties each acceptance bullet.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (consumes the frozen `ApprovalQueueRow` reconciled at 053 B).
- **Orchestrator doc rows (Step 9):** the `ui/CLAUDE.md` approval-card row → **044 [med] fully RESOLVED (both the clean + the per-hunk paths)**. No `ARCHITECTURE.md` edit.
- **Shared-contract (cross-area) model touched?** No (053 B did the `ApprovalQueueRow` reconcile + its snapshot pin).

## Things to flag at Step 2.5
1. **Re-fetch vs subscribe for the ApprovalQueue.** Default: a **one-shot `get_projection("ApprovalQueue")` re-fetch** on the enrich (the ApprovalQueue isn't subscribed — 052 Q3 spread; a live ApprovalQueue subscription is a later projection-coverage slice). Confirm; flag if a subscription belongs here.
2. **Absent-row timing — pending vs poll/retry.** Default: a single re-fetch; absent → honest pending (the live `preview_action` carries the risk number meanwhile, like the clean path) — NO poll/retry loop for MVP. Flag if a bounded retry is wanted.
3. **The match key.** Default: exact `action_request_id` match (the ack's id == `ApprovalQueueRow.action_request_id`); never a fuzzy/first-row fallback (a security pin — the wrong approval's risk must never surface).
4. **DiffReview threading.** If `enrichHunkAction` needs the `gateway`/`get_projection` handle threaded from DiffReview, flag the DiffReview touch at Step 2.5.

## Dependencies + sequencing
- **Depends on:** 053 (the frozen `ApprovalQueueRow` + the clean `enrichApproval` pattern) + the live `UdsGatewayPort.get_projection` (L1 ✅).
- **Blocks:** nothing in the regen round — **this COMPLETES the round (053 + 053b) + the 044 [med] (both paths).** After the round seal: HALT (the wholesale team+lead closeout) → the fresh team does the connection-state reconcile → the **L2 cat-1 checkpoint** → L2 (the per-hunk submit goes live, exercising this plumbing).

## Estimated commit count
**1** (the focused per-hunk swap — one concern, the security-sensitive path). **NON-cat-1** (read-shape, exposed-ahead-of-L2; the submit stays HELD) — **`security-reviewer` REQUIRED**.

## Lessons-logged candidates anticipated
- **Convention candidate** — folds into the 053 frozen-shadow + approval-card-real-risk lesson (the per-hunk path completes it): a post-submit per-resource approval sources real risk by an **exact `action_request_id` re-fetch + match** (absent → honest pending, never a fixture/fuzzy fallback). Surface at the round seal.
- **Architecture-doc note candidate** — the 044 [med] is fully resolved (both approval-card paths read real risk); the only L2 blockers left are the connection-state reconcile + the cat-1 checkpoint.

## How to invoke
1. **Read this brief end-to-end** — the per-hunk re-fetch/match/absent-pending + the 4 Step-2.5 questions.
2. Pre-flight: `track/ui` (053 A+B `280862d` + C landed; 0.31.0; 337 green).
3. **Run `/tdd per_hunk_enrich_real_risk`** (same session as 053 — no `/session-start`).
4. Step 0/1 — confirm Feature + Files.
5. **Step 2.5** — answer the 4 questions + send the test-design write-up + coverage map; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` REQUIRED (the per-hunk real-risk match + the absent-pending + the exact-match-no-wrong-approval pin).
7. Step 9 — the cross-doc flag (044 [med] fully RESOLVED) + the round-seal lesson; then `/session-end` (the closeout round: 053 + 053b).
