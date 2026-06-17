# ui-016 — the post-D-series boundary merge (0.33→0.38) + the ui-060 Zod regen-to-green

- **Date:** 2026-06-16
- **Phase:** Phase 6 (ui-resume) — **P6.8 / §5.0 + §5.1** (the generated, drift-caught Zod consumer of the frozen Rust schema — the boundary-merge regen, same home as the 047/053/ui-058 regens)
- **Predecessor:** [ui-015](ui-015-2026-06-15-approvalqueue-live-subscription.md)
- **Successor:** _(none yet)_
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `team-lead`

## Why this session existed

The daemon track finished the full **UI-unblock work order** (`docs/planning/daemon-unblock-work-order.md`, D1–D5 — PullRequestRow freeze, the survival fold, the live-delta emission, the rich PR + structured reviews), lead-verified on main@`3d0cba5` at CONTRACT **0.38.0**. The user directed the `main→ui` boundary merge. The orchestrator ran the merge (`2106864`, CONTRACT 0.33→0.38); that left the ui's generated §5.0 Zod layer **RED by design** — `generated.ts` still at 0.33 vs the merged 0.38 frozen schema. This session restores the §5.0 drift gate to GREEN (the ui-060 regen-to-green slice) so the now-unblocked feature arcs (Phase-7-UI / survival UI / whole-cockpit-live / Session refetch-on-nudge) can build against the 0.38 contract in their own later rounds.

The 0.33→0.38 ui-relevant delta is **enum-set + version only** (generated.ts is enum-only): a NEW `ReviewState` enum (5 values), `ProjectionName` gains `"Review"`, and `x-contract-version` 0.33→0.38. The new D-series **object** rows (`PullRequestRow`/`SessionRow`/`ReviewRow`/`ReviewSynced`) are deliberately NOT shadowed this round — shadow-on-consume, each consumed in its feature arc.

## What was built (1 commit — atomic regen-to-green)

| Commit | What | reviewers |
|---|---|---|
| `8c373a0` | `pnpm gen:contracts` regen of `generated.ts` (0.33→0.38) + the `mock.test.ts` §5.0 version tripwire bump (+changelog). The §5.0 drift gate restored to GREEN. | code-quality CLEAR (1 medium = the ui/CLAUDE.md cross-doc staleness → orchestrator-territory Step-9 flag) · security skipped per policy (NON-cat-1, no invariant/security surface) |

**Context — two orchestrator-territory merges this round (no `ui/src` impact):**
- `2106864` — the main→ui D-series UI-unblock boundary merge (CONTRACT 0.33→0.38). Touched only `shared/` + `daemon/` + `docs/`; the ui suite was byte-identical to its pre-merge green state except the schema-reading drift tests + the hardcoded §5.0 tripwire.
- `91c7d59` — the user's scaffolding upgrade (`491dfb02→7f60f251`) folded as a 2nd, **non-contract** merge after the ui-060 commit. Orchestrator territory; no `ui/src` change.

### Files modified (this session's slice — `ui/src` only)

- `ui/src/contracts/generated.ts` — **regenerated** via `pnpm gen:contracts` (never hand-edited, `ui/LESSONS.md` §1/§14). Exact delta: NEW `ReviewState` enum (`approved`/`changes_requested`/`commented`/`dismissed`/`pending`), `ProjectionName` +`"Review"` (10→11), `CONTRACT_VERSION` 0.33.0→0.38.0, header `x-contract-version`→0.38.0. The ONLY production file changed; 38 enum `$defs` (was 37).
- `ui/src/gateway-client/mock.test.ts` — the §5.0 version tripwire (`mock_get_capabilities_reports_contract_version`) `toBe("0.33.0")`→`"0.38.0"` + the explanatory changelog comment appended (D1 PullRequestRow 0.34 / D2 SessionRow recovery 0.35 / D5a mergeable+checks 0.36 / D5b-1 review vertical 0.37 / D5b-2 sync_reviews 0.38). Test-only; the tripwire's designed maintenance (GREEN pre-regen → NOT in RED → bumped at GREEN, ui-058 precedent).

### Files created

- `docs/sessions/ui-016-2026-06-16-boundary-merge-regen-0.38.md` — this session doc.

## Decisions made

