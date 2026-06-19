# /tdd brief — intent_seam_foundation

## Feature
Build the **intent-seam FOUNDATION** — the UI's **FIRST mutation/intent-submission
path**, **in isolation** (Scope A, lead-endorsed; no UI consumer wired). Add the
mutation-intent methods to the `GatewayPort` interface + `MockGatewayPort`, and a
**`canSubmitIntent`-gated submit-intent seam** (a hook + the request/result types)
that submits typed intents to the daemon's Action Gateway and surfaces the daemon's
response **without optimistic rendering**. The `GatewayModal` stays **DISABLED**
(slice 2 wires it). INV-SEC-1: the UI submits intents only, **NEVER executes**.

> **⚠️ CATEGORY-1 SAFETY SURFACE — the UI's first mutation path.** The lead ruled the
> 7 cat-1 safety questions (`docs/planning/intent-seam-cat1-safety-design.md` → "LEAD
> RULINGS"); **each ruled invariant is a PINNED TEST in the "Cat-1 safety design"
> section below** (these ARE the seam's safety contract, not prose). **`security-reviewer`
> is REQUIRED** (invariant policy). The real `UdsGatewayPort` transport + the live
> `ServerFrame.rpc_response` demux + the `GatewayModal`-real wiring are **SEPARATE later
> slices** — this slice builds + security-reviews the INV-SEC-1 seam ALONE.

## Use case + traceability
- **Task ID:** P6.3d
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (GatewayPort method
  surface), `§6.2` (intent→policy→approval→execute pipeline), `§4.2` (the 3 laws /
  INV-SEC-1), `§11.1` (read-only/degraded gate), `§11.5` (Gateway card semantics),
  `§11.7` (honest degradation), `§6.4` (IPC error codes), `§15` (INV-SEC-1) / `§17`
  (safety cards), forbidden #2.
- **Widens phase scope because** the intent seam is a **CLIENT of the §6 Gateway
  mutation contract + the §15/§17 safety invariants** — Phase-2/daemon anchors a
  Phase-6 UI surface now consumes (the parked Decision-C work, unblocked by the 0.23.0
  freeze + the boundary merge). Same cross-phase contract-consumption widen as
  040/041/042. *(The `§2`/`§4` Lesson/`#N` tokens elsewhere are LESSONS/template refs.)*
- **Related context:** the cat-1 rulings (`docs/planning/intent-seam-cat1-safety-design.md`);
  the parked Decision-C carry-forward; `ui/LESSONS.md §4` (fail-safe `canSubmitIntent`)
  + `§11` (degraded/safety states fail-closed) + `§13` (UI logic over state); the current
  `gateway-client/types.ts` (read-only `GatewayPort`) + `mock.ts` + the `GatewayModal`
  stub (Approve/Deny already disabled — "INV-SEC-1 stays daemon-side").

## Cat-1 safety design (each ruled invariant → a PINNED TEST) — LEAD-RULED
> Source: `docs/planning/intent-seam-cat1-safety-design.md` "LEAD RULINGS" (away-authority,
> logged for user return-review). Commit that file with this slice. The pins are the
> seam's safety contract — `security-reviewer` verifies them.

1. **INV-SEC-1 / Q1 (RATIFIED A):** the seam SUBMITS intents only (`submit_action`/
   `approve`/`deny`/`preview_action` via the `GatewayPort`) — **NO execution path**;
   mutation flows ONLY through the `GatewayPort` (never a direct FS/git/state write).
   **Pin `intent_seam_submits_only_no_executor`** — the seam's surface exposes only the
   intent methods + every mutation call routes through the single `GatewayPort` (forbidden #3).
2. **Fail-closed gate / Q2 (RATIFIED A):** the seam **REFUSES to submit when
   `canSubmitIntent` is false** (returns a typed read-only rejection; NEVER calls the
   `GatewayPort`). **DEFENSE-IN-DEPTH (non-negotiable, state it):** `canSubmitIntent` is
   **NOT the sole guard — the daemon Gateway (INV-SEC-1) is the real chokepoint**; a
   UI-enable bug must still be rejected daemon-side. **Pin
   `submit_blocked_when_canSubmitIntent_false`** — no `GatewayPort` mutation call when the
   gate is false.
3. **No-optimistic-render / Q3 (RATIFIED A):** the seam surfaces the daemon's
   `ActionAck.status`; **NEVER synthesizes "succeeded."** A "done" state comes ONLY from a
   confirming projection / `ActionResult` (a slice-2/projection concern). **Pin
   `submit_never_reports_done_from_ack_alone`** — the seam's result carries the daemon's
   reported status, never a UI-synthesized success.
4. **Policy-from-daemon / Q4 (RATIFIED A):** `approve`/`deny` submit an `Approval` per
   the frozen contract; the seam **NEVER computes its own risk/approval-requirement** — it
   passes through the daemon's `PolicyDecision`/`ActionPreview` (the 2.2 catalog-authoritative-risk
   ruling). **Pin `approve_deny_carry_daemon_approval_no_ui_risk`** — approve/deny carry the
   daemon's `approval_id`/`action_request_id`; no UI-derived risk/requirement.
5. **No-invented-preview / Q5 (RATIFIED A):** `preview_action` surfaces the daemon's
   `ActionPreview` (or `cannot_preview_reason`/pending); **NEVER fabricates "what will
   happen"** (forbidden #2). **Pin `preview_surfaces_daemon_preview_never_synthesized`.**
6. **§6.4 codes / Q6 (RATIFIED A):** the seam surfaces the daemon's `IpcErrorCode`
   (`WireError`) **VERBATIM**; never collapses/remaps it (the consumer routes each to its
   mandated §11.5 card in slice 2; `fencing_conflict` stays never-auto-resolved #6).
   **Pin `seam_surfaces_daemon_error_code_verbatim`.**
7. **No-caching / Q7 (RULED A for the seam; B/C PARKED-for-user):** the seam holds **NO
   intent state** across a disconnect — no cache, no auto-retry (the fail-safe do-nothing
   baseline). **Pin `seam_holds_no_intent_state_across_disconnect`.** **PARKED (not
   dropped):** cache+retry is a user-facing enhancement; **lead's recorded lean — if ever
   added it must be (C) MANUAL resubmit, NOT (B) auto-replay** (`idempotency_key` is
   dedup-safe but not consent-fresh; auto-acting after a gap is against human-gated-mutation).
   → carry-forward, consumer-marked.

## Acceptance criteria (what "done" means)
- [ ] `GatewayPort` (types.ts) gains the mutation-intent methods (`submit_action` /
      `preview_action` / `approve` / `deny`) — Promise-returning, typed against the frozen
      `ActionRequest`/`ActionAck`/`ActionPreview`/`Approval`/`Ack` contracts (no UI-invented shape).
- [ ] `MockGatewayPort` (mock.ts) implements them with **deterministic fixtures** (an
      `ActionAck` carrying a daemon-reported status; an `ActionPreview`; an `Ack`; a
      `WireError`/`IpcErrorCode` path).
- [ ] The submit-intent seam (`ui/src/intent/`) — a `canSubmitIntent`-gated hook + the
      result types; surfaces the daemon's ack/status/error **without optimism**; holds **no
      intent state**.
- [ ] **All 7 cat-1 pins green** (Cat-1 safety design above).
- [ ] `GatewayModal` stays **DISABLED** (NO wiring this slice — slice 2).
- [ ] Whole suite green; `/preflight` clean (oxlint + tsc + test:run).
- [ ] **`security-reviewer` PASS** (invariant policy — the isolated INV-SEC-1 seam).
- [ ] Cross-doc invariant flagged at Step 9 (the `GatewayPort` mutation surface added; the
      seam's INV-SEC-1 / fail-closed / no-optimism posture; Q7-(B)/(C) PARKED-for-user).

## Wiring / entry point (Step 7.5)
**none new — intentional (Scope A, lead-endorsed).** The seam is **exposed-ahead-of-consumer**
— built + security-reviewed in **ISOLATION**; the `GatewayModal` stays disabled; **slice 2**
wires the real approval card (Approve/Deny → `Approval` intents + render `PolicyDecision`/
`ActionPreview`). The `MockGatewayPort` is the §14-sanctioned test/dev seam; the real
`UdsGatewayPort` + the live `ServerFrame.rpc_response` id-correlation/demux are a **SEPARATE
transport slice**. Flag at 7.5 as expected (the benign 040/041/042 exposed-ahead pattern) —
NOT a wiring miss.

## Files expected to touch
**New:**
- `ui/src/intent/submit-intent.ts` (or `useSubmitIntent.ts`) — the `canSubmitIntent`-gated
  submit-intent seam + the result/request types.
- `ui/src/intent/submit-intent.test.ts(x)` — the 7 cat-1 pins + the seam-behavior tests.

**Modified:**
- `ui/src/gateway-client/types.ts` — `GatewayPort` gains the mutation-intent methods (typed
  against the frozen contracts).
- `ui/src/gateway-client/mock.ts` — `MockGatewayPort` implements them (deterministic fixtures).

If implementation needs files beyond this list (e.g. a small `intent/types.ts`), **flag at
Step 2.5** before GREEN. **Do NOT touch `GatewayModal.tsx`** (stays disabled — slice 2).

## RED test outline (Step 2)
The 7 cat-1 pins above (each a `spec(§…)`-tagged test) + the seam-behavior tests:
1. `intent_seam_submits_only_no_executor` — `spec(§4.2/§15)`.
2. `submit_blocked_when_canSubmitIntent_false` — `spec(§11.1/§15)`.
3. `submit_never_reports_done_from_ack_alone` — `spec(§4.2-law2/§11.7)`.
4. `approve_deny_carry_daemon_approval_no_ui_risk` — `spec(§6.2)`.
5. `preview_surfaces_daemon_preview_never_synthesized` — `spec(forbidden#2)`.
6. `seam_surfaces_daemon_error_code_verbatim` — `spec(§6.4)`.
7. `seam_holds_no_intent_state_across_disconnect` — `spec(§6.1-no-authoritative-state)`.
8. **`mock_gateway_mutation_methods_return_frozen_shapes`** — the `MockGatewayPort` fixtures
   parse against the frozen `ActionAck`/`ActionPreview`/`Approval` validators (no invented shape).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none to the frozen contract (the seam CONSUMES it). The
  `GatewayPort` interface gains the §6.1 mutation methods (a UI-client surface, not a frozen
  model). The seam's result types are UI-local.
- **Orchestrator doc rows to write hot (Step 9):** note the `GatewayPort` mutation surface +
  the intent-seam INV-SEC-1 posture in `ui/CLAUDE.md` (a new cross-doc row or the forbidden-#3
  note). No `ARCHITECTURE.md` edit (the §6.1/§6.2 surface is daemon-authored).
- **§2.5-seam model touched?** The seam consumes the §6.1/§6.2 §2.5-seam contracts (already
  drift-pinned by the generated/provisional tests). Pin #8 adds a fixture-parses-frozen-shape check.

## Things to flag at Step 2.5
1. **The seam's shape.** (A) a thin `canSubmitIntent`-gated `useSubmitIntent` hook over the
   `GatewayPort` (mirroring `ReadOnlyProvider`/`active-project` — Lesson §13); (B) an
   `IntentProvider` + hook (context-held); (C) a plain async service module. **Default vote:
   (A)** — minimal, mirrors the established read-side patterns, no context needed (the seam is
   stateless per Q7). Flip to (B) only if a consumer needs shared in-flight state (it doesn't — Q7=no-state).
2. **The result shape.** A typed **discriminated result** — `{ ok: ActionAck } | { error:
   WireError } | { readOnly: true }` — so a (slice-2) consumer renders honestly (no optimism,
   the read-only path explicit). **Default vote: yes, a discriminated union** (forces the
   consumer to handle each case; no silent success).
3. **The `ServerFrame.rpc_response` demux scope.** The mock returns Promises directly, so the
   **live id-correlation/demux is NOT exercised here** — it belongs with the real
   `UdsGatewayPort` transport slice. **Default vote: DEFER the live demux** — slice 1's seam
   sits ABOVE the `GatewayPort` (transport-agnostic); the seam's result types align with
   `ServerFrame.rpc_response`'s shape (contract-aligned) but the correlation lands with the
   real transport. Confirm.

## Dependencies + sequencing
- **Depends on:** 042 (landed `b6b7e7f`, 0.23.0); the **cat-1 rulings**
  (`docs/planning/intent-seam-cat1-safety-design.md`); the frozen Gateway mutation contract
  (P2 / 0.23.0, merged). Nothing else.
- **Blocks:** **slice 2** (`GatewayModal`-real — wire Approve/Deny + render the daemon's
  `PolicyDecision`/`ActionPreview`); the real **`UdsGatewayPort`** transport slice (+ the live
  `rpc_response` demux); **6.3e** per-hunk; and every later intent consumer (Dispatch, Brain
  Run-via-Gateway, restart-session, policy grants — §11.5).

## Estimated commit count
**1.** The isolated INV-SEC-1 seam. **SAFETY-CRITICAL → its OWN commit** (cat-1 surface, per
the "safety-critical pin gets its own commit" rule) **+ `security-reviewer` REQUIRED**
(invariant policy — this slice touches INV-SEC-1). Single logical unit; does NOT bundle.

## Lessons-logged candidates anticipated
- **Convention candidate** — the UI intent-submission seam pattern: `canSubmitIntent`-gated,
  pure-submitter (no execution path), no-optimistic-render (status from the daemon ack only),
  no-cache/no-state — with the daemon Gateway as the real chokepoint (the UI gate is
  defense-in-depth). The seam exposes the §6.1 mutation methods through the single `GatewayPort`.
- **Architecture-doc note candidate** — the `GatewayPort` UI-client mutation surface is now
  consumed (the §6.1 `submit_action`/`approve`/`deny`/`preview_action` methods); the INV-SEC-1
  UI boundary (submit-only).
- **Future TODO — next-brief working set** — **slice 2** (`GatewayModal`-real); the real
  **`UdsGatewayPort` + the `rpc_response` demux** transport slice; **Q7-(B)/(C) cache-retry
  PARKED-for-user** (lead lean: manual not auto); `precondition_stale` re-approvable stays the
  parked permission-card carry-forward.

## How to invoke
1. **Read this brief end-to-end** — especially the **Cat-1 safety design** (the 7 lead-ruled
   pins) + "Things to flag at Step 2.5" (3 design questions).
2. Pre-flight: confirm you're on `track/ui` in the `NexusOps-ui` worktree, `cd ui`.
3. **Run `/tdd intent_seam_foundation`.**
4. Step 0 (Restate) — confirm against the Feature line + the cat-1 surface.
5. Step 1 (Identify files) — confirm against "Files expected to touch" (do NOT touch GatewayModal).
6. **Step 2.5** — answer the 3 design questions (or defaults) + the **coverage map mapping each
   cat-1 pin to its test**; send the write-up; wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
7. **Step 8** — run `security-reviewer` (invariant policy — REQUIRED this slice).
8. Step 9 — surface the cross-doc flag + confirm all 7 cat-1 pins green + the Q7 PARKED-for-user carry-forward.
