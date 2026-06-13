# /tdd brief — project_rescan_executor

## Feature
Wire the **first real edges mutator-path Action**: a `ProjectExecutor` (`ExecutorKind::Project`) that runs
the existing detection engine for a `project.rescan` Action and emits the `ProjectRescanned` event through
the Gateway's in-txn §15 redaction gate — with the git `remote_url` credential userinfo **stripped at the
emit source**. Registers the executor into the now-delivered `CatalogExecutor` registration seam.

## Use case + traceability
- **Task ID:** P5.1
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (ActionTypeCatalog / executor dispatch),
  `§9` (project detection), `§7.2` (the read model the `ProjectRescanned` event feeds), `§15` (redaction —
  `remote_url` strip-at-source, rule #5/#3/#4).
- **Related context:** `docs/planning/edges-R5-wiring-plan.md` (Wave-A slice 1); the R1 seam +
  `ProjectRescanned` type landed via the merge `bd3ee31` (CONTRACT 0.26.0). Precedent: the
  `SessionExecutor` in-txn `EmittedEvent` path (`daemon/src/gateway/session_executor.rs` +
  `request::emitted_event_intent`). Detection engine: `daemon/src/git/detect.rs::detect_git` +
  `daemon/src/workflow/detect.rs::detect_workflow` (both already in-lane, fully tested).

## Acceptance criteria (what "done" means)
- [ ] `ProjectExecutor` implements `ActionExecutor`; `execute` for `project.rescan` runs `detect_git` +
      `detect_workflow` over the scan path and returns `ExecutionOutcome::Succeeded` with
      `side_effect_applied: false` (detection is read-only; the event is the audit record, not an external mutation).
- [ ] The outcome carries an `EmittedEvent` for `ProjectRescanned` whose payload maps the two detection
      structs into the frozen `ProjectRescanned` field set (`is_git, repo_root, remote_url, branch, detached,
      is_dirty, workflow_pack, cc_crew, plan_file, brain, scanned_at`); the pipeline appends it in txn-B,
      ATOMIC with `ActionSucceeded`, through the §15 gate.
- [ ] **SECURITY PIN (§15 rule #5):** `remote_url` userinfo (`user:token@`) is **stripped at the emit
      source** before the payload is constructed — `https://user:token@github.com/o/r` → `https://github.com/o/r`;
      a scp-style ssh remote (`git@github.com:o/r`) is left intact (no secret). The Redactor is the backstop only.
- [ ] A non-git / missing scan path → `ProjectRescanned { is_git: false, repo_root: None, … }` (degraded
      detection, never an `Err`/panic — `detect_git`/`detect_workflow` already degrade).
- [ ] The scan path is read from `req.inputs` (the Action carries which path to scan); a missing/invalid
      path → `ExecutionOutcome::Failed`, never a silent skip / never a scan of the daemon's own cwd.
- [ ] `project.rescan` dispatches to the registered `ProjectExecutor` (not the fallback stub) — verified
      end-to-end via `submit_action` (risk-0 auto-execute) producing a real `ProjectRescanned` row.
- [ ] All unit tests + the integration test pass; `/preflight` clean (fmt + clippy -D + check + test).

## Wiring / entry point (Step 7.5)
**Production entry point:** `daemon/src/main.rs` — where the production `CatalogExecutor` is constructed for
the Gateway, add `catalog_executor.register(ExecutorKind::Project, Arc::new(ProjectExecutor::new(...)))`
**before** it is handed to the Gateway. Reachable on the real path via the **existing `submit_action` IPC**:
a `project.rescan` `ActionRequest` → Gateway → catalog risk-0 auto-execute → `CatalogExecutor` dispatch →
`ProjectExecutor::execute`. **No new IPC method** (`submit_action` is generic). Confirm at Step 1 that
`main.rs` builds the Gateway over a `CatalogExecutor` you can `register()` into (the `SessionExecutor` doc
notes "main.rs keeps CatalogExecutor").

**Deferred (NOT this slice):** the read-model **projector** that splits `ProjectRescanned` into a private
`projects`/`repositories` registry — those tables don't exist yet → it needs **MIGRATION_9** (gated; the
lead is relaying the migration ruling before the Wave-C `integration_connections` slice). This slice ships
the executor + emission + the §15 strip; the event lands in the immutable audit log (replayable), and the
registry projector rebuilds from it when the migration lands. Say so explicitly — `none for the read-model
projection — lands in the registry-migration slice`.

## Files expected to touch
**New:**
- `daemon/src/project/mod.rs` — new `project` namespace module (decl only).
- `daemon/src/project/executor.rs` — `ProjectExecutor` (`impl ActionExecutor`) + the `strip_userinfo` helper.
- `daemon/tests/project_executor.rs` — integration tests (submit_action end-to-end) + unit tests for the strip helper.

**Modified:**
- `daemon/src/lib.rs` — `pub mod project;`.
- `daemon/src/main.rs` — `register(ExecutorKind::Project, …)` at Gateway construction.
- `daemon/src/gateway/executor.rs` — **(daemon-core; see Step-2.5 Q1)** add an `EmittedEvent` variant for
  `ProjectRescanned`.
- `daemon/src/gateway/request.rs` — add the `emitted_event_intent` match arm for the new variant (serialize
  payload → `gateway_event_intent` → §15 append). Set the envelope `project_id`/object identity if applicable.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2 — `daemon/tests/project_executor.rs`)
