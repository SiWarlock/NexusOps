# /tdd brief — session_kill_control (WAVE-1 Slice B)

## Feature
Wire the cockpit's **Kill** (stop) control on SessionsTable rows: a per-row "Kill" affordance that
submits a `session.kill` intent for that session via the generic `submit_action` path (CLIENT-mints the
action id, session `resource_ref`, risk-0 auto-execute → NO approval modal). The daemon's live
`SessionExecutor::execute_kill` routes `SessionCommand::Kill` to the supervised session; the row
transitions to `killed`/`failed` via the live Session projection nudge — THAT is the success signal. The
other half of the air-traffic-control loop: the cockpit can now STOP agents, not just launch + view them.

## Use case + traceability
- **Task ID:** **W1-B** — `session.kill` "Kill" control UI wiring (the unticked `- [ ] **W1-B**` line under
  `### WAVE-1` in `IMPLEMENTATION_PLAN.md`, homed by daemon-orchestrator; WAVE-1 spec anchors §9.1 §6.1 §6.3 §11 §15).
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (harness/session lifecycle — Kill),
  `§6.1` (the `submit_action` intent surface), `§6.3` (`session.kill` catalog risk-0), `§11` (cockpit),
  `§15` (the UI submits intents only — the daemon Gateway is the INV-SEC-1 chokepoint).
- **Phase scope:** this brief **widens phase scope because** it is a UI cockpit-wiring slice (the
  agent-stop control), not a daemon-phase slice — the `§`-references are cross-doc context.
