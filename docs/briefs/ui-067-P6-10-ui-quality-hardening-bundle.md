# /tdd brief — ui_quality_hardening_bundle

## Feature
A bundled UI quality/hardening pass — four independent, low-risk polish/test items across the DiffReview, GatewayModal, and `gateway-uds` surfaces: (1) a null-safe PR-number chip, (2) an empty-reason deny client guard, (3) an absent-policy render-depth test, (4) a fixed-id `sample_preview` test fixture.

## Use case + traceability
- **Task ID:** P6.10
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (PR display), `§11.5` (Gateway-modal safety treatment), `§11.7` (honest states — never blank/fabricated), `§6.1` (the id-based wire: `deny(approval_id, reason)` + the crate RPC fixtures)
- **Related context:** the ungated quality/hardening queue (IMPLEMENTATION_PLAN.md "Currently in progress" + the ui-066 carry-forward triage). Origins: the L2-A review-lows (2026-06-14) + the 053 / L2-prep carries (2026-06-14). LESSONS [[16]]/[[17]]/[[24]] (the intent-seam consumer + frozen-row shadows). **None of these items changes an INV-SEC-1 / cat-1 invariant** — the daemon Gateway stays the sole mutation chokepoint; the deny guard is defense-in-depth client-side validation only.

## Acceptance criteria (what "done" means)
- [ ] **(item 1)** `DiffReview` renders **no bare `#` chip** when a PR item's `pr_number` is null — the `MetaChip` is null-safe (per the Step-2.5 Q1 decision), and the card still identifies the PR via the label (the `:485` `pr_id` fallback is unchanged).
- [ ] **(item 1)** a PR item **with** a `pr_number` still renders `#<n>` in the chip (the happy path is preserved).
- [ ] **(item 2)** a deny submitted with a **whitespace-only** reason does **not** send the whitespace to the wire — the daemon receives the explicit default reason, never a blank/whitespace audit reason (defense-in-depth; the daemon is the chokepoint).
- [ ] **(item 2)** a deny submitted with a **real** reason still sends that reason verbatim (the existing `"not now"` behavior is preserved).
- [ ] **(item 3)** `GatewayModal` rendered with an **empty `PolicyDecision`** (no required approvals, empty reasons, null safer_alt) shows "no further approval", renders **no** reasons block and **no** safer-alt block, and does not crash — the `:134`/`:138`/`:141` branches are pinned.
- [ ] **(item 4)** `sample_preview()` (and `sample_action_request()`) return a **fixed, deterministic** `action_request_id` (`ActionRequestId::parse("act_<ULID>")`), not `::new()` — two calls produce equal ids; the existing crate tests still pass.
- [ ] All unit tests in `ui/src/views/code/DiffReview.test.tsx` + `ui/src/overlays/GatewayModal.test.tsx` pass.
- [ ] All crate tests in `ui/gateway-uds/` pass (`cargo test -p nexusops-gateway-uds`).
- [ ] `/preflight` clean (oxlint + tsc + vitest); `cargo test -p nexusops-gateway-uds` green.

## Wiring / entry point (Step 7.5)
All items are behind **already-wired production surfaces** — no new entry point:
- item 1 → the `DiffReview` PR-list (the PRsTab Kanban, reached from the live cockpit; `DiffReview.tsx` is rendered by the Code view).
- items 2/3 → `GatewayModal` (the live approval card; reached from the ApprovalQueue / per-hunk DiffReview approve-deny path — L2 go-live is COMPLETE).
- item 4 → a test fixture inside the `gateway-uds` crate (no production wiring; it hardens existing `#[test]`s).

## Files expected to touch
**Modified:**
- `ui/src/views/code/DiffReview.tsx` — null-safe the `MetaChip` at `:475` (item 1)
- `ui/src/views/code/DiffReview.test.tsx` — the null-pr_number chip test + the present-pr_number regression (item 1)
- `ui/src/overlays/GatewayModal.tsx` — trim the deny reason at `:93` (item 2)
- `ui/src/overlays/GatewayModal.test.tsx` — the whitespace-reason guard test (item 2) + the absent-policy render test (item 3)
- `ui/gateway-uds/src/lib.rs` — fixed-id `sample_preview()` / `sample_action_request()` + the fixed-id assertion test (item 4)

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)

**`ui/src/views/code/DiffReview.test.tsx`:**
1. **`pr_chip_null_safe_when_pr_number_absent`** — render the PR list with a PR item whose `pr_number` is `null`.
   - Asserts: no rendered chip element has textContent exactly `"#"` (no bare-`#`); the card still surfaces the PR identity via the label (the `pr_id` fallback at `:485`).
   - Why: `§11.2`/`§11.7` — honest display, null-safe like the label; the ui-061 nullable-`pr_number` reconcile.
2. **`pr_chip_shows_number_when_present`** — render with `pr_number: 101`.
   - Asserts: the chip renders `#101` (the happy path is preserved).
   - Why: `§11.2` — regression guard for the fix.

**`ui/src/overlays/GatewayModal.test.tsx`:**
3. **`deny_whitespace_reason_never_sent_as_blank`** — set the deny-reason input to `"   "` (whitespace-only) → click Deny.
   - Asserts: `port.deny` is called with the explicit default reason (`"Denied by the operator"`), **not** `"   "`.
   - Why: `§11.5`/`§6.1` — a blank/whitespace reason must never become the recorded audit reason (defense-in-depth; the daemon Gateway is the real chokepoint). **security-reviewer** slice.
