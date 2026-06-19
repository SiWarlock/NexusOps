# /tdd brief — gatewaymodal_real

## Feature
Wire the **real `GatewayModal` permission/approval card** to the **043 intent seam**
(slice 2 of the intent seam). Today `GatewayModal` is **DISABLED** (Approve/Deny/always-allow
are dead controls — slice-1 Scope A). This slice makes it a **working, daemon-driven approval
card**: Approve/Deny submit through `useSubmitIntent` (the 043 seam); the card **renders the
daemon's `PolicyDecision` + `ActionPreview`** (never UI-derived risk, never invented
consequences); each §6.4 rejection `IpcErrorCode` routes to its **distinct §11.5 card**; and
the net-new **`precondition_stale` re-approvable card** lands here. INV-SEC-1: the modal
**SUBMITS intents only — never executes**; the daemon Gateway stays the real chokepoint.

> **⚠️ CATEGORY-1 SAFETY SURFACE — the live human-approval UI over the mutation path.** This
> slice CONSUMES the 7 cat-1 rulings already made for the seam
> (`docs/planning/intent-seam-cat1-safety-design.md` → "LEAD RULINGS"). **The rulings are DONE
> + durable — do NOT re-open them.** Each ruled invariant is re-pinned **at the consumer
> (modal) level** in the "Cat-1 safety design" section below (the modal is the surface a human
> actually clicks — the pins move from the seam to the card it drives). **`security-reviewer`
> is REQUIRED** (invariant policy). The real `UdsGatewayPort` transport + the live
> `ServerFrame.rpc_response` demux + the standing-grant **"Always allow" `policy_grant`** path
> are **SEPARATE later slices** (see Deferred).

## Use case + traceability
- **Task ID:** P6.3d
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (GatewayPort method surface),
  `§6.2` (intent→policy→approval→execute pipeline; `PolicyDecision`/`ActionPreview`/`Approval`),
  `§4.2` (the 3 laws / INV-SEC-1), `§11.1` (read-only/degraded gate), `§11.5` (Action Gateway
  card semantics + the 3 distinct rejection cards), `§11.7` (honest degradation), `§6.4` (IPC
  error codes → cards), `§15` (INV-SEC-1) / `§17` (the safety cards), forbidden #2.
- **Widens phase scope because** the modal is a **CLIENT of the §6 Gateway mutation contract +
  the §15/§17 safety invariants** — a Phase-6 UI surface consuming Phase-2/daemon anchors (the
  parked Decision-C work, unblocked by the 0.23.0 freeze + the boundary merge). Same cross-phase
  contract-consumption widen as 040/041/042/043. *(The `§2`/`§4` Lesson/`#N` tokens elsewhere
  are LESSONS/template refs.)*
- **Related context:** the cat-1 rulings (`docs/planning/intent-seam-cat1-safety-design.md` —
  Q1–Q7, all RATIFIED/RULED); the 043 brief + the landed seam (`ui/src/intent/submit-intent.ts`
  — `useSubmitIntent`/`IntentResult`); `ui/LESSONS.md §16` (the intent seam), `§11`
  (degraded/safety states fail-closed), `§13` (UI logic over state); the current
  `GatewayModal.tsx` stub (disabled controls + the hardcoded "what will happen" note to REPLACE);
  the existing rejection-card surfaces `safety/HardConflictCard.tsx` + `safety/AuditIntegrityAlert.tsx`
  + `safety/model.ts` (`describeConflict`/`describeAuditIntegrity` — the §11.5 treatments to reuse);
  `gateway-client/mock.ts` (the mutation fixtures + the `mutationError` test hook); the Shell
  gateway-overlay mount (`shell/Shell.tsx` — the production entry point).

## Cat-1 safety design (each ruled invariant → a PINNED TEST, AT THE MODAL) — LEAD-RULED
> Source: `docs/planning/intent-seam-cat1-safety-design.md` "LEAD RULINGS" (away-authority,
> logged for user return-review). **These are the SAME Q1–Q7 rulings the seam pinned — re-pinned
> here at the card the human drives.** `security-reviewer` verifies them. Do NOT re-open any ruling.

1. **INV-SEC-1 / Q1 (RATIFIED A):** the modal's Approve/Deny/preview go **only** through the
   043 seam (`useSubmitIntent` → the single `GatewayPort`) — **NO execution path**, no direct
   FS/git/state write, no daemon-DB call. **Pin `modal_actions_route_through_seam_no_executor`** —
   Approve/Deny invoke the seam's `approve`/`deny`; the modal exposes no mutation path that
   bypasses it.
