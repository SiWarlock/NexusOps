# /tdd brief — delta_regen_0230_and_ui_ci_blocking

## Feature
Delta-regenerate the UI's Zod contract layer from **0.19.0** to the now-merged
frozen **0.23.0** schema (the daemon Phase-3 round: §9.1 HarnessAdapter, §6.4
Terminal Channel, §6.4/§7.1 Claude adapter), clear the **re-armed §5.0/§6.4 drift
tripwire**, and **promote the ui CI job from advisory (`continue-on-error`) to a
hard merge gate** (the §5.0 drift sentinel now clears with this regen).

> **This is the SECOND ui-resume slice — it consumes the 0.23.0 contract the
> 040→041 boundary merge (`d21d6ed`) brought into `track/ui`.** Bounded: the
> 0.19.0→0.23.0 enum delta is tiny (**+1 value-set, +1 member**) plus **one new
> §6.4 `ServerFrame` variant**; `WireError`/`IpcErrorCode`/`ProjectionDelta`/
> `DeltaKind` are **UNCHANGED** (verified). Smaller than 040 (no renames).

## Use case + traceability
- **Task ID:** P6.2
- **Architecture sections it implements:** `ARCHITECTURE.md §5.0` (generated,
  drift-caught Zod consumer), `§5.1` (status-machine enum strings), `§6.4` (IPC
  frame-mux + the Terminal-Channel `terminal_output` frame).
- **Widens phase scope because** it continues the same §5.0/§6.4 contract-resume
  widen brief 040 already declared — re-cut against the 0.23.0 freeze + the §6.4
  Terminal-Channel `terminal_output` frame (P3.4) the boundary merge brought.
- **Related context:** brief `040-P6-2-contract-regen-0190-reconcile.md` (the
  A-lite decoupling + `validators`-from-`bundle.shape` + the `ServerFrame`
  discriminated-union + the `id`-field-type pin mechanics it established);
  **Lesson §14** (contract-bump discipline — the rule this slice re-applies);
  the boundary-merge commit `d21d6ed`; the **§5.0-CI residual (i)** in
  Carry-forward (ui-job promote-to-blocking at the ui resume); `ui/CLAUDE.md`
  Generated-contract cross-doc row.

## Current state (RED already present)
The boundary merge (`d21d6ed`, main@`e77078a` → `track/ui`) re-armed the drift
tripwire. **4 tests are RED on the committed tree:**
1. `generated_contract_version_matches_frozen_schema` — `CONTRACT_VERSION`
   `"0.19.0"` ≠ schema `"0.23.0"`.
2. `generated_zod_member_sets_equal_frozen_schema` — generated has **33**
   value-sets; the frozen 0.23.0 schema has **34**.
3. `generated_zod_accepts_every_canonical_enum_value` — `TerminalControlKind`
   (+ the new `ExecutorKind.adjudication` member) have no generated validator yet.
4. `serverframe_variant_fields_match_frozen_schema` — the provisional
   `ServerFrame` has **2** variants; the 0.23.0 schema has **3** (the new
   `terminal_output`).

That RED **is** this slice's RED — the regen + the `ServerFrame` variant drive it
GREEN. Whole-suite green (target **216**) is the GREEN bar.

## The delta (verified via `jq` diff of the 0.23.0 schema vs `cc2cc78`'s 0.19.0)
- **+1 new value-set: `TerminalControlKind`** = `["pause","resume"]` (§6.4
  terminal control; consumed by the 6.3d terminal well).
- **`ExecutorKind` +1 member: `adjudication`** (10→11 — the 043 cat-1
  agent-mutation adjudication `ActionRequest`; an enum **member** add, not a new set).
- **`ServerFrame` +1 variant: `terminal_output`** =
  `{ frame_type: "terminal_output", terminal_id: string (opaque daemon-minted
  handle — NOT a frozen-22 ID; re-minted on resume), seq: uint64 (integer ≥0),
  data: string (base64-encoded raw PTY bytes) }` — **all 4 required**.
- **UNCHANGED (verified):** `WireError`, `IpcErrorCode`, `ProjectionDelta`,
  `DeltaKind`, the two 040 renames (`ActionRequestStatus`/`ApprovalStatus`), and
  every other value-set's member set. The scope is genuinely bounded.

## Acceptance criteria (what "done" means)
- [ ] `ui/src/contracts/generated.ts` is **REGENERATED via `npm run gen:contracts`**
      (NOT hand-edited — Lesson §1/§14): `CONTRACT_VERSION === "0.23.0"`, **34**
      value-sets present (incl. `TerminalControlKind`; `ExecutorKind` includes
      `adjudication`).
- [ ] All three `generated.test.ts` drift checks pass.
- [ ] `index.ts`: **`validators` stays derived from `bundle.shape`** (Lesson §14) —
      auto-covers `TerminalControlKind` + `ExecutorKind`'s new member; **no
      hand-listing, no `index.ts` edit** unless a new value-set gains a *typed
      consumer* (it does not — exposed-ahead-of-consumer). If the regen surfaces a
      consumed-enum need, flag at Step 2.5.
