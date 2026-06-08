# /tdd brief — ui_foundation_contract_layer

## Feature
Stand up the `ui/` track foundation: the Vite + React 19 + Vitest + oxlint (strict-tsconfig) scaffold, a **generated Zod contract layer** (checked-in artifact regenerated from the frozen `shared/contracts/schema/nexusops-contract.schema.json`), the single **`gateway-client`** seam with a **parse-don't-trust boundary validator** + a **`MockGatewayPort`** read surface returning **contract-valid fixture projections** (§14 mandate). This is slice **6.1a** — the foundation every other Phase-6 screen mounts into; the shell + read-only degraded mode is the separate **6.1b** slice.

## Use case + traceability
- **Task ID:** P6.1a (split from 6.1; lead-approved split 6.1a foundation → 6.1b shell)
- **Architecture sections it implements:** `ARCHITECTURE.md §5.0` (contract SoT & propagation — Option A: generated, drift-caught Zod consumer), `§4.2` (the three laws — UI reads projections, submits intents only, never writes the DB), `§6.1` (GatewayPort method surface), `§7.2` (source-of-truth; UI never treats a value as authoritative), `§14` (frontend testing strategy — **mock GatewayPort + fixture projections** are the sanctioned UI test seam), `§11` (projection-driven reattaching client).
- **Related context:**
  - Frozen contract is **enum-only** (`0.5.0`): 9 status machines + `ActorType` + `IdKind` + `DesktopObjectKind`. **`ExecutionProfile` is deliberately absent** (held for 0.5b) — do NOT hand-add it.
  - The **object models** (projection-row shapes, GatewayPort params/results, `ActionRequest`/`ActionPlan` objects) are **Appendix A prose, NOT yet a generated artifact**. The UI types them **provisionally** for now (see Step-2.5 Q2).
  - The proven gen command already lives in `shared/contracts/verify/run.sh` + `verify.py` (`npx json-schema-to-zod`).
  - Track rule (root `CLAUDE.md` + `MVP_TASKS.md §parallelization`): build against the **mock** + fixtures now; integrate the **real UDS** `GatewayPort` per-slice once the daemon-side contract (Phase 1.5) is live. **The real UDS transport (§6.4 framing/handshake) is OUT of 6.1a** — interface + mock only.

