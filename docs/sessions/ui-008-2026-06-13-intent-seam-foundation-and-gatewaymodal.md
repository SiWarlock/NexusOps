# ui-008 — intent-seam foundation (043) + GatewayModal-real (044), atop the ui-resume contract regens (040–042)

- **Date:** 2026-06-13
- **Phase:** Phase 6 (ui-resume) — P6.2 contract regens · P6.4b usage model · **P6.3d the intent seam (cat-1)**
- **Predecessor:** [ui-007](ui-007-2026-06-09-prototype-faithful-styling-rebuild.md)
- **Successor:** _(none yet)_
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `ui-team-lead`

## Why this session existed

Freeze-resume of the paused ui track (the 2026-06-12 TCC lockout, cleared). The ui layer was paused at CONTRACT `0.12.0` while the daemon advanced `shared/` to `0.19.0` (Phase-2 Action Gateway) then `0.23.0` (Phase-3 harness/terminal/Claude). This session re-cut the ui contract layer against the frozen daemon contracts, then built the **first ui mutation path** — the INV-SEC-1 intent seam (043) and the live human-approval card over it (044). Five slices, two of them category-1 safety surfaces.

## What was built (by slice)

### 040 (P6.2) — contract regen 0.19.0 + bounded provisional reconcile — `e1e2730`
- Regenerated `ui/src/contracts/generated.ts` to `CONTRACT_VERSION 0.19.0` (20→33 value-sets) via `pnpm gen:contracts`; cleared the §5.0 drift tripwire.
- `index.ts`: the two Gateway-status `$def` **renames** (`ActionRequest`→`ActionRequestStatus`, `Approval`→`ApprovalStatus`); `validators` now **derives from the generated bundle** (`= shape`), never hand-listed.
- `provisional.ts`: retargeted the delegations + `AuditOutcomeStatus.extract`; adopted the §6.4 `ServerFrame`/`WireError` as drift-pinned provisional types.
- **Step-8 caught a real bug:** `ServerFrame.rpc_response.id` modeled `z.string()` but the frozen schema types it `uint64` integer → fixed test-first.

### 041 (P6.2) — delta-regen 0.23.0 + promote ui CI to blocking — `63ebb1d`
- Delta-regen 0.19.0→0.23.0 (33→34 value-sets): `+TerminalControlKind`, `ExecutorKind`+`adjudication`, `ServerFrame`+`terminal_output` variant (seq pinned uint64).
- Flipped the `.github/workflows/ci.yml` `ui:` job from advisory (`continue-on-error`) to a **blocking** merge gate (the §5.0 drift now clears). actionlint OK.

### 042 (P6.4b) — pool-kind-aware credit-pool state — `b6b7e7f`
- `creditPoolState(used, limit, kind)` is now kind-aware: `hard_stop` reachable **only** for `kind="sdk"` (capped monthly, no fallback); the auto-resetting `interactive` pool never hard-stops (§9.1 two-pool semantics). `CreditPool` gained a **required** `kind` discriminator.
- Lead-required `MetricQuality` drift-pin (frozen-but-generator-pending shadow).

### 043 (P6.3d) — intent-seam FOUNDATION (cat-1, Scope A isolation) — `a198af7`
**The UI's FIRST mutation/intent-submission path**, built in isolation (GatewayModal stayed disabled).
- **Files created:** `ui/src/intent/submit-intent.ts` (`createIntentSeam` + `useSubmitIntent`), `ui/src/intent/submit-intent.test.ts` (8 cat-1 pins + a field-set drift-pin + a re-throw pin), `ui/src/contracts/intent-contracts.ts` (provisional frozen-shadow shapes).
- **Files modified:** `gateway-client/types.ts` (+4 §6.1 mutation methods, **id-based wire** per `daemon/src/ipc/methods.rs`), `gateway-client/mock.ts` (impls + an injectable `WireError` path), `contracts/index.ts` (re-export), `contracts/provisional.ts` (`WireError` → `.strict()`), `shell/Shell.test.tsx` (fake-port stubs).
- The seam: `canSubmitIntent`-gated, **pure-submitter** (no executor), **non-optimistic** (surfaces the daemon's `ActionAck.status`, never synthesizes success), **§6.4 codes verbatim**, **stateless** (no cache/retry).
- **Notable:** the security-reviewer's suggested `.strict()` fix was insufficient (`Error`'s `message`/`stack` are non-enumerable, so a keys-only parse passes) → went deeper with an **`instanceof Error` discriminant** (a daemon frame is plain data, never an Error), proven by an adversarial re-throw pin that RED'd against `.strict()`-only.

