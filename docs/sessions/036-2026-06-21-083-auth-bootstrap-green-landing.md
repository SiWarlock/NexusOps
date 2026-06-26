# Session 036 — 083 auth-bootstrap GREEN landing (the 6-commit Option-B live vertical)

- **Date:** 2026-06-21
- **Phase:** Phase 4 / **P4.7** (the auth-bootstrap arc — the live-mutation go-live prerequisite)
- **Predecessor:** [035 — 083 auth-bootstrap held pre-GREEN at the HARD-STOP boundary](035-2026-06-21-083-auth-bootstrap-held-pre-green.md)
- **Successor:** [037-2026-06-26-p5.3b-profile-secrets-086-smoke-cli-084-parked.md](037-2026-06-26-p5.3b-profile-secrets-086-smoke-cli-084-parked.md)

## Why this session existed

The predecessor (035) cycled at a clean pre-GREEN boundary: 083 (the cat-1 GitHub auth-bootstrap — the FIRST secret entering the daemon) was at an APPROVED Step-2.5 with 14 RED tests written but uncommitted, before the multi-commit secret arc started. This session resumed from that boundary with a fresh context budget and drove 083 RED→GREEN to a landed, pushed-ready round. Mid-slice the USER ruled **Option B** (the full live vertical, not just the deterministic mechanism), which extended the slice to light up the entire authenticated-GitHub path end-to-end.

## What was built