## Acceptance criteria (what "done" means)
- [ ] `ui/` scaffolds (Vite + React 19 + Vitest + oxlint + `tsconfig` `strict:true`); `pnpm typecheck` + `pnpm test:run` + `pnpm oxlint` (or the npm-equivalent per Step-2.5 Q4) all run clean.
- [ ] A **generated Zod contract layer** exists as a **checked-in artifact** (`ui/src/contracts/generated.ts`), regenerated from `shared/contracts/schema/nexusops-contract.schema.json` via a committed regen script (`ui/scripts/gen-contracts.*`) — reads the frozen schema **read-only**, never mutates `shared/`.
- [ ] **Every** canonical value of all 13 frozen value-sets parses; **every unknown value is rejected** (closed-enum / reject-unknown — `§5.0` pt 4, fail-closed `§15`/`§17`).
- [ ] A **drift test** pins the generated Zod enum members === the frozen schema `$defs.*.enum` arrays (mirrors the Rust CI schema-diff gate, `§5.0` pt 2) — reads the JSON file at test time, no `cargo` needed.
- [ ] The single **`gateway-client`** exposes a `GatewayPort` interface (mirroring the `§6.1` read methods) + a **boundary validator** that Zod-validates every projection payload (**parse, don't trust**) before it reaches view logic; malformed payloads (unknown status / missing field) are **rejected at the boundary**, never returned to callers.
- [ ] A **`MockGatewayPort`** implementing the `§6.1` **read surface** (`get_projection`, `subscribe`, `get_capabilities`) returns **contract-valid** fixture projections (every fixture status value is a member of the frozen `§5.1` enums).
- [ ] Provisional UI-local object types (`ui/src/contracts/provisional.ts`) are clearly marked `// PROVISIONAL — not frozen; reconciles when the daemon freezes object schemas`, and every enum-typed field references the **generated** Zod enums (no re-declared status string unions).
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5) — read honestly
6.1a is a **foundation slice**: its production-code deliverables (generated Zod layer, boundary validator, `GatewayPort` interface) are **reachable-by-6.1b** — the shell (the immediate next slice) mounts the `gateway-client` as the single daemon-access seam. Within 6.1a alone there is **no running production entry point yet** (no shell until 6.1b). This is the sanctioned "belongs-to-the-next-slice, not silently unreachable" situation (exactly like the daemon's 0.5 contract was reachable-by-Phase-1). The `MockGatewayPort` + fixtures are **§14-sanctioned test/dev infrastructure**, not dead production code.
- **Entry-point to name at Step 7.5:** the `gateway-client` `GatewayPort` seam (consumed by 6.1b shell). **Flag** that production wiring completes in 6.1b — an acceptable, tracked foundation gap, not an unreachable-code finding.
- The **real `UdsGatewayPort`** (same interface; `§6.4` transport) is a **later per-slice integration**, gated on daemon **1.5** going live — not in scope here.

## Files expected to touch
**New (scaffold):**
- `ui/package.json`, `ui/tsconfig.json`, `ui/vite.config.ts`, `ui/vitest.config.ts`, oxlint config, `ui/index.html`, `ui/src/main.tsx` (minimal mount stub — real shell is 6.1b) — the lockfile is per Step-2.5 Q4.
- `ui/scripts/gen-contracts.*` — regen Zod from the frozen schema (mirror `shared/contracts/verify/run.sh`).

**New (contract layer):**
- `ui/src/contracts/generated.ts` — **generated** (checked-in artifact); do not hand-edit.
- `ui/src/contracts/provisional.ts` — hand-authored, clearly-marked provisional object shapes.
- `ui/src/contracts/index.ts` — re-export the individual enum validators (`Session`, `Task`, …) + provisional types.

**New (gateway-client seam):**
- `ui/src/gateway-client/types.ts` — `GatewayPort` interface (the `§6.1` read methods for now).
- `ui/src/gateway-client/boundary.ts` — parse-don't-trust boundary validator.
- `ui/src/gateway-client/mock.ts` — `MockGatewayPort`.

**New (fixtures + tests):**
- `ui/src/projections/fixtures/` — fixture projection data (at least `proj_session`).
- `ui/src/contracts/generated.test.ts`, `ui/src/gateway-client/boundary.test.ts`, `ui/src/gateway-client/mock.test.ts`.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)

**`ui/src/contracts/generated.test.ts`:**
1. **`generated_zod_accepts_every_canonical_enum_value`** — for each of the 13 frozen value-sets, every member from the schema `$defs` `parse()`s successfully.
   - Asserts: round-trip accept for all canonical values.
   - Why: `§5.0` pt 3 (generated, drift-caught consumer); `§11.3` (status keys == `§5.1` verbatim, snake_case).
2. **`generated_zod_rejects_unknown_enum_value`** — `Session.parse("bogus")` / `ActorType.parse("hacker")` / etc. throw.
   - Asserts: closed-enum rejection (≥3 representative enums).
   - Why: `§5.0` pt 4 reject-unknown end-to-end; `§15`/`§17` fail-closed posture. **[safety-adjacent pin]**
3. **`generated_zod_member_sets_equal_frozen_schema`** — load `nexusops-contract.schema.json` at test time; assert each generated `z.enum(...).options` set === the corresponding `$defs.<T>.enum` array.
   - Asserts: zero generation drift vs the checked-in frozen schema.
   - Why: `§5.0` pt 2 (drift-gate pattern, the TS mirror of the Rust CI schema-diff).

**`ui/src/gateway-client/boundary.test.ts`:**
4. **`boundary_parse_accepts_valid_projection_payload`** — a well-formed fixture `proj_session` page passes the boundary parser and returns the typed value.
   - Asserts: valid payload → parsed/typed result.
   - Why: `ui/CLAUDE.md` typing posture ("every IPC payload Zod-validated at the boundary — parse, don't trust"); `§4.2` law 2.
