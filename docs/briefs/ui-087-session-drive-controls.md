# /tdd brief — session_drive_controls (WAVE-1 W1-C-b)

## Feature
Wire the cockpit's session-DRIVE controls — **Send message**, **Pause monitoring**, **Resume monitoring**
— each a `submit_action` intent routed to the daemon's live W1-exec executor bodies. All three are
**approval-gated** (risk-1/2 → they enter the live ApprovalQueue, not auto-execute), held behind a
default-OFF gate. **🔴 Pause/Resume are labeled HONESTLY** — "Pause monitoring" / "Resume monitoring",
NOT "stop"/"suspend": MVP `session.pause` is **SOFT** (it gates the cockpit's drive/observe loop, does
NOT OS-suspend the agent — the agent keeps running, its tool calls still intercepted via the Gateway, so
INV-SEC-1 holds). The real OS-suspend is a deferred follow-on (daemon LESSON 71 / W1-exec-follow-ons).

## Use case + traceability
- **Task ID:** **W1-C** — the send_message / pause / resume drive controls (the WAVE-1 session-control
  cluster's drive slice; profile_change shipped via ui-086; attach_terminal stays a later follow-on).
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (session lifecycle/drive), `§6.1` (the
  `submit_action` surface), `§6.3` (the 3 catalog entries), `§6.2` (the approval pipeline these enter),
  `§11` (cockpit), `§15` (the UI submits intents only — the daemon Gateway + interception are the chokepoint).
- **Phase scope:** this brief **widens phase scope because** it is a UI cockpit-wiring slice, not a
  daemon-phase slice — the `§`-references are cross-doc context.
- **Related context (verified against the SEALED W1-exec, daemon WAVE-1 complete + pushed `50172f2`):**
  - **All three are GENERIC `submit_action`** (no dedicated methods); the executors are LIVE + registered
    (`execute_send_message`, `execute_route` for pause/resume; session_executor.rs:369/333/407-409). All
    key on the target session `resource_ref` (`validated_target` → `SessionId::parse`).
  - **Risk + inputs (catalog `shared/src/catalog.rs`):** `session.send_message` = **risk-2**, `I::FromInputs`
    — REQUIRED non-empty `inputs.message` (the prompt routed as `SessionCommand::SendMessage(text)`;
    empty → daemon rejects). `session.pause` = **risk-1**, `NaturalResourceRef` (no inputs).
    `session.resume` = **risk-2**, `NaturalResourceRef`. **All risk≥1 → APPROVAL-GATED** (NOT auto-execute
    — only risk-0 auto-executes): the submit returns `awaiting_approval` → the live ApprovalQueue → the user
    approves via the existing card. The profile_change (ui-086) precedent — this slice only SUBMITS.
  - **🔴 HONEST LABELING (daemon finding via the lead — LOAD-BEARING, daemon LESSON 71):** MVP `session.pause`
    is SOFT — it gates the cockpit's drive/observe loop, does NOT OS-suspend the agent (the agent keeps
    running; tool calls still Gateway-intercepted → INV-SEC-1 holds). The control MUST read "Pause
    monitoring"/"pauses monitoring" — NEVER "stop the agent"/"suspend" (a user must not believe Pause halts
    the agent). The real OS-suspend (SIGSTOP-class) is the deferred carry-forward.
  - **No UI-readable paused state:** there is NO `paused`/`monitoring` field in `proj_session`/`SessionRow`/
    the §5.1 Session status enum (the soft pause is not projected). So the MVP shows **both** "Pause
    monitoring" + "Resume monitoring" as available actions (no stateful toggle — the UI can't know if a
    session is currently paused). Honestly labeled; the user manages it.
  - **Transport EXISTS:** `submit_action` (generic) is live. NO new Tauri command. The only transport change
    is the new default-OFF gate.
  - **Client-mint** the id (empty-PK lesson 39) for all three (the session.kill/profile_change precedent).

## Acceptance criteria (what "done" means)
- [ ] Builders: `buildSendMessageActionRequest({ session_id, message }, createdAt)` (action_type
      `session.send_message`, `resource_refs:[{type:"session",id}]`, `inputs:{message}` — `.trim()`'d,
      empty → the control blocks submit, never an empty intent; `risk_level:2`); `buildPauseActionRequest`
      / `buildResumeActionRequest({ session_id }, createdAt)` (action_types `session.pause`/`session.resume`,
      `resource_refs:[session]`, `inputs:{}`, `risk_level:1`/`2`). All CLIENT-mint the id.