### 044 (P6.3d) — GatewayModal-real (cat-1, 2 commits) — L1 `259801c` + L2 `0a05b27`
The first **live human-approval card** over the mutation path.
- **L1 (`259801c`):** the `PolicyDecision` provisional shadow in `intent-contracts.ts` (`.strict()`, field-set drift-pinned).
- **L2 (`0a05b27`):**
  - **Files created:** `ui/src/overlays/GatewayModal.test.tsx` (10 cat-1 modal pins + a preview-failed pin).
  - **Files modified:** `overlays/GatewayModal.tsx` (rewritten — wired Approve/Deny → the seam, renders the daemon's `PolicyDecision`+`ActionPreview`, `IntentResult` branches, daemon-status-never-done), `safety/model.ts` (new `describeRejection` — each §6.4 code → its distinct §11.5 treatment), `shell/display-meta.ts` (the `gatewayApprovalEnrichment` side-map), `shell/Shell.tsx` (the gateway-overlay mount), `overlays/overlays.test.tsx` (HIQ-opens-modal test updated for the wired card).
  - The net-new **`precondition_stale` re-approvable** card (regenerate preview + fresh approval); `fencing_conflict` **never re-approvable by construction** (#6); unmapped/transport codes → an honest `generic` rejection, never swallowed. "Always allow" `policy_grant` pinned **disabled** (deferred — own cat-1 checkpoint).

## Decisions made
- **A-lite rename decoupling (040):** the schema `$def` rename drives the contract exports/`validators`, but the UI status-machine identifiers (`"ActionRequest"`/`"Approval"`) stay UI-render-policy, bridged by a 2-entry alias. Drift-pin intact.
- **`validators` derives from `bundle.shape` (040):** the hand-list is what drifted at 0.12.0; deriving self-maintains across bumps.
- **Required `CreditPool.kind` (042, lead-ruled):** no silent default — tsc forces every construction site to declare, so a future interactive pool can't inherit `"sdk"` and false-alarm a hard-stop.
- **Intent-shape modeling in `contracts/` not `intent/` (043):** so `gateway-client` typing them stays a downward dependency.
- **id-based GatewayPort wire (043, orchestrator TWEAK):** verified against `daemon/src/ipc/methods.rs` — the wire takes ids; the higher-level seam accepts the daemon's rendered objects and extracts the verbatim id (Q4 provenance).
- **`instanceof Error` error-classification (043, deeper than the review):** the load-bearing discriminant for "daemon error frame vs real bug."
- **Q1=A data-sourcing (044):** the modal is a pure renderer of `{approval, policyDecision}` (a Shell side-map fixture pending the daemon projection) + fetches the `ActionPreview` via the seam on open.
- **`describeRejection` SEMANTICS reuse (044):** the §11.5 treatments rendered within the modal's reject state (not mounting the standalone projection cards).

## Decisions explicitly NOT made (deferred)
- **"Always allow" `policy_grant` standing-grant** — a distinct trust surface NOT covered by the Q1–Q7 rulings; its own cat-1 checkpoint (orchestrator escalates before authoring). Stays disabled.
- **Q7-(B)/(C) intent cache + auto-retry** — PARKED-for-user (lead's recorded lean: MANUAL resubmit, never auto-replay).
- **The real `UdsGatewayPort` transport + the live `ServerFrame.rpc_response` demux** — a separate transport slice; the seam sits above the GatewayPort, transport-agnostic.
- **`require_step_approval` bundled-plan step UI · actionable `safer_alt` · 6.3e per-hunk** — later slices.
- **The generator `oneOf`-of-`const` extension + MetricQuality provisional→generated reconcile** — a generator follow-up (042).

## TDD compliance
- **Proper RED→GREEN** on the behavioral drivers: 042 `credit_pool_interactive_exhaustion` (the current 2-arg impl returned hard_stop → RED), the 043 `seam_rethrows_non_wireerror` pin (RED'd against `.strict()`-only, proving the `instanceof` fix), the 044 L1 `policydecision_shadow_matches_frozen_field_set` drift-pin, the 044 `preview_failed_renders_honest_note` pin. The 040/041 RED **pre-existed** (the §5.0 drift checks).
- **Disclosed deviation (cat-1 slices 043 + 044-L2):** for these safety surfaces I drafted the seam/modal **concretely alongside the tests** (not strict test-first), to make the modeling/wiring reviewable as real code. This was stated transparently at Step 2.5; the **pins are the safety contract** and were the Step-2.5 review surface + were re-verified by the `security-reviewer` (PASS, all 7 invariants in both impl and pins). Not a silent back-fill — the pins were reviewed before approval. No safety-critical TDD was skipped.

## Reachability
- **040/041/042 (contract/model):** consumed by already-wired code (the `gateway-client` boundary parsers, `StatusPill`/`descriptors`, `safety/model`, `UsageDashboard`). New value-sets (`TerminalControlKind`/`ExecutorKind.adjudication`) + `ServerFrame`/`WireError` are exposed-ahead-of-consumer (the intent seam / 6.3d terminal well).
- **043 (intent seam):** intentionally **exposed-ahead (Scope A isolation)** — `GatewayModal` stayed disabled; consumed by 044.
- **044 (GatewayModal-real):** **REAL entry, `/wired` confirmed** — Shell gateway overlay (opened from `HumanInputQueue.onOpenApproval` + the pending-approvals palette) → `GatewayModal` Approve/Deny → `useSubmitIntent` seam → `GatewayPort`; preview fetched on open. No tested-but-unwired gaps.

## Open follow-ups (Step-9 categorized — orchestrator routes/routed; carry-forward for future-you)
- **Cross-doc rows (orchestrator territory, hot-routed):** the `ui/CLAUDE.md` Generated-contract row (20→33→34 value-sets, 0.19.0→0.23.0) + the GatewayPort-mutation-surface consumer row + the `PolicyDecision`/intent shadows. Lessons §14 (contract-bump), §15 (safety-gating discriminator), §16 (the intent seam), §17 (the consumer-side card) — orchestrator-written.
- **Carry-forward (consumer-marked):** the "Always allow" `policy_grant` slice (own cat-1 checkpoint) · **`gatewayApprovalEnrichment` → the real daemon projection-enrichment + preview/policy RPC at the `UdsGatewayPort` slice** (the security-reviewer [med] — a real human must not approve against fixture risk values) · the real `UdsGatewayPort`/`rpc_response` demux transport slice · 6.3e per-hunk · `require_step_approval` · actionable `safer_alt` · Q7-(B)/(C) cache-retry (PARKED-for-user) · the generator `oneOf`-of-`const` + MetricQuality reconcile.
- **Deferred quality nits (044):** `refreshPreview` unmount guard (React 19 no-ops unmounted setState) · the degraded-vs-loading risk-header token · the test `as const` `IpcErrorCode` typing.

## How to use what was built
- The intent seam: `createIntentSeam(port, canSubmit)` (pure, testable) or `useSubmitIntent(port)` (the hook) → `{ ok | error | readOnly }`. Approve/Deny accept the daemon's rendered `Approval`, extract the verbatim `approval_id`.
- The approval card: mounted at the Shell gateway overlay; renders daemon data only; `describeRejection(error)` is the single source for §6.4-code → §11.5-treatment routing.
- **For the live transport slice:** replace `gatewayApprovalEnrichment` with the daemon projection BEFORE a real human approves, and wire the real `UdsGatewayPort` (the seam + modal are transport-agnostic above the `GatewayPort`).