5. **`boundary_parse_rejects_payload_with_unknown_status`** — a payload whose row carries a non-`§5.1` status (or a missing required field) is rejected at the boundary (error result / throw), never returned to view logic.
   - Asserts: malformed payload fails closed at the boundary.
   - Why: parse-don't-trust; `§15` fail-closed; `ui/CLAUDE.md` forbidden-patterns #2/#3. **[safety-adjacent pin]**

**`ui/src/gateway-client/mock.test.ts`:**
6. **`mock_get_projection_returns_contract_valid_fixtures`** — `mock.get_projection('Session', …)` rows all pass the generated Zod enum layer.
   - Asserts: fixtures never drift from the frozen enums.
   - Why: `§14` mock-GatewayPort mandate; prevents fixture/contract drift.
7. **`mock_subscribe_streams_validated_delta`** — `mock.subscribe({projection:'Session', …})` yields a delta that passes the boundary parser.
   - Asserts: subscription deltas validate end-to-end.
   - Why: `§11` projection-driven deltas; `§7` ("the UI subscription pushes deltas").
8. **`mock_get_capabilities_reports_contract_version`** *(light; keep unless it bloats the slice)* — `mock.get_capabilities()` returns `{ protocol_version, contract_version: "0.5.0", … }`.
   - Asserts: capabilities shape + frozen `contract_version`.
   - Why: `§6.4` handshake; feeds the 6.1b version-skew surface.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. The generated Zod layer is a **generated consumer** of the daemon-authored frozen schema — the UI introduces no new contract; the **drift test is the enforcement**.
- **Orchestrator doc rows to write hot (Step 9):** flag that the (currently-empty) `ui/CLAUDE.md` **Cross-doc invariants** table should get its **first row** — the generated contract layer mirroring `shared/contracts/schema` (`§5.0`/`§5.1`). Orchestrator writes it; implementer does NOT touch `ui/CLAUDE.md`. (6.2 then extends it with the status-enum / attention-rank coupling.)

> **Implementer never edits `ui/CLAUDE.md`, `ARCHITECTURE.md`, `MVP_TASKS.md`, or `ui/LESSONS.md`** — orchestrator territory. Flag at Step 9 categorized.

## Things to flag at Step 2.5
1. **Zod-gen output is one bundled schema; we need individual exported enum validators.** `json-schema-to-zod` emits a single object schema with the `$defs` inlined/nested. We need `Session`, `Task`, `ActorType`, … as **individually exported** `z.enum` validators. Options: (a) post-process the generated file to extract + export each `$def`; (b) generate per-`$def` separately; (c) generate the bundle and derive individual validators from it in `index.ts`. My default vote: **(c) generate the bundle as `generated.ts` (never hand-edited), then export the individual validators from `index.ts`** — keeps `generated.ts` a clean machine artifact and the drift test pins `.options` regardless. Confirm the generator actually preserves the `enum` members (the drift test will catch it if not).
2. **Object shapes not in the frozen schema (projection rows, GatewayPort params/results, `ActionRequest` object).** The 0.5 freeze is **enum-only**. Default vote: **hand-author minimal provisional TS types in `provisional.ts`, marked non-frozen, with all enum-typed fields referencing the generated Zod enums; carry-forward a reconcile item for when the daemon freezes object schemas (Phase 1/2 contract bump).** Alternative (block 6.1a until object schemas are frozen) — **rejected**: defeats the parallel-track plan; `§14` explicitly sanctions building the UI against the mock now. *If you find this actually forces a contract-surface decision (vs. a local test type), stop and flag it to me — that would be a category-4 escalation.*
3. **Real UDS transport — confirm it is OUT of 6.1a.** Default vote: **interface + `MockGatewayPort` only; `UdsGatewayPort` (`§6.4` framing/handshake) is a later per-slice integration gated on daemon 1.5.** This keeps 6.1a fully buildable against frozen contracts + mock. Flag if you think the interface can't be designed cleanly without the transport (I don't expect so).
4. **Package manager — `pnpm` is BROKEN locally (corepack signature-verification / keyid-mismatch bug; verified).** `node 22.13` + `npm 10.9.2` + `npx` all work; `json-schema-to-zod` runs fine via `npx`. The canonical stack is pnpm (`ui/CLAUDE.md` commands assume it). Options: (a) `npm install -g pnpm` to bypass the broken corepack shim → keep pnpm + `pnpm-lock.yaml` canonical; (b) fall back to `npm install` + commit `package-lock.json`, carry-forward a "reconcile to pnpm" item. My default vote: **(a) — restore canonical pnpm via a direct npm-global install** (lowest divergence; matches `ui/CLAUDE.md`). **Verify it works before committing a lockfile.** If (a) fails too, take (b) and flag at Step 9 which path landed so I update `ui/CLAUDE.md`/runbook. *(A global install is within normal setup latitude — take the default; only ping me if both paths fail.)*
5. **Test-file location.** Default vote: **co-located `*.test.ts`** next to source (standard Vitest). Flag if you prefer a `tests/` tree.