1. **`test_strip_userinfo_https_with_creds`** — `https://user:token@github.com/o/r` → `https://github.com/o/r`.
   - Asserts: userinfo removed, scheme/host/path preserved. Why: §15 rule #5 strip-at-source (the load-bearing pin).
2. **`test_strip_userinfo_https_no_creds_unchanged`** — `https://github.com/o/r` → unchanged.
   - Asserts: a URL without userinfo is byte-identical out. Why: no false mutation.
3. **`test_strip_userinfo_scp_ssh_unchanged`** — `git@github.com:o/r` → unchanged.
   - Asserts: scp-style ssh (no scheme) is NOT mangled (`git@` is a username, not a secret). Why: §15 targets credential userinfo, not ssh user.
4. **`test_strip_userinfo_ssh_scheme_with_user`** — `ssh://git@host/o/r` → per the Q2 ruling.
   - Asserts: the agreed behavior (default: strip scheme-URL userinfo). Why: pin the scheme-URL edge.
5. **`test_project_rescan_emits_project_rescanned`** — submit `project.rescan` (path=a hermetic fixture git repo) via `submit_action`.
   - Asserts: `ActionSucceeded` + exactly one `ProjectRescanned` event whose fields match the fixture's detected state. Why: §6.3 dispatch + §7.2 read-model emission.
6. **`test_project_rescan_emitted_remote_url_stripped`** — fixture repo with `origin = https://user:token@host/o/r`.
   - Asserts: the emitted `ProjectRescanned.remote_url == "https://host/o/r"`; the token never appears in the persisted event payload. Why: §15 end-to-end (the security pin, distinct from the unit strip test).
7. **`test_project_rescan_non_git_path`** — submit with a non-git temp dir.
   - Asserts: `ProjectRescanned { is_git: false, repo_root: None, … }`, `ActionSucceeded`. Why: degraded detection, never error.
8. **`test_project_rescan_missing_path_input_fails`** — submit with no/blank path input.
   - Asserts: `ExecutionOutcome::Failed` (no event, no cwd scan). Why: fail-closed input handling.
9. **`test_project_rescan_dispatches_to_registered_executor`** — with `ProjectExecutor` registered.
   - Asserts: the `ProjectRescanned` event proves the real executor ran (not the side-effect-free stub). Why: registration/reachability pin.
10. **`test_project_rescan_side_effect_applied_false`** — inspect the outcome.
    - Asserts: `side_effect_applied == false` → a txn-B append failure rolls back cleanly (stays `executing`), NOT `ActionPartiallySucceeded`. Why: read-only detection has no durable external side effect (the pipeline's existing fail-closed rollback semantics).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none to a `shared/` contract — `ProjectRescanned` is already frozen (CONTRACT
  0.26.0, merged). The new `EmittedEvent::ProjectRescanned` variant + `emitted_event_intent` arm are
  **daemon-internal** (`gateway/`, not `shared/`) — no CONTRACT bump.
- **Shared-contract (schema-snapshot) model touched?** No new/extended `shared/` model → no schema-snapshot test
  needed in this slice (the existing `shared/tests/contract.rs` already pins `ProjectRescanned` at 0.26.0).
- **Orchestrator doc rows to write hot (Step 9):** a `daemon/CLAUDE.md` EventTypeRegistry-row note that
  `ProjectRescanned` now has a live edges emitter (was "no daemon emission") + the §6.3 row note that
  `ExecutorKind::Project` is now registered; LESSON candidate (→ LESSON 30) for the strip-at-source pattern.