2. **Fail-closed gate / Q2 (RATIFIED A):** Approve/Deny are **disabled when `canSubmitIntent`
   is false** (read-only/degraded), and a `{readOnly: true}` seam result is handled WITHOUT a
   mutation render. **DEFENSE-IN-DEPTH (non-negotiable, state it):** `canSubmitIntent` is **NOT
   the sole guard — the daemon Gateway (INV-SEC-1) is the real chokepoint**; a UI-enable bug must
   still be rejected daemon-side. **Pin `approve_deny_disabled_when_canSubmitIntent_false`** +
   **`readonly_result_renders_no_mutation`**.
3. **No-optimistic-render / Q3 (RATIFIED A):** on a successful `approve`/`deny`
   (`{ok: ActionAck}`) the modal renders the **daemon-reported `ActionAck.status`** (e.g.
   `approved`/`denied`/`submitted`) — it **NEVER** shows "succeeded"/"done"/"executed." A "done"
   state comes ONLY from a confirming projection / `ActionResult` (a later subscription slice,
   NOT here). **Pin `modal_renders_daemon_status_never_optimistic_done`** — given an ack of
   `approved`, the modal shows the decision status, never a completed/executed claim.
4. **Policy-from-daemon / Q4 (RATIFIED A):** the card renders the **daemon's `PolicyDecision`**
   (`status`/`reasons`/`required_approvals`/`constraints`/`safer_alt`) + the risk from the
   daemon's `PolicyDecision`/`ActionPreview` — the modal **NEVER computes its own
   risk/approval-requirement** (the 2.2 catalog-authoritative-risk ruling). **Pin
   `card_risk_and_requirement_read_from_policydecision_never_ui_derived`** — the rendered
   risk/required-approver come from the `PolicyDecision`/`ActionPreview` data, and rendering the
   same card with a DIFFERENT `PolicyDecision` changes the displayed requirement (proves it's
   read, not hardcoded).
