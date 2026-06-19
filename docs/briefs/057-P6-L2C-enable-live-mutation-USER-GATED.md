# /tdd brief — l2_enable_live_mutation

> **🔒 USER-GATED (L2-O3). HELD — do NOT dispatch until the user explicitly signs off.** The lead runs
> the sign-off via `AskUserQuestion` (the verification surface below); on the user's GO the orchestrator
> dispatches this slice. This brief is authored + ready, NOT yet dispatched.

## Feature
**L2-C (the THIRD + FINAL L2 sub-slice — the GO-LIVE flip, USER-gated).** Flip the production Shell's
`UdsGatewayPort` construction to `mutationsEnabled: true` — the single switch that lights up BOTH the
mutation transport (the port methods `invoke` instead of throw-not-enabled) AND the UI controls (the
`GatewayModal` approve/deny + `DiffReview` per-hunk submit enable via `canSubmitIntent &&
mutationsEnabled`). After this, **a real human can approve/deny/submit a real, daemon-risk-classified
mutation from the cockpit** — the daemon's Action Gateway executes it (INV-SEC-1) and records the audit
event. This is the trust boundary going live. **`security-reviewer` REQUIRED** (the cat-1 go-live — the
full live path end-to-end). **The deterministic pin is one line; the load-bearing acceptance is the live
real-daemon verification (the manual operator gate the user signs off on).**

## Use case + traceability
- **Task ID:** P6.8 L2-C (the live mutation transport, sub-slice 3 of 3; A crate RPCs ✅ → B Tauri+TS wire [disabled] ✅ → **C enable-live [USER-gated]**)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the live mutation submit — reaching the daemon's intent→policy→approval→execute→audit pipeline), `§11.5` (the approval card drives the live approve/deny), `§11.4` (`canSubmitIntent` — now load-bearing), `§4.2` (the UI submits intents only; the daemon executes).
- **Reference:**
  - **The L2 cat-1 checkpoint** (`docs/planning/L2-live-mutation-transport-cat1-checkpoint.md`, lead-RULED): **🔒 L2-O3 = (A) slice C gated on EXPLICIT USER SIGN-OFF**; Part A (D1–D7) re-pinned on the live path; L2-O2 live preview rides the enable; L2-O4 idempotency/fencing pass-through.
  - **The single switch (L2-B `b3ffcb3`-successor):** `mutationsEnabled` is a `GatewayPort` field; `UdsGatewayPort` default false. The flip site: `ui/src/shell/Shell.tsx:124` (`gateway ?? new UdsGatewayPort()` → `gateway ?? new UdsGatewayPort({ mutationsEnabled: true })`). The 3 layers L2-B built (controls disabled / seam readOnly / port throw) all key off this one flag — flipping it lights them all up.
  - The durable cat-1 Q1–Q7 (`docs/planning/intent-seam-cat1-safety-design.md`) — consume; the seam is a pure submitter, no optimistic-done, daemon-driven card, verbatim §6.4 cards (043/044). L2-B/LESSON 27.
  - The §11.5 cards (044/LESSON 17), the verbatim §6.4 routing (LESSON 16/26), the live transport (LESSON 22/23/26/27).

