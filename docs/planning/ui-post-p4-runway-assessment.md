# ui track — post-P4/0.28.0 buildable-vs-gated runway assessment

> **Orchestrator-authored** for `ui-team-lead` (requested at the 048 round seal). Verifies
> what the daemon's full Phase-4 merge (CONTRACT 0.23.0→0.28.0, on `track/ui` via `a154733`)
> NEWLY unblocked for the ui track to build IN-LANE — so the next-direction call is precise,
> not a blind repeat of the prior pause. Sources cited (catalog/schema/IPC methods).
>
> **Headline correction to the earlier `ui-post-6.3e-direction.md`:** the prior "in-lane =
> low-value polish" framing was WRONG. The P4 merge unblocked a **HIGH-VALUE in-lane item I
> underweighted — the live `UdsGatewayPort` transport (the "go-live" integration)**. The
> daemon serves everything; the ui transport client is just unbuilt. It is **ui-buildable-now
> but CAT-1** (it takes the real mutation path live).

## 1. Is 6.3e truly complete? — **YES (in-lane); residuals are P4/Phase-5**

6.3e (047 + 048 + fixup) is functionally complete: the diff is sourced from `get_diff`, the
per-hunk stage/unstage/discard bar submits typed `git.*` intents over the seam, the daemon's
approval card renders, the resource_ref is verbatim-from-displayed (submitted==displayed,
adversarially verified). `security-reviewer` PASS, visual gate PASS. **No in-lane diff-review
residual** — the 3 frozen `git.*` types are all wired (`shared/src/catalog.rs:105-107,277-289`);
the git EXECUTOR bodies are Phase-5 stubs (edges) — but that's the daemon's executor, not a ui
gap. Deferred (non-blocking, additive-later): the daemon's `DiffResult.is_binary` / `\ No newline`
enrichers. **6.3 is now LOGIC COMPLETE** (all 5 core screens built).

## 2. What the P4/0.28.0 merge NEWLY unblocked in-lane

### 2a. The live `UdsGatewayPort` transport — **VERDICT: ui-buildable-now · HIGH-VALUE · CAT-1**

**The daemon side is feature-complete + frozen at 0.28.0.** Verified `daemon/src/ipc/methods.rs:69-88`
dispatches **every** method the ui needs: `get_projection`, `get_diff`, `submit_action`,
`submit_action_plan`, `approve`, `deny`, `preview_action`, `session.create`, plus `subscribe`/
`get_capabilities`/the live `intercept`. The wire contract is frozen (`shared/src/ipc.rs`):
the 4-byte-len+JSON codec, `HelloFrame`/`HelloAck` + `protocol_version {1,1}`, the `ServerFrame`
demux (`rpc_response` id-correlation / `subscription_push` / `terminal_output`), `getpeereid`
peer-auth, the 10-code `IpcErrorCode`. The daemon SERVES this over the UDS socket today (P1.5/1.6
read+subscribe serving; P4 the live drive loop).

**The ui side is greenfield.** There is NO `UdsGatewayPort` (only `boundary.ts`/`mock.ts`/`types.ts`
in `ui/src/gateway-client/`) and NO UDS bridge in `ui/src-tauri/` (verified empty of socket code).
The ui sits on `MockGatewayPort`. The seam/modal/diff/terminal surfaces all sit ABOVE the
transport-agnostic `GatewayPort` — they swap the Mock for the real client here.

**So building the transport client is a ui-track task needing NO daemon change.** It spans
`ui/src-tauri/` (a Rust UDS client + a Tauri-command bridge — the frontend can't open a Unix
socket directly) + `ui/src/gateway-client/uds.ts` (the TS `GatewayPort` impl). The deterministic
parts (frame codec, `ServerFrame` demux, handshake, rpc-correlation) are TDD-able against a
fake/recorded socket; the socket I/O is a thin integration-tested adapter.

**Why CAT-1:** going live takes the **real mutation path** live — the moment a real human
`approve`s a real `submit_action` against the real daemon. This is the §4.2/§15 INV-SEC-1
boundary the intent seam was built for. **It needs its own cat-1 checkpoint before authoring**
(handoff §4; the lead flagged it).

**Recommended SHAPE — phase it (the 043/044 seam-then-consumer pattern):**
- **L1 — the READ transport (NON-safety, high-value):** the real UDS client carrying
  `get_projection`/`subscribe`/`get_diff`/`get_capabilities`. The UI shows REAL daemon data
  (projections, diffs, the live terminal once 2b lands). No mutation → not cat-1. This is the
  big "the app is alive against the real daemon" win, buildable without the cat-1 gate.
