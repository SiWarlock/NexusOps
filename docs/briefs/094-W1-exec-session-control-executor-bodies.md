# /tdd brief — session_control_executor_bodies

## Feature
Implement the real `session.send_message` / `session.pause` / `session.resume` Gateway-executor bodies in `SessionExecutor` — routing typed `SessionCommand`s to the live session actor (the `execute_kill` precedent) so the cockpit can **drive** (not just view) running agents. Today these three action types fall through `SessionExecutor::execute`'s `_ => self.inner.execute(req)` arm → the empty inner `CatalogExecutor` → a side-effect-free stub (the agent receives nothing). (`session.attach_terminal` is a SEPARATE sibling slice **W1-exec-term** — the §6.4 terminal-channel attach, NOT a `SessionCommand`.)

## Use case + traceability
- **Task ID:** W1-exec
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (session lifecycle — the `SessionActor`/`SupervisorHandle` the bodies drive), `§6.3` (the action-catalog precondition the bodies validate), `§17` (the honest `side_effect_applied`/failure outcome), `§15` (INV-SEC-1 no-bypass — these bodies run ONLY via the gated Gateway execute path; the agent's subsequent tool calls remain intercepted).
- **Related context:** the body precedents in `daemon/src/gateway/session_executor.rs` — `execute_kill` (validate-precondition-first → route `SessionCommand::Kill` → `side_effect_applied = delivered`) and `execute_profile_change` (target-session resource_ref + `inputs.*` required). `SessionCommand` (`daemon/src/session/actor.rs:56`) currently has ONLY `Kill`. The `write_initial_prompt` PTY-write + LESSON 30 (a PTY write on a safety path that the INVARIANT does not depend on → degrade SOFT). The interception-still-holds analysis = LESSON 25/26. The catalog froze these types at 4.0b-1: `send_message` risk-2 `I::FromInputs`, `pause` risk-1 `I::NaturalResourceRef`, `resume` risk-2 `I::NaturalResourceRef` — all `requires_resource_refs=true`, `ExecutorKind::Session`.

## Acceptance criteria (what "done" means)
- [ ] **`session.send_message`** routes the message to the target session via a new `SessionCommand::SendMessage(String)`; the message text comes from `inputs` (`I::FromInputs`) — absent/empty → `Failed` BEFORE any route (a send with no message is invalid).
- [ ] **`session.pause`** / **`session.resume`** route `SessionCommand::Pause` / `SessionCommand::Resume` to the target session.
- [ ] Each body **validates `requires_resource_refs` FIRST** (`self.inner.validate(req)`) + parses the target `SessionId` from the first resource_ref → missing/invalid → `Failed` (no silent skip — the `execute_kill`/`execute_profile_change` precedent).
- [ ] **`side_effect_applied = delivered`** — honest §17: the command was enqueued to a LIVE supervisor (`supervisor.route(...) == true`); a gone/unknown session (`false`) → nothing changed → a lost terminal write rolls back cleanly, NOT a false partial-success (the `execute_kill` precedent).
- [ ] The `SessionActor` command loop (`daemon/src/session/actor.rs`) **handles** the new `SessionCommand` variants (not a no-op fall-through): `SendMessage` writes the text to the session's terminal/PTY (the `write_initial_prompt` precedent; LESSON 30 fail-soft — see Step-2.5 Q1); `Pause`/`Resume` control the actor's read/drive loop (see Step-2.5 Q2).
- [ ] **INV-SEC-1 (§15):** the three bodies are reachable ONLY via the registered `ExecutorKind::Session` Gateway execute path (no new entry); `send_message` FEEDS the agent — it grants **no new authority**, the agent's resulting tool calls are still adjudicated by the existing interception (the LESSON 25/26/30 analysis — pin it, don't re-derive).
- [ ] **Contract-neutral** — no `shared/` type change (the action types + catalog froze at 4.0b-1; `inputs`/`resource_refs` are generic). **If `pause`/`resume` is found to need a NEW §5.1 `Session` enum value, STOP and escalate** (the 17-value Session enum is a frozen contract — a new value is NOT a silent add; Step-2.5 Q2).
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`daemon/src/gateway/session_executor.rs::SessionExecutor::execute` — add `SESSION_SEND_MESSAGE` / `SESSION_PAUSE` / `SESSION_RESUME` `const`s + match arms (alongside `SESSION_CREATE`/`SESSION_KILL`/`SESSION_PROFILE_CHANGE`) → `execute_send_message` / `execute_pause` / `execute_resume`, each `self.supervisor.route(target, SessionCommand::…)`. The `SessionActor` command loop (`daemon/src/session/actor.rs`) handles the new variants. Reachable from the production Gateway execute via the registered `ExecutorKind::Session` executor (the `session.kill` `/wired` precedent — pin reachability from the real gateway execute, not just a unit).

## Files expected to touch
**Modified:**
- `daemon/src/gateway/session_executor.rs` — 3 new `execute_*` methods + 3 match arms + the 3 action-type `const`s.
- `daemon/src/session/actor.rs` — `SessionCommand` += `SendMessage(String)` / `Pause` / `Resume`; the actor command-loop handling.
- `daemon/tests/session_executor.rs` — extend (the 3 bodies: validate-first / route / delivered-honesty / send-empty-fail / INV-SEC-1 reachability).
- `daemon/tests/session.rs` — the actor handles the new commands.

