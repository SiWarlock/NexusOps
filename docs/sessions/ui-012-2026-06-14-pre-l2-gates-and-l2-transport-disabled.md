# ui-012 — the pre-L2 gates (044 + 052 resolved) + the L2 mutation transport built & guarded-disabled (L2-A + L2-B)

- **Date:** 2026-06-14
- **Phase:** Phase 6 (ui-resume) — **P6.8 L2** (the two pre-L2 NON-cat-1 gates + the first two L2 cat-1 sub-slices, foundation-first L2-O1=(B))
- **Predecessor:** [ui-011](ui-011-2026-06-14-L2prep-regen-0.31-and-approval-real-risk.md)
- **Successor:** _(none yet)_
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `ui-team-lead`

## Why this session existed

After the L2-prep regen sealed (ui-011, 053 → 0.31.0 + the clean `enrichApproval` real-risk swap), the lead ruled L2 GO with a foundation-first sequence (**L2-O1=(B): A crate RPCs → B Tauri+TS wire [disabled] → C enable-live [USER-gated]**). This round cleared the two remaining **pre-L2 NON-cat-1 gates** (the 044 [med] per-hunk half + the 052 two-writer connection-state Finding) and then built the **first two L2 cat-1 sub-slices** — the crate mutation transport (L2-A) and the Tauri+TS mutation bridge (L2-B), the latter wired but **provably unreachable in production** behind a single `mutationsEnabled` go-live switch. L2-C (the USER-gated enable) is the only L2 work left; it is authored-but-HELD for the lead's user sign-off (L2-O3).

## What was built (4 slices — 4 commits)

| Slice | Commit | What | security-reviewer |
|---|---|---|---|
| 053b — per-hunk real-risk swap (NON-cat-1) | `ca42732` | `enrichHunkAction` swapped off the fixture → the REAL `ApprovalQueueRow` via a `get_projection("ApprovalQueue")` re-fetch + EXACT `action_request_id` match (reuse `enrichApproval`); absent → honest awaiting; malformed → honest degrade. **Resolves the 044 [med] on BOTH approval-card paths.** | CLEAR |
| 054 — connection single-authority (NON-cat-1) | `b3ffcb3` | Collapse the two-writer `connection`-state race to one authority (the port): new `notifyConnectionState` drive + a `streamDegraded` axis suppressing the read-path UPGRADE while the stream is degraded; the Shell's one React writer = `onConnectionChange`. **Resolves the 052 Finding.** | CLEAR |
| 055 / L2-A — crate mutation RPCs (cat-1 Part A) | `49b61d5` | The 4 §6.1 mutation helpers in `ui/gateway-uds` (`submit_action`/`preview_action`/`approve`/`deny`), reusing `call`/`demux_rpc_response` (verbatim §6.4 inherited); pure transport, `nexusops-shared`-only, exposed-ahead. | CLEAR |
| 056 / L2-B — mutation bridge + TS wire [disabled] (cat-1 Part B) | `dee5316` | 4 typed Tauri mutation commands (no `gateway_call`) + the TS `UdsGatewayPort` mutation methods wired to invoke + boundary-parse, GATED behind `mutationsEnabled` (default false). 3 defense-in-depth no-reach layers. | CLEAR |

### Files created
- `docs/sessions/ui-012-2026-06-14-pre-l2-gates-and-l2-transport-disabled.md` (this doc).

### Files modified
- **053b (`ca42732`):** `ui/src/shell/display-meta.ts` (`enrichHunkAction` → async real-row re-fetch/match/absent-pending; drops the `actionType` param) + `display-meta.test.ts` (6 pins); `ui/src/views/code/DiffReview.tsx` (call-site `await`; the honest `enrich-unavailable` degrade try/catch) + `DiffReview.test.tsx` (matched-row + degrade pins).
- **054 (`b3ffcb3`):** `ui/src/gateway-client/types.ts` (+`notifyConnectionState`) · `uds.ts` (the drive method + `streamDegraded` derived-from-committed-state + the `markConnected` suppression) · `mock.ts` (guarded `notifyConnectionState`; raw `setConnectionState` kept) · `ui/src/shell/Shell.tsx` (supervisor binds to `client.notifyConnectionState`; dropped the 2nd raw setter + the unused `canTransition` import) + `uds.test.ts` / `mock.test.ts` / `Shell.test.tsx` (8 pins).
- **L2-A (`49b61d5`):** `ui/gateway-uds/src/lib.rs` (the 4 mutation helpers + module-header update + 8 `mod tests` pins).
- **L2-B (`dee5316`):** `ui/src-tauri/src/commands.rs` (4 mutation commands + marshal fns + 3 tests) · `ui/src-tauri/src/lib.rs` (registration) · `ui/src/gateway-client/types.ts` (+`readonly mutationsEnabled`) · `uds.ts` (constructor + `assertMutationsEnabled` guard + rewired methods) · `boundary.ts` (`parseAck`/`parsePreview`) · `mock.ts` (`mutationsEnabled` option, default true) · `ui/src/overlays/GatewayModal.tsx` + `ui/src/views/code/DiffReview.tsx` (controls fold `&& gateway.mutationsEnabled`) + the 4 test files (uds/GatewayModal/DiffReview/Shell).

## Decisions made