- **L2 — the MUTATION transport (CAT-1):** carrying `submit_action`/`approve`/`deny`. Takes the
  seam + the approval card live. Needs the cat-1 checkpoint AND the approval-enrichment (2c)
  resolved first (no real human approves against fixture risk).

### 2b. §8 survival/recovery surface — **VERDICT: cross-track-gated (P4 not done)**

Phase-4 did NOT freeze the survival/recovery schema. Verified: `shared/` + the 0.28.0 schema
carry **no** `ResumeResult`/`ResumeMode`/`RecoveryStatus`/`RecoveryState` `$defs`; the daemon
emits no recovery events. The ui's `RecoveryState`/`ResumeMode`/`RecoveryStatus` are PROVISIONAL
inventions (`ui/src/contracts/provisional.ts:25-39`), the recovery banner is fixture-driven. A
survival-UI slice is a no-op fixture→projection swap, blocked on the daemon's survival logic +
event emission (P4.1/4.2, not yet landed). **Distinct from 8.2 Brain.** Still gated.

### 2c. Carried-forward security items — **VERDICT: needs a SMALL daemon follow-up**

The 044 [med] (extended by 048's `enrichHunkAction`): the `gatewayApprovalEnrichment` /
`enrichHunkAction` side-maps in `ui/src/shell/display-meta.ts` are daemon-SHAPED **fixtures**
(risk + approval_id). The real `proj_approval_queue` projection is thin (no `risk_level`, no
`policy_decision`). `preview_action` IS served (so the PREVIEW is real) but keys off
`action_request_id`, not `approval_id`. **Before any real human approves (i.e. before L2 above),
the daemon needs a small follow-up:** either enrich `proj_approval_queue` with `risk_level` +
`policy_decision_json`, OR add a `get_approval_policy(approval_id) → {approval, PolicyDecision}`
RPC. This is a **②-mini daemon packet** (small, well-scoped) — the gating dependency for L2
go-live. The other 048 items (the concurrent-submit guard, the worktree_id source) are benign
ui polish.

## 3. Still cross-track-gated (confirmed, not assumed)

- **7.2 PR Review Workspace — cross-track-gated (edges Phase-7).** `PullRequest` IS in the frozen
  `ProjectionName`, but the row is MINIMAL (`pr_number`+`status`+`title?`, `provisional.ts:157-164`)
  — no branch/checks/reviews/mergeability. The rich PR data + the PR-review actions are edges P7.1/
  P7.2 (the `PullRequestSynced` event exists @ 0.26.0 but edges emit it; the rich projection isn't
  on `track/ui`). Confirmed gated.
- **8.2 Brain drawer — cross-track-gated (Phase-8).** The catalog has `brain.*` SUBMISSION intents
  (`brain.ask`/`brain.sync`/`brain.summarize_session`) but the executor is a parked stub; there is
  NO Brain projection, NO plan/evidence/Brain-card contract in the 0.28.0 schema. Confirmed gated.

## 4. Recommendation (with the 6-tail Finding's rigor)

**The situation is materially better than the prior pause — there IS high-value in-lane work: the
live transport (go-live).** My ranked recommendation:

1. **BEST (if the user wants to keep the ui track productive): build the live transport, phased —
   L1 the READ transport first (non-safety, high-value, the app goes live against the real daemon),
   then L2 the cat-1 mutation transport** (its own checkpoint) **once the ②-mini approval-enrichment
   daemon packet lands** (2c). L1 is a clean, high-value, non-cat-1 slice buildable NOW; it de-risks
   and front-loads the value while L2's daemon dependency is arranged.
2. **If the user prefers to keep the daemon/edges on their critical paths: clean-pause** (the prior
   pattern) — but note this now leaves a HIGH-VALUE in-lane item (the transport) on the table, which
   it didn't last time. Less compelling than before.
3. **Lower priority: 6.6 graph bench / polish** — fine "keep-warm" filler, but the transport L1
   dominates it on value.
4. **7.2 / 8.2** stay the user's cross-track packet-routing call (genuinely gated).

**The one cross-track ask either way for FULL go-live: the ②-mini approval-enrichment daemon packet
(2c)** — small, well-scoped, gates L2 (real human approval). Worth routing alongside an L1 decision.

**Cat-1 discipline reminder (handoff §4):** the L2 mutation transport + the "Always allow"
standing-grant are cat-1 — I escalate a checkpoint before authoring either; I will NOT author them
on my own initiative.