- **⚠️ Cross-track surface (orchestrator is surfacing to the lead):** this slice edits `gateway/executor.rs`
  + `gateway/request.rs` (the `EmittedEvent` bridge) — daemon-core files the daemon track also edits at
  P4.0b-2. Additive (a new enum variant + a new match arm) → low merge contention, but it IS a gateway/
  touch (the R1 packet said "edges writes impls in git/+integrations/ — no gateway/ edit"; emission through
  the §15 gate requires this bridge). Flagged like the MIGRATION_9 coordination item.

## Things to flag at Step 2.5
1. **`EmittedEvent` bridge shape — typed variant vs generic.** The enum currently has only
   `SessionStarted`. Option A: add a typed `EmittedEvent::ProjectRescanned { project_id?, payload }` (matches
   the SessionStarted precedent, type-safe; each future edges event adds a variant). Option B: add ONE
   generic `EmittedEvent::Namespaced { event_type, payload_json, object_ref, session_id? }` that ALL 6 edges
   events reuse (one gateway/ edit total, lower cross-track contention, but introduces a second pattern
   alongside the typed one). **My default vote: A (typed)** — consistency with the daemon's established
   pattern + compile-time safety outweighs saving ~5 small additive edits; the contention is low either way.
   (If you prefer B for contention reasons, say so — it's a reasonable call.)
2. **ssh-scheme userinfo strip.** For `ssh://git@host/o/r`, strip the `git@` userinfo or keep it? My default
   vote: **strip userinfo from any scheme-bearing URL** (`scheme://[userinfo@]host…`) uniformly, and leave
   scp-style `git@host:path` (no scheme) intact. Rationale: a scheme URL's userinfo is the credential slot
   (§15); the scp-style `git@` is the standard ssh username, not a secret. Pure, deterministic, prefix-free.
3. **Scan-path input key + canonicalization.** Read the path from `req.inputs["path"]` (string)? And
   canonicalize before scanning? My default vote: **`req.inputs["path"]`, required, scanned as-given**
   (detection already canonicalizes `repo_root` internally; don't double-canonicalize the input — a
   non-existent path degrades to `is_git:false`, which test 7 pins).
4. **`side_effect_applied` = false.** Confirm detection is read-only → false (so the pipeline rolls back clean on a
   txn-B fault rather than `ActionPartiallySucceeded`). My default vote: **false.**
5. **Executor module home.** New `daemon/src/project/` (its own namespace, parallel to git/integrations) vs.
   folding into `workflow/`. My default vote: **new `project/` module** — `ExecutorKind::Project` deserves its
   own namespace + it composes both git/ and workflow/ detection (neither is a clean owner).

## Dependencies + sequencing
- **Depends on:** the merge `bd3ee31` (R1 seam + `ProjectRescanned` type + catalog `project.rescan`→`Project`); detection engine (in-lane, landed).
- **Blocks:** the registry-migration slice (the `projects`/`repositories` projector that consumes `ProjectRescanned`); sets the `EmittedEvent`-bridge precedent for all Wave-B/C/D emission slices.

## Estimated commit count
**1.** One focused security-load-bearing slice (the §15 strip pin is the core). It gets its own commit — do
NOT bundle. (`security-reviewer` runs on this slice per the `invariant` policy: it touches INV-SEC-1 /
the §15 gate / the first real edges Gateway-mutation path.)

## Lessons-logged candidates anticipated
- **Convention candidate (→ LESSON 30)** — "edges executors emit their lifecycle events via the in-txn
  `EmittedEvent` bridge through the §15 gate (SessionExecutor precedent); credential-bearing fields
  (`remote_url`) are stripped AT THE EMIT SOURCE, the Redactor is the backstop only (LESSON 13 prefix-free)."
- **Architecture-doc note candidate** — `ExecutorKind::Project` registered; `ProjectRescanned` has a live
  edges emitter; the read-model projector is migration-gated (deferred).
- **Future TODO — phase** — the `projects`/`repositories` registry projector + MIGRATION_9 (Wave-C-adjacent).

## How to invoke
1. **Read this brief end-to-end** (esp. the Step-2.5 questions — answer before tests).
2. **Run `/tdd project_rescan_executor`.**
3. **Step 2.5** — send the test-design write-up (one `Asserts: <invariant> (§anchor)` line per test + the
   acceptance-bullet coverage map) + your answers/objections to the 5 questions. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
4. **Step 9** — surface categorized flags (esp. the cross-track gateway/-touch + the LESSON-30 candidate).