5. **No-invented-preview / Q5 (RATIFIED A):** the "What will happen" section renders the
   daemon's **`ActionPreview`** (`summary`/`changed_resources`/`risk_reasons`) — or, when the
   daemon has no preview, the honest **`cannot_preview_reason` / a pending note** — and **NEVER
   fabricates consequences** (forbidden #2). The current stub's hardcoded consequence lines are
   **REPLACED**. **Pin `preview_section_renders_daemon_actionpreview_or_honest_pending`** + a pin
   that a `cannot_preview_reason` renders the reason (not invented consequences).
6. **§6.4 reject codes → distinct §11.5 cards / Q6 (RATIFIED A):** a rejected `approve`/`deny`
   (`{error: WireError}`) routes each `IpcErrorCode` to its **mandated, distinct** treatment:
   `fencing_conflict` → the **never-auto-resolved** hard-conflict treatment (**rule #6 — NO retry
   affordance**); `internal_error` → the **fail-closed** integrity alert (rule #5, non-dismissible);
   `precondition_stale` → the **re-approvable** card (regenerate preview + require fresh approval —
   the net-new build); `policy_denied` → the **deny** render. **Collapsing them breaks #6.**
   **Pin `reject_codes_route_to_distinct_cards`** + **`fencing_conflict_never_rendered_as_reapprovable`**.
7. **No-caching / Q7 (RULED A; B/C PARKED-for-user):** the modal holds **NO intent state** across
   a disconnect — it adds no cache/auto-retry on top of the (already stateless) seam. **Pin
   `modal_holds_no_intent_state_no_autoretry`.** **PARKED (not dropped):** cache+retry is a
   user-facing enhancement; **lead's recorded lean — if ever added it must be (C) MANUAL resubmit,
   NOT (B) auto-replay**. Stays carry-forward, consumer-marked.

## Acceptance criteria (what "done" means)
- [ ] **`PolicyDecision` provisional shadow** added to `ui/src/contracts/intent-contracts.ts`,
      modeled against the frozen `$def` (`status`=`PolicyDecisionStatus` [generated], `reasons`:
      `string[]`, `required_approvals`: `RequiredApprover[]` [already modeled], `constraints`:
      `string[]`, `safer_alt`: `string|null`), **`.strict()`** (frozen `additionalProperties:false`),
      **field-set drift-pinned** (the existing `intent-contracts` drift-pin pattern).
- [ ] **`GatewayModal` wired real:** Approve → `seam.approve(approval)`, Deny → `seam.deny(approval,
      reason)`; the buttons are **enabled only when `canSubmitIntent`** (disabled otherwise, Q2);
      the card renders the daemon's `PolicyDecision` + `ActionPreview` (Q4/Q5); the post-action
      render shows the **daemon-reported status, never optimistic "done"** (Q3).
- [ ] **Rejection routing (Q6):** `fencing_conflict`/`internal_error`/`precondition_stale`/
      `policy_denied` → their distinct §11.5 treatments (reuse `describeConflict` /
      `describeAuditIntegrity` semantics where they apply; `fencing_conflict` has **no retry**).
- [ ] **`precondition_stale` re-approvable card built** (net-new) — regenerate preview + require
      fresh approval (§17/§6.2); **NOT** a never-auto-resolved hard conflict (distinct from fencing).
- [ ] **All cat-1 pins green** (Cat-1 safety design above).
- [ ] **"Always allow in this project" stays DISABLED** (the `policy_grant` standing-grant is a
      DEFERRED separate slice — see Deferred); no behavior change to that control this slice.
- [ ] The modal's mutation fixtures (`mock.ts`) provide a `PolicyDecision` + a full `Approval` (and
      reuse the existing `ActionPreview`/`ActionAck`/`mutationError` hooks) so the pins drive real shapes.
- [ ] Whole suite green; `/preflight` clean (oxlint + tsc + test:run).
- [ ] **`security-reviewer` PASS** (invariant policy — the live approval UI over the mutation path).
- [ ] Cross-doc invariant flagged at Step 9 (the `PolicyDecision` shadow added; the GatewayPort
      mutation-surface row extended to note the consumer-side card; the `precondition_stale` card).
- [ ] **VISUAL gate** (Lesson §10/§12): the real card rendered vs the kit prototype — green tests
      do NOT verify it looks right. Confirm at Step 9 (dev server vs `ui_kits/control-plane`).

## Wiring / entry point (Step 7.5)
**REAL entry point this slice (NOT exposed-ahead — unlike slice 1).** `GatewayModal` is mounted in
`shell/Shell.tsx` (`overlay?.kind === "gateway"` → `<GatewayModal .../>`), opened from the
**HumanInputQueue** (`onOpenApproval`) + the **Command Center / palette** (`setOverlay({kind:"gateway",
approval})`). The newly-wired production path is **Shell gateway overlay → GatewayModal Approve/Deny →
`useSubmitIntent` seam → `GatewayPort`**. Name it at Step 7.5 and **`/wired GatewayModal`** (or the
approve flow) — it must trace from the Shell entry through the seam to the port (reachable, not dead).
The `MockGatewayPort` is the §14-sanctioned test/dev seam; the real `UdsGatewayPort` is a later transport slice.

## Files expected to touch
**New:**
- `ui/src/safety/` (or `ui/src/overlays/`) — the **`precondition_stale` re-approvable card** +
  its descriptor (mirror `HardConflictCard`/`describeConflict`; re-approvable, NOT never-auto-resolved).
- test files for the modal-real behavior + the precondition_stale card + the `PolicyDecision` shadow drift-pin.

**Modified:**
- `ui/src/overlays/GatewayModal.tsx` — wire Approve/Deny → the seam; render `PolicyDecision` +
  `ActionPreview`; handle the `IntentResult` branches (`ok`/`error`/`readOnly`); replace the
  hardcoded consequence note.
- `ui/src/contracts/intent-contracts.ts` — add the `PolicyDecision` shadow (+ export).
- `ui/src/gateway-client/mock.ts` — add a `PolicyDecision` + full `Approval` fixture for the modal
  (keep the existing `submit_action`/`preview_action`/`approve`/`deny`/`mutationError` behavior).
- the `intent-contracts` drift-pin test (extend for `PolicyDecision`).
- possibly `shell/display-meta.ts` / the modal's prop shape (how it sources the `Approval`/
  `PolicyDecision`/`ActionPreview`) — **flag the data-sourcing at Step 2.5** before GREEN.

If implementation needs files beyond this list, **flag at Step 2.5**. **Do NOT change the seam
(`submit-intent.ts`) or the `GatewayPort` mutation signatures** (frozen at 043) — this slice is a CONSUMER.

