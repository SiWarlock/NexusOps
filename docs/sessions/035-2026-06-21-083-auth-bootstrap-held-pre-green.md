# Session 035 — 083 auth-bootstrap held pre-GREEN at the HARD-STOP safety-net boundary

- **Date:** 2026-06-21
- **Phase:** Phase 4 / **P4.7** (the auth-bootstrap arc — the live-mutation go-live prerequisite)
- **Predecessor:** [034 — P5.3a registry · head_sha · the confused-deputy closure](034-2026-06-21-p5.3a-profiles-headsha-confused-deputy.md)
- **Successor:** _(the post-cycle fresh implementer — resumes 083 from this clean pre-GREEN boundary)_

## Why this session existed

After the §4.7 round sealed + pushed (`6b9a6c8`), the cat-1/secret-touching auth-bootstrap slice (083) was dispatched: the keychain-write primitive + `gh auth token` reuse + per-account keychain-backed octocrab clients + the live-writes toggle — the last live-mutation go-live prerequisite. `/tdd github_auth_bootstrap_keychain_gh_reuse` reached an **APPROVED Step-2.5** (after one TWEAK), then a `/context-check` **HARD-STOP** (daemon-implementer=80%) hit. The lead first held, then reversed (slice-atomicity — land the sensitive slice clean); the implementer invoked the **safety net's re-review-strain branch** and stopped at the clean pre-GREEN boundary rather than risk a strained/half landing.

## What was built (RED only — NOT landed)

**No production code committed this session.** The 14 RED tests are written + left **uncommitted in the working tree** (preserved WIP for the successor):
- `daemon/tests/keychain.rs` (NEW, 8) — the auth surface against injected `SecretStore` + `GhTokenSource` seams: keychain roundtrip, ref-only-no-secret, gh-reuse acquisition, gh-absent-typed-error, per-account token selection (confused-deputy), no-token-fail-closed, toggle-off-blocks-write, `read_also_gated_on_toggle`.
- `shared/tests/contract.rs` (3) — `IntegrationLiveWritesSet` event snapshot, `integration.set_live_writes` catalog entry (risk-2/Integration/NON-standing-grantable), CONTRACT pin → 0.45.0.
- `daemon/tests/integration_connections_proj.rs` (1) — `test_live_writes_set_folds_to_proj` (derive-from-event fold, default OFF, rebuild-equivalent).
- `daemon/tests/policy.rs` (1) — `test_set_live_writes_denied_for_non_ui_requester` (UI/IPC-only).
- `daemon/tests/integration_connect.rs` (1) — `test_set_live_writes_emits_event`.

These reference not-yet-existing symbols (`nexusopsd::integrations::keychain`, `::auth`, `IntegrationLiveWritesSet`, the `integration.set_live_writes` catalog entry, the `live_writes_enabled` column) → they are compile-RED by design; **Step 3 (confirm RED) was NOT reached.**

## Decisions made (Step-2.5 — settled, APPROVED-to-GREEN for the successor)

- **Q1 gh acquisition** = shell `gh auth token` over the `GhTokenSource` seam; copy-to-keychain per account; gh-absent → typed `AuthError::GhUnavailable` (cleanly handled) → **device-flow 084 stays the fast-follow** (not promoted to first).
- **Q2 keychain_ref** = structured `keychain_ref_for(provider, account)` (e.g. `nexusops/github/octocat`), per-account keyed.
- **Q3 the toggle = a Gateway-action+event+projector CONTRACT vertical** (lead-ruled): `integration.set_live_writes` (risk-2, UI/IPC-only, NON-standing-grantable) + `IntegrationLiveWritesSet{connection_id, enabled}` folded into `proj_integration_connection.live_writes_enabled` (derive-from-event, default OFF, rebuild-safe). **CONTRACT 0.44.0→0.45.0.** Connection identity = **input-carried** (`{connection_id, enabled}`, requires_resource_refs=false) **+ ADD: validate connection_id references a REGISTERED connection → else fail-closed** (the resolve_profile-on-unknown precedent; needs a `set_live_writes_fails_closed_on_unknown_connection` test at GREEN).
- **Q4 gate BOTH reads + writes on the toggle** (single `resolve_authed_token` gate) — no authed call goes live pre-toggle (the re-review certifies the read-path confused-deputy too). Public-repo reads unaffected.
- **Q5 the mandatory in-arc security re-review** gates C1 (secret-write) + C5 (toggle-enable) — each held until the security-reviewer `invariant` pass is CLEAR; surface the result at Step 9 before C5.
- **Commit cut = ~5, freeze-then-vertical** (D9/D10 precedent): C1 keychain primitive (own commit, safety-pin) · C2 gh-reuse acquisition + connection registration · C3 clients read keychain-token-by-resolved-account (main.rs) + the toggle gate-check · C4 the toggle contract-freeze (event + catalog + 0.45.0 + 3-way verify) · C5 the toggle vertical (executor emit + projector fold + MIGRATION live_writes_enabled column + policy gate + the live-writes-enable).

## Decisions explicitly NOT made (deferred)

- **All of 083's GREEN** — deferred to the successor (the HARD-STOP cycle; no clean re-review-CLEAR committed boundary was reachable at 80%).
- **084 device-flow** — HALTED (not dispatched); the token-lifetime fork (non-expiring vs expiring; client_secret-can't-ship) is a user decision pending for 084.

## TDD compliance

Clean so far — RED-first; Step-2.5 reviewed + APPROVED (one TWEAK applied). No GREEN written → no possibility of test-after-impl. No commits → no violations.

## Reachability

N/A this session (no production code landed). On resume: the keychain primitive + the per-account token clients wire at `main.rs` (`ExecutorKind::Github` clients + the read client at `:290`/`:435`, replacing `Octocrab::default()`); the `integration.set_live_writes` action routes through the catalog-gated pipeline → `IntegrationExecutor`; the gh-reuse acquisition is an IPC-triggered daemon path.

## Open follow-ups

- **RESUME 083 (the fresh successor):** brief `083` is committed in seal `6b9a6c8`; the 14 RED tests are uncommitted WIP in the working tree. Confirm RED → GREEN C1→C5 (the settled design above) → security-reviewer `invariant` every layer; C1 + C5 each gated on the re-review CLEAR; surface the re-review result at Step 9 before C5.
- **Security re-review status: NOT RUN** (would strain at 80%). The live-writes toggle is therefore **NOT enabled** — no commit landed; the live Merge/Review buttons remain gated.
- **Cross-doc (when 083 lands):** the §15 #3/#4 keychain-primitive AS-BUILT + the §9 auth-bootstrap (gh-reuse) AS-BUILT + a LESSON (the daemon-sourced no-inbound-IPC-secret keychain-write primitive) + the §4.7 reconcile (auth-bootstrap interim → live buttons unblocked, device-flow 084 remains) + the CONTRACT 0.45.0 event/catalog rows. (Orchestrator writes hot when 083 lands — none due now since 083 didn't land.)
- **084 device-flow** — the fast-follow after 083.

## How to use what was built

Nothing landed. The successor resumes 083 from the preserved RED-WIP + the settled Step-2.5 design.
