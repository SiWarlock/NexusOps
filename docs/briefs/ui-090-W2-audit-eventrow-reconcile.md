# /tdd brief — W2_audit_eventrow_reconcile (the W2-audit UI half)

## Feature
Reconcile the provisional `AuditEventRow` shadow to the daemon-frozen 8-field shape (CONTRACT 0.49.0),
reconcile the `AuditTrail` consumer to the renamed/added fields (`actor_type`→`actor_label`,
`summary`→`headline`, + `event_type`/`occurred_at`/`sensitivity`), and un-degrade the Audit tile — so
the cockpit's audit timeline renders the REAL daemon projection (event-type namespace-filter + icons live).

## Use case + traceability
- **Task ID:** W2-audit (the UI half — pairs with the daemon `W2-audit` #25 / CONTRACT 0.49.0).
- **Architecture sections it implements:** `ARCHITECTURE.md §7` (projections / §7.2 audit projection),
  `§5.0` (contract SoT + the regen drift gate), `§11.2` (the cockpit Audit tile), `§15` (the daemon
  serves redaction-safe `headline`/`actor_label` — never the raw payload).
- **Related context:** LESSON [[24]] (frozen projection-row → `.strict()` frozen-shadow, drift-pinned to
  `shared/src/projections.rs`; the `ApprovalQueueRow`/`SessionRow`/`PullRequestRow` precedents), [[40]]
  (provisional → frozen shadow discipline), [[38]] (ui-079 per-projection resilience — the Audit tile's
  honest degraded banner this slice un-degrades), [[8]] (projection-item id namespacing). Daemon side =
  daemon `W2-audit` (#25), daemon LESSONS §69 (paired UI regen).

### 🔒 LOCKED `AuditEventRow` shape (daemon-frozen at #25 Step-2.5; CONTRACT 0.48.0→0.49.0; relayed)
The 5th frozen typed projection row (`deny_unknown_fields` → the UI shadow is `.strict()`):
```
{
  event_id: string,
  seq: number,
  project_id: string | null,     // was .optional() → now nullable (present-but-null)
  occurred_at: string,           // NEW — the event timestamp (the seq-only placeholder's real value)
  event_type: string,            // already in the shadow; now actually SERVED (drives namespace-filter + icons; open set → plain string, NOT an enum)
  headline: string,              // RENAME from `summary`; now REQUIRED (daemon redaction-safe render, §15)
  actor_label: string | null,    // RENAME from `actor_type`; plain string NOT the ActorType enum (daemon wire-string render, forward-compat)
  sensitivity: string,           // NEW — plain string NOT the Sensitivity enum (forward-compat for the degradable tile)
}
```
**Reconcile notes (from the daemon-orch relay):** `actor_label` + `sensitivity` are plain `z.string()`
(do NOT bind to `ActorType`/`Sensitivity` Zod enums — the daemon serves wire-string renders); `scope_json`
/`outcome` are NOT served (always-NULL, dropped by the daemon retain-whitelist — must NOT appear in the
shadow, and `.strict()` rejects them); the serve is fail-closed typed → a daemon read error degrades the
tile via the ui-079 honest banner ([[38]]), so the Audit tile un-degrade lands here too.

## Acceptance criteria (what "done" means)
- [ ] **Regen** (gated — see Dependencies): `generated.ts` regenerated via `gen-contracts.mjs` → `CONTRACT_VERSION` 0.48.0→**0.49.0** (`x-contract-version` matches); the §5.0 drift test green; **value-set count HELD at 42** (`event_type`/`sensitivity`/`actor_label` are struct fields / plain strings, NOT new enum `$defs`).
- [ ] `AuditEventRow` shadow reconciled to the frozen 8-field shape above, **`.strict()`**, with `actor_label`/`sensitivity` as plain `z.string()` (unbound from the enums), `project_id` nullable, `headline` required; **field-set drift-pinned to `shared/src/projections.rs` `AuditEventRow`** (the [[24]] snapshot test).
- [ ] `AuditTrail.tsx` consumer reconciled: the row label renders `e.headline` (no `?? event_type` fallback — headline is required); the actor renders `e.actor_label` (with a null fallback); the timeline meta renders `e.occurred_at` (the real timestamp) where it showed `#{e.seq}` ("timestamps land with the daemon enrichment" — `occurred_at` IS it); the namespace-filter + `eventIcon` continue keying on `e.event_type` (now real).
- [ ] The local `ACTOR_LABEL` map is **REPLACED, re-keyed to the new actor WIRE values** (`human`/`system`/`agent`/`brain`/`pack`/`remote_client`/…), **tolerant of an unknown key → raw-string fallback**, UNBOUND from the `ActorType` Zod enum (Q1 RESOLVED); no reference to the removed `actor_type` field remains (tsc-enforced).
- [ ] `proj_audit_trail.ts` fixtures updated to the frozen 8-field shape (so the Mock + tests use the real shape); the boundary `parseProjectionPage("AuditTrail")` accepts the frozen row + rejects an unknown key (`.strict()`); **the Audit tile no longer degrades** (the shadow matches the served shape).
- [ ] A NEW `AuditTrail.test.tsx` pins the consumer reconcile (none exists today — a pre-existing test gap this slice closes).
- [ ] `/preflight` clean. Cross-doc invariant (the `AuditEventRow` drift row) flagged at Step 9 for the orchestrator.

## Wiring / entry point (Step 7.5)
The `AuditTrail` view is already mounted in the Shell (the cockpit Audit tile, fed by the Shell's
`get_projection("AuditTrail")` load + the ui-079 per-projection `settle()` wrapper). This slice changes
the SHAPE the existing wired path parses + renders — no new entry point. The un-degrade is reached the
moment the reconciled shadow matches the daemon's 0.49.0 served shape (the Shell's existing AuditTrail load).

## Files expected to touch
**Modified:**
- `src/contracts/generated.ts` — regen → 0.49.0 (NEVER hand-edit; via `ui/scripts/gen-contracts.mjs`; gated).
- `src/contracts/provisional.ts` — `AuditEventRow` → the frozen 8-field `.strict()` shadow.
- `src/contracts/provisional.test.ts` — the `AuditEventRow` 8-field/`.strict()` drift-pin (+ the daemon `projections.rs` field-set snapshot).
- `src/views/audit/AuditTrail.tsx` — the consumer reconcile (headline / actor_label / occurred_at / drop the enum-map+styling).
- `src/projections/fixtures/proj_audit_trail.ts` — fixtures → the frozen 8-field shape.
- `src/gateway-client/boundary.test.ts` (or the boundary test home) — the AuditTrail page accepts the frozen row + rejects unknown.

**New:**
- `src/views/audit/AuditTrail.test.tsx` — the consumer-reconcile pins (closes the pre-existing test gap).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
`src/contracts/provisional.test.ts`:
1. **`audit_event_row_frozen_8_field_strict_shadow`** — the shadow has EXACTLY the 8 frozen fields, `.strict()` rejects an unknown key (e.g. `scope_json`), `actor_label`/`sensitivity` accept arbitrary strings (NOT enum-validated), `project_id` nullable, `headline` required. Why: [[24]] frozen-shadow + the daemon `deny_unknown_fields`/plain-string notes.
2. **`audit_event_row_drift_pinned_to_projections_rs`** — the field-set snapshot matches `shared/src/projections.rs` `AuditEventRow`. Why: §5.0 drift discipline ([[24]]).

`src/views/audit/AuditTrail.test.tsx` (NEW):
3. **`renders_headline_actor_label_and_occurred_at`** — a fixture row renders its `headline` (the label), its `actor_label` (the actor), and its `occurred_at` (the timestamp), NOT seq-as-time. Why: the consumer reconcile.
4. **`actor_label_null_falls_back`** — a `actor_label: null` row renders the fallback (Step-2.5 Q1), never an empty/`undefined`. Why: nullable-field render discipline ([[32]]).
5. **`event_type_drives_namespace_filter_and_icon`** — filtering "Git" shows only `git.*` rows; a `git.*` row gets the git icon; an unknown namespace → the default Dot icon. Why: §11.2 + the real `event_type`.
6. **`actor_label_maps_known_and_falls_back_on_unknown`** — a KNOWN actor wire value (e.g. `remote_client`) renders humanized ("Remote Client"); an UNKNOWN value (outside the wire set) renders the RAW string (forward-compat, never fail-closed / never enum-rejected). Why: the daemon snake_case actor wire value + the UI-side tolerant map (Q1 RESOLVED).

`boundary` test:
7. **`audit_trail_page_accepts_frozen_row_rejects_unknown`** — `parseProjectionPage("AuditTrail")` accepts the frozen 8-field row + rejects an unknown key. Why: parse-don't-trust ([[22]]) + un-degrade.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `AuditEventRow` — `actor_type`→`actor_label` (+ unbind from `ActorType`), `summary`→`headline` (+ required), +`occurred_at`, +`sensitivity`, `project_id` nullable. **CONTRACT 0.48.0→0.49.0.**
- **Orchestrator doc rows to write hot (Step 9 routing):** the `ui/CLAUDE.md` cross-doc "Generated Zod contract layer" row gets a 0.49.0 regen note (value-set HELD at 42; the `AuditEventRow` shadow reconciled provisional→frozen, the 5th frozen row) + the `AuditEventRow` Appendix-A/projections row. I (orchestrator) write these hot.
- **§2.5-seam model touched?** `AuditEventRow` is a `shared/src/projections.rs` row (a §2.5 cross-language contract) → the RED outline INCLUDES the `projections.rs` field-set drift-pin (test 2), authored this cycle.

## Things to flag at Step 2.5
1. **`actor_label` rendering — RESOLVED (daemon-orch, authoritative from `audit.rs`).** `actor_label` is the **snake_case actor WIRE value** (`wire_value(&env.actor_type)` → `human`/`system`/`agent`/`brain`/`pack`/`remote_client`/…), NOT humanized — and a **DIFFERENT enum than the audit `ActorType`** (so binding to the `ActorType` Zod enum would fail-closed on `human`/`agent`/`brain`/`pack`). **Decision:** keep a thin **string-keyed snake_case→humanized map UI-side** (e.g. `remote_client`→"Remote Client", `brain`→"Brain"), **tolerant of an unknown key → fallback to the raw string** (a future actor value never fail-closes the tile), **UNBOUND from the `ActorType` Zod enum**. The current `ACTOR_LABEL` map is keyed on the WRONG (ActorType) values → **REPLACE it, re-keyed to the new actor wire values**. The per-actor cosmetic styling (`=== "user"|"project_brain"`) either re-keys to the new values (`human`→accent, `brain`→brain-ink) or drops — implementer's call (cosmetic; flag if dropping).
2. **`occurred_at` display format.** Absolute locale time vs relative ("2m ago"). Default vote: **absolute locale time** (matches the prototype's clock-time slot; keep `seq` as the canonical sort key — the sort stays `b.seq - a.seq`). The `#{seq}` placeholder + its "timestamps land with the daemon enrichment" tooltip are removed.
3. **Keep `sensitivity` rendered or hold it?** The daemon serves it but the current tile doesn't show it. Default vote: **carry it in the shadow but do NOT add a new UI surface this slice** (a sensitivity badge is a separate design step) — un-bind from the enum, render-later. Avoids scope creep.

## Dependencies + sequencing
- **GATED on the daemon `W2-audit` (#25) sealing CONTRACT 0.49.0** — both the regen source (`shared/contracts/schema/nexusops-contract.schema.json@0.49.0`) AND the drift-pin target (`shared/src/projections.rs` `AuditEventRow`) land with that seal. The daemon-orch pings the commit hash; the lead push-gates the round on ui-regen-green. **Do NOT dispatch until that seal.** (This brief is pre-staged against the LOCKED shape so dispatch is zero-latency.)
- **Blocks:** nothing downstream (the AuditTrail live-subscribe / seq-cursor delta is a SEPARATE deferred daemon ask — AuditTrail stays refresh-on-open per [[29]]; out of scope here).

## Estimated commit count
**1.** The shadow reconcile + consumer + fixtures + regen + the new test are one cohesive, non-bisectable
reconcile (the rename forces all sites together); NOT a safety-invariant slice (a read-projection reconcile)
→ bundle. code-quality reviewer per the every-slice policy; no security-reviewer trigger (no §15 invariant
authored — the daemon owns the redaction-safe serve).

## Lessons-logged candidates anticipated
- **Cross-doc invariant change** — `AuditEventRow` 0.48→0.49 reconcile (orchestrator writes the rows hot).
- **Convention candidate** — a daemon-rendered display field (`actor_label`/`headline`) is consumed as a
  plain forward-compat string (NOT bound to the source enum) so the degradable tile survives a daemon-added
  actor/value; the UI renders the daemon's render, never re-deriving identity from it.
- **Architecture-doc note candidate** — §7.2: `proj_audit_trail` serves the redaction-safe `headline` +
  `actor_label` + the raw `event_type` (for namespace-filter/icons) + `occurred_at`; `scope_json`/`outcome`
  retain-whitelisted out.

## How to invoke
1. **Read this brief end-to-end** — the LOCKED shape block + Step-2.5 Q1 (actor_label).
2. **Run `/tdd W2_audit_eventrow_reconcile`** AFTER the daemon 0.49.0 seal (the regen needs the frozen schema + the `projections.rs` drift target).
3. **Step 2.5** — ping the test-design write-up + the Q1 actor_label decision before GREEN.
4. **Step 9** — flag the CONTRACT 0.48→0.49 cross-doc invariant + the plain-string-forward-compat convention.