## RED test outline (Step 2)
The cat-1 consumer pins above (each `spec(§…)`-tagged) + the card-behavior tests:
1. `modal_actions_route_through_seam_no_executor` — `spec(§4.2/§15)`.
2. `approve_deny_disabled_when_canSubmitIntent_false` + `readonly_result_renders_no_mutation` — `spec(§11.1/§15)`.
3. `modal_renders_daemon_status_never_optimistic_done` — `spec(§4.2-law2/§11.7)`.
4. `card_risk_and_requirement_read_from_policydecision_never_ui_derived` — `spec(§6.2)`.
5. `preview_section_renders_daemon_actionpreview_or_honest_pending` (+ `cannot_preview_reason` honest) — `spec(forbidden#2)`.
6. `reject_codes_route_to_distinct_cards` + `fencing_conflict_never_rendered_as_reapprovable` — `spec(§6.4/§11.5/§17)`.
7. `modal_holds_no_intent_state_no_autoretry` — `spec(§6.1-no-authoritative-state)`.
8. `precondition_stale_card_is_reapprovable_not_hard_conflict` — `spec(§17/§6.2)` (the net-new card: offers a
   regenerate-preview + fresh-approval path; NOT the never-auto-resolved treatment).
9. `policydecision_shadow_matches_frozen_field_set` — the drift-pin (`.strict()`, field-set vs the frozen `$def`).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none to the frozen contract (the modal CONSUMES it). **New provisional
  shadow `PolicyDecision`** (drift-pinned; enum field delegated to the generated bundle, Lesson §2) —
  flag it as a provisional-shadow addition.
- **Orchestrator doc rows to write hot (Step 9):** extend the `ui/CLAUDE.md` cross-doc row "GatewayPort
  mutation surface + the intent seam" to note the **consumer (GatewayModal-real)** — renders the daemon's
  `PolicyDecision`/`ActionPreview`, routes §6.4 codes to distinct §11.5 cards, no-optimistic-done; the new
  `PolicyDecision` shadow. No `ARCHITECTURE.md` model edit (§6.1/§6.2/§11.5 are daemon-authored).
- **§2.5-seam model touched?** The modal consumes the §6.1/§6.2 §2.5-seam contracts (already drift-pinned);
  pin #9 extends the drift-pin to `PolicyDecision`.

## Things to flag at Step 2.5
1. **Data-sourcing of `Approval` + `PolicyDecision` + `ActionPreview`.** The modal today receives only
   an `ApprovalQueueRow`; the real card needs a full `Approval` (the seam's `approve`/`deny` take an
   `Approval`), the `PolicyDecision`, and the `ActionPreview`. **Options:** **(A)** the modal receives
   `{approval: Approval, policyDecision: PolicyDecision}` as props sourced from a fixture/side-map (mirror
   `approvalDisplayFixture`) + fetches the `ActionPreview` via `seam.previewAction(action_request_id)` on
   open — the modal stays a pure renderer of daemon-provided data; the real daemon source (enriched
   ApprovalQueue projection row / a preview RPC) wires later. **(B)** widen the `ApprovalQueueRow`
   provisional shape to carry the PolicyDecision. **Default vote: (A)** — least invasive, keeps the card a
   pure renderer, and the projection-row shape stays daemon-frozen. Confirm.
2. **Reject-card rendering — reuse vs inline.** The existing `HardConflictCard`/`AuditIntegrityAlert` are
   standalone projection-fed surfaces; a rejection inside the modal is a different entry. **Default vote:**
   reuse the **descriptor/model semantics** (`describeConflict` never-auto-resolved message; the
   audit-integrity treatment for `internal_error`) rendered **within the modal's reject state**, rather
   than mounting the standalone projection cards — the SEMANTICS (never-auto-resolved / re-approvable /
   fail-closed / deny) are what the pins assert, not the exact component instance. Confirm.
3. **Post-action presentation (non-safety UX).** After a successful approve/deny, does the modal close,
   show an inline daemon-status confirmation, or hand off to a tray? **Constraint (safety):** whatever it
   shows must be the **daemon-reported status, never "done"** (Q3). The presentation choice itself is your
   non-safety UX call — note it.
4. **Commit layering.** Default **2 commits** (Lesson §7): (L1) the `PolicyDecision` shadow + drift-pin
   (contracts); (L2) the `GatewayModal`-real wiring + the `precondition_stale` card (the safety-critical UI
   unit — its own commit, `security-reviewer`). Collapse to 1 only if you judge them one tight unit — flag it.

## Dependencies + sequencing
- **Depends on:** 043 (landed `a198af7` — the seam: `useSubmitIntent`/`IntentResult`/the mock mutation
  methods); the cat-1 rulings; the frozen Gateway contract (0.23.0, merged). Nothing else.