- **Related context:**
  - **Slice A precedent (ui-083, `bb90a34`→`b7e36d6`):** `session.create` "Launch agent" landed as a
    dedicated-method transport + a header control, HELD behind a default-OFF `enabledSessionLaunch` gate.
    Slice B mirrors the GATE posture but uses a DIFFERENT transport (generic `submit_action`, not a
    dedicated method).
  - **Daemon contract (verified against live code):** `session.kill` is a **GENERIC `submit_action`**
    action — there is **no dedicated `session.kill` IPC method** (`smoke.rs:162-193` is the reference
    client). The executor is **LIVE + registered** (`SessionExecutor::execute_kill`,
    `session_executor.rs:274`; `SESSION_KILL` arm `:317`). NO daemon-stub gap. `session.kill` is **risk-0
    auto-execute** in the catalog (`shared/src/catalog.rs:95,215`; `policy.rs:58`) → NO approval modal
    (like `project.rescan` / `session.create`).
  - **`execute_kill` keying:** NaturalResourceRef-keyed — the executor reads `req.resource_refs.first()`
    → `SessionId::parse(rref.id)` (`session_executor.rs:279-293`). So the intent carries
    `resource_refs:[{type:"session", id:<session_id>}]`. **`"session"` is the confirmed `ResourceType`
    wire value** (`generated.ts` `ResourceType` enum includes `"session"`; daemon `ResourceType::Session`).
    `inputs` is `{}` (empty — the resource_ref is the only operand). A gone/unknown session →
    `execute_kill` returns `Succeeded{side_effect_applied:false}` (a clean no-op, NOT an error) — the
    ActionAck is still `ok`; the UI surfaces no error.
  - **Client-mint (empty-PK lesson 39):** the daemon trusts the wire id as the `action_requests` PK; only
    the dedicated `session.create` method mints daemon-side. EVERY generic `submit_action` intent MUST
    carry a client-minted canonical ULID via `mintActionRequestId()` (`intent/mint-id.ts`) — an empty id
    collides on a 2nd same-session mutation → `AuditWriteFailed`→`precondition_stale` (ui-080 / lesson 39). Mirror
    `buildMergePrActionRequest` (`intent/pr-mutation-request.ts`).
  - **Transport ALREADY EXISTS:** `GatewayPort.submit_action` + `UdsGatewayPort.submit_action`
    (`uds.ts:372`) + `MockGatewayPort.submit_action` are live. So **NO new Tauri command / no gateway-uds
    crate layer** (unlike Slice A's dedicated method). The only transport change is the new default-OFF gate.
  - **Render-from-projection (forbidden #2):** the row already transitions via the live Session
    refetch-on-nudge (D3, ui-062 — `applySessionDelta` deleted, refetch-on-nudge live). No optimistic
    local state; the killed row appears via the daemon projection.

## Acceptance criteria (what "done" means)
- [ ] `buildKillSessionActionRequest({ session_id }, createdAt) → ActionRequest` exists: CLIENT-mints
      `action_request_id` via `mintActionRequestId()`, `action_type:"session.kill"`,
      `requester_type:"user"`, `resource_refs:[{type:"session", id:session_id}]`, `inputs:{}`,
      `risk_level:0` (non-authoritative hint), `status:"submitted"`, `created_at:createdAt`.
- [ ] A new **default-OFF `enabledSessionKill` gate** on `GatewayPort`/`UdsGatewayPort` (the
      `enabledSessionLaunch` mirror): `submit_action` THROWS-never-invokes when
      `request.action_type === "session.kill"` && `!enabledSessionKill` (an `assertSessionKillEnabledFor`
      keyed on action_type — the `assertPrMutationsEnabledFor` pattern). `UdsGatewayPort` defaults it
      FALSE (production HELD until a USER cat-1 sign-off + visual gate); `MockGatewayPort` defaults it TRUE.
      A `isSessionKillEnabled(gateway)` reader (the `isPrMutationEnabled` mirror).
- [ ] The per-row "Kill" control submits `buildKillSessionActionRequest` via `gateway.submit_action`; on
      `ok` → NO persistent success notice (the row transitions via the live nudge — error-only, §11.7,
      the LaunchAgentControl precedent); on a §6.4 rejection → the verbatim code; a transport fault caught
      honestly (classify by `instanceof Error`, LESSON 16/22).
- [ ] The Kill control is **disabled** unless `canSubmitIntent && gateway.mutationsEnabled &&
      gateway.enabledSessionKill` AND the session is **non-terminal** (a terminal session has nothing to
      kill). A synchronous in-flight guard prevents a double-submit (the ui-083-review `inFlight` ref).
- [ ] Kill renders/enables ONLY for non-terminal sessions — terminal Session statuses
      `{failed, completed, archived, killed}` show no Kill (or a disabled one).
- [ ] All unit tests pass; `/preflight` clean; `security-reviewer` (the new gated transport + the
      submit-only Kill path — INV-SEC-1: the UI submits an intent; the daemon Gateway + executor are the
      single chokepoint + mutator).

## Wiring / entry point (Step 7.5)
Production: a per-row **Kill** control in the SessionsTable body (a new **Actions** column or a per-row
slot), reachable from the Shell's live `UdsGatewayPort`. `SessionsTable` is presentational today (props
only — `sessions`/`projects`/`headerActions`; it does NOT hold the gateway). The control's gateway +
session id flow in via a NEW **`rowActions?: (row: SessionRowVM) => ReactNode`** render-prop slot on
SessionsTable (mirrors the existing `headerActions` slot — keeps the table gateway-agnostic / purely
presentational, forbidden #2), supplied by **`SessionTerminal.tsx`** (which already holds `gateway` +
`activeProjectId` and renders `<SessionsTable headerActions={<LaunchAgentControl/>}/>` at
`views/terminal/SessionTerminal.tsx:51-56`). The killed session transitions in-place via the live Session
projection nudge (already wired). **Confirm the placement at Step-1** (Actions column vs per-row
inline-action vs row context-menu).

## Files expected to touch (multi-commit — enumerate the layers)
**Commit 1 — intent builder + the default-OFF transport gate:**
- `ui/src/intent/kill-session-request.ts` — `buildKillSessionActionRequest` + `SESSION_KILL_ACTION_TYPE`
  const + `isSessionKillEnabled(gateway)` reader (+ `kill-session-request.test.ts`).
- `ui/src/gateway-client/types.ts` — `readonly enabledSessionKill: boolean` on `GatewayPort`.
- `ui/src/gateway-client/uds.ts` — the `enabledSessionKill` field (ctor option, default false) +
  `assertSessionKillEnabledFor` called in `submit_action` (+ `uds.test.ts`).
- `ui/src/gateway-client/mock.ts` — the `enabledSessionKill` option (default true) (+ `mock.test.ts`).

**Commit 2 — the control + the row slot + wiring:**
- `ui/src/views/sessions/KillSessionControl.tsx` — the per-row Kill control (gated + non-optimistic +
  `instanceof Error` classification + `inFlight` double-submit guard) (+ `KillSessionControl.test.tsx`).
- `ui/src/views/sessions/SessionsTable.tsx` — the `rowActions` render-prop slot + an Actions column
  (+ `SessionsTable.test.tsx`).
- `ui/src/views/sessions/model.ts` — a `SESSION_TERMINAL_STATUSES` set + `isSessionKillable(status)`
  helper (or reuse if one exists) (+ `model.test.ts`).
- `ui/src/views/terminal/SessionTerminal.tsx` — supply `rowActions={(row) => <KillSessionControl
  gateway={gateway} sessionId={row.id} status={row.status} />}`.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`build_kill_session_action_request`** (`kill-session-request.test.ts`) — Asserts: a fresh canonical
   `act_`-prefixed ULID id (NOT empty), `action_type:"session.kill"`, `resource_refs:[{type:"session",
   id}]`, `inputs:{}`, `status:"submitted"`, `created_at` passthrough. Why: lesson-39 client-mint + the
   `execute_kill` resource_ref keying.
2. **`uds_submit_action_session_kill_gated`** (`uds.test.ts`) — Asserts: `submit_action` with
   `action_type:"session.kill"` THROWS-never-invokes when `enabledSessionKill` false; invokes
   `gateway_submit_action` when true; a NON-kill action_type is unaffected by the kill gate. Why: the
   default-OFF go-live hold (the lead's standing constraint) + the `assertPrMutationsEnabledFor` precedent.
3. **`mock_submit_action_session_kill_enabled_by_default`** (`mock.test.ts`) — Asserts: the Mock defaults
   `enabledSessionKill` true (a working test/dev port). Why: the `enabledSessionLaunch`/`enabledPrMutations`
   Mock-defaults-true precedent.
4. **`is_session_killable`** (`model.test.ts`) — Asserts: terminal `{failed, completed, archived, killed}`
   → not killable; a live status (e.g. `active`, `waiting_on_permission`) → killable. Why: §5.1 terminal set.
5. **`kill_control_submits_for_row`** (`KillSessionControl.test.tsx`) — Asserts: clicking Kill calls
   `gateway.submit_action` with the built `session.kill` request for that row's `session_id`; disabled
   when `!canSubmitIntent` / `!mutationsEnabled` / `!enabledSessionKill` / terminal status; the `inFlight`
   guard blocks a double-fire. Why: the gated-control + double-submit precedent (ui-083 review).
6. **`kill_notice_non_optimistic`** (`KillSessionControl.test.tsx`) — Asserts: on `ok` → NO persistent
   "Killed" notice (the row transitions via the nudge); on a §6.4 rejection → the verbatim code; a
   transport fault → honest degrade (`instanceof Error`). Why: §11.7 + LESSON 16/22.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — `session.kill` params + the `ActionRequest`/`ActionAck`/`ResourceType`
  shapes are frozen daemon contract; the UI mirrors them. **No `shared/` change; CONTRACT-neutral; no regen.**
- **Orchestrator doc rows to write hot (Step 9 routing):** none (a new UI transport gate + control, not a
  contract change). A likely **Convention candidate** lesson (below).
- **§2.5-seam (shared-contract) model touched?** No — no Appendix-A model field changes.

## Things to flag at Step 2.5
1. **The gate: a NEW `enabledSessionKill` boolean vs reusing `enabledSessionLaunch` vs a set.** My default
   vote: **a separate `enabledSessionKill` boolean** — it mirrors the just-landed `enabledSessionLaunch`,
   gives the finest-grained go-live staging (the user can enable Launch and Kill independently), and keeps
   each high-consequence live-write on its own default-OFF gate (the `enabledPrMutations` philosophy).
   Reusing Launch's gate couples two different-risk capabilities (creative vs destructive). NOT a
   cat-1/contract decision (internal go-live staging; the user's sign-off is preserved either way).
2. **Control placement.** Default: a per-row **Kill** in a new SessionsTable **Actions** column, via the
   `rowActions` render-prop slot (mirrors `headerActions`; keeps the table presentational). My default
   vote: **Actions column + render-prop slot**. Confirm vs an inline per-row icon-button or a row
   context-menu.
3. **Killable-state UX.** Default: render the Kill control ONLY for non-terminal sessions (terminal set
   `{failed, completed, archived, killed}`); a terminal row shows no Kill. My default vote: **hide on
   terminal** (vs render-disabled). Define `SESSION_TERMINAL_STATUSES` in `model.ts` (no existing helper).
4. **Confirm-before-kill?** Kill is destructive but daemon-risk-0 (no approval modal, per the catalog +
   the lead's framing). My default vote: **NO modal; an optional lightweight inline confirm-click**
   (Kill → "Confirm?" → submit) — a two-step click guards an accidental stop without a blocking dialog
   (and we never trigger a browser/JS modal). Confirm whether to include the inline confirm in v1 or ship
   a bare button.
5. **Submit path: direct `gateway.submit_action` vs the `useSubmitIntent` seam.** My default vote:
   **direct `gateway.submit_action`** + `instanceof Error` classification (byte-mirrors LaunchAgentControl;
   the seam adds a `readOnly` branch the disabled-control already covers). Confirm.

## Dependencies + sequencing
- **Depends on:** nothing blocking — the daemon `session.kill` executor + the `submit_action` transport +
  the live L2 seam are all LIVE. Reuses the established `submit_action` + client-mint patterns.
- **Blocks:** the rest of WAVE-1 session-lifecycle UI (W1-C: `send_message`/`pause`/`resume`/
  `profile_change`/`attach_terminal` controls), which additionally need the daemon W1-prof (093,
  `get_execution_profiles` + 0.48 regen) + W1-exec (094, the executor bodies) to land first.
- **Note:** WAVE-1 Slice B; brief number `ui-084` (the next ui-track brief after `ui-083`).

## Estimated commit count
**2** (intent builder + default-OFF transport gate / the control + row slot + wiring) — a multi-commit
slice (LESSON ui [[7]]: the implementer drives layer→layer; the orchestrator wakes it at each commit).
The `enabledSessionKill` gate is a defense-in-depth go-live hold, NOT the load-bearing safety control (the
daemon Gateway + executor are the INV-SEC-1 chokepoint, forbidden #3) — so it bundles with the builder;
but `security-reviewer` runs on the new gated transport + the submit-only Kill path.

## Lessons-logged candidates anticipated
- **Convention candidate** — a destructive session live-write (Kill) rides a GENERIC `submit_action` +
  client-minted id (no dedicated IPC method, no daemon-mint) + its OWN default-OFF go-live gate
  (`enabledSessionKill`), disabled-unless-non-terminal, no approval modal (daemon risk-0) — the
  per-action-gated-live-write pattern generalized from `enabledPrMutations`/`enabledSessionLaunch`.
- **Architecture-doc note candidate** — the cockpit's session-lifecycle controls each carry an
  independent default-OFF gate until the user's per-capability cat-1 sign-off (the deferred-go-live posture).

## How to invoke
1. Read this brief end-to-end. Don't skip "Things to flag at Step 2.5".
2. Run `/tdd session_kill_control`.
3. Step 1 → confirm the control placement + the `rowActions` slot + the layer file list.
4. Step 2.5 → ping back with answers to the 5 design Qs (or take defaults).
5. Step 9 → flag the next WAVE-1 dependency (W1-C is gated on daemon 093 + 094).