- [ ] `provisional.ts`: `ServerFrame` gains the **`terminal_output`** variant (the
      field-type modeling per Q1); the "Terminal-Channel tag space is RESERVED (no
      variant — a Phase-3 decision)" comment (`provisional.ts:298`) updated to
      "defined at 0.23.0 (P3.4 §6.4)".
- [ ] `serverframe_variant_fields_match_frozen_schema` passes (3 variants);
      `serverframe_variant_id_type_matches_frozen_schema` stays green; (per Q2) the
      field-type snapshot extends to pin `terminal_output.seq` (uint64).
- [ ] `ui/src/gateway-client/mock.test.ts` tripwire bumped `0.19.0`→`0.23.0`
      (if it pins the version).
- [ ] **ui CI job promoted to blocking:** remove `continue-on-error: true` from the
      `ui:` job in `.github/workflows/ci.yml` (~line 58) **and** update the advisory
      comment block (~lines 50–54) to record that the §5.0 drift now clears at the
      ui resume. **Sequence: regen GREEN first, then flip** (a flipped-but-failing
      gate would red the CI).
- [ ] Whole suite green (the 4 RED → green; **no regressions** — target 216/216).
- [ ] `/preflight` clean (oxlint + tsc + test:run).
- [ ] Cross-doc invariant flagged at Step 9 (the Generated-contract row: 33→**34**
      value-sets, `0.19.0`→`0.23.0`, + the `terminal_output` variant) — orchestrator
      writes the `ui/CLAUDE.md` row.

## Wiring / entry point (Step 7.5)
**none new** — same as 040. The regenerated layer is consumed by **already-wired**
boundary parsers (`gateway-client`, `StatusPill`/`descriptors`, `safety/model`).
`TerminalControlKind`, `ExecutorKind.adjudication`, and the `terminal_output`
`ServerFrame` variant are **intentionally exposed-ahead-of-consumer**: the **6.3d
Session Terminal well** (now **UNBLOCKED** — P3.4 is an ancestor of `track/ui`
post-merge, the decode gate lifted) consumes `TerminalControlKind` + the
`terminal_output` frame; the intent seam consumes `ExecutorKind.adjudication`.
Flag at Step 7.5 as **expected, not a wiring miss** — same pattern as the 040
exposed-ahead value-sets. The CI-promote is config (takes effect on the next push,
which is user-gated; locally it is a yaml-only edit).

## Files expected to touch
**Modified:**
- `ui/src/contracts/generated.ts` — REGENERATED (`npm run gen:contracts`); never hand-edited.
- `ui/src/contracts/provisional.ts` — add the `ServerFrame` `terminal_output`
  variant (`provisional.ts:301-317` discriminated union) + update the reserved-tag
  comment (`:298`).
- `ui/src/contracts/provisional.test.ts` — (Q2) extend the field-type snapshot to
  `terminal_output.seq` if decided.
- `ui/src/gateway-client/mock.test.ts` — tripwire `0.19.0`→`0.23.0` (if pinned).
- `.github/workflows/ci.yml` — remove `continue-on-error` on the `ui:` job + update the comment.

`index.ts`: expected **NO change** (`validators` derives from `bundle.shape`; no new
typed consumer). If implementation needs files beyond this list, **flag at Step 2.5**.

## RED test outline (Step 2)
The RED **pre-exists** (the 4 drift checks). The slice makes them GREEN. The only
**net-new** test is conditional:

1. **(pre-existing) `generated.test.ts` ×3** — version / member-set / accept-canonical at 0.23.0.
   - Asserts: generated layer == frozen 0.23.0 schema (34 value-sets, version match).
   - Why: §5.0 drift-caught consumer; Lesson §1/§14.
2. **(pre-existing, RED) `provisional.test.ts` `serverframe_variant_fields...`** — GREEN once the `terminal_output` variant lands.
   - Asserts: the `ServerFrame` variant tag-set + per-variant field-sets == the frozen `ServerFrame.oneOf` (now 3 variants).
   - Why: §6.4 §2.5-seam frame-mux; a daemon frame-shape change fails loudly.
3. **NEW — only if Q2 = "pin the new variant's field types":** extend
   `serverframe_variant_id_type...` (or add a sibling) to assert
   `terminal_output.seq` is the **uint64 numeric** type (`z.number().int().nonnegative()`).
   - Asserts: the `seq` field's Zod type matches the frozen `integer/uint64`.
   - Why: Lesson §14 — a seam type adopted ahead of its consumer carries a
     field-**type** pin (the field-name snapshot alone missed `rpc_response.id`'s
     integer-vs-string at 040).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** value-set count **33→34** (+`TerminalControlKind`),
  `ExecutorKind` +`adjudication`, `CONTRACT_VERSION` `0.19.0`→`0.23.0`, `ServerFrame`
  +`terminal_output` variant. No new UI-authored invariant; the generated layer
  mirrors the daemon authority.