1. **Regen-to-green, no new test authored.** The existing dynamic §5.0 drift pins ARE the spec (the 047/053/ui-058 pattern). The RED preexisted the implementation (RED confirmed before regen) — TDD-clean.
2. **All 3 RED pins are one root cause.** The version gate, the member-set + keys equality, AND `accepts_every_canonical_enum_value` were all RED pre-regen for the SAME 0.33→0.38 enum+version drift (the stale generated `ProjectionName` rejects the new canonical `"Review"`). The brief predicted RED = version + member-set only and called the accept-test "stays green"; that prediction was the miss (orchestrator confirmed the correction). All 3 go green on regen — clean, not an unanticipated consumer drift.
3. **Defer all 4 new object-row shadows** (`PullRequestRow`/`SessionRow`/`ReviewRow`/`ReviewSynced`) — shadow-on-consume (`ui/LESSONS.md` §2); generated.ts is enum-only so they never enter the regen anyway. Carry-forwards for the feature arcs.
4. **No bespoke per-value assertion** for `ReviewState`/`Review` — rely on the existing dynamic member-set pin (`generated_zod_member_sets_equal_frozen_schema` already proves both against the schema; a hardcoded duplicate drifts).
5. **No consumer reconcile this round.** Verified `ProjectionPageByName` is a hand-maintained **decoupled 6-key subset** of the frozen `ProjectionName` (it already omits ProjectGraph/PlanProgress/Worktree/AgentTeam), so `"Review"` does not trip it; `mock.ts` fixtures keyed by that subset are unchanged; `ReviewState` is exposed-ahead (no `index.ts` re-export until a typed consumer lands — the `ExecutionProfile`/`Provider` exposed-ahead pattern).

## Decisions explicitly NOT made (deferred)

- **The 4 object-row provisional shadows + their consumers** — `PullRequestRow`→Phase-7-UI · `SessionRow`→survival UI · `ReviewRow`/`ReviewSynced`+`ReviewState`→the review surface. Each freezes with its feature arc (own later round; lead/user picks the order).
- **The feature arcs themselves** — Phase-7-UI, survival UI, whole-cockpit-live, the Session UI refetch-on-nudge (the UI half of D3). Unblocked by this regen; not dispatched this round.
- **The `gen-contracts.mjs` `oneOf`-const generator extension** (retires the ResumeMode/RecoveryState/MetricQuality drift-pinned shadows) — the in-lane daemon-independent fallback, still parked.
- **The `/preflight` prettier-gate honesty fix** (ui-014 #4 — prettier is not a ui dep; `pnpm prettier --check` is a structural no-op masked as success). Pre-existing; not introduced here.

## TDD compliance

**Clean.** Regen-to-green: the §5.0 dynamic drift pins existed before the change and were confirmed RED (3 pins, one root cause) before `pnpm gen:contracts` turned them GREEN. No production behavior was authored ahead of a test. The `mock.test.ts` tripwire bump is designed test-only maintenance (the hardcoded served-version assertion), GREEN pre-regen so explicitly not part of the RED. No new test authored — by design, the dynamic pins are the spec. No safety-critical surface (NON-cat-1).

## Reachability

- **The regenerated §5.0 value-sets** — reachable from the existing contract layer: `contracts/index.ts` derives `validators` from the generated bundle (`= bundle.shape`), consumed at `gateway-client/boundary.ts` (parse-don't-trust) and throughout the cockpit. No new wiring; the regen restores the already-wired layer to 0.38.
- **`ReviewState`** — exposed-ahead, no consumer yet (no `index.ts` re-export until a typed consumer lands). Not a gap — the established exposed-ahead pattern; freezes with the review surface.
- **`ProjectionName.Review`** — flows through the generated bundle with no new consumer; `ProjectionPageByName` (decoupled subset) does not reference it. No new reachable symbol.

No tested-but-unwired gap. No `/wired` target (the wiring is the existing contract layer).

## Open follow-ups

Step-9 categorized list (already routed hot to the orchestrator during the session — it writes the doc rows at `/orchestrate-end`; listed here for continuity, NOT for me to re-route):

- **[Cross-doc invariant change — orchestrator writes hot]** `ui/CLAUDE.md` "Generated Zod contract layer" row is stale: `CONTRACT_VERSION 0.33.0 → 0.38.0`, enum-`$def` count **37→38** (+`ReviewState`), `ProjectionName` +`Review`, the 4 object rows shadow-deferred. (code-quality-reviewer flagged the same medium; territory rule — implementer does not edit `ui/CLAUDE.md`.)
- **[Future TODO — feature arcs]** the 4 object-row shadows + their consumers (defers #1 above). Origin: ui-060.
- **[Future TODO — future contract]** **D4b:** `proj_project` / `proj_repository` / `proj_integration_connection` have no `ProjectionName` variant → not live-subscribe-able without a contract change.
- **[Architecture doc note]** none — the ui implements no new §; the daemon owns the 0.38 contract.

## How to use what was built

The cockpit's Zod contract layer is now at **0.38.0**, drift-gate GREEN. Any feature-arc slice consuming the new daemon surface (`PullRequestRow`, `SessionRow` recovery fields, the review vertical) builds against the frozen 0.38 schema: add the provisional shadow in `provisional.ts` (shadow-on-consume, `ui/LESSONS.md` §2/§24), drift-pin it to `shared/src/projections.rs`, and re-export the relevant enum in `contracts/index.ts` when a typed consumer lands.
