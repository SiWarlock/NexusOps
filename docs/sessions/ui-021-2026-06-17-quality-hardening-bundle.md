# ui-021 — UI quality/hardening bundle (ui-067 / P6.10)

- **Date:** 2026-06-17
- **Phase:** Phase 6 — **P6.10** (ungated quality/hardening queue; `ARCHITECTURE.md §11.2`/§11.5/§11.7/§6.1)
- **Predecessor:** [ui-020](ui-020-2026-06-17-graph-render-benchmark.md)
- **Successor:** _(none yet)_
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `team-lead`
- **Slice commit:** `59b3238` (`fix(ui): quality/hardening bundle — null-safe PR chip, deny-reason trim, test depth (P6.10)`)

## Why this session existed

A fresh team respawned after the ui-066 clean-boundary context cycle. The orchestrator dispatched the
**ungated quality/hardening queue** — four independent, low-risk polish/test items accreted as carry-forwards
across the L2-A review-lows (2026-06-14) and the ui-061 nullable-`pr_number` reconcile. None changes an
INV-SEC-1/cat-1 invariant; the daemon Action Gateway stays the sole mutation chokepoint throughout.

## What was built

**Files modified:**
- `ui/src/views/code/DiffReview.tsx` — **item 1**: null-safe the PR-number `MetaChip` at `:475`. A `tone="pr"`
  chip is a "#&lt;number&gt;" badge; when `pr_number` is null it is now **omitted entirely**
  (`{p.pr_number != null ? <MetaChip…> : null}`) instead of rendering a bare `#`. The `:485` label's `pr_id`
  fallback carries the PR identity (already-correct ui-061 behavior).
- `ui/src/views/code/DiffReview.test.tsx` — **item 1** tests: `pr_chip_null_safe_when_pr_number_absent` (dual-null
  fixture — `title:null` AND `pr_number:null` → asserts no bare-`#` chip + identity via the `:485` pr_id fallback,
  exercising the otherwise-untested `: p.pr_id` branch) + `pr_chip_shows_number_when_present` (happy-path regression).
- `ui/src/overlays/GatewayModal.tsx` — **item 2**: trim the deny reason at the submit seam
  (`denyReason.trim() || "Denied by the operator"`) so a whitespace-only reason can never become the recorded
  audit reason (defense-in-depth; the daemon is the chokepoint).
- `ui/src/overlays/GatewayModal.test.tsx` — **item 2** test `deny_whitespace_reason_never_sent_as_blank` (spies
  `port.deny`, asserts the default rides not `"   "`) + **item 3** test `absent_policy_renders_no_further_approval_gracefully`
  (empty-`PolicyDecision` render-depth pin of the `:134`/`:138`/`:141` branches; test-only, no production change).
- `ui/gateway-uds/src/lib.rs` — **item 4**: `sample_preview()` + `sample_action_request()` now use a fixed
  `ActionRequestId::parse("act_01ARZ3NDEKTSV4RRFFQ69G5FAV")` instead of `::new()`, so the test fixtures are
  deterministic; new test `sample_fixtures_use_fixed_action_request_ids`; `FIXED_AR_ID` const placed above its
  first use-site.

## Decisions made

- **Item 1 (Q1) = (a) omit the chip** when `pr_number` is null — a number-badge with no number is meaningless,
  and a composite `pr_id` ("repo_1#101") reads poorly in a small chip. The label already carries identity.
- **Item 2 (Q2) = (B) normalize-fallback** (`.trim() || default`) — fixes the real gap (a whitespace reason
  reaching the audit) while preserving the established "(optional)" product decision. The Deny button stays
  enabled; (A) mandatory-reason was surfaced as a product change, not taken.
- **Item 3 (Q3) = pin the empty-`PolicyDecision` render-depth** (test-only, no production change).
- **Item 4 (Q4) = fix both fixtures** for uniform deterministic ids + one fixed-id assertion test.
- **Step-9 reviewer nits #3 + #4 folded in-slice** (orchestrator-accepted): the `FIXED_AR_ID` const ordering
  and the tautological `getByTestId(...).toBeTruthy()` crash-guard.

## Decisions explicitly NOT made (deferred)

- **Nit #1 — zero-width chars survive `.trim()`** (U+200B/ZWNJ/ZWJ/U+2060): a non-whitespace-invisible deny
  reason could still ride. **Deferred** — pre-existing (identical in the old `denyReason || default`), cosmetic
  (audit-quality, not a bypass), and the daemon is the real reason-content guard. Carry-forward candidate.
- **Nit #2 — `queryByText("#")` forward-fragility** (DiffReview.test): correct as-written and proven RED→GREEN,
  but a fully robust assertion would need a `data-testid` on the **shared-kit** `MetaChip` (cross-track territory).
  **Deferred** / carry-forward.

## TDD compliance

**Clean.** All 4 RED-driving tests were written first and confirmed RED before the production fix
(item 1 `pr_chip_null_safe...` RED on the bare-`#` span; item 2 `deny_whitespace...` RED on `"   "` reaching the
wire; item 4 `sample_fixtures...` RED on two differing `::new()` ids). Item 3 (`absent_policy_renders...`) is a
deliberate **test-only render-depth pin** of existing behavior — GREEN-by-design, no production change (not a
TDD violation). No safety-critical logic shipped untested.

## Cross-doc invariant audit

**Clean — NONE.** No contract/shadow model field was added, removed, or renamed this session. The `gateway-uds`
fixture change is test-only and touches no frozen shape. (Multi-track memory check: no field change to flag at
Step 9; the orchestrator confirmed "cross-doc invariant NONE" in its SHIP routing.)

## Reachability

- **Item 1** (null-safe chip) — reachable from the live cockpit: the **Code view → `DiffReview` → "Pull requests"
  Kanban (`PRsTab`)** renders the PR cards. Pre-wired (ui-064); no new entry point.
- **Items 2/3** (deny guard / policy render-depth) — reachable from the **`GatewayModal`** (the live approval card,
  reached from the ApprovalQueue and the per-hunk DiffReview approve/deny path; L2 go-live COMPLETE). Pre-wired.
- **Item 4** (fixed-id fixtures) — `#[cfg(test)]` module-local helpers in the `gateway-uds` crate; no production
  wiring (hardens existing `#[test]`s). No reachability obligation.
- No tested-but-unwired gaps introduced.

## Open follow-ups

- **[carry-forward] nit #1** — zero-width-char normalization on the deny reason (pre-existing; daemon is the real
  guard; cosmetic). Optional belt-and-suspenders parity with the daemon's reason-emptiness check.
- **[carry-forward] nit #2** — a `data-testid` on the shared-kit `MetaChip` to de-fragilize the `queryByText("#")`
  null-chip assertion (cross-track — touches `NexusOps-ui-kit`).
- **[orchestrator, rides `/orchestrate-end`]** lessons §32 (null-safe at EVERY render site, not just the primary
  label) + §33 (client input feeding an audited field is trimmed/normalized) to bank; no `IMPLEMENTATION_PLAN.md`
  / contract change. No material Finding (security-reviewer CLEAR).

## Gates

`vitest run` 393/393 (62 files) · `tsc --noEmit` clean · `oxlint` clean · `cargo test -p nexusops-gateway-uds`
29/29 · `cargo clippy` clean. security-reviewer **CLEAR** (item 2 — full §15/INV-SEC-1 invariant PASS).
