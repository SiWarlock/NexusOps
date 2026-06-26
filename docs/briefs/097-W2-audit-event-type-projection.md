# /tdd brief — audit_trail_event_type (the WAVE-2 projection-honesty opener)

## Feature
Surface the raw machine `event_type` on the `proj_audit_trail` projection so the cockpit Audit tile's namespace-filter (e.g. `github.*` / `session.*`) + per-type icons work. Today the projector renders a redaction-safe human `headline` from the event type but **discards the raw `event_type`** — the column doesn't exist, and the row is served loose. Add the `event_type` column, persist it (the projector already has `env.event_type` — it's the `headline_for` input), and **freeze `AuditEventRow` as the 5th typed `shared/` projection-row served typed/fail-closed** (the §37 pattern; pays down the provisional→generated AuditEventRow debt) — a CONTRACT bump with a paired UI regen (LESSONS §69). **User-flagged.** WAVE-2.

## Use case + traceability
- **Task ID:** W2-audit
- **Architecture sections it implements:** `ARCHITECTURE.md §7`/`§7.2` (projections / the `proj_audit_trail` read model), `§6.1` (the typed `get_projection(AuditTrail)` serve), `§5.0` (Rust-authority contract → schema → Zod, the §2.5-seam), `§15` (the headline stays the redaction-safe render; `event_type` is a non-secret machine string).
- **Related context:** `daemon/src/projections/audit.rs` (`AuditProjector::apply` — a BLANKET projector: every event → one row; LESSONS §51 D4a; it already computes `headline_for(&env.event_type)` so `env.event_type` is in hand) · `daemon/src/eventstore/schema.rs:239` (`proj_audit_trail` DDL — has `headline`/`actor_label`/`sensitivity`/`scope_json`/`outcome`; the projector populates only event_id/seq/project_id/occurred_at/headline/actor_label/sensitivity — `scope_json`/`outcome` are always NULL today) · the **frozen-typed-row precedents** `shared/src/projections.rs` (ApprovalQueueRow/PullRequestRow/SessionRow/ReviewRow — `deny_unknown_fields`) + `daemon/src/ipc/methods.rs` (`read_*_typed`; AuditTrail currently falls through the generic loose `read_table_as_json`) · the migration precedent `MIGRATION_19_PROJECT_ACTIVITY_NAME` (ALTER + offset-reset so the projector re-folds historical rows). **Next numbers:** `MIGRATION_20`, `SUPPORTED_USER_VERSION` 19→20, `CONTRACT_VERSION` 0.48.0→**0.49.0**.
- **The UI need (cross-track):** the UI's provisional `AuditEventRow` uses field NAMES `actor_type`/`summary`, but the daemon serves `actor_label`/`headline`, and there's NO `event_type` at all → the namespace-filter + icons can't work. This slice freezes the daemon's REAL shape (`headline`/`actor_label`/`event_type`); the paired UI slice (ui-orchestrator) regenerates the Zod + reconciles consumers (`actor_type`→`actor_label`, `summary`→`headline`, consume `event_type`) + un-degrades the Audit tile (the ui-079 honest-banner).

## Acceptance criteria (what "done" means)
- [ ] **`MIGRATION_20`** ALTERs `proj_audit_trail` to add `event_type TEXT` (NULL-able for the ALTER; populated by the projector) + the **offset-reset** so a catch-up re-fold repopulates `event_type` for historical rows (the MIGRATION_19 precedent — historical audit rows should carry their type, not NULL). `SUPPORTED_USER_VERSION` 19→20. The historical `proj_audit_trail` CREATE (MIGRATION_3-era) stays UNCHANGED (editing it duplicate-column-fails a fresh DB — LESSONS §50).
- [ ] **`AuditProjector::apply`** persists `env.event_type` into the new column (INSERT + the `ON CONFLICT … DO UPDATE` set). Rebuild-safe: every event re-folds (blanket projector); rebuild-equivalent (LESSONS §4/§17). The headline path is UNCHANGED (still redaction-safe; `event_type` is the raw machine type, a non-secret).
- [ ] **Freeze `AuditEventRow`** in `shared/src/projections.rs` (the 5th typed row; `deny_unknown_fields`) — field set Step-2.5-locked, default `{event_id, seq, project_id: Option<String>, occurred_at, event_type, headline, actor_label: Option<String>, sensitivity}` (OMIT the always-NULL `scope_json`/`outcome` — the SessionRow retain-whitelist precedent; or include as `Option` if Step-2.5 prefers).
- [ ] **Serve typed/fail-closed:** `get_projection(AuditTrail)` branches to a new `read_audit_typed` (deserialize each row → `AuditEventRow` → serialize; a corrupt/mis-typed row fails the read closed — the LESSONS §37 pattern; the degradable Audit tile handles a read error via the UI's ui-079 honest-banner, so fail-closed is safe here). Test BOTH a populated row + the fail-closed arm.
- [ ] **CONTRACT 0.48.0→0.49.0** + the §2.5-seam schema snapshot + the 3-way verify (LESSONS §15 emission gotchas). **Paired UI regen is REQUIRED (LESSONS §69)** — the orchestrator coordinates it with ui-orchestrator the moment the `AuditEventRow` shape locks at Step-2.5.
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`get_projection(AuditTrail)` (`daemon/src/ipc/methods.rs`) → the new `read_audit_typed` (alongside `read_session_typed`/`read_review_typed`). The projector path is already production-reachable (the blanket fold runs on every event-commit txn; `/wired` the AuditTrail serve branch + the projector fold). No new `ProjectionName` (AuditTrail already exists — contrast Review/LESSONS §54).

## Files expected to touch
**Modified:**
- `daemon/src/eventstore/schema.rs` — `MIGRATION_20_AUDIT_EVENT_TYPE` const + register it in the migration list + `SUPPORTED_USER_VERSION` 19→20.
- `daemon/src/projections/audit.rs` — persist `env.event_type` (INSERT col + ON CONFLICT).
- `shared/src/projections.rs` — the `AuditEventRow` struct (+ the doc comment already lists it as a planned reconcile).
- `daemon/src/ipc/methods.rs` — `read_audit_typed` + the `get_projection` AuditTrail branch.
- `shared/contracts/schema/` (regen) + `shared/tests/contract.rs` (the AuditEventRow snapshot @0.49.0) + the 3-way verify.
- tests: `daemon/tests/projections.rs` (fold + rebuild-equivalence + typed-serve Some/corrupt) + the migration floor test (`gateway_plan.rs` exact-latest pin per LESSONS §50).

**New:** none expected.

## RED test outline (Step 2)
1. **`audit_trail_persists_event_type`** — append events of distinct types → each `proj_audit_trail` row carries the raw `event_type` (alongside the unchanged headline). Why: the core fold.
2. **`audit_trail_rebuild_repopulates_event_type`** — rebuild → historical rows carry `event_type` (the offset-reset re-fold), rebuild-equivalent. Why: LESSONS §4/§17 + the migration offset-reset.
3. **`read_audit_typed_round_trips`** — a populated table serves a `Vec<AuditEventRow>` (typed). Why: §6.1 typed serve.
4. **`read_audit_typed_fails_closed_on_corrupt_row`** — a row with a bad/mis-typed value → the read fails closed (`InternalError`), no partial/loose leak. Why: LESSONS §37.
5. **`migration_20_floor`** — `user_version >= 20` + the `event_type` column exists (the per-migration FLOOR test, LESSONS §50) + the ONE exact-latest runtime pin bumped.
6. **`audit_event_row_contract_snapshot`** (shared) — the frozen `AuditEventRow` shape @0.49.0 + `deny_unknown_fields` rejects an unknown field. Why: §5.0/§15.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field change (NEW frozen row):** `AuditEventRow` added to `shared/` → a cross-doc invariant. The implementer FLAGS at Step 9; the orchestrator writes the `daemon/CLAUDE.md` "MVP projections" row + the Appendix-A entry + the §7.2 note hot.
- **CONTRACT bump → paired UI regen (LESSONS §69):** 0.48.0→0.49.0 reds the UI generated-version drift test → the orchestrator coordinates the ui regen (gen-contracts.mjs) + the AuditEventRow consumer reconcile with ui-orchestrator. **The lead gates the push on ui-regen-green (the 0.48 arc precedent).**
- **§2.5-seam:** YES (a new shared projection-row) — snapshot-pin + 3-way verify.

## Things to flag at Step 2.5 (locks the contract shape + the ui coordination)
1. **Scope fork — (b) freeze-typed [DEFAULT] vs (a) minimal-loose.** Default **(b)**: freeze `AuditEventRow` typed + CONTRACT 0.49.0 + UI regen — matches the lead's contract-bearing framing + the 4-frozen-row trend + the planned AuditEventRow reconcile, and is the WAVE-2 "projection honesty" intent. **(a) fallback** (the user's original "just persist the column" framing): add the column + serve LOOSE (no shared/ freeze, NO CONTRACT bump, the UI updates its hand-written provisional Zod) — lighter, but leaves AuditEventRow unfrozen. Pick (b) unless a reason to defer the freeze surfaces. **This fork decides whether LESSONS §69 fires — flag it the moment you choose so I can pre-stage the ui coordination.**
2. **`AuditEventRow` field set** — include the always-NULL `scope_json`/`outcome` as `Option` (forward-looking) or OMIT them (the SessionRow retain-whitelist; cleaner). Default: OMIT (add when a producer populates them).
3. **Serve posture** — fail-closed typed (LESSONS §37, consistent with the 4 rows; the UI's ui-079 honest-banner degrades the tile gracefully on a read error) vs a lenient skip-bad-row (the audit tile is degradable, not safety). Default: fail-closed typed (consistent; the UI degrades, not the daemon).
4. **Migration offset-reset** (re-fold so historical rows get `event_type`) vs leave-NULL-for-historical. Default: offset-reset (the MIGRATION_19 precedent — old audit rows should show their type/icon too).

## Dependencies + sequencing
- **Depends on:** the existing `AuditProjector` + the typed-row serve pattern (`read_session_typed` etc.) + the migration/offset-reset precedent (MIGRATION_19). All landed.
- **Blocks / pairs:** the UI Audit-tile reconcile (ui-orchestrator — regen to 0.49 + AuditEventRow consumers + un-degrade the tile). **Coordinated at Step-2.5** (LESSONS §69); the lead push-gates on ui-regen-green.
- **Then:** the rest of WAVE-2 (UsageLedger `UsageRow` reconcile + `creditPool`; Plan/Team projectors).

## Estimated commit count
**1** daemon commit (the migration + projector + AuditEventRow freeze + typed serve + contract snapshot are one cohesive contract-bearing unit — the LESSONS §53 LOCKSTEP for an enriched typed row applies: column + fold + row-field + typed-serve + CONTRACT-bump are atomic). `code-quality-reviewer` per the `every-slice` policy; **security-reviewer is NOT mandatory** (no INV-SEC/safety surface — `event_type` is a non-secret machine string, the headline stays the redaction-safe render; run it only if Step-2.5 surfaces a §15 angle). The paired UI regen is a SEPARATE ui-track commit.

## Lessons-logged candidates anticipated
- Likely none NEW (this composes LESSONS §37 typed-fail-closed-row + §53 lockstep-enrichment + §69 contract-bump-pairs-UI-regen + §51 blanket-projector). If the offset-reset-for-a-blanket-projector has a wrinkle, flag it.

## How to invoke
1. Read this brief end-to-end (esp. Step-2.5 Q1 — the (b)-vs-(a) scope fork decides the contract bump + the ui pairing).
2. Run `/tdd audit_trail_event_type`.
3. Step 2.5 — ping back with the locked scope + AuditEventRow shape (I pre-stage the ui regen coordination on your answer).
4. Step 9 — flag the `AuditEventRow` cross-doc freeze + the CONTRACT bump for the orchestrator's hot docs + the ui-regen pairing.
