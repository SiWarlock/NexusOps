# ui-014 — the post-edges boundary merge (0.31→0.33) + the ui-058 Zod regen

- **Date:** 2026-06-15
- **Phase:** Phase 6 (ui-resume) — **P6.8 / §5.0** (the live `UdsGatewayPort` contract layer; the boundary-merge regen — same home as the 047/053 regens; NON-cat-1)
- **Predecessor:** [ui-013](ui-013-2026-06-14-l2-c-go-live-mutation-transport-live.md)
- **Successor:** [ui-015](ui-015-2026-06-15-approvalqueue-live-subscription.md)
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `team-lead`

## Why this session existed

The user merged the edges track into `main` (main → `95df2e0`, CONTRACT **0.33**: edges' `integration.connect` mutator + `ExecutorKind::Integration` ratified, plus the daemon's 0.32 `SessionFailed`). To keep `track/ui` current, the orchestrator ran the **main→ui boundary merge** (`f1bdf0d`) — which touched only `shared/` (the 0.33 frozen schema), leaving every `ui/` frontend file byte-identical to its pre-merge green state **except the §5.0 contract drift tests**, which read the schema and went RED. This session is the **0.31→0.33 regen-to-green** that restores the §5.0 drift gate — a single regen slice (ui-058), the same shape as the prior 053 (0.28→0.31) and 047 (0.28) regens.

## What was built (1 slice — 1 commit `ba95b3c`)

| Slice | Commit | What | reviewers |
|---|---|---|---|
| ui-058 — regen ui Zod contract 0.31→0.33 (boundary-merge; NON-cat-1) | `ba95b3c` | `pnpm gen:contracts` regenerated `generated.ts` from the merged 0.33.0 schema: `ExecutorKind` enum +`integration` (11→12 values), `CONTRACT_VERSION` 0.31.0→0.33.0. The §5.0 version tripwire in `mock.test.ts` bumped to match. | security-reviewer CLEAR · code-quality 1 low (carry-forward, not ui territory) |

### Files created
- `docs/sessions/ui-014-2026-06-15-boundary-merge-0.33-regen.md` (this doc).

### Files modified
- `ui/src/contracts/generated.ts` — **regenerated** via `pnpm gen:contracts` (never hand-edited; `ui/LESSONS.md` §1/§14). Delta = exactly 3 lines: the header `x-contract-version` comment 0.31.0→0.33.0, the `ExecutorKind` z.enum gaining `"integration"`, and `export const CONTRACT_VERSION = "0.33.0"`. No other enum `$def` changed.
- `ui/src/gateway-client/mock.test.ts` — the **§5.0 version tripwire** (`mock_get_capabilities_reports_contract_version`): the hardcoded `expect(caps.contract_version).toBe(...)` bumped 0.31.0→0.33.0 (mock.ts serves `contract_version: CONTRACT_VERSION`), the explanatory comment literal updated, and a `→ 0.33.0` changelog line appended.

## Decisions made

- **`SessionFailed` (new 0.32 object `$def`) is NOT shadowed — DEFER (Step-2.5 Q1).** The ui reads `proj_session` (where the `failed` status already exists + is mapped in `descriptors.ts`), not the raw `SessionFailed` event; no ui consumer exists yet (the session-card "Failed + restart affordance" recovery-UX is a future slice). Shadow-on-consume (`ui/LESSONS.md` §2) — a shadow with no consumer is dead surface. Carry-forward recorded (origin: this regen).
- **No bespoke per-value test for `integration`.** The dynamic `generated_zod_member_sets_equal_frozen_schema` pin already proves the value-set against the frozen schema; a hardcoded duplicate drifts (047/053 pattern). (Step-2.5 Q2.)
- **No consumer reconcile for the new `ExecutorKind` value.** Verified `ExecutorKind` is referenced ONLY in `generated.ts` (the schema object) — not re-exported in `index.ts`, not in descriptors/views, no `.options`/`.extract` completeness pin. `integration` flows through the generated bundle with no new consumer. (Step-2.5 Q3.)
- **The §5.0 tripwire bump was in-scope.** The brief's "Files expected to touch" listed only `generated.ts` and (wrongly) assumed all test drift-asserts were schema-dynamic; the orchestrator's **dispatch** correctly named "the §5.0 version tripwire" — and `mock.test.ts:97` IS exactly that (a hardcoded `contract_version` assertion, green pre-regen, so absent from the Step-2.5 RED). Bumping it is the tripwire's designed maintenance. Orchestrator confirmed at SHIP.

