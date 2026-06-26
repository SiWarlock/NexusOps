# Session 038 — smoke-CLI add-project driver (088) · 087 retracted · 084 still parked

- **Date:** 2026-06-26
- **Phase:** Phase 4.7 (live-validation CLI tooling) — post the ui projection-boundary merge
- **Predecessor:** [037-2026-06-26-p5.3b-profile-secrets-086-smoke-cli-084-parked.md](037-2026-06-26-p5.3b-profile-secrets-086-smoke-cli-084-parked.md)
- **Successor:** _(next session — the rescan project_id-mint fix + resume parked 084)_

## Why this session existed

Continuation after session 037 (085/P5.3b + 086 smoke-CLI shipped). Two user-priority active-blockers + a retraction, all while 084 stays parked:
the cockpit-load fix (087, retracted → ui-track) and the "can't add a project" fix (088). The ui team merged its projection-boundary
fix into the shared `main` mid-session (HEAD `b18c6f2`→`f225348`).

## What was built

- **088 / smoke-CLI rescan driver — ✅ shipped `2d8d98c`** (on `f225348`). `daemon/src/smoke.rs`: a `rescan --path <repo>` subcommand +
  the pure `build_rescan_request` helper (builds the EXISTING risk-0 `project.rescan` `{path}`, no resource_refs, via `submit_action` →
  auto-executes) + USAGE/module-doc. `daemon/tests/smoke_request_builders.rs`: +3 asserts (shape + missing/empty/whitespace `--path`).
  ZERO new daemon surface, NO CONTRACT bump, dev-client-gated. security not-triggered; code-quality 2 LOW fixed in-slice.
- **087 / ProjectionPage envelope — ❌ RETRACTED before GREEN.** Dispatched then retracted (the daemon's bare-array `get_projection` is
  contract-faithful; the fix routed to ui-track, landed via the `f225348` merge). My uncommitted RED-test edits were reverted — zero footprint.
- **084 / device-flow — ⏸️ still parked pre-GREEN.** Untouched. RED tests at `daemon/tests/device_flow.rs.parked084` (restore on resume).

## Decisions made

- 088 wraps the EXISTING risk-0 `project.rescan` (no new surface, the 086 pattern); `--path` passed verbatim (daemon owns validation); fail-closed on missing/empty/whitespace path.
- 087 retracted (lead/user routed to ui-track) — daemon stays contract-faithful.
- Committed 088 on the merged base `f225348` after re-verifying GREEN there (the ui merge was ui-only, non-overlapping).

## TDD compliance

- **088:** clean RED-first (the 2 new builder tests confirmed RED [unresolved import] → GREEN). The whitespace-path case was added at code-quality review (additive).
- **087:** RED tests written then DISCARDED on retraction (no impl, no commit).

## Cross-doc invariant audit

- **088:** no model/contract change (a dev-tool wrapping an existing action) — nothing to mirror.
- **087:** retracted — CONTRACT stays 0.46.0 (no daemon change).

## Reachability

- **088:** `nexusopsd smoke rescan --path <repo>` → run() dispatcher → `build_rescan_request` + `submit` (submit_action) → the existing GatewayPort → `project.rescan`/`ProjectExecutor` (main.rs) → risk-0 auto-execute → `ProjectRescanned`. Wired (`--features dev-client`).

## Preflight

- Re-verified GREEN on the merged base `f225348` before the 088 commit: daemon+shared default suite **82/0**, the 088 builder test **8/8** (`--features dev-client`), clippy clean (default + dev-client). The sole standing fmt-check failure is the pre-existing cross-track ui/ condition (not daemon files).

## Open follow-ups

- **🔴 rescan project_id gap (the next slice).** `smoke rescan` submits `project.rescan` with `project_id: None` → the `proj_project` projector
  HEALTHY-SKIPS the `ProjectRescanned` (it keys the row off the envelope `project_id`) → **no project row is registered + no id is surfaced** to
  the user. The 088 driver SUBMITS + auto-executes correctly, but the project doesn't appear. **Fix (next slice):** `smoke rescan` (or
  `build_rescan_request`) mints a `ProjectId::new()`, sets it on the `ActionRequest.project_id` envelope, and PRINTS it (so the user has the id for
  subsequent commands). A 1-commit dev-client follow-up; daemon-side `proj_project` fold is unchanged (it already keys on the envelope id).
- **084 resume:** rename `device_flow.rs.parked084` → `device_flow.rs`, confirm RED, GREEN the FSM (C1) → wiring (C2, the `device_flow_status` read RPC). After the validation window.
- **Queued daemon asks (lead-routed, NOT yet authored):** the AuditTrail `event_type` slice (persist raw `event_type` in `proj_audit_trail` + migration — fixes the degraded Audit tile); the UsageLedger `creditPool` Mock-vs-real gap (the daemon projection doesn't serve `creditPool`).
- **Standing residual:** the dev-client tests stay CI-dark in default `cargo test`/`/preflight` (LESSON §29; verified via `--features dev-client`). The orchestrator routes the 083-runbook `rescan` step + the §4.7 task line at `/orchestrate-end`.