- [ ] A new **default-OFF `enabledSessionControls: ReadonlySet<string>`** gate on `GatewayPort`/
      `UdsGatewayPort` (the `enabledPrMutations` mirror — a SET, so it seeds the per-control boolean
      consolidation): `submit_action` THROWS-never-invokes when the action_type is a gated session-drive
      type (`session.send_message`/`pause`/`resume`) NOT in the set — INDEPENDENT of `mutationsEnabled`.
      `UdsGatewayPort`/prod default EMPTY (all held until a USER cat-1 sign-off); `MockGatewayPort` default
      = the full drive set; an `isSessionControlEnabled(gateway, actionType)` reader + a held-flip guard.
- [ ] **Pause/Resume controls are labeled "Pause monitoring"/"Resume monitoring"** (NOT "stop"/"suspend") —
      a load-bearing honesty pin (a test asserts the rendered label text; daemon LESSON 71).
- [ ] **Send message control:** a text input + Send; submits `buildSendMessageActionRequest` with the
      trimmed non-empty message; empty/whitespace blocks submit (no empty intent — the §6.3 daemon rejects
      it anyway, but the UI blocks first).
- [ ] **Non-optimistic + honest** (all three): on `ok` → a "submitted for approval" notice (the daemon's
      `ActionAck.status`, NEVER a fabricated "sent"/"paused"); §6.4 rejection → verbatim code; transport
      fault → honest degrade (`instanceof Error`).
- [ ] All three controls **disabled** unless `canSubmitIntent && mutationsEnabled &&
      isSessionControlEnabled(gateway, <type>)`; an `inFlight` double-submit guard; hidden/disabled for
      ended sessions (reuse `isEndedSession`).
- [ ] All unit tests pass; `/preflight` clean; `security-reviewer` (the new gated transport + the
      submit-only drive paths — INV-SEC-1: the UI submits intents; the daemon Gateway + interception are
      the chokepoint; the pause does NOT bypass interception — the honest-label pin reflects that).

## Wiring / entry point (Step 7.5)
Production: the drive controls reachable from the Shell's live `UdsGatewayPort`. **Placement — confirm at
Step-1** (the Actions column is getting crowded with Kill + Change profile + 3 drive actions): default vote
is **the selected-session detail surface** (`views/terminal/SessionTerminal.tsx` — where the user actively
drives one session) for Send message (a prompt input wants room) + Pause/Resume monitoring, keeping the
per-row Actions column for the lifecycle actions (Kill, Change profile). Alternative: a per-row overflow/
kebab menu. The submit routes through `gateway.submit_action`; the approval surfaces in the live ApprovalQueue.

## Files expected to touch (multi-commit — enumerate the layers)
**Commit 1 — builders + default-OFF transport gate:**
- `ui/src/intent/session-drive-request.ts` — `buildSendMessageActionRequest`/`buildPauseActionRequest`/
  `buildResumeActionRequest` + the gated action-type consts + `isSessionControlEnabled` reader (+ test).
- `ui/src/gateway-client/types.ts` — `readonly enabledSessionControls: ReadonlySet<string>` on `GatewayPort`.
- `ui/src/gateway-client/uds.ts` — the field (ctor option, default empty) + the `submit_action` enforcement
  (+ test).
- `ui/src/gateway-client/mock.ts` — the option (default = the full drive set) (+ test) + `shell/Shell.test.tsx` stub.

**Commit 2 — the controls + wiring:**
- `ui/src/views/sessions/SessionDriveControls.tsx` (or per-control components) — Send message (input + submit),
  Pause monitoring, Resume monitoring (gated/non-optimistic/inFlight; honest labels) (+ test).
- the confirmed placement surface (`views/terminal/SessionTerminal.tsx` detail, or SessionsTable rowActions)
  — supply the controls.
- the corresponding `*.test.tsx`.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`build_session_drive_requests`** — Asserts: each builder forms the right action_type +
   `resource_refs:[session]` + inputs (`{message}` trimmed for send_message; `{}` for pause/resume) +
   risk_level (2/1/2) + a fresh `act_` ULID. Why: lesson-39 client-mint + the executor keying.
2. **`send_message_blocks_empty`** — Asserts: an empty/whitespace message → the control blocks submit (no
   intent built). Why: §6.3 (daemon requires non-empty) + honest UI.
3. **`uds_submit_action_drive_gated`** — Asserts: each drive type throws-never-invokes when not in
   `enabledSessionControls` even with `mutationsEnabled:true`; invokes when present; a non-drive action is
   unaffected. Why: the default-OFF go-live hold + the action-scoped Set gate.
4. **`mock_enabled_session_controls_defaults_full_set`** — Asserts: the Mock defaults the full drive set ON.
   Why: a working dev/test port.
