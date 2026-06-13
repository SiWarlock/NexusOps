# /tdd brief — linear_issue_richer_fields

## Feature
Extend the daemon-internal `LinearIssue` (+ its GraphQL query + `extract_issue` mapping) with Linear's
richer issue signals — **description, priority, team, created/updated timestamps** — beyond the current
minimal MVP-chip set. Forward-laying for the gated Task Inbox; **user-directed completeness** (the
struct doc comment names these "a later refinement"). Pure, deterministic, fixture-driven.

## Use case + traceability
- **Task ID:** P7.1 (in-lane — the Linear read model; the Task Inbox consumer is gated)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (Linear read client — the integration read
  model). *The derived Task (external_task, R-8) status is UNCHANGED by this slice (edges-013 owns it); the
  gated Task Inbox is the eventual consumer of these secondary signals — both are downstream context, not
  anchors this slice implements.*
- **Related context:** edges-014 (`6ebdc4e`, `LinearIssue` + `extract_issue` + the wire structs),
  edges-013 (status derivation — UNCHANGED, still the single status authority), edges-015
  (`7445ae7`+`581fa61`, the `ISSUE_QUERY` + `LinearGraphqlReadClient`). The `LinearIssue` doc comment
  (`integrations/linear.rs`) explicitly frames `description/team/priority/timestamps` as "a later refinement."

## Acceptance criteria (what "done" means)
- [ ] `LinearIssue` gains: `description: Option<String>`, `priority: Option<u8>` (0–4), a team field
      (shape per Step-2.5 Q1), `created_at: Option<Timestamp>`, `updated_at: Option<Timestamp>`. **All
      `Option`** — see "tolerant wire" below.
- [ ] `ISSUE_QUERY` is extended to request the new fields: `description priority team { id name key }
      createdAt updatedAt` (alongside the existing selection).
