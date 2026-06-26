# ui-027 — WAVE-1 session-control lane (the cockpit can DRIVE agents)

**Date:** 2026-06-26
**Role:** ui-orchestrator (converged team `nexusops-daemon`; single working tree on `main`)
**Round:** the WAVE-1 ui session-lifecycle-control lane — 4 slices, all CONTRACT-neutral, suite GREEN throughout (→ 616/0).

## What was built

The cockpit gained the ability to **drive** agents, not just view them — the #1 WAVE-1 gap. Every live-write is held behind a **default-OFF go-live gate** pending the user's cat-1 sign-off (the deferred-go-live posture the lead set as a standing constraint).

| Slice | Commits | What |
|---|---|---|
| **ui-084** — W1-B `session.kill` | `997b203` + `2f6c677` | "Kill" control (per-row, SessionsTable Actions column via a new `rowActions` slot). Generic `submit_action` + client-mint + `{type:"session"}` resource_ref; **risk-0 auto-execute** (no modal) → error-only notice. New default-OFF `enabledSessionKill` gate. |
| **ui-085** — W1-prof UI half | `516c3c7` | Regen `generated.ts` 0.47→**0.48** (cleared the `generated_contract_version` drift RED) + the `get_execution_profiles` read transport (a `shared/src/ipc.rs` read-RESULT type — NOT a projection-row → a gateway-client read method + `parseExecutionProfilesResult` + a `provisional.ts` `.strict()` shadow + the allowlisted Tauri cmd). |
| **ui-086** — W1-C-a `session.profile_change` | `9083f8f` + `28f9874` | "Change profile" control: an on-demand native `<select>` reading `get_execution_profiles` (pre-select `is_default`, "needs credential" off `has_credential`, shows the current profile), **risk-2 → approval-gated** → a non-optimistic "submitted for approval" notice. New default-OFF `enabledProfileChange` gate. |
| **ui-087** — W1-C-b drive controls | `e1f681f` + `ff8389b` | Send message + **Pause monitoring** + **Resume monitoring** in the SessionTerminal detail header (replaced the old disabled-Pause placeholder). All **risk≥1 → approval-gated**. New default-EMPTY `enabledSessionControls: Set` gate (seeds the per-control-boolean consolidation). |

## Decisions made

- **Gate posture per the lead's standing constraint:** every new session live-write rides its own default-OFF gate (`enabledSessionKill`/`enabledProfileChange` booleans; the `enabledSessionControls: Set` at ui-087, seeding consolidation). Action-scoped + `mutationsEnabled`-independent (a new high-consequence write never auto-rides the live L2 flip) + a held-flip production guard. **All stay default-OFF** until the user's cat-1 go-live sign-off.
- **Notice keyed to the catalog RISK class:** risk-0 (kill) → error-only (result via the live nudge); risk≥1 (profile_change / drive) → non-optimistic "submitted for approval" (the daemon `ActionAck.status`, enters the live ApprovalQueue). → LESSON [[39]].
- **`get_execution_profiles` is a read-RPC RESULT type, not a projection-row** → a hand-modeled `provisional.ts` `.strict()` shadow + a dedicated boundary parser, consumed directly (the daemon-orchestrator clarified the `shared/src/ipc.rs` placement). → LESSON [[40]].
- **Honest labeling for the soft pause** (the lead's load-bearing finding, daemon LESSON §71): "Pause monitoring"/"Resume monitoring", never stop/suspend — pinned across visible text AND `title`/`aria` (a security-LOW caught "stop" in a tooltip). → LESSON [[41]].
- **profile_change Q2 (the load-bearing call):** a risk-2 approval-gated action STILL gets a default-OFF UI gate (the `enabledPrMutations` precedent + defense-in-depth; the daemon approval is the operative §15 #8 checkpoint).

## Decisions explicitly NOT made

- **No gate was flipped.** Launch (ui-083, prior round), Kill, Change-profile, and the drive controls are all wired but HELD default-OFF — the user's cat-1 go-live sign-off + visual gate is theirs on return.
- **No `shared/` change in any slice** (CONTRACT-neutral; the 0.47→0.48 bump was the daemon's W1-prof seal `1cbb712`, mirrored by the ui-085 regen — LESSON §69).
- **Did not refactor the signed-off launch/kill/profile_change booleans** into `enabledSessionControls` (the Set seeds it; the migration is a deferred follow-on).

## Open follow-ups (carry-forwards — routed to the daemon-orchestrator for the shared-doc edits)

- **W1-C-d (attach_terminal)** — the last W1-C piece — gated on daemon W1-exec-term + an xterm.js host (neither built).
- **git-hunk UI activation** — the wired-but-stubbed ui-6.3e staging surface becomes functional when daemon W1-git-stage (095) lands (in progress). **My next ui work when it seals.**
- **Carry-forwards** (for IMPLEMENTATION_PLAN.md, via the daemon-orchestrator): consolidate the per-control booleans → `enabledSessionControls: Set` · the kill `isSessionKillable` denylist → an `isLiveSession` allowlist (security-LOW hardening) · the profile_change picker pre-select current-else-default (go-live UX) · an AbortController on the on-demand `get_execution_profiles` fetch · the profile_change notice info-vs-error styling split · (daemon LESSON §71) the real OS-suspend pause + a projected paused-state.
- **Plan ticks (via the daemon-orchestrator):** W1-A / W1-B / W1-C (the ui session-control lane) — reconcile against the landed slices.
- **The session.kill operative-hold arch note** (ARCHITECTURE.md, via the daemon-orchestrator): a risk-0 session control has NO daemon-side go-live toggle → the UI default-OFF gate is the OPERATIVE go-live hold (raises the held-flip's weight at the user's cat-1 sign-off).
- **The ui/CLAUDE.md "Generated Zod contract layer" cross-doc row** is behind (last note @0.46; needs the 0.47 [092] + 0.48 [W1-prof] regens) — coordinating ownership with the daemon-orchestrator (it's ui-area but the lead routed "the 0.48 contract-layer doc-row" to the shared-doc owner).