## Dependencies + sequencing
- **Depends on:** Phase 0.5 contract freeze (landed, `06f9576`) — the frozen `shared/contracts/schema/nexusops-contract.schema.json`. `NexusOps-ui-kit/` present (not consumed until 6.1b).
- **Blocks:** **6.1b** (shell + design-system integration + daemon-connection/read-only mode — mounts the `gateway-client`); **6.2** (status binding — extends the generated enum layer + the cross-doc table); all later Phase-6 screens (render via the mock + fixtures).
- **Later integration (not a blocker):** real `UdsGatewayPort` ← daemon **1.5** (UDS GatewayPort transport + handshake).

## Estimated commit count
**1–2.** One logical foundation unit. A clean split if it helps bisection: **(1)** scaffold + generated Zod layer + drift test; **(2)** `gateway-client` interface + boundary + `MockGatewayPort` + fixtures. No safety **invariant** is mutated (the reject-unknown pins consume the frozen contract; they don't define it), so a single bundled slice is acceptable — implementer's call on 1 vs 2 commits at Step 9.

## Lessons-logged candidates anticipated
- **Convention candidate** — "UI contract enums are **generated** from the frozen `shared/` schema (checked-in artifact + drift test), never hand-declared; status fields reference the generated validators." (Likely the first `ui/LESSONS.md` entry.)
- **Convention candidate** — "Object shapes not yet in the frozen schema are **provisional UI-local types**, explicitly marked, enum fields delegated to the generated layer; reconcile on the next contract bump."
- **Future TODO — operational** — corepack/pnpm fix (if the npm-global workaround is taken, track restoring a clean pnpm setup) → runbook.
- **Architecture-doc note candidate** — none expected (6.1a consumes frozen contracts; introduces no new architectural surface). If the GatewayPort interface needs a method shape the docs don't pin, that's a `§6.1` flag.

## How to invoke
1. **Read this brief end-to-end** — especially "Things to flag at Step 2.5" (5 pre-loaded questions; pnpm is the live one).
2. **First slice of the session →** run `/session-start` first (orient the implementer session), then `/tdd ui_foundation_contract_layer`.
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line (foundation only; shell is 6.1b).
4. **Step 1 (Identify files)** — confirm against "Files expected to touch"; verify the toolchain (Q4) at this step and surface blockers at Step 2.5.
5. **Step 2.5** — send the tight test-design write-up (one `Asserts: <invariant> (§anchor)` line per test) + your answers/defaults to the 5 questions. Wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
6. **Step 7.5** — name the `gateway-client` seam as the entry point; flag the foundation reachability gap (production wiring lands in 6.1b) honestly.
7. **Step 9** — surface the cross-doc first-row flag, the pnpm path taken, and anything outside the anticipated lessons-logged candidates.
