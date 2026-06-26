# /tdd brief — session_profile_change_control (WAVE-1 W1-C-a)

## Feature
Wire the cockpit's **Change profile** control: a per-session affordance that picks a new execution
profile (from the ui-085 `get_execution_profiles` list) and submits a `session.profile_change` intent for
that session. Unlike Kill/Launch (risk-0 auto-execute), `session.profile_change` is **risk-2 →
approval-gated** (the §15 #8 no-silent-account-hop human checkpoint), so the submit surfaces in the live
ApprovalQueue and the user approves it via the existing approval card. Held behind a default-OFF gate
(the session-live-write posture). Dogfoods the just-landed `get_execution_profiles` transport.

## Use case + traceability
- **Task ID:** **W1-C** (the first slice — W1-C-a; the unticked `- [ ] **W1-C**` line under `### WAVE-1`;
  the line lists `session.profile_change / send_message / pause / resume / attach_terminal` — this slice
  delivers `profile_change`, the only one unblocked now [send_message/pause/resume gated on W1-exec/094,
  attach_terminal on W1-exec-term]).
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (session lifecycle — profile rebind),
  `§6.1` (the `submit_action` intent surface + `get_execution_profiles` read), `§6.3`
  (`session.profile_change` catalog risk-2), `§6.2` (the approval pipeline a risk-2 action enters),
  `§15` (#8 no-silent-account-hop — the approval IS the gate; the UI never sets/changes a profile silently).
- **Phase scope:** this brief **widens phase scope because** it is a UI cockpit-wiring slice (the
  profile-change control), not a daemon-phase slice — the `§`-references are cross-doc context.
- **Related context (verified against live code):**
  - **Daemon contract:** `session.profile_change` is a **GENERIC `submit_action`** (no dedicated method;
    `SessionExecutor::execute_profile_change`, session_executor.rs:199; arm registered :401; catalog
    `shared/src/catalog.rs:96,251`). **risk-2 → approval-gated** (the §15 #8 no-account-hop APPROVAL gate,
    PIN c) — so the submit returns `awaiting_approval` (NOT auto-execute); the change applies only after a
    human approves it in the ApprovalQueue. **UI/IPC-requester-only** (`PROFILE_MUTATION_TYPES`,
    policy.rs:33 — deny-before-risk for Agent/Brain/Pack/System).
  - **Action shape:** `resource_refs:[{type:"session", id:<session_id>}]` (required — execute_profile_change
    keys on the first resource_ref → `SessionId::parse`, :207) + `inputs:{ execution_profile_id }`
    (REQUIRED — "requires inputs.execution_profile_id (no default)", :236; parsed → `ExecutionProfileId`,
    fail-closed on invalid). Client-mint the `action_request_id` (empty-PK lesson 39, the session.kill
    precedent). `risk_level:2` (non-authoritative hint — catalog-authoritative).
  - **The picker source (ui-085, LANDED `516c3c7`):** `GatewayPort.get_execution_profiles() →
    GetExecutionProfilesResult { profiles: ProfileRow[] }`; `ProfileRow { execution_profile_id, provider,
    harness, model?, account_alias?, status: ExecutionProfile, is_default, has_credential }` (secret-free,
    §15 #4). The picker fetches on-demand (when the user opens it — not per-row eagerly), pre-selects
    `is_default`, surfaces "needs credential" off `has_credential:false`.
  - **The current profile:** `SessionRow.execution_profile_id` (ui-062, the 10-field strict shadow) → the
    control shows the session's CURRENT profile + offers a different one.
  - **Approval flow (LIVE):** a submitted risk-2 action → `awaiting_approval` → the live ApprovalQueue
    (ui-059 refetch-on-nudge) → the user approves via the existing GatewayModal/approval card (the §15 #8
    human checkpoint). This slice only SUBMITS; the approval + execution + the SessionProfileChanged result
    ride the existing infrastructure (the PR-mutations risk-3 precedent).
  - **Transport EXISTS:** `submit_action` (generic) + `get_execution_profiles` (ui-085) are both live. NO
    new Tauri command. The only transport change is the new default-OFF gate.

## Acceptance criteria (what "done" means)
- [ ] `buildProfileChangeActionRequest({ session_id, execution_profile_id }, createdAt) → ActionRequest`:
      CLIENT-mints the id (`mintActionRequestId()`), `action_type:"session.profile_change"`,
      `requester_type:"user"`, `resource_refs:[{type:"session", id:session_id}]`,
      `inputs:{ execution_profile_id }`, `risk_level:2` (hint), `status:"submitted"`, `created_at` passthrough.
- [ ] A new **default-OFF gate** (the `enabledSessionKill`/`enabledPrMutations` mirror) on
      `GatewayPort`/`UdsGatewayPort`: `submit_action` THROWS-never-invokes for
      `action_type==="session.profile_change"` until the gate flips — INDEPENDENT of `mutationsEnabled`
      and action-scoped. `UdsGatewayPort`/prod default OFF (held until a USER cat-1 sign-off — defense-in-
      depth to the daemon's risk-2 approval); `MockGatewayPort` default ON; a held-flip production guard.
- [ ] The "Change profile" control: opens a picker that fetches `get_execution_profiles` on-demand,
      pre-selects `is_default`, marks `has_credential:false` profiles "needs credential", and shows the
      session's CURRENT `execution_profile_id`; submitting calls
      `buildProfileChangeActionRequest({session_id, execution_profile_id})` via `gateway.submit_action`.
- [ ] **Non-optimistic + honest:** on `ok` (the daemon's `ActionAck.status`, e.g. `awaiting_approval`) →
      a "submitted for approval" notice (the daemon-reported status, NEVER a fabricated "changed");
      on a §6.4 rejection → the verbatim code; a transport fault → honest degrade (`instanceof Error`).
- [ ] The control is **disabled** unless `canSubmitIntent && mutationsEnabled && <the new gate>`; an
      `inFlight` double-submit guard; disabled/hidden for ended sessions (reuse `isEndedSession`).
- [ ] All unit tests pass; `/preflight` clean; `security-reviewer` (the new gated transport + the
      submit-only profile-change path — INV-SEC-1: the UI submits an intent; the daemon Gateway + the
      risk-2 approval are the chokepoint; §15 #8 — the UI never silently sets a profile).

## Wiring / entry point (Step 7.5)
Production: a per-session **Change profile** affordance reachable from the Shell's live `UdsGatewayPort` —
default placement the **SessionsTable Actions column** (the ui-084 `rowActions` slot, next to Kill), as a
button that opens an inline/popover picker. The picker reads `get_execution_profiles`; the submit routes
through `gateway.submit_action`; the approval surfaces in the live ApprovalQueue. **Confirm placement at
Step-1** (per-row Actions popover vs the SessionTerminal detail view — the picker wants more room than Kill).

## Files expected to touch (multi-commit — enumerate the layers)
**Commit 1 — builder + default-OFF transport gate:**
- `ui/src/intent/profile-change-request.ts` — `buildProfileChangeActionRequest` + `SESSION_PROFILE_CHANGE_ACTION_TYPE`
  const + an `isProfileChangeEnabled(gateway)` reader (+ test).
- `ui/src/gateway-client/types.ts` — the new default-OFF gate field on `GatewayPort`.
- `ui/src/gateway-client/uds.ts` — the gate field (ctor option, default false) + the `submit_action`
  enforcement (+ test).
- `ui/src/gateway-client/mock.ts` — the gate option (default true) (+ test) + `shell/Shell.test.tsx` stub field.

**Commit 2 — the picker + control + wiring:**
- `ui/src/views/sessions/ProfileChangeControl.tsx` — the "Change profile" control + the on-demand picker
  (fetch `get_execution_profiles`, pre-select default, "needs credential", current-profile display) +
  the gated/non-optimistic/inFlight submit (+ test).
- `ui/src/views/sessions/SessionsTable.tsx` / `views/terminal/SessionTerminal.tsx` — supply the control via
  the `rowActions` slot (alongside Kill) at the confirmed placement.
- the corresponding `*.test.tsx`.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`build_profile_change_action_request`** — Asserts: fresh `act_` ULID; `action_type:"session.profile_change"`;
   `resource_refs:[{type:"session",id}]`; `inputs:{execution_profile_id}`; `risk_level:2`; `created_at`
   passthrough. Why: lesson-39 client-mint + the execute_profile_change resource_ref/inputs keying.
2. **`uds_submit_action_profile_change_gated`** — Asserts: `session.profile_change` throws-never-invokes
   when the gate is OFF even with `mutationsEnabled:true`; invokes when ON; a non-profile-change action is
   unaffected. Why: the default-OFF go-live hold (lead's standing constraint) + the action-scoped gate.
3. **`mock_profile_change_gate_defaults_true`** — Asserts: the Mock defaults the gate ON. Why: a working dev/test port.
4. **`profile_change_control_fetches_and_submits`** — Asserts: opening the picker fetches
   `get_execution_profiles`; submitting calls `gateway.submit_action` with a `session.profile_change`
   request for THIS session + the chosen `execution_profile_id`; pre-selects `is_default`; disabled unless
   `canSubmitIntent && mutationsEnabled && <gate>` / non-ended / not in-flight. Why: §6.1 + the picker UX.
5. **`profile_change_notice_non_optimistic`** — Asserts: on `ok` → a daemon-status notice (e.g.
   "submitted for approval", NEVER "changed"); §6.4 rejection → verbatim; transport fault → honest degrade.
   Why: §11.7 + §6.2 (risk-2 enters the approval flow, not immediate) + LESSON 16/22.
6. **`profile_change_marks_needs_credential`** — Asserts: a `has_credential:false` profile is rendered
   "needs credential" (a disabled/annotated option). Why: the §2.8/§15 #8 credential-state surfacing.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — `session.profile_change` params + the `ProfileRow`/ActionAck shapes are
  frozen daemon contract; the UI mirrors them. **CONTRACT-neutral; no `shared/` change; no regen.**
- **Orchestrator doc rows to write hot (Step 9 routing):** none (a new UI gate + control). A likely
  Convention candidate (the first approval-gated session control + its default-OFF gate).
- **§2.5-seam:** none touched.

## Things to flag at Step 2.5
1. **The gate: a new boolean `enabledProfileChange` vs a shared session-control set.** My default vote: **a
   new boolean `enabledProfileChange`** (mirrors enabledSessionLaunch/enabledSessionKill). Note the
   boolean proliferation (launch/kill/profile_change + future send_message/pause/resume) — a future
   consolidation to an `enabledSessionControls: Set<actionType>` is worth a carry-forward, but refactoring
   the existing booleans is out of scope here.
2. **Does an approval-gated (risk-2) action even NEED a default-OFF UI gate?** My default vote: **YES** —
   the `enabledPrMutations` precedent (risk-3, approval-gated, default-OFF) + the lead's "any new session
   live-write rides the default-OFF posture" + defense-in-depth. The daemon approval is the *operative*
   safety checkpoint; the UI gate is the go-live hold. (If the implementer disagrees, this is the one to
   ping back on — it's the load-bearing design call.)
3. **Placement.** My default vote: **the SessionsTable Actions column** (the ui-084 `rowActions` slot,
   next to Kill) with a popover picker. Confirm vs the SessionTerminal detail view (more room for the picker).
4. **On-`ok` notice.** My default vote: **a daemon-status "submitted for approval" notice** (non-optimistic
   — the real `ActionAck.status`, telling the user to approve it in the queue), NOT error-only (Kill was
   error-only because it auto-executes; profile_change queues an approval, so silence would confuse).
5. **Picker fetch timing.** My default vote: **on-demand** (fetch `get_execution_profiles` when the picker
   opens, not eagerly per row). Confirm.

## Dependencies + sequencing
- **Depends on:** ui-085 (`get_execution_profiles` transport, LANDED `516c3c7`) + the daemon
  `session.profile_change` executor (LIVE, 085) + the live ApprovalQueue (ui-059). Nothing blocking.
- **Blocks:** nothing directly. Sits alongside the rest of W1-C (send_message/pause/resume — gated on
  W1-exec/094; attach_terminal — gated on W1-exec-term).

## Estimated commit count
**2** (builder + default-OFF gate / picker + control + wiring). The gate is a defense-in-depth go-live hold
(the daemon risk-2 approval is the operative §15 #8 checkpoint) → bundles with the builder; `security-reviewer`
runs on the new gated transport + the submit-only profile-change path.

## Lessons-logged candidates anticipated
- **Convention candidate** — the first APPROVAL-GATED session control: a risk-2 `submit_action` +
  client-mint + a default-OFF UI gate (defense-in-depth to the daemon approval, the enabledPrMutations
  precedent) + a non-optimistic "submitted for approval" notice (the action enters the approval flow, not
  immediate execution) — the risk-0-auto-execute kill/launch pattern extended to an approval-gated write.
- **Future TODO — operational** — consolidate the per-control default-OFF booleans (launch/kill/
  profile_change/…) into an `enabledSessionControls` set when the count grows (carry-forward).

## How to invoke
1. Read this brief end-to-end. Don't skip "Things to flag at Step 2.5" — esp. Q2 (the gate-for-an-
   approval-gated-action call).
2. Run `/tdd session_profile_change_control`.
3. Step 1 → confirm the placement + the layer file list.
4. Step 2.5 → ping back with answers to the 5 design Qs (or take defaults).
5. Step 9 → flag the next W1-C slice (send_message/pause/resume — gated on W1-exec/094).