### Files created
- `daemon/src/integrations/keychain.rs` — the keychain-write primitive: `SecretStore` trait + `KeyringSecretStore` (OS keychain via the `keyring` crate) + `FakeSecretStore` (cfg-gated). The single home for secret-at-rest (§15 #3/#4).
- `daemon/src/integrations/auth.rs` — the gh-reuse auth surface: `keychain_ref_for`/`acquire_via_gh` (token `Zeroizing`-scrubbed) · the SINGLE `resolve_authed_token` live-writes gate · `AuthError` · `GhTokenSource`+`GhCliTokenSource` · `LiveWritesGate` trait · `GithubAuthResolver` (`resolve_token_for`/`octocrab_for` — the per-owner authed handle) · `GhConnector`+`GhCliConnector` (the "Connect via gh" worker) · cfg-gated fakes.
- `daemon/src/integrations/connections.rs` — the read-only-WAL seams kept OUT of the executor module (INV-SEC-1 grep guard): `SqliteConnectionLookup` (registered-connection gate) + `SqliteLiveWritesGate` (the per-(provider,account) toggle read, fail-closed).
- `daemon/tests/keychain.rs` — 10 deterministic tests over the `SecretStore`/`GhTokenSource`/`LiveWritesGate` seams (roundtrip, ref-only-no-secret, gh-reuse, gh-absent, per-account confused-deputy, no-token fail-closed, toggle on/off, read-also-gated, the `GithubAuthResolver` decision).

### Files modified
- `shared/src/events.rs` — NEW `IntegrationLiveWritesSet{connection_id, enabled}` event (no secret).
- `shared/src/catalog.rs` — NEW `integration.set_live_writes` catalog entry (risk-2, `ExecutorKind::Integration`, `requires_resource_refs=false`, `standing_grant_eligible=false`); MVP 31→32.
- `shared/src/ipc.rs` — NEW `ConnectViaGhParams`/`ConnectViaGhResult`/`ConnectViaGhStatus` §6.1 wire types (NO token field).
- `shared/src/lib.rs` — `CONTRACT_VERSION` 0.44.0 → **0.45.0** (+ doc).
- `shared/src/schema.rs` + `shared/contracts/schema/nexusops-contract.schema.json` — bundle the new types + regenerated golden.
- `daemon/src/integrations/connect.rs` — the `integration.set_live_writes` executor arm (emits the event) + the `ConnectionLookup` trait + fail-closed-on-unknown validation.
- `daemon/src/projections/integration_connections.rs` — folds `IntegrationLiveWritesSet` → `proj_integration_connection.live_writes_enabled` (derive-from-event, default OFF, rebuild-safe, LESSON §17).
- `daemon/src/eventstore/schema.rs` + `migrations.rs` — `MIGRATION_18` (the `live_writes_enabled` column, ALTER ADD); `SUPPORTED_USER_VERSION` 17→18.
- `daemon/src/gateway/policy.rs` — deny-before-risk gate: `integration.set_live_writes` is UI/IPC-requester-only.
- `daemon/src/integrations/github.rs` + `github_write.rs` — the live read+write octocrab clients now build a PER-OWNER handle via `GithubAuthResolver::octocrab_for(owner)` (replacing `Octocrab::default()`).
- `daemon/src/ipc/methods.rs` — the `connect_via_gh` handler (+ routing).
- `daemon/src/ipc/server.rs` + `daemon/src/runtime/listener.rs` — thread the `&dyn GhConnector` (post-auth).
- `daemon/src/main.rs` — construct `KeyringSecretStore` + `SqliteLiveWritesGate` + the resolver for both github clients + `IntegrationExecutor` with `SqliteConnectionLookup` + `GhCliConnector` into the accept loop.
- `daemon/Cargo.toml` + `Cargo.lock` — add `keyring` (C1) + `zeroize` (C2).
- tests: `integration_connect.rs`, `integration_connections_proj.rs`, `policy.rs`, `gateway_plan.rs`, `ipc.rs`, `runtime.rs`, `shared/tests/contract.rs` — the toggle vertical, the gate, the connect_via_gh dispatch, the wire-type snapshot, the threading.

### Commits (the 6-commit Option-B cut; Option-1 whole-file landing)
- `335929b` C1 — keychain-write primitive (the secret-write safety-pin, its own commit)
- `5572386` C2 — gh-reuse acquisition + the `resolve_authed_token` gate (+ `zeroize`)
- `4ca090a` C4 — contract freeze (`IntegrationLiveWritesSet` + `integration.set_live_writes` + connect_via_gh wire types + CONTRACT 0.45.0 + schema)
- `266162e` C5 — toggle vertical (executor + `ConnectionLookup` + projector + MIGRATION_18 + policy)
- `e0830ed` C3a — live keychain-backed github clients (per-owner auth)
- `b61cad4` C3b — the `connect_via_gh` IPC trigger (daemon-sourced gh token → keychain)

## Decisions made
- **Option B (USER ruling) — full live vertical this slice.** Escalated the C3 scope (mechanism-first vs full live wiring) at Step 7.5; the orchestrator routed it to the user (a brief-acceptance-criterion deferment), who ruled **B**: wire the live github clients (per-owner keychain auth) + build the "Connect via gh" IPC trigger now.
- **Connection identity input-carried + validate-REGISTERED (settled Step-2.5).** `integration.set_live_writes{connection_id, enabled}` (requires_resource_refs=false), with the executor validating the connection is REGISTERED via the `ConnectionLookup` seam → else fail-closed (the `resolve_profile`-on-unknown precedent, LESSON §62).
- **The single `resolve_authed_token` gate (Q4).** BOTH the authed read + write clients gate on the same per-connection toggle; default OFF; per-owner token selection (confused-deputy-safe, consuming the §63-audited owner — never `inputs`).
- **The keychain write is a non-Gateway substrate op (LESSON §49).** `connect_via_gh` writes the keychain but emits no audit event; the audit trail is the subsequent `integration.connect` registration (which records the keychain_ref pointer). The mandatory in-arc **security re-review ruled this INV-SEC-1-acceptable on the merits** (not a new safety fork, not a Finding) — consistent with LESSON §49/§58/§62.
- **The 2 first-pass security LOWs folded in:** `FakeSecretStore`/`FakeGhTokenSource`/`FakeGhConnector` cfg-gated out of the prod binary; the transient gh token wrapped in `Zeroizing` (scrubbed after the keychain write).
- **Commit mechanics = Option 1 (orchestrator pick).** The 6 logical commits land whole-file in order; the final committed tree == the verified green state. `git add -p` is unavailable, and several files span multiple logical layers (`auth.rs`=C2/C3a/C3b, `main.rs`=C3a/C3b/C5, `mod.rs`=all-3) — so intermediate commits group by layer but are not each independently `cargo build`-clean. C1 isolates the secret-write file (the safety-pin's review/revert intent at the file level).

## Decisions explicitly NOT made (deferred)
- **084 device-flow "Connect GitHub"** — HELD pending the user's next-slice pick (lead-directed; do NOT start). The token-lifetime fork (non-expiring vs expiring; client_secret-can't-ship) remains the user decision for 084.
- **An acquisition-audit observation event** (a `*SecretAcquired` System-actor event for the keychain write) — security-ruled an OPTIONAL future enhancement, NOT a current gap.
- **End-to-end Zeroizing on the consumption path** — `resolve_authed_token`'s read-back returns a plain `String` (handed to octocrab's own `SecretString`); the brief's "zeroized after the keychain write" property is met on the ACQUISITION transient. Optional future hardening.
- **The gh-stdout `Vec<u8>`/error-case buffer zeroize** — best-effort (documented in Cargo.toml); the `String` backing is scrubbed, the partial/error-path buffer is not.