4. **`absent_policy_renders_no_further_approval_gracefully`** — render with `policyDecision = { status, required_approvals: [], reasons: [], safer_alt: null }`.
   - Asserts: `policy-requirement` textContent contains "no further approval"; `policy-reasons` is absent; `policy-safer-alt` is absent; the modal renders (no throw).
   - Why: `§11.5` — pins the empty-policy branches at `:134`/`:138`/`:141` (modal render-depth).

**`ui/gateway-uds/src/lib.rs`:**
5. **`sample_fixtures_use_fixed_action_request_ids`** — call `sample_preview()` and `sample_action_request()`.
   - Asserts: each returns the **fixed** `action_request_id` (`ActionRequestId::parse("act_<ULID>").unwrap()`); two calls return equal ids (deterministic).
   - Why: `§6.1` — deterministic test fixtures; no random `::new()` id in a fixture.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — no contract/shadow field is added, removed, or renamed.
- **Orchestrator doc rows to write hot (Step 9 routing):** none anticipated. (The `GatewayModal` / `PrWorkspace` cross-doc rows in `ui/CLAUDE.md` already describe the surfaces; these are behavior-preserving hardenings.)
- **Shared-contract seam (cross-language Appendix-A) model touched?** No — no Appendix-A model field set changes; the `gateway-uds` fixture change is test-only and does not touch a frozen shape.

## Things to flag at Step 2.5
1. **(item 1) When `pr_number` is null, what does the chip render?** Options: **(a) omit the chip entirely** (the label at `:485` still carries identity via `pr_id`); (b) render `pr_id` inside the chip (parity with the label fallback); (c) a neutral placeholder (`#—`). My default vote: **(a) omit** — a `tone="pr"` chip is specifically a "#<number>" badge; with no number it's meaningless, and a long composite `pr_id` ("repo_1#101") reads poorly in a small chip. The label already identifies the PR.
2. **(item 2) Deny-reason guard — normalize or require?** Options: **(B) normalize the fallback** (`denyReason.trim() || "Denied by the operator"` — whitespace-only falls back; the Deny button stays enabled; the placeholder stays "reason (optional)"); (A) require a non-empty reason (disable Deny until `trim()` is non-empty — forces an auditable reason, but contradicts the existing "(optional)" placeholder / product decision). My default vote: **(B)** — it fixes the real gap (a whitespace reason reaching the audit) while preserving the established "reason optional" product decision. (A) is a product change; if the user wants reasons mandatory, that's their call — surface it, don't default to it.
3. **(item 3) "Absent policy" scope.** `policyDecision` is a **required** prop, so "absent" = the **empty** `PolicyDecision` (no approvals/reasons/safer_alt), exercising the `: "no further approval"` / `: null` branches. My default vote: **pin the empty-policy render** (a test-depth addition, no production change). Confirm this matches the intended interpretation.
4. **(item 4) Fix `sample_preview` only, or `sample_action_request` too?** Both currently use `ActionRequestId::new()`. My default vote: **fix both** (uniform deterministic fixtures) + one fixed-id assertion test. Minimal and consistent.

## Dependencies + sequencing
- **Depends on:** none — daemon-independent ui polish; all surfaces are live.
- **Blocks:** nothing. (Carry-forward cleanup; clears the ungated quality queue.)

## Estimated commit count
**1** — one logical "ui quality/hardening" slice (4 small, related polish/test items in the same `ui/` area; well under the bundle size heuristic; no safety invariant in the slice). Bisectability stays meaningful as one unit. The Rust fixture (item 4) is the only non-TS change and is ~2 lines + a test; if the implementer finds a clean cut, it MAY be a separate trivial commit, but one Step-10 commit is the default. **The deny-guard item (2) takes the `security-reviewer` at Step 8** (intent-seam touch; defense-in-depth, not an invariant change).

## Lessons-logged candidates anticipated
- **Convention candidate** — "a nullable display field gets the same null-safe treatment at EVERY render site, not just the primary label" (item 1: the chip lagged the `:485` label fix); and "client-side input that feeds an audited field is trimmed/normalized so whitespace can't become the recorded value" (item 2).
- **Future TODO — operational** — none anticipated.
- **Architecture-doc note candidate** — none anticipated (behavior-preserving hardenings; the surfaces are already documented).

## How to invoke
1. **Read this brief end-to-end.** Don't skip "Things to flag at Step 2.5" — items 1, 2, 4 each carry a real design choice.
2. **Run `/tdd ui_quality_hardening_bundle`** in the implementer session.
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line.
4. **Step 1 (Identify files)** — confirm the file list matches Files expected to touch.
5. **Step 2.5 (test review pause)** — ping back with answers to the 4 design questions (or take defaults). Don't proceed to Step 4 until orchestrator sign-off.
6. **Step 8** — dispatch the **`security-reviewer`** on the deny-guard slice (item 2 touches the intent-submission surface; defense-in-depth).
7. **Step 9 (summarize)** — surface anything that didn't fit the anticipated lessons-logged candidates; flag any security-reviewer finding as a **Finding** (the lead routes it to the user).
