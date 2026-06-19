# /tdd brief — regen_0280_and_diff_read_surface

## Feature
Delta-regenerate the UI's Zod contract layer from **0.23.0** to the now-merged
frozen **0.28.0** schema (the daemon Phase-4 round + the §6.3e freeze), clear the
re-armed **§5.0 drift tripwire**, and adopt the **6.3e contract surface**
exposed-ahead-of-consumer: the read-only **`get_diff` RPC** + the new **diff object
shapes** (`DiffResult`/`Hunk`/`DiffLine`/`GetDiffParams`) + the 3 **per-hunk
`git.*` action-type identifiers**. This is the **contract-adoption half** of 6.3e
— the actual per-hunk-button WIRING + diff render is the **next slice (6.3e proper,
cat-1, `security-reviewer` REQUIRED)**; this slice wires **no mutation path**.

> **This is the FIRST ui-resume slice after the `main@26c87a3` → `track/ui`
> boundary merge (`a154733`)** which brought CONTRACT 0.28.0 into the worktree.
> Bounded delta: **+3 enum value-sets, +1 enum member, +4 new object shapes, +1
> read RPC**. Every frozen object the ui drift-pins (ServerFrame variants,
> ActionRequest, PolicyDecision, ActionPreview, Approval) is **UNCHANGED at
> 0.24–0.28** (verified — zero field drift). The pattern is the 040/041 regen
> re-applied (Lesson §14).

## Use case + traceability
- **Task ID:** P6.3e (the regen/contract-adoption half; the wiring half is the next 6.3e slice)
- **Architecture sections it implements:** `ARCHITECTURE.md §5.0` (generated,
  drift-caught Zod consumer), `§6.1` (the `get_diff` `GatewayPort` method), `§6.3`
  (the MVP action-type catalog — the 3 new `git.*` per-hunk types), `§6.4` (IPC
  frame-mux + error codes — `IpcErrorCode::not_found`).
- **Widens phase scope because** it continues the same §5.0/§6.4 contract-resume
  widen briefs 040/041 already declared — re-cut against the 0.28.0 freeze — and
  additionally adopts the §6.1 `get_diff` read method + the §6.3 per-hunk action
  identifiers (daemon-authored contract surface the ui consumes; the §2.5 seam).
- **Related context:** brief `041-P6-2-delta-regen-0230-and-ui-ci-blocking.md` (the
  `validators`-from-`bundle.shape` self-maintenance + the `ServerFrame`
  discriminated-union + the uint64 field-type-pin mechanics this slice re-uses);
  **Lesson §14** (contract-bump discipline — the rule this slice re-applies); the
  boundary-merge commit `a154733`; the pause handoff
  `docs/team-handoffs/ui-002-2026-06-13-6tail-cross-track-pause.md` §2(i) (the
  6.3e unblock packet, now frozen); `ui/CLAUDE.md` Generated-contract +
  GatewayPort-method cross-doc rows.

## Current state (RED already present)
The boundary merge (`a154733`, `main@26c87a3` → `track/ui`) re-armed the drift
tripwire. **`ui/src/contracts/generated.test.ts` is RED on the committed tree:**
1. `generated_contract_version_matches_frozen_schema` — `CONTRACT_VERSION` `"0.23.0"`
   ≠ schema `x-contract-version` `"0.28.0"`.
2. `generated_zod_member_sets_equal_frozen_schema` — generated has **34** value-sets;
   the frozen 0.28.0 schema has **37** (+`DiffLineKind`/`ExecutionProfile`/`Provider`)
   — and `IpcErrorCode` is **9** generated vs **10** frozen (the new `not_found`).
3. (the missing-validator / accept-canonical assertion for the 3 new value-sets).

That RED **is** this slice's RED — the regen + the new provisional shapes drive it
GREEN. Whole-suite green (target **271 + the net-new pins**) is the GREEN bar.

## The delta (verified — schema $defs diff `27a2c34` vs the merged 0.28.0)
- **+3 GENERATED value-sets** (straight `gen:contracts`, no hand-edit):
  - `DiffLineKind` = `["context","added","removed"]` (§6.4 / the get_diff hunk lines).
  - `ExecutionProfile` = `["available","active","in_use","rate_limited","auth_expired","misconfigured","disabled","unknown","credit_exhausted"]` (the 0.5b 9-value runtime-state machine; **exposed-ahead** — the Settings ExecutionProfile tab is still gated).
  - `Provider` = `["github","linear"]` (edges-R1 P5/P7; **exposed-ahead** — 7.2 PR workspace is future).