- **Blocks:** 6.3e per-hunk review actions; the real `UdsGatewayPort` transport slice; the **"Always allow"
  `policy_grant` standing-grant** slice (deferred — own cat-1 checkpoint); and every later intent consumer
  (Dispatch, Brain Run-via-Gateway, restart-session — §11.5).

## Deferred (explicitly OUT of this slice — captured, not dropped)
- **"Always allow in this project" — the `policy_grant` standing-grant.** A standing grant pre-authorizes a
  **CLASS of future mutations** without per-action human approval — a **distinct trust surface NOT covered by
  the Q1–Q7 rulings** (those cover the per-action approve/deny path). **Kept DISABLED (status quo).** A
  dedicated policy-grant slice builds it **with its own category-1 safety checkpoint** (orchestrator escalates
  to the lead before authoring it). → carry-forward, consumer-marked. *(Flagged to the lead at authoring.)*
- **The `require_step_approval` / O-3 bundled-plan per-step approval path** (`submit_action_plan`/`PlanAck`/
  `step_id`). This slice is **single-action** approve/deny; the bundled-plan step UI is a later slice. The
  card renders the `PolicyDecision.status` honestly (incl. `require_step_approval`) but does not yet drive a
  multi-step plan flow.
- **An actionable `safer_alt` affordance.** `PolicyDecision.safer_alt` is rendered **display-only** (the
  daemon's suggestion); a one-click "switch to the safer alternative" submit (a NEW intent) is a later slice.
- **Q7-(B)/(C) intent cache + retry — PARKED-for-user** (lead lean: manual, never auto).

## Estimated commit count
**2** (Lesson §7 — enumerated layers): (L1) the `PolicyDecision` provisional shadow + drift-pin; (L2) the
`GatewayModal`-real wiring + the `precondition_stale` re-approvable card. **L2 is SAFETY-CRITICAL → its own
commit + `security-reviewer` REQUIRED** (invariant policy — the live approval UI over the mutation path). The
orchestrator drives L1→L2 (one wake per layer at the commit boundary). Collapse to 1 only if flagged at Step 2.5.

## Lessons-logged candidates anticipated
- **Convention candidate** — the consumer side of the intent seam: a daemon-driven approval card renders the
  daemon's `PolicyDecision`/`ActionPreview` (never UI-derived risk, never invented consequences), shows the
  daemon-reported ack status (no optimistic "done"), and routes each §6.4 reject code to its distinct §11.5
  card (`fencing_conflict` never re-approvable, #6) — extends Lesson §16.
- **Architecture-doc note candidate** — the §11.5 Action Gateway card is now consumer-wired (the per-action
  approve/deny path); the `precondition_stale` re-approvable card (distinct from the never-auto-resolved
  fencing card, #6).
- **Future TODO — next-brief working set** — the **"Always allow" `policy_grant` standing-grant slice** (own
  cat-1 checkpoint); 6.3e per-hunk; the real `UdsGatewayPort`/`rpc_response` demux; `require_step_approval`
  bundled-plan step UI; actionable `safer_alt`; Q7-(B)/(C) cache-retry PARKED-for-user.

## How to invoke
1. **Read this brief end-to-end** — especially the **Cat-1 safety design** (the lead-ruled pins, NOT re-opened)
   + "Things to flag at Step 2.5" (data-sourcing, reject-card reuse, post-action presentation, commit layering).
2. Pre-flight: confirm you're on `track/ui` in the `NexusOps-ui` worktree, `cd ui`.
3. **Run `/tdd gatewaymodal_real`.**
4. Step 0 (Restate) — confirm against the Feature line + the cat-1 surface (consumer of the 043 rulings).
5. Step 1 (Identify files) — confirm against "Files expected to touch"; confirm how the modal obtains the
   seam/port (the Shell gateway context/prop).
6. **Step 2.5** — answer the 4 design questions (or defaults) + the **coverage map** mapping each cat-1 pin to
   its test; send the write-up; wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
7. Drive L1 (PolicyDecision shadow) → commit; then L2 (GatewayModal-real + precondition_stale card).
8. **Step 8** — run `security-reviewer` (invariant policy — REQUIRED this slice).
9. **Step 7.5** — `/wired GatewayModal` (the Shell → seam path is REAL this slice).
10. Step 9 — surface the cross-doc flags (PolicyDecision shadow, the consumer row, precondition_stale) + the
    VISUAL gate + the deferred "Always allow" policy_grant carry-forward + confirm all cat-1 pins green.