- [ ] `extract_issue` maps each new field (total/no-panic): `description` (null/absent → None); `priority`
      (Linear `Float` 0–4 → `Some(u8)`; out-of-range/absent → None); `team` (→ the Q1 shape; absent → None);
      `createdAt`/`updatedAt` (ISO-8601 `DateTime` → `Timestamp::parse`; malformed/absent → None — mirrors
      `CommitInfo.timestamp`'s "None only for an out-of-range/unparseable instant").
- [ ] **Backward-compatible:** the existing Linear extract/mapping tests stay GREEN — the new wire-struct
      fields are tolerant (`Option` + `#[serde(default)]`), so a fixture WITHOUT the new fields still
      deserializes (→ the new fields read `None`). The `state.type`-derived `status` (edges-013) + `state_name`
      + `assignee` are UNCHANGED.
- [ ] `LinearIssue` keeps `#[derive(… PartialEq, Eq)]` — so `priority` is `Option<u8>` (NOT `Option<f64>`;
      `f64` isn't `Eq`); the wire `IssueNode.priority` is `f64`, mapped to `u8` in `extract_issue`.
- [ ] `/preflight` clean.
- [ ] **Daemon-internal — no `shared/` change, no `CONTRACT_VERSION` bump** (`LinearIssue` is daemon-internal
      per its doc comment; confirmed `shared/` untouched).

## Wiring / entry point (Step 7.5)
No new entry point. The extended `extract_issue` + `ISSUE_QUERY` are reachable from the live read path
(`LinearGraphqlReadClient::fetch_issue`, edges-015) and the `LinearReadClient` trait. The Task Inbox
consumer (the eventual reader of the richer fields) stays **gated** (standing edges posture — read model
in-lane, consumer gated). No new dead-wiring gap.

## Files expected to touch
**Modified:**
- `daemon/src/integrations/linear.rs` — extend `LinearIssue`; extend the wire `IssueNode` (+ a `TeamNode`
  per Q1) with tolerant `Option`+`default` fields + `#[serde(rename = "createdAt"/"updatedAt")]` (camelCase,
  like the existing `state.type` rename); extend `ISSUE_QUERY`; extend `extract_issue`'s mapping.
- The Linear extract test file (confirm at Step 1 — `daemon/tests/linear_read_client.rs` is the
  `extract_issue`/`LinearIssue` home from edges-014) — RED tests for the new fields.

If implementation needs files beyond this list, **flag at Step 2.5**.

## RED test outline (Step 2)
In the `extract_issue`/`LinearIssue` test home (`daemon/tests/linear_read_client.rs` — confirm at Step 1):

1. **`extract_issue_maps_richer_fields`** — a full fixture (issue with description, priority `2`, team
   `{id,name,key:"ENG"}`, createdAt/updatedAt ISO-8601) → `LinearIssue` with each field populated
   (`description=Some`, `priority=Some(2)`, team mapped, both timestamps `Some(parsed)`).
   - Why: the user-directed completeness mapping (§9 read model — the Task's secondary signals).
2. **`extract_issue_richer_fields_absent_are_none`** — a MINIMAL fixture (only the existing
   id/identifier/title/url/state/assignee selection, NO new fields) → the new fields all `None`, and the
   existing fields (status via edges-013, state_name, assignee) UNCHANGED.
   - Why: backward-compat — tolerant wire deserialize; the old fixtures must still work.
3. **`extract_issue_priority_out_of_range_is_none`** — a fixture with `priority: 9.0` (or a fractional/bad
   value) → `priority = None` (total mapping; 0–4 only).
   - Why: total/no-panic over an unexpected wire value (the codebase's robust-parse posture).
4. **`extract_issue_malformed_timestamp_is_none`** — a fixture with a non-ISO-8601 `createdAt` → `created_at
   = None` (the rest still maps).
   - Why: `Timestamp::parse` failure degrades to None, never panics (mirrors `CommitInfo.timestamp`).

(If Q1 = nested team, add a `team` absent → None assertion folded into test 2.)

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `LinearIssue` gains 5 fields — but `LinearIssue` is **daemon-internal** (no
  Appendix-A model, no `shared/` surface) → **no `CONTRACT_VERSION` bump, no cross-doc table row.**
- **Shared-contract (cross-track) seam model touched?** NO → no schema-snapshot test.
- **Orchestrator doc rows to write hot:** none to the table. ONE **§B arch-note candidate** (accumulate for
  the phase-exit PLAN-DELTA, like the other edges arch-notes): "the §9 Linear read model carries richer
  secondary signals — description/priority(0–4)/team/created+updated timestamps — all `Option`, total-mapped;
  status stays `state.type`-derived (edges-013)." Flag at Step 9.

## Things to flag at Step 2.5
1. **Team shape.** (A) a nested `LinearTeam { id: String, name: String, key: String }` (completeness — captures
   the full team identity) vs (B) flattened `team_key: Option<String>` (+ maybe `team_name`) to match the
   existing `assignee: Option<String>` flatten pattern. My default vote: **A (nested `LinearTeam`)** — the
   user directed completeness, and a team is a richer entity than the single-name assignee; the small struct
   is cheap. Flag if you prefer consistency-with-assignee (B).
2. **Priority representation.** `priority: Option<u8>` (the numeric 0–4 — sortable urgency) only, vs also a
   `priority_label: Option<String>` (Linear's `priorityLabel`, e.g. "Urgent"). Default: **numeric `Option<u8>`
   only** — the label is derivable from the numeric; keep it lean. (Also required for the `Eq` derive — see below.)
3. **Tolerant wire + `Eq` preservation.** Confirm the new `IssueNode` fields are `Option` + `#[serde(default)]`
   (so old fixtures deserialize), and `LinearIssue.priority` is `Option<u8>` not `Option<f64>` (so `Eq` holds).
   Default: **yes to both** — non-negotiable for backward-compat + the existing `Eq` derive.
4. **Exact Linear nullability.** Real Linear: `description: String` (nullable), `priority: Float!`,
   `team: Team!`, `createdAt/updatedAt: DateTime!` (non-null in live responses). We model ALL as `Option`
   anyway (tolerance + backward-compat with the minimal fixtures). Confirm the fixture shapes match Linear's
   real JSON (Context7 `/websites/studio_apollographql_public_linear-api_variant_current` if needed).

## Dependencies + sequencing
- **Depends on:** edges-014 (`LinearIssue`/`extract_issue`/wire structs), edges-013 (status derivation),
  edges-015 (`ISSUE_QUERY`/client). All LANDED.
- **Blocks:** nothing in-lane. Feeds the gated Task Inbox.

## Estimated commit count
**1.** One logical unit — extend the Linear read model with its richer fields (one file `integrations/linear.rs`
+ its test home); no safety invariant; all-`Option` additive.

## Reviewer posture (Step 8)
- **security-reviewer:** policy `invariant` → **SKIP** (no safety invariant; pure mapping; the api_key never
  enters `extract_issue`/the wire structs; total/no-panic). Note the skip.
- **code-quality-reviewer:** policy `every-slice` → runs on the slice diff (watch the priority Float→u8 bounds
  + the timestamp-parse totality).

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — the §9 Linear read model's richer secondary-signal set (for the §B
  PLAN-DELTA accumulation at phase-exit).
- Low expectation of a convention lesson (a faithful additive mapping extension).