- **053b absent-row `risk_level` = fail-safe `4` (non-displayed structural placeholder).** The GatewayModal shows risk from the live `preview_action` (`previewRisk`), NOT `approval.risk_level` — so the `4` over-warns and can never surface as a fabricated shown number. Orchestrator-ruled at Step 2.5.
- **053b honest-degrade on the re-fetch** (review-driven, in-slice): a `BoundaryValidationError` on the post-submit `get_projection` → an honest `enrich-unavailable` notice (mirrors the file's `get_diff` read-degrade, §11.7), never a silent stall.
- **054 authority model = port single `ConnectionState` + a `streamDegraded` axis** (not two-axis worst-wins). `streamDegraded` derived from the **committed** state (a rejected/no-op hop never sets it — review-driven fix). DEGRADE flows freely; only the read-path UPGRADE is suppressed while the stream is degraded.
- **L2-A: the bridge uses the generic `connect_and_call`; the typed helpers pin the contract.** The L2-A typed helpers are the tested param/return spec; L2-B's bridge marshals + uses the generic transport, the read precedent.
- **L2-B `mutationsEnabled` is a `GatewayPort` interface field** — UdsGatewayPort default **false** (production), MockGatewayPort default **true** (test/dev UI flows stay green). BOTH the port (throw-never-invoke) AND the controls (disabled) honor it → **L2-C is one flip** (`new UdsGatewayPort({mutationsEnabled:true})`). TWEAK-driven: the controls gate on it too (honest disabled, not enabled-buttons-that-throw).

## Decisions explicitly NOT made (deferred)

- **L2-C (the USER-gated enable)** — flip `mutationsEnabled` true + the controls light up. Authored-but-HELD; the orchestrator escalates the user sign-off (L2-O3) to the lead before dispatch.
- **053b `gatewayApprovalEnrichment` → test-fixture relocation** — still a test-only export in `display-meta.ts`; a future cleanup.
- **L2-A's typed helpers as the bridge transport** — left as the contract spec; the bridge uses `connect_and_call` (intentional, exposed-ahead). A typed mutation connect adapter could land later if wanted.
- **The DiffReview pre-existing onClick error-boundary** (054/053b note) — the `seam.submitAction` transport-Error re-throw at the click boundary stays unwrapped (pre-existing, intentional per LESSON §16); a separate hardening follow-on, not a regression.

## TDD compliance

**Clean — all 4 slices were test-first** (RED confirmed for the right reason on each: missing function / compile error / assertion mismatch, then GREEN). Review-driven additions were either test-strengthening (054 streamDegraded-from-committed pin; L2-A approve/deny ack-field assertions; L2-B approve-None invoke-arg + malformed-preview pins) or in-slice honest-degrade behavior with its own pin (053b `enrich-unavailable`). No TDD violations; no safety-critical skips.

## Reachability (Step 7.5, carried forward)

- **053b** — `enrichHunkAction` reachable from `Shell.tsx:467` (`<DiffReview gateway={client}>`) → `onAction` → `gateway.get_projection("ApprovalQueue")` (live L1 read). Per-hunk submit stays L2-HELD.
- **054** — `notifyConnectionState` reachable from `Shell.tsx:247` (the always-on subscribe-supervisor effect) → port `setConnection` → `onConnectionChange` (`Shell.tsx:152`, the ONE React writer) → `canSubmitIntent`. Suppression on `markConnected`, reached by every `invokeRead`.
- **L2-A** — the 4 typed crate helpers are reachable **only from the crate's own `mod tests`** — **by design** (exposed-ahead; they pin the contract, the L2-B bridge uses the generic `connect_and_call`). NOT a gap.
- **L2-B** — the 4 Tauri commands are registered (reachable from the TS invoke allowlist); the TS mutation methods contain the live invoke path but are **guarded off** (`mutationsEnabled` false; the Shell's `new UdsGatewayPort()` → false; verified repo-wide nothing passes true). The controls are disabled. **No production path reaches a live mutation** — the enable is L2-C.

## Open follow-ups (Step-9 categorized — already routed hot to the orchestrator)

- **Architecture doc notes (orchestrator writes in `/orchestrate-end`):** the 044 [med] FULLY RESOLVED (both approval-card paths); the 052 Finding RESOLVED (single connection authority); the `gateway-uds` crate now carries the mutation RPCs (transport only); the Tauri bridge carries the mutation allowlist (still no `gateway_call`); the TS port has the live mutation wire, guarded off until L2-C. → the `ui/CLAUDE.md` "Live UdsGatewayPort transport client" + "Tauri host" rows.
- **Convention candidates (LESSONS):** the post-submit per-resource real-risk re-fetch+match (053b); the connection single-authority + fail-safe-asymmetric-upgrade (054); the L2 transport reuses the read demux + verbatim-code inheritance (L2-A); the mutation bridge mirrors the read allowlist + the `mutationsEnabled` single-switch no-reach guard (L2-B).
- **Cross-doc invariant change:** NONE in `shared/` this round — every slice consumes frozen contracts (no CONTRACT bump, no schema-snapshot). The UI-local `GatewayPort` interface additions (`notifyConnectionState` @054, `mutationsEnabled` @056) were each flagged at Step 9 as UI-local (not a frozen shared-contract model). **Multi-track memory check: no frozen-model field changed; nothing un-flagged.**
- **Future TODOs (carry-forward, orchestrator triages):** L2-C dispatch (HELD for the user sign-off); the `gatewayApprovalEnrichment` test-fixture relocation; the L2-A connect-adapter (if a typed mutation connect is wanted at L2-C); the 2 deferred 053b review lows (the `no_mutation_reach` runtime-vs-compile-time test framing; the absent `required_approver` pin); the Shell single-writer test-strength limitation (054).

## Cross-doc invariant audit

Clean. No frozen `shared/` model field changed this round (all 4 slices consume frozen `ApprovalQueueRow`/`ActionRequest`/`ActionAck`/`ActionPreview`). The 2 UI-local `GatewayPort` interface additions were flagged at Step 9. No drift.