If implementation needs files beyond this list (e.g. a `SessionActor` adapter-stdin seam), **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/session_executor.rs` + `daemon/tests/session.rs`:

1. **`session_send_message_routes_to_supervisor`** — valid target + message → `SessionCommand::SendMessage(text)` routed; `side_effect_applied == delivered`. Why: §9.1.
2. **`session_send_message_empty_message_fails`** — empty/absent `inputs.message` → `Failed`, no route. Why: §9.1 (`I::FromInputs` required input).
3. **`session_send_message_missing_or_invalid_target_fails`** — no resource_ref / unparseable `SessionId` → `Failed` BEFORE route. Why: §6.3 precondition (the `execute_kill` precedent).
4. **`session_pause_routes_pause`** / **`session_resume_routes_resume`** — route the right `SessionCommand`; `delivered` honesty. Why: §9.1.
5. **`session_pause_validates_precondition_first`** — `requires_resource_refs` validated FIRST. Why: §6.3.
6. **`session_actor_handles_new_commands`** (`session.rs`) — the actor command loop processes `SendMessage`/`Pause`/`Resume` (not a silent drop). Why: §9.1 actor.
7. **`session_send_message_pty_write_error_degrades_soft`** — IF `SendMessage` writes the PTY in the actor: a write error does NOT fail/kill the session (the invariant is independent of the write landing — LESSON 30; the `test_prompt_write_error_does_not_fail_the_session` precedent). Why: §15/§9.1 + LESSON 30.
8. **`session_control_bodies_only_via_gateway_execute`** — the bodies are reached only through the registered `ExecutorKind::Session` execute path (the cat-1 `/wired`/import-grep precedent); `send_message` adds no executor-side authority. Why: §15 INV-SEC-1.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (contract-neutral — no `shared/` touch; **unless** Q2 forces a §5.1 value, which → escalate, not a silent add).
- **Orchestrator doc rows to write hot (Step-9 routing):** a note on the `daemon/CLAUDE.md §9.1 / SessionExecutor` row that `send_message`/`pause`/`resume` executor bodies are now LIVE (the §9.1/Appendix-A mirror). Minor; orchestrator writes it.
- **§2.5-seam (shared-contract) model touched?** NO (no shared model; `SessionCommand` is daemon-internal).

## Things to flag at Step 2.5
1. **`send_message` delivery semantics** — best-effort-enqueue (`side_effect_applied = route-accepted`; the actual PTY write is the actor's LESSON-30 soft-degrade) vs synchronous-confirm. My default vote: **best-effort-enqueue** — the executor runs on the write-actor thread and MUST NOT block on the async actor (cat-1 no-stall, the `route`/`execute_kill` precedent); the PTY write degrades soft (LESSON 30 — a failed feed = "agent idle," never "un-intercepted agent").
2. **`pause`/`resume` §5.1 status mapping** — does `pause` set a §5.1 `Session` status, or is it actor-internal control only? My default vote: **actor-internal control, NO new §5.1 value** (the actor suspends its read/drive loop; the adapter-derived status [LESSON 25] is unchanged). **If a §5.1 value is genuinely needed → STOP and escalate** (the 17-value Session enum is frozen — a new value is a contract decision for the lead, not a silent add).
3. **`session.resume` vs the survival `decide_resume`** — confirm these are DISTINCT (`session.resume` = un-pause a live actor; the daemon-restart-survival `decide_resume` is the separate survival-ladder concern). My default vote: **distinct** — no conflation; `session.resume` routes `SessionCommand::Resume` to a live paused actor.
4. **Where the actor writes `SendMessage`** — the session's terminal/PTY (the `write_initial_prompt` precedent) vs an adapter stdin seam. My default vote: **the terminal/PTY the actor owns** (the `write_initial_prompt` precedent; the adapter-stdin seam is a later refinement if a harness needs structured input).

## Dependencies + sequencing
- **Depends on:** the 4.0a/4.0b session spine (`SessionActor`/`SessionSupervisor`/`SupervisorHandle`) + the registered `ExecutorKind::Session` `SessionExecutor` (landed).
- **Blocks:** the ui **W1-C** send/pause/resume controls.
- **Sibling (NOT this slice):** **W1-exec-term** = `session.attach_terminal` (the §6.4 terminal-channel attach — a different surface, pairs with the ui xterm.js host).

## Estimated commit count
**1.** The three bodies + the actor handling are one cohesive, contract-neutral unit in the same code area. **cat-1-adjacent** (the `SessionExecutor` is the cat-1 session executor) → the implementer runs `security-reviewer` (the `invariant` policy) at Step 8; the INV-SEC-1 pins (tests 7+8: no new authority, soft-degrade) are the load-bearing asserts. The send_message-grants-no-authority analysis is settled (LESSON 25/26/30) → NOT a pre-Step-2.5 human escalation — but if the implementer's Step-2.5 surfaces a genuinely NEW safety-design question (or Q2 forces a §5.1 value), escalate before GREEN.

## Lessons-logged candidates anticipated
- **Convention candidate** — "a session-control executor body routes a typed `SessionCommand` to the supervisor (the `execute_kill` precedent); `side_effect_applied = delivered` (honest §17 — undelivered to a gone session = clean rollback); the actual PTY/stdin write is the actor's LESSON-30 best-effort; feeding an agent grants no new authority (tool calls still intercepted)."
- **Architecture-doc note candidate** — the `SessionExecutor` now drives `send_message`/`pause`/`resume` (the §9.1 / SessionExecutor row).

## How to invoke
1. Read this brief end-to-end (esp. Step-2.5 Q2 — the §5.1-frozen-enum escalation trigger).
2. Run `/tdd session_control_executor_bodies`.
3. Step 2.5 — ping back with answers (or take defaults). Escalate immediately if `pause`/`resume` needs a new §5.1 Session value.
4. Step 9 — surface the §9.1/SessionExecutor doc note + any new safety finding.