- **`IpcErrorCode` +1 member: `not_found`** (9→10 — the get_diff "worktree/file not found" read-error; `precondition_stale`/`internal_error` were semantically wrong for it). An enum **member** add (regen auto-covers the validator), **but** see Q2 — every exhaustive match over `IpcErrorCode` must handle it.
- **+4 NEW object shapes** (objects are **NOT generated** — hand-modeled drift-pinned in `provisional.ts`, the established frozen-shadow pattern):
  - `DiffResult` = `{ hunks: Hunk[] }` — **1 field, required**.
  - `Hunk` = `{ header: string, old_start: uint32, old_lines: uint32, new_start: uint32, new_lines: uint32, lines: DiffLine[] }` — **6 fields, all required**.
  - `DiffLine` = `{ kind: DiffLineKind, content: string }` — **2 fields, required**; `kind` **delegates to the generated `DiffLineKind` validator** (don't re-literal it).
  - `GetDiffParams` = `{ worktree_id: string, file: string }` — **2 fields, required** (`worktree_id` is a `wt_…` frozen-22 id; resolves daemon-side via `proj_worktree.path`).
- **+1 read RPC method** on `GatewayPort`: `get_diff(worktree_id, file) → DiffResult` (§6.1; read-only git2 HEAD→workdir diff; daemon `methods.rs` dispatch already live).
- **+3 per-hunk action-type identifiers** (string identifiers only): `git.stage_hunk`, `git.unstage_hunk`, `git.discard_hunk`. **These are NOT a schema enum `$def`** — they live as Rust string consts in `shared/src/catalog.rs` (`MVP_ACTION_TYPES`), and the ui's `ActionRequest.action_type` is `z.string()`. See Q3 for whether/where to add a typed handle. **The ui adopts ONLY the string identifiers — never the risk class or `standing_grant_eligible`** (daemon-authoritative policy, cat-1 Q4: the UI renders the daemon's policy, never derives its own).
- **UNCHANGED (verified — zero field drift 0.24→0.28):** `ServerFrame` (3 variants, unchanged), `ActionRequest`, `PolicyDecision`, `ActionPreview`, `Approval`, `WireError`, `ProjectionDelta`, `DeltaKind`, every existing value-set's member set (besides `IpcErrorCode`). The 040/041/044 drift-pins stay valid.

## Acceptance criteria (what "done" means)
- [ ] `ui/src/contracts/generated.ts` is **REGENERATED via `pnpm gen:contracts`**
      (NOT hand-edited — Lesson §1/§14): `CONTRACT_VERSION === "0.28.0"`, **37**
      value-sets present (incl. `DiffLineKind`/`ExecutionProfile`/`Provider`;
      `IpcErrorCode` includes `not_found`).
- [ ] All three `generated.test.ts` drift checks pass.
- [ ] `index.ts`: **`validators` stays derived from `bundle.shape`** (Lesson §14) —
      auto-covers the 3 new value-sets + the `IpcErrorCode` member. Re-export the
      new enum(s) **only** where a typed consumer needs them (`DiffLineKind` is
      consumed by the `DiffLine` provisional shape — re-export per Q1); the other
      two are exposed-ahead (no consumer yet → no export unless one lands).
- [ ] `provisional.ts`: adds `DiffResult`/`Hunk`/`DiffLine`/`GetDiffParams` as
      **`.strict()` drift-pinned** object shapes (`DiffLine.kind` delegates to the
      generated `DiffLineKind`; the four `Hunk` offsets are `z.number().int().nonnegative()`
      per the uint32 frozen type).
- [ ] `gateway-client/types.ts`: adds `get_diff(worktree_id: string, file: string)
      → Promise<DiffResult>` to the `GatewayPort` interface.
- [ ] `gateway-client/mock.ts`: a fixture `get_diff` impl returning a **contract-shaped
      `DiffResult`** (NOT coupled to the `DiffReview.tsx` render fixture — see Q1).
