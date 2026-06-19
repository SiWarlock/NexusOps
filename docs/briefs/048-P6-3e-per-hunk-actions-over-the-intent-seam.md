# /tdd brief — per_hunk_actions_over_the_intent_seam

## Feature
**6.3e proper (cat-1)** — wire `DiffReview.tsx`'s Review tab to the live mutation
path: source diff content from the **`get_diff`** read RPC (047), enable the per-hunk
**stage / unstage / discard** buttons, and submit each as a typed `git.*` `ActionRequest`
over the **043/044 intent seam** (the daemon adjudicates → the `GatewayModal` approval
card renders the daemon's policy/preview → the human approves/denies). The UI **never
mutates** — it submits a typed intent and renders what the daemon reports. **No
optimistic "done."** The `\x1f`-delimited resource_ref must target the **exact**
displayed hunk (the load-bearing security property — a mismatch stages/**discards** the
wrong content).

> **Cat-1 mutation-wiring slice.** It CONSUMES the already-ruled intent seam (Q1–Q7,
> durable — `docs/planning/intent-seam-cat1-safety-design.md`) — it does NOT re-open
> them. `security-reviewer` **REQUIRED** (invariant policy). Built against the
> `MockGatewayPort` (the real `UdsGatewayPort` transport + the real daemon
> projection-enrichment + the Phase-5 git executor BODIES are all parked — the
> 044/046 exposed-ahead pattern). The **"Always allow" standing-grant stays
> DISABLED** (its own cat-1 checkpoint — NOT in scope; `git.discard_hunk` is
> non-standing-grantable daemon-side regardless).

## Use case + traceability
- **Task ID:** P6.3e (the per-hunk wiring half — the contract-adoption half landed at 047)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (`get_diff` / the
  GatewayPort surface), `§6.2` (intent→policy→approval→execute), `§6.3` (the per-hunk
  `git.*` catalog + the resource_ref encoding), `§4.2` (the 3 laws — submit-not-execute),
  `§15` (INV-SEC-1), `§11.5` (the Action Gateway approval-card semantics), `§11.6`
  (wire-or-disable), `§17` (the safety cards).
- **Widens phase scope because** it wires the UI's per-hunk mutation path over the
  `§6.1`/`§6.2`/`§6.3` Gateway contract + the `§15` INV-SEC-1 / `§17` safety-card /
  `§2.5`-seam anchors — the same mutation-path widen 043/044 already declared for the
  intent seam. The UI **consumes** these daemon-authored contract sections (it does not
  author them); Phase-6 scope is extended to them exactly as the 6.3d intent-seam slices did.
- **Cat-1 safety design:** ratified rulings Q1–Q7 in `docs/planning/intent-seam-cat1-safety-design.md`
  (LEAD RULINGS, durable) — **consumed, NOT re-derived**. Each invariant → a pinned
  test (below).
- **Related context:** brief `047-P6-3e-regen-0280-and-diff-read-surface.md` (the
  `get_diff`/diff-shapes/`PerHunkGitActionType` read surface this consumes); brief
  `044-P6-3d-gatewaymodal-real.md` + **LESSONS §16/§17** (the seam + the approval card
  this reuses); the daemon `§6.3` cross-doc row (`daemon/CLAUDE.md`) for the FROZEN
  resource_ref hunk encoding + the per-hunk risk/preview/standing-grant facts;
  `ui/CLAUDE.md` GatewayPort-mutation-surface row.

## The frozen contract this wires (verified — `daemon/CLAUDE.md` §6.3 / `shared/src/catalog.rs`)
- **`git.stage_hunk`** = risk-2, `preview_class=Git`, `executor=Git`, `requires_resource_refs`, **standing_grant_eligible=true**.
- **`git.unstage_hunk`** = risk-2, same.
- **`git.discard_hunk`** = **risk-3, DESTRUCTIVE (irreversible content loss)**, `preview_class=Diff` (the daemon's preview shows EXACTLY the hunk content discarded), **NON-standing-grantable** (always a per-action human approval — never folded into an approve-all).
- **Resource-ref hunk encoding (FROZEN convention — the ui PRODUCES it, the Phase-5 git executor parses it back):**
  `ResourceRef{ type: "file", id: "{worktree_id}\x1f{file}\x1f{old_start},{old_lines},{new_start},{new_lines}" }`
  — the frozen `$def` field is **`type`** (not `resource_type`) and `ResourceType` is lowercase **`"file"`** (the daemon prose `ResourceType::File` is informal Rust-variant shorthand → serializes `"file"`); `{resource_type:"File"}` fails `ResourceRef.parse`. `\x1f` = the **U+001F unit-separator** delimiter; the 4 positions come **verbatim** from the displayed `Hunk` (the `get_diff` result). Hunk-precise (distinct hunks → distinct keys, no false dedup); **read↔mutate consistent**.
- **The daemon mints `action_request_id`** + **reconciles risk to the catalog** (risk is catalog-authoritative, LESSON 19 — the UI's submitted `risk_level` is overwritten; the UI MUST render the daemon's `PolicyDecision` risk, never its own).
- **The git.* executor BODIES are Phase-5 stubs** (R1-A registry) → against the real daemon a submitted hunk action is adjudicated + approval-gated but not yet really applied; against the `MockGatewayPort` it returns a fixture ack/policy. **This slice is the UI wiring** — exposed-ahead of the live transport + the real git executor.

## Acceptance criteria (what "done" means)
**L1 — diff sourced from `get_diff` (READ, non-safety):**
- [ ] The Review tab sources its diff from **`port.get_diff(worktree_id, file)`** (→ `DiffResult`), mapped to the kit `DiffHunk` render, replacing the static `diffFixture` import. (Worktree/file context: a fixture `wt_…` id for now — the real worktree-projection source is a flagged follow-on, like the live transport.)
- [ ] A `get_diff` read error / `not_found` renders an **honest empty/error state** (per `describeRejection` → generic; never a fabricated diff, forbidden #2).
- [ ] A clean file (no hunks) renders an honest "no changes" state.

**L2 — per-hunk submission over the seam (CAT-1, `security-reviewer` REQUIRED, OWN commit):**
- [ ] The per-hunk **stage / unstage / discard** buttons are **enabled only when `canSubmitIntent`** is true (§11.6 wire-or-disable; the fail-safe gate — Q2); disabled (not faked) otherwise.
- [ ] Clicking a per-hunk button **submits a typed `ActionRequest`** via `useSubmitIntent` (the seam) with `action_type` = the `PerHunkGitActionType` id + `resource_refs: [{ type: "file", id: "{worktree_id}\x1f{file}\x1f{old_start},{old_lines},{new_start},{new_lines}" }]` formed **verbatim** from the displayed `Hunk` — **the UI never performs the git op** (Q1, pure submitter).
- [ ] **THE security-critical pin — resource_ref correctness:** the submitted `resource_ref.id` round-trips to the **displayed** hunk's `{worktree_id, file, old_start, old_lines, new_start, new_lines}` (the `\x1f` = U+001F delimiter exact; a conformance test on the encoder). **The submitted hunk == the displayed hunk** (a mismatch → wrong content staged/**discarded** — catastrophic for the irreversible discard).
- [ ] On submit, the **`GatewayModal` approval card** renders the daemon's `PolicyDecision` + `ActionPreview` (Q4/Q5 — never UI-derived risk, never a fabricated preview; the `discard_hunk` risk-3 + the Diff preview [the exact hunk lost] come from the **daemon**); Approve/Deny submit per the frozen contract.
- [ ] **No optimistic "done"** (Q3) — the submission renders the daemon-reported status (`ActionAck.status` = pending/awaiting-approval); a hunk flips to "staged/discarded" ONLY on a confirming projection/`ActionResult`, never on submit.
- [ ] A submission rejection routes through **`describeRejection`** → the distinct §11.5 cards (Q6 — `fencing_conflict` never re-approvable #6; `precondition_stale` re-approvable; `internal_error` fail-closed; `not_found`/transport → honest generic).
- [ ] The **"Always allow" / standing-grant affordance stays DISABLED** (its own cat-1 checkpoint; `git.discard_hunk` is non-standing-grantable daemon-side regardless).
- [ ] All 7 cat-1 invariants pinned (the RED outline); `security-reviewer` PASS on L2.
- [ ] Whole suite green (279 + the net-new pins; no regressions); `/preflight` clean.
- [ ] Cross-doc flagged at Step 9 (the GatewayPort-consumer / DiffReview wiring row).

## Wiring / entry point (Step 7.5)
**REAL entry — the Code/Diff Review tab.** `DiffReview.tsx` → the Review tab → the kit
`DiffHunk` per-hunk action bar (currently `actions={false}`) → `onClick` →
`useSubmitIntent(port).submit(...)` → the Shell gateway overlay (`GatewayModal`,
opened for the new approval — see Q2 below) → Approve/Deny → the seam. Diff content
sourced from `port.get_diff(worktree_id, file)` on tab/file open. Confirm `/wired`:
the per-hunk button → the seam → `GatewayPort.submit_action`, a real path (not just
tests). The standing-grant button stays disabled (no path).

## Files expected to touch
**Modified:**
- `ui/src/views/code/DiffReview.tsx` — `ReviewTab`: source from `get_diff`; enable the per-hunk action bar wired to the seam; open the approval card on submit.
- `ui/src/views/code/DiffReview.test.tsx` (NEW or extended) — the L1 render + the L2 cat-1 pins.
- `ui/src/intent/` or a small helper — the **resource_ref encoder** (`Hunk` → the `\x1f`-delimited `File` resource_ref) + its conformance test (the security-critical pin); decide placement at Step 2.5.
- Possibly `ui/src/shell/Shell.tsx` / the gateway-overlay open path — if the per-hunk submit opens the `GatewayModal` for the new approval (Q2).
- `ui/src/views/display-fixtures.ts` — a contract-shaped `get_diff` worktree/file fixture context (if needed for the mock-driven render).

**Reused as-is (do NOT re-implement):** `useSubmitIntent`/`createIntentSeam` (the seam), `GatewayModal` (the approval card), `safety/model.ts` `describeRejection` (the rejection routing), `intent-contracts.ts` `PerHunkGitActionType` + `ActionRequest` (047).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## Cat-1 safety design (Q1–Q7 ratified — each invariant → a pinned test)
> Consumed from `docs/planning/intent-seam-cat1-safety-design.md` (LEAD RULINGS). Do NOT re-open.
- **Q1 (A) — pure submitter, no UI execution.** Pin: the per-hunk handler calls the seam's `submit`, never a git/FS op; no executor on the UI path.
- **Q2 (A) — `canSubmitIntent` fail-safe gate.** Pin: the per-hunk buttons are disabled when `canSubmitIntent` is false (defense-in-depth; the daemon Gateway is the real chokepoint).
- **Q3 (A) — no optimistic done.** Pin: submit renders the daemon-reported pending status; a hunk shows applied ONLY on a confirming projection/`ActionResult`.
- **Q4 (A) — render the daemon's policy, never UI-derived risk.** Pin: the displayed risk / approval-requirement is read from the daemon's `PolicyDecision` (the submitted `risk_level` is daemon-reconciled/ignored — catalog-authoritative); the UI never computes `discard`'s risk-3 or standing-grant-eligibility.
- **Q5 (A) — the daemon's `ActionPreview` only.** Pin: the `discard_hunk` consequence preview (the hunk lost) comes from the daemon's `ActionPreview` (`preview_class=Diff`), never fabricated; honest pending / `cannot_preview_reason` otherwise.
- **Q6 (A) — distinct rejection cards.** Pin: a rejection routes through `describeRejection`; `fencing_conflict` is never re-approvable (#6); `not_found`/transport → honest generic.
- **Q7 (A) — no cache/auto-retry.** Pin: no held intent state across a disconnect (the seam's existing posture; B/C parked-for-user).
- **THE NEW security-critical pin (resource_ref correctness):** the ui-formed resource_ref id == the displayed hunk's positions, `\x1f`=U+001F exact — the cross-track conformance pin (the ui-producer side; the daemon-consumer side is the Phase-5 git executor). `security-reviewer` focus: **the submitted hunk == the displayed hunk** (no off-by-one / wrong-file / wrong-hunk → wrong content discarded).

## RED test outline (Step 2)
**L1 (provisional/render):**
1. `review_tab_sources_diff_from_get_diff` — Asserts: the Review tab calls `port.get_diff(wt, file)` and renders its `DiffResult` hunks (not the static fixture). Why: §6.1 read surface (LESSON 33).
2. `get_diff_not_found_renders_honest_empty` — Asserts: a `not_found`/error → honest empty/error, never a fabricated diff. Why: forbidden #2.

**L2 (cat-1 — the safety contract):**
3. `per_hunk_button_disabled_when_cannot_submit` — Q2 fail-safe gate.
4. `stage_hunk_submits_typed_intent_never_executes` — Q1: submit via the seam, no UI git op.
5. `resource_ref_encodes_displayed_hunk_exactly` — **the security pin**: id == `{wt}\x1f{file}\x1f{os},{ol},{ns},{nl}` from the displayed `Hunk`; `\x1f`=U+001F; round-trip/decoder conformance.
6. `submit_renders_daemon_policy_not_ui_risk` — Q4: the card's risk/approval-requirement from the daemon's `PolicyDecision`; discard's risk-3 is daemon-sourced.
7. `discard_preview_is_daemon_actionpreview_not_fabricated` — Q5: the discard consequence from the daemon's `ActionPreview`, never synthesized.
8. `no_optimistic_done_on_submit` — Q3: pending per the ack; applied only on a confirming projection/result.
9. `rejection_routes_through_describe_rejection` — Q6: distinct §11.5 cards; fencing never re-approvable.
10. `standing_grant_affordance_stays_disabled` — the standing-grant is disabled (own cat-1 checkpoint).
Each carries `Asserts: <invariant> (§anchor / Q#)`; the coverage map ties each acceptance bullet.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (consumes 047's `get_diff`/diff-shapes/`PerHunkGitActionType`; no new contract type). The resource_ref encoding is the FROZEN daemon convention — the UI conforms to it (a conformance pin), it does not define it.
- **Orchestrator doc rows to write hot (Step 9):** the `ui/CLAUDE.md` GatewayPort-mutation-surface row — extend with the 6.3e per-hunk wiring (the seam's third consumer; the resource_ref-encoding conformance; standing-grant-disabled). No `ARCHITECTURE.md` edit (daemon-authored, already at 0.28.0).
- **§2.5-seam model touched?** The resource_ref **encoding** crosses the daemon↔ui §6.3 seam — the conformance pin (RED #5) is the ui-producer half of the cross-track conformance test (the carry-forward item); flag it so the daemon-consumer half (Phase-5 git executor) pairs.

## Things to flag at Step 2.5
1. **Submit → approval-card flow.** On a per-hunk submit (risk-2/3, approval-required), does the UI **immediately open the `GatewayModal`** for the just-submitted action, or surface it in the pending-approvals queue? **Default vote:** open the `GatewayModal` for the new approval immediately (immediate feedback; reuses the 044 card; the no-optimistic-done invariant holds — it shows the daemon's pending status). This is a UX-presentation detail (Q3 says settle-able, not safety).
2. **The submitted `risk_level`.** The contract's `ActionRequest.risk_level` is required but daemon-reconciled (catalog-authoritative, ignored). **Default vote:** pass the catalog value as a non-authoritative hint (or 0) but **NEVER display it** — the card's risk is the daemon's `PolicyDecision` (Q4). Pin the "displayed risk == PolicyDecision, never the request" property.
3. **The resource_ref encoder placement.** A small pure `hunkResourceRef(worktreeId, file, hunk)` helper. **Default vote:** co-locate with the intent surface (`intent/`) or a `views/code/` helper — wherever the conformance test reads cleanly. It is the security-critical unit → its own focused test.
4. **The worktree_id source.** `get_diff` needs a `wt_…` id; the real worktree projection isn't consumed yet. **Default vote:** use a fixture `wt_…` context for this mock-driven slice + **flag the real worktree-projection → worktree_id wiring as a follow-on** (it rides the worktree-projection consumption + the live transport, both parked). Do NOT build the worktree projection here (scope creep).

## Dependencies + sequencing
- **Depends on:** slice 047 (the `get_diff`/diff-shapes/`PerHunkGitActionType` read surface — **landed `fbd6adc`**); 043/044 (the intent seam + `GatewayModal` + `describeRejection`).
- **Blocks:** 6.7 (the §18 diff-open benchmark measures this `get_diff`→render path). The live end-to-end (UI → real `UdsGatewayPort` → the Phase-5 git executor) needs the parked transport + the edges-track git executor — a later integration.

## Estimated commit count
**2.** **L1** = the `get_diff`-sourced diff render (READ, **non-safety** — no mutation wired). **L2** = the per-hunk submission wiring (**CAT-1 — its OWN commit**, `security-reviewer` REQUIRED; a safety-critical slice never bundles). The L1/L2 split keeps the cat-1 review surface isolated + bisectable (the 043/044 foundation-then-consumer pattern). If L1 + L2 prove too coupled to split cleanly (the buttons need the rendered hunks), flag at Step 2.5 — but default to the split (L2 carries the safety contract).

## Lessons-logged candidates anticipated
- **Convention candidate** — likely: "a per-hunk (or any resource-precise) mutation forms its resource_ref VERBATIM from the displayed read source (`get_diff`'s `Hunk`) — the submitted target == the displayed target; the `\x1f` encoding is conformance-pinned both sides" (the read↔mutate-consistency rule; extends LESSON §17's read↔mutate discipline to the UI producer). Surface at Step 9.
- **Architecture-doc note candidate** — the UI per-hunk surface is the third intent-seam consumer (after the GatewayModal approval flow + the reject cards); the resource_ref encoding is daemon-frozen + ui-conformed.
- **Future TODO — next-brief working set** — the **"Always allow" standing-grant** (its own cat-1 checkpoint — escalate BEFORE authoring); the real `UdsGatewayPort` transport + `gatewayApprovalEnrichment`→real-projection (before any real human approves); the real worktree-projection → worktree_id source; the cross-track resource_ref conformance pairing (Phase-5 git executor).

## How to invoke
1. **Read this brief end-to-end** — especially "Cat-1 safety design" (Q1–Q7 → pins) + "Things to flag at Step 2.5" (4 questions). The Q1–Q7 rulings are durable — consume, don't re-derive.
2. Pre-flight: confirm you're on `track/ui` in the `NexusOps-ui` worktree, `cd ui`.
3. **Run `/tdd per_hunk_actions_over_the_intent_seam`.**
4. Step 0 (Restate) — confirm against the Feature line.
5. Step 1 (Identify files) — confirm against "Files expected to touch".
6. **Step 2.5** — answer the 4 design questions + send the test-design write-up (one `Asserts: <invariant> (§/Q)` line per test + the coverage map); wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN. **A NEW safety fork → escalate to the orchestrator before sign-off** (e.g. if the design forces enabling the standing-grant).
7. **Step 8** — `security-reviewer` REQUIRED (cat-1 mutation path; focus: resource_ref correctness [submitted == displayed hunk] + the destructive-discard render + the 7 invariants).
8. Step 9 — surface the cross-doc flag + the resource_ref-conformance lesson candidate + anything beyond the anticipated lessons.