- **Orchestrator doc rows to write hot (Step 9 routing):** update the **`ui/CLAUDE.md`
  Generated-contract row** (33→34, `0.19.0`→`0.23.0`, + the `terminal_output`
  variant). The `ARCHITECTURE.md` §6.4 / Appendix-A is **daemon-authored and already
  at 0.23.0** (the boundary merge brought it) — **no ui-side edit needed**.
- **§2.5-seam model touched?** Yes — `ServerFrame.terminal_output` crosses the
  daemon↔ui §6.4 seam. The field-set test (pre-existing) + the (Q2) field-type pin
  cover it.

## Things to flag at Step 2.5
1. **`terminal_output` field modeling.** The frozen variant is `{terminal_id:
   string, seq: uint64≥0, data: base64 string}`. **My default vote:** model
   `terminal_id`/`data` as `z.string()`, `seq` as `z.number().int().nonnegative()`
   — mirrors the established `rpc_response.id` uint64 pattern (the 040 [high] fix)
   + the frozen schema types. No enum here, so no generated delegation. (`data` is
   base64-encoded raw bytes — stays an opaque `z.string()`; decoding is the 6.3d
   terminal well's job, not the contract layer's.)
2. **Field-type pin for `terminal_output.seq`.** Lesson §14 says a seam type
   adopted-ahead carries a field-**type** snapshot pin. **Default vote:** extend the
   field-type assertion to `terminal_output.seq` (cheap; catches a daemon `seq`-type
   change). Flip to "the field-name-set pin is enough" only if you judge `seq`
   low-risk — but §14's whole point is that the name-set pin already missed a type
   mismatch once.
3. **CI-promote in this slice vs separate.** **Default vote:** flip
   `continue-on-error` **in THIS slice, after the regen is green** — the lead's
   forward sequence and the Carry-forward residual (i) both scope it into 041, and
   it bundles cleanly (one logical "the ui contract now matches → the sentinel goes
   live" unit). It's a yaml-only change validated on the next push (user-gated).
   Flip to a separate follow-up only if you want the CI flip bisectably isolated
   (the lead bundled it — default is together).

## Dependencies + sequencing
- **Depends on:** the 040→041 boundary merge (**landed `d21d6ed`** — 0.23.0 in
  `track/ui`); slice 040 (the 0.19.0 layer this delta-regens). Nothing else.
- **Blocks:** the **6-tail** — **6.3d** (Session Terminal well + inline permission
  card; now **UNBLOCKED**, P3.4 is an ancestor) consumes `TerminalControlKind` +
  the `terminal_output` frame; the **intent seam** consumes `ExecutorKind.adjudication`
  + the §6.3 Gateway enums. Promoting the ui CI to blocking **closes the §5.0-CI
  residual (i)**.

## Estimated commit count
**1.** One logical unit (delta-regen + the bounded `ServerFrame` variant + the
CI-promote). **Not safety-critical** — no intent/mutation path is wired (INV-SEC-1
stays daemon-side; this is the read/contract + CI-config layer), so **no
`security-reviewer`** and no own-commit safety rule. The lead scoped the CI-promote
into 041 → bundles cleanly.

## Lessons-logged candidates anticipated
- **Convention candidate** — likely **none net-new**: Lesson §14 already captures the
  contract-bump discipline, and this slice is its **first re-application** — the
  delta-regen self-maintained via `validators = bundle.shape` exactly as §14
  predicts (a pure `gen:contracts` run + one provisional variant, no `index.ts`
  churn). Worth a one-line "§14 validated in practice" note at Step 9, not a new lesson.
- **Architecture-doc note candidate** — the value-set count is **34 at 0.23.0**; the
  §6.4 `ServerFrame` is now **3-variant** (`terminal_output` defined, no longer reserved).
- **Future TODO — next-brief working set** — the 6.3d terminal well (now unblocked)
  consumes `TerminalControlKind` + `terminal_output`; the intent seam consumes
  `adjudication`; the `ProjectionName`→`ProjectionNameEnum` bare-name retirement +
  `ProjectionDelta.row` projection-discrimination stay flagged for those slices.

## How to invoke
1. **Read this brief end-to-end** — especially "Things to flag at Step 2.5" (3 design questions).
2. Pre-flight: confirm you're on `track/ui` in the `NexusOps-ui` worktree, `cd ui`.
3. **Run `/tdd delta_regen_0230_and_ui_ci_blocking`.**
4. Step 0 (Restate) — confirm against the Feature line.
5. Step 1 (Identify files) — confirm against "Files expected to touch".
6. **Step 2.5** — answer the 3 design questions (or take defaults) and send the
   test-design write-up; wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
7. Step 9 — surface the cross-doc invariant flag (Generated-contract row) + anything
   beyond the anticipated lessons.