- [ ] **`IpcErrorCode` 9→10 stays tsc-clean:** wherever `IpcErrorCode` is matched
      exhaustively (notably `safety/model.ts` `describeRejection`), `not_found` is
      handled — routed to the **honest-generic** treatment, **never a fabricated
      safety card** (it is a READ error, not a mutation rejection; forbidden #2 / Q5/Q6).
- [ ] **Drift-pin tests (the §2.5-seam schema-snapshot obligation)** in
      `provisional.test.ts`: field-set snapshots for `DiffResult`/`Hunk`/`DiffLine`/
      `GetDiffParams` == the frozen schema, + the `Hunk`-offset uint32 field-type pin.
- [ ] A `mock.test.ts` test that `get_diff` returns a schema-valid `DiffResult`.
- [ ] `mock.test.ts` version tripwire bumped `0.23.0`→`0.28.0` (if it pins the version).
- [ ] Whole suite green (the RED → green; **no regressions** — 271 + the net-new pins).
- [ ] `/preflight` clean (oxlint + tsc + test:run).
- [ ] Cross-doc invariant flagged at Step 9 (the Generated-contract row 34→**37**,
      `0.23.0`→`0.28.0`, `IpcErrorCode` 9→10, + the diff shapes + the `get_diff`
      method) — orchestrator writes the `ui/CLAUDE.md` rows.

## Wiring / entry point (Step 7.5)
**none new — wiring lands in the next slice (6.3e proper).** The regenerated layer
is consumed by **already-wired** boundary parsers (`gateway-client`,
`StatusPill`/`descriptors`, `safety/model`) + the drift test. `get_diff`,
`DiffResult`/`Hunk`/`DiffLine`/`GetDiffParams`, the 3 `git.*` action identifiers,
and the `DiffLineKind`/`ExecutionProfile`/`Provider` value-sets are **intentionally
exposed-ahead-of-consumer**: the **6.3e Code/Diff wiring slice** consumes the diff
shapes + `get_diff` + the action identifiers (it wires `DiffReview.tsx`'s disabled
per-hunk buttons to the 043/044 intent seam + sources diff content from `get_diff`);
`ExecutionProfile`/`Provider` are consumed by future Settings/PR slices. Flag at
Step 7.5 as **expected, not a wiring miss** — the 040/041 exposed-ahead pattern.
**No mutation entrypoint is added** (`get_diff` is read-only; the action identifiers
are strings, no submit path) → INV-SEC-1 stays daemon-side; this is the read/contract
layer only.

## Files expected to touch
**Modified:**
- `ui/src/contracts/generated.ts` — REGENERATED (`pnpm gen:contracts`); never hand-edited.
- `ui/src/contracts/index.ts` — re-export `DiffLineKind` (consumed by `DiffLine`); `validators` stays `= bundle.shape`. (Other two enums: export only if a consumer lands — flag at 2.5.)
- `ui/src/contracts/provisional.ts` — add `DiffResult`/`Hunk`/`DiffLine`/`GetDiffParams`.
- `ui/src/contracts/provisional.test.ts` — drift-pin the 4 new shapes (field-set + `Hunk`-offset uint32 type pin).
- `ui/src/gateway-client/types.ts` — add the `get_diff` method to `GatewayPort`.
- `ui/src/gateway-client/mock.ts` — fixture `get_diff` impl (contract-shaped `DiffResult`).
- `ui/src/gateway-client/mock.test.ts` — `get_diff` test + version tripwire bump.
- `ui/src/safety/model.ts` — handle `not_found` in `describeRejection` if exhaustive (route to generic).
- *(per Q3)* `ui/src/contracts/intent-contracts.ts` — a minimal provisional `git.*` per-hunk action-identifier handle (typing convenience; no risk/policy).

`Shell.test.tsx` / any fake-port stub may need the `get_diff` method added to satisfy
the `GatewayPort` interface — flag at Step 2.5 if so. If implementation needs files
beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
The version/member-set RED **pre-exists** (the drift checks). The slice makes them
GREEN and adds the seam drift-pins:

1. **(pre-existing) `generated.test.ts` ×3** — version / member-set / accept-canonical at 0.28.0.
   - Asserts: generated layer == frozen 0.28.0 schema (37 value-sets, version match, `IpcErrorCode` incl. `not_found`).
   - Why: §5.0 drift-caught consumer; Lesson §1/§14.
2. **NEW — `diff_shapes_field_sets_match_frozen_schema`** (`provisional.test.ts`).
   - Asserts: the field-name sets of `DiffResult`/`Hunk`/`DiffLine`/`GetDiffParams` == the frozen schema `$defs`.
   - Why: §6.1/§6.3 §2.5-seam — a daemon diff-shape change must fail loudly (the schema-snapshot obligation).
3. **NEW — `hunk_offsets_are_uint32`** (`provisional.test.ts`).
   - Asserts: `Hunk.old_start/old_lines/new_start/new_lines` are `z.number().int().nonnegative()` (not `z.string()`).
   - Why: Lesson §14 — a seam type adopted-ahead carries a field-**type** pin (the name-set pin missed `rpc_response.id`'s integer-vs-string once at 040).
4. **NEW — `mock_get_diff_returns_valid_diffresult`** (`mock.test.ts`).
   - Asserts: `mock.get_diff(wt, file)` resolves to a value that `DiffResult.parse()` accepts.
   - Why: §6.1 — the read method's fixture is contract-valid so 6.3e can consume it.
5. **NEW (only if Q2 surfaces an exhaustive match) — `describe_rejection_handles_not_found_generically`**.
   - Asserts: `describeRejection({code:"not_found", ...})` yields the honest-generic treatment, **not** a fencing/internal-error safety card.
   - Why: forbidden #2 / Q5/Q6 — a read-not-found must never render as a fabricated mutation-rejection card.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** value-set count **34→37** (+`DiffLineKind`/`ExecutionProfile`/`Provider`),
  `IpcErrorCode` 9→10 (+`not_found`), `CONTRACT_VERSION` `0.23.0`→`0.28.0`, +4 object
  shapes (`DiffResult`/`Hunk`/`DiffLine`/`GetDiffParams`), +1 `GatewayPort` method
  (`get_diff`). No new UI-authored invariant — the generated layer mirrors the daemon
  authority; the diff shapes are drift-pinned shadows of the frozen schema.
- **Orchestrator doc rows to write hot (Step 9 routing):** the `ui/CLAUDE.md`
  **Generated-contract row** (34→37, `0.23.0`→`0.28.0`, +`not_found`) + the
  **GatewayPort-method row** (+`get_diff` read method + the diff shapes). The
  `ARCHITECTURE.md` §6.1/§6.3/§6.4 + Appendix-A are **daemon-authored and already at
  0.28.0** (the merge brought them) — **no ui-side ARCHITECTURE edit needed**.
- **§2.5-seam model touched?** Yes — `DiffResult`/`Hunk`/`DiffLine`/`GetDiffParams`
  cross the daemon↔ui §6.1/§6.3 seam. The field-set test (RED #2) + the field-type
  pin (RED #3) are the schema-snapshot obligation — authored this cycle, reviewed at Step 2.5.

## Things to flag at Step 2.5
1. **`get_diff` fixture shape.** The `DiffReview.tsx` `diffFixture` is a *render*
   shape (kit `DiffHunk`), which may differ from the contract `DiffResult`. **My
   default vote:** the mock `get_diff` returns a **contract-shaped `DiffResult`**
   (its own small fixture), NOT the render fixture — keep the contract layer
   decoupled from the render layer (6.3e maps `DiffResult` → the render). Re-export
   `DiffLineKind` from `index.ts` so the `DiffLine` provisional can delegate to it.
2. **`not_found` in `describeRejection`.** Confirm whether `safety/model.ts`
   `describeRejection` switches **exhaustively** over `IpcErrorCode` (tsc will flag
   the new member if so). **Default vote:** route `not_found` to the **honest-generic**
   rejection treatment — it is a READ error, never a mutation-rejection safety card
   (fencing/internal_error stay distinct; forbidden #2). If get_diff errors are
   surfaced on a separate path (not via `describeRejection`), `not_found` just needs
   the exhaustiveness satisfied — no card at all.
3. **Per-hunk action-type handle — add now or defer to 6.3e?** The lead's sequence
   put "adopt the per-hunk git action types" in this slice. **Default vote:** add a
   minimal provisional `PerHunkGitActionType = z.enum(["git.stage_hunk",
   "git.unstage_hunk","git.discard_hunk"])` in `intent-contracts.ts` as a **typing
   convenience only** — it carries **no risk class, no `standing_grant_eligible`**
   (those are daemon-authoritative, cat-1 Q4; the ui renders the daemon's policy).
   Flip to "defer to 6.3e" only if you judge the handle premature without a consumer
   — but a typed identifier is harmless adoption and the lead scoped it here.
4. **Parked `MetricQuality` `oneOf`-of-`const` reconcile.** The daemon's 4.0b-T fix
   taught the §5.0 *verify* extractors about const-unions — but **`gen-contracts.mjs`
   (the ui generator) may still emit flat-`.enum` only**. **Default vote:** run
   `pnpm gen:contracts` and CHECK whether `MetricQuality` now appears in
   `generated.ts`. If yes → drop the provisional `MetricQuality` shadow + its
   drift-pin (consume the generated one). If no → **keep the shadow** and do **NOT**
   extend `gen-contracts.mjs` in this slice (the generator `oneOf`-const support is a
   separate Carry-forward follow-up — no scope creep here).

## Dependencies + sequencing
- **Depends on:** the boundary merge (**landed `a154733`** — 0.28.0 in `track/ui`);
  slices 040/041 (the 0.23.0 layer this delta-regens) + 043/044 (the intent seam +
  the `GatewayPort` mutation methods the diff method sits beside). Nothing else.
- **Blocks:** **6.3e proper** (the Code/Diff per-hunk wiring slice, cat-1,
  `security-reviewer` REQUIRED) consumes `get_diff` + the diff shapes + the per-hunk
  action identifiers; **6.7** (the §18 diff-open benchmark) measures the `get_diff`
  render path; future Settings/7.2 slices consume `ExecutionProfile`/`Provider`.

## Estimated commit count
**1 (optionally 2).** One logical "the ui contract layer → 0.28.0 + the 6.3e read
surface" unit. **Not safety-critical** — **no intent/mutation path is wired**
(`get_diff` is read-only; the action identifiers are strings with no submit path;
INV-SEC-1 stays daemon-side), so **no `security-reviewer`** and no own-commit safety
rule. The implementer MAY split the pure `generated.ts` regen (drives §5.0 green)
from the `get_diff`/diff-shape adoption for bisectability — both are additive and
share context. **NOTE for the orchestrator:** the **next** slice (6.3e proper — the
per-hunk-button wiring + diff render over the intent seam) **IS cat-1 and REQUIRES
`security-reviewer`**; this slice deliberately is not.

## Lessons-logged candidates anticipated
- **Convention candidate** — likely **none net-new**: Lesson §14 already captures the
  contract-bump discipline; this is its third re-application (after 040/041). A
  one-line "§14 re-validated against the 0.28.0 delta" Step-9 note, not a new lesson.
  *Possible* net-new: if the `MetricQuality` `oneOf`-const reconcile (Q4) resolves
  cleanly via the generator, note the generator now handles const-unions.
- **Architecture-doc note candidate** — the value-set count is **37 at 0.28.0**;
  `IpcErrorCode` is now **10-member** (`not_found` added); the `GatewayPort` read
  surface gains `get_diff`.
- **Future TODO — next-brief working set** — 6.3e proper (the per-hunk wiring + diff
  render over the intent seam) consumes everything this slice adopts; the
  **"Always allow" `policy_grant`** is its OWN cat-1 checkpoint (the orchestrator
  escalates BEFORE authoring if 6.3e reaches it); the `gatewayApprovalEnrichment`→
  real-daemon-projection swap + the real `UdsGatewayPort` transport stay parked.

## How to invoke
1. **Read this brief end-to-end** — especially "Things to flag at Step 2.5" (4 design questions).
2. Pre-flight: confirm you're on `track/ui` in the `NexusOps-ui` worktree, `cd ui`.
3. **Run `/tdd regen_0280_and_diff_read_surface`.**
4. Step 0 (Restate) — confirm against the Feature line.
5. Step 1 (Identify files) — confirm against "Files expected to touch".
6. **Step 2.5** — answer the 4 design questions (or take defaults) and send the
   test-design write-up (one `Asserts: <invariant> (§anchor)` line per test + the
   coverage map); wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
7. Step 9 — surface the cross-doc invariant flag (Generated-contract + GatewayPort-method
   rows) + the `MetricQuality`-reconcile outcome (Q4) + anything beyond the anticipated lessons.