## TDD compliance
- **Core 083 (C1/C2/C4/C5): strict RED-first** — the 14 pre-written RED tests drove the deterministic core (keychain primitive, gh-reuse, the gate, the event/catalog/contract, the toggle vertical) RED→GREEN; the `set_live_writes_fails_closed_on_unknown_connection` ADD landed RED-first at GREEN per the settled Step-2.5.
- **TDD nuance (flagged, not a safety skip):** the C3 deterministic seams (`resolve_token_for`, `SqliteLiveWritesGate`, the `connect_via_gh` handler branches) were tested **after** the impl during the mid-slice Option-B extension, not strictly RED-first. Each added test asserts a real invariant (per-account/toggle/fail-closed; the gate read; the connect_via_gh connected/gh_unavailable/keychain-fault branches) + the reviewer-requested edge cases (NULL-account fail-closed, keychain-fault→internal_error) — none was back-filled to paper a green pass. The C3 NON-deterministic edges (live octocrab clients, `gh` shell-out, real keychain) are TDD-exempt per the project's non-deterministic-coverage path (injected fakes + the mandatory live-path security re-review + manual verification).
- No test was modified to make a stuck impl pass; no implementation-before-test on a safety-critical deterministic path in the core.

## Cross-doc invariant audit
Model/contract field changes this session (ALL flagged at Step 9; the orchestrator confirmed receipt in its SHIP message and routes the doc rows hot at `/orchestrate-end`):
- NEW event `IntegrationLiveWritesSet` (EventTypeRegistry) · NEW catalog action `integration.set_live_writes` (MVP 31→32) · NEW projection column `proj_integration_connection.live_writes_enabled` (MIGRATION_18, SUPPORTED_USER_VERSION 17→18) · NEW §6.1 `connect_via_gh` method + the 3 wire types · CONTRACT 0.45.0.
- Single-track checkout: the `ARCHITECTURE.md`/Appendix-A/`daemon/CLAUDE.md`-table rows are the orchestrator's `/orchestrate-end` hot-routing (sealing in parallel) — the documented stagger, not a drift violation. No unflagged field change.

## Reachability
- `integration.set_live_writes` — reachable: catalog → policy (deny-before-risk) → `IntegrationExecutor` (registered in main.rs with `SqliteConnectionLookup`) → emits `IntegrationLiveWritesSet` → `IntegrationConnectionProjector` (registered) folds the column.
- `resolve_authed_token` / `GithubAuthResolver` — reachable: both live github clients call `octocrab_for(owner)` per request; main.rs builds them KeyringSecretStore + SqliteLiveWritesGate-backed.
- `connect_via_gh` / `acquire_via_gh` / `GhCliConnector` — reachable: IPC dispatch (`methods.rs` → `server.rs` → `runtime/listener.rs`, wired in main.rs `spawn_accept_loop`); peer-authed (getpeereid §15 #7 runs first).
- `KeyringSecretStore` / `SqliteLiveWritesGate` / `SqliteConnectionLookup` — constructed in main.rs.
- No tested-but-unwired gaps. The full auth path is production-reachable end-to-end (the prior C3 reachability gap is CLOSED).

## Open follow-ups
- **084 device-flow** (HELD; user's next-slice pick) + the token-lifetime fork resolution.
- **Optional acquisition-audit observation event** (`*SecretAcquired`) — security-noted, not required.
- **Zeroize consumption-path residual** + the gh-stdout error-buffer best-effort scrub — optional hardening.
- **gh-reuse per-owner interim limitation:** a repo whose owner ≠ the connected gh account won't resolve a token (connect the matching account) — documented; the per-account model is the confused-deputy-safe design.
- **3-way schema verify** (zod/pydantic) — the off-loop `/phase-exit` gate (LESSON §29); not run this slice (flagged at Step 9).
- **Cross-doc rows** (CONTRACT 0.45.0 + connect_via_gh method/wire types + §15/§9-NOW-LIVE/§6.1 AS-BUILT + the keychain-primitive LESSON) — the orchestrator's `/orchestrate-end` hot-routing.

## How to use what was built
- A user connects GitHub: the UI calls `connect_via_gh{provider, account}` → the daemon reads `gh auth token` → stores it under `nexusops/github/<account>` in the OS keychain → returns the keychain_ref; the UI then registers the connection via `integration.connect` (audited) and enables live writes via `integration.set_live_writes{connection_id, enabled:true}` (risk-2, UI-approved). Thereafter the live github read+write clients resolve the per-owner token through the toggle gate; with the toggle OFF (default) every authed call fails closed (unauth → the existing AuthFailed path).