## Acceptance criteria (what "done" means)
- [ ] **The production Shell constructs the port mutations-enabled.** `Shell.tsx` default `new UdsGatewayPort({ mutationsEnabled: true })`. Pin: the production-default port has `mutationsEnabled === true` (and the injected-Mock path is unaffected).
- [ ] **The controls enable when connected.** With the production port + a connected+version-compatible state (`canSubmitIntent` true), the `GatewayModal` approve/deny + `DiffReview` submit controls are ENABLED (the L2-B disabled-when-`mutationsEnabled`-false pins now flip to enabled). Pin: connected + the live port → controls enabled.
- [ ] **The live submit path reaches the daemon (no new behavior, re-pinned live).** The port mutation methods `invoke` (no longer throw-not-enabled); a submit/approve/deny renders the daemon's `ActionAck` status — **no optimistic "done"** (Q3/L2-D4 — status from the daemon only); a §6.4 `WireError` renders its distinct §11.5 card (`fencing_conflict`→hard-conflict #6, etc. — Q6/L2-D6, verbatim). These are L2-A/B-tested; re-assert at the Shell-integration level that the live port + enabled controls compose correctly.
- [ ] **The `policy_grant` "always allow" standing-grant STAYS disabled** (its own cat-1 — NOT this slice). Pin: the standing-grant control stays disabled even with mutations enabled.
- [ ] **`security-reviewer` REQUIRED:** the full live path (submit→daemon→ack-render; no optimistic-done; verbatim §6.4 cards; `canSubmitIntent` load-bearing + fail-safe [the 054 single-authority gate]; the daemon Gateway is the real INV-SEC-1 chokepoint, the UI gate defense-in-depth); the standing-grant stays disabled.
- [ ] **The real-daemon live verification (the user's sign-off surface).** A manual operator gate (the live Tauri-window cross-track step, the LESSON 22/23 precedent — no daemon in the ui worktree): launch the cockpit against a real daemon → submit/approve a real action → the daemon executes + records the audit event → the cockpit reflects the daemon-confirmed status. **This is the verification the lead presents to the user for sign-off.**
- [ ] Whole suite green; `/preflight` clean; cross-doc flagged at Step 9.

## Wiring / entry point (Step 7.5)
**LIVE — this IS the go-live.** `Shell.tsx:124` constructs the production `UdsGatewayPort` mutations-enabled
→ `GatewayModal`/`DiffReview` submit handlers → the port's live `invoke` → the Tauri mutation command →
the daemon Action Gateway. `/wired`: the mutation methods now trace from the enabled controls through
the live invoke to the daemon (no longer throw-not-enabled). The `canSubmitIntent` gate (054 single
authority) is the load-bearing pre-submit gate; the daemon Gateway is the real chokepoint.

## Files expected to touch
**Modified:**
- `ui/src/shell/Shell.tsx` — the one-line `mutationsEnabled: true` flip + its test (`Shell.*.test.tsx`).
- Possibly a Shell-integration test asserting the live port + enabled controls compose (the controls enable when connected; the standing-grant stays disabled).

If the flip needs more than the Shell construction (it should not — the single flag drives all 3 layers), **flag at Step 2.5**.

## RED test outline (Step 2)
1. `production_shell_port_has_mutations_enabled` — the production-default Shell port → `mutationsEnabled === true` (the Mock-injected path unaffected). — Asserts: the go-live flip (§6.1).
2. `controls_enabled_when_connected_and_mutations_enabled` — connected (`canSubmitIntent` true) + the live port → the `GatewayModal` approve/deny + `DiffReview` submit are ENABLED. — Asserts: the controls light up (§11.4/§11.5).
3. `controls_still_disabled_when_disconnected` — disconnected (`canSubmitIntent` false) → the controls stay disabled even with mutations enabled (the fail-safe gate holds — the 054 single authority). — Asserts: §11.4 fail-safe (the gate is still load-bearing).
4. `standing_grant_stays_disabled_live` — the `policy_grant` "always allow" control stays disabled even live. — Asserts: the standing-grant is its own cat-1 (not this slice).
5. (live, manual) `real_daemon_submit_approve_executes_and_audits` — the operator gate: a real submit/approve → the daemon executes + audits → the cockpit shows the daemon-confirmed status (no optimistic-done). — Asserts: the live go-live (the user's sign-off surface).
Each carries `Asserts: <invariant> (§anchor)`; the coverage map ties each acceptance bullet.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (the flip consumes the L2-B `mutationsEnabled` field). No `shared/` change; no CONTRACT bump; no schema-snapshot.
- **Orchestrator doc rows (Step 9):** the `ui/CLAUDE.md` "Live `UdsGatewayPort` transport client" row → **L2 COMPLETE / the mutation transport is LIVE** (the go-live flip, user-signed-off) + a LESSON (the go-live = one flip + the live-verification operator gate). No `ARCHITECTURE.md` edit.
- **Shared-contract (cross-area) model touched?** No.

## Things to flag at Step 2.5
1. **Flip-only, or a feature-flag/setting?** Default: a **direct `mutationsEnabled: true` flip** at the Shell construction (the simplest go-live; the user signed off on going live, so a runtime toggle is unneeded). Alternative: a build/env flag (`L2_LIVE`) for staged rollout. Default vote: **the direct flip** (the user gate IS the rollout control; no extra surface). Flag if a flag is wanted.
2. **The live verification scope.** Default: the manual real-daemon operator gate (submit + approve one action, observe execute + audit + the confirmed status) — the LESSON 22/23 manual-cross-track precedent (no daemon in the ui worktree). Flag if a fuller live matrix is wanted before sign-off.
3. **Optimistic-done re-assert.** Default: re-assert at the Shell-integration level that an in-flight submit renders the daemon status (not optimistic) — the 044 pins hold; this just confirms they compose with the live port. Flag if a new pin is needed.

## Dependencies + sequencing
- **Depends on:** L2-A ✅ (the crate RPCs) + L2-B ✅ (the bridge + the `mutationsEnabled` guard + the controls gating) + **the user's explicit sign-off (L2-O3)**.
- **Blocks:** nothing — this COMPLETES L2 (the live mutation transport) + the go-live. Follow-ons (parked): the `policy_grant` standing-grant (own cat-1), Q7-B/C caching (parked-for-user), `submit_action_plan`/per-step (follow-on), the other-5-projection live deltas.

## Estimated commit count
**1** (the go-live flip — one focused cat-1 unit). **security-reviewer REQUIRED** (the cat-1 go-live). **🔒 USER-GATED — held until the user signs off.**

## Lessons-logged candidates anticipated
- **Convention candidate** — the cat-1 go-live is a SINGLE flag flip (the L2-B `mutationsEnabled` gate both the transport + the controls honor) gated on explicit user sign-off + a real-daemon live-verification operator gate; the deterministic surface is one line, the load-bearing acceptance is the live verification. Completes the L2 lesson arc (26/27).
- **Architecture-doc note candidate** — L2 COMPLETE: the UI's live mutation path is on (a real human approves real, daemon-risk-classified actions); the daemon Gateway remains the INV-SEC-1 chokepoint.

## How to invoke
> **Only after the user signs off (the lead relays the GO).** Until then this brief is HELD.
1. **Read this brief end-to-end** — the one-line flip + the live-verification gate + the 3 Step-2.5 flags.
2. Pre-flight: `track/ui` (L2-A + L2-B sealed). Same session — no `/session-start`.
3. **Run `/tdd l2_enable_live_mutation`**.
4. Step 0/1 — confirm Feature + Files.
5. **Step 2.5** — answer the 3 flags + send the test-design write-up + coverage map; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` REQUIRED (the full live path / no optimistic-done / verbatim §6.4 cards / `canSubmitIntent` load-bearing / standing-grant stays disabled).
7. Step 9 — the cross-doc flag (L2 COMPLETE) + the go-live lesson + the live-verification record; then `/session-end` (L2 is done).