## Decisions explicitly NOT made (deferred)

- **The `SessionFailed` ui provisional shadow + its session-failure-UX consumer** — deferred to the future slice that builds the failed-session card / restart affordance (shadow-on-consume).
- **`Shell.uds-swap.test.tsx:126`'s `contract_version: "0.31.0"` fixture refresh** — left as-is; it is a never-compared mock handshake INPUT (production gates the handshake only on `protocol_version`, not `contract_version` equality — confirmed no prod `=== CONTRACT_VERSION`), so it stays green and is out of the regen's scope. An optional cosmetic refresh for a future session.
- **The `shared/` schema `ExecutorKind` `description`-field gap** (below) — daemon/shared territory, not ui.

## TDD compliance

**Clean.** This is a regen-to-green slice: the failing tests already existed (the §5.0 drift pins — version gate + `ExecutorKind` member-set + accept-canonical — went RED on the boundary merge). RED was confirmed for the right reason (stale 0.31 generated layer vs merged 0.33 schema) and isolated to `generated.test.ts` before regen; `pnpm gen:contracts` turned them GREEN. The `mock.test.ts` edit updates an existing tripwire assertion to match the bumped contract — not new implementation-before-test. No new test authored (the dynamic drift pins ARE the spec). No TDD violations; no safety-critical skips.

## Reachability (Step 7.5)

**No new reachable symbol** — this is a regen of the already-wired §5.0 contract layer. `ui/src/contracts/index.ts` derives the validators from the generated bundle, consumed at `ui/src/gateway-client/boundary.ts` (parse-don't-trust) and throughout the cockpit. The new `ExecutorKind` `integration` value flows through the generated bundle with **no new consumer** (verified — referenced only inside `generated.ts`). The `mock.test.ts` change is test-only. No `/wired` target; no tested-but-unwired gaps introduced.

## Open follow-ups (Step-9 categorized — already routed hot to the orchestrator)

1. **Cross-doc row (orchestrator writes hot at `/orchestrate-end`):** the `ui/CLAUDE.md` "Generated Zod contract layer" row → `CONTRACT_VERSION` 0.31.0→0.33.0; `ExecutorKind` 12 values (+`integration`); **enum-`$def` count UNCHANGED at 37** (`SessionFailed` is an OBJECT `$def`, not an enum); `SessionFailed` shadow-deferred.
2. **Carry-forward (Q1 defer):** the `SessionFailed` ui provisional shadow + its session-failure-UX consumer (origin: this regen).
3. **Carry-forward (NOT ui territory — code-quality low):** the frozen `shared/contracts/schema/*.json` `ExecutorKind` `description` field names only `adjudication` as the odd-one-out, not `integration` — a daemon/shared schema-authoring gap; the ui Zod enum itself is correct. For whoever next edits the shared `ExecutorKind` description.
4. **Observation (preflight gap — orchestrator surfacing to lead):** the ui `/preflight` Step-3 format-check (`pnpm prettier --check .`) is a structural **NO-OP** in the ui area — prettier is not a ui dependency (`pnpm prettier` → "command not found"; no `.prettierrc`; no root monorepo `package.json`). The earlier "All files formatted correctly" was an RTK-fabricated summary over the exit-1 error. `generated.ts` (a generated single-line artifact, never hand-edited) was never prettier-normalized — the pre-regen committed file is equally "different". Not introduced by this slice; the meaningful gates (oxlint, tsc, tests) all passed.
5. **Info (left as-is):** `Shell.uds-swap.test.tsx:126`'s never-compared `contract_version: "0.31.0"` mock handshake input (above).

## Cross-doc invariant audit

**Clean (multi-track memory check).** No frozen `shared/` model field changed this session — the ui authored no contract change; it consumed the daemon's frozen 0.33 schema. The generated value-set `ExecutorKind` grew by one **daemon-frozen** value, flagged at Step 9 (follow-up #1; orchestrator confirmed receipt + writing the row hot). No un-flagged drift.

## How to use what was built

The §5.0 drift gate is GREEN again at 0.33: the ui's parse-don't-trust Zod boundary now recognizes the daemon-frozen `integration` `ExecutorKind` value and reports `CONTRACT_VERSION` 0.33.0. No behavior change for the operator — the regen keeps the reject-unknown posture intact (security-reviewer CLEAR) and unblocks the Phase-7-UI work (7.2 PR Review Workspace / 7.3 Task Inbox), whose rich PR-projection contract is now present in `track/ui`.