5. **`pause_resume_labeled_monitoring_not_suspend`** — Asserts: the rendered labels are "Pause
   monitoring"/"Resume monitoring"; NO "stop"/"suspend"/"halt the agent" text. Why: daemon LESSON 71 — MVP
   pause is soft; an honest label is load-bearing.
6. **`drive_controls_submit_and_notice_non_optimistic`** — Asserts: clicking each submits the right request
   for the session; on `ok` → a "submitted for approval" notice (NEVER "sent"/"paused"); §6.4 rejection →
   verbatim; transport fault → honest degrade; disabled unless `canSubmitIntent && mutationsEnabled &&
   isSessionControlEnabled` / non-ended / not in-flight. Why: §6.2 approval flow + §11.7 + LESSON 16/22.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — the 3 action types' params + ActionAck are frozen daemon contract.
  **CONTRACT-neutral; no `shared/` change; no regen.**
- **Orchestrator doc rows to write hot (Step 9 routing):** none (a new UI gate + controls). A likely
  Convention candidate (the honest-labeling-for-a-soft-capability pattern + the Set-gate consolidation seed).
- **§2.5-seam:** none touched.

## Things to flag at Step 2.5
1. **The gate: `enabledSessionControls: Set` vs more booleans.** My default vote: **the Set** (the
   `enabledPrMutations` mirror) holding the 3 drive types — it avoids 3 more booleans AND seeds the
   consolidation the implementer flagged at ui-086 (a future slice migrates launch/kill/profile_change into
   it; NOT this slice — don't refactor the signed-off booleans here). Confirm.
2. **Placement (the Actions column is crowding).** My default vote: **the SessionTerminal selected-session
   detail surface** for the 3 drive controls (Send message wants a prompt input; Pause/Resume monitoring fit
   the active-drive surface), keeping the Actions column for Kill + Change profile. Confirm vs a per-row
   overflow/kebab menu.
3. **Pause/Resume with no readable state.** My default vote: **both actions always available** (the soft
   pause isn't projected → no accurate toggle) — honestly labeled "Pause monitoring"/"Resume monitoring".
   Confirm (vs a single stateless "Pause monitoring" + a separate Resume).
4. **Send message input.** My default vote: a single-line text input (Enter or Send submits; trimmed;
   empty blocks). Confirm (single-line vs a textarea; whether to clear on submit).
5. **One bundled slice vs split send_message from pause/resume.** My default vote: **bundle** (all three
   share the gate + the submit→approval pattern + the same area). Split only if the placement diverges
   enough to warrant it (flag at Step-1).

## Dependencies + sequencing
- **Depends on:** the daemon W1-exec executor bodies (LIVE — daemon WAVE-1 complete + pushed `50172f2`) +
  the live ApprovalQueue (ui-059) + the live L2 seam. Nothing blocking. (Reuses the ui-084/086 builder +
  gate + submit→approval patterns.)
- **Blocks:** nothing directly. W1-C-d (attach_terminal) remains gated on W1-exec-term + the xterm.js host.

## Estimated commit count
**2** (builders + default-OFF Set gate / the controls + wiring). The gate is a defense-in-depth go-live hold
(the daemon approval is the operative checkpoint for risk≥1); `security-reviewer` runs on the new gated
transport + the submit-only drive paths + the honest-label pin (the soft-pause INV-SEC-1 reflection).

## Lessons-logged candidates anticipated
- **Convention candidate** — honest labeling for a SOFT capability: when an MVP action does LESS than its
  verb implies (soft pause = gates monitoring, not OS-suspend), the control must be labeled to its REAL
  effect ("Pause monitoring"), pinned by a label test — never the aspirational verb. (daemon LESSON 71, UI side.)
- **Convention candidate** — `enabledSessionControls: Set` as the consolidation of the per-control default-OFF
  booleans (launch/kill/profile_change → migrate into the Set in a follow-on refactor).
- **Future TODO — operational** — the real OS-suspend pause + a projected paused-state (for an accurate
  toggle) are deferred (daemon LESSON 71 / W1-exec-follow-ons).

## How to invoke
1. Read this brief end-to-end. Don't skip "Things to flag at Step 2.5" — esp. the honest-labeling pin (load-bearing).
2. Run `/tdd session_drive_controls`.
3. Step 1 → confirm the placement + the bundle-vs-split call + the layer file list.
4. Step 2.5 → ping back with answers to the 5 design Qs (or take defaults).
5. Step 9 → flag W1-C-d (attach_terminal — gated on W1-exec-term + xterm.js) as the remaining W1-C piece.
