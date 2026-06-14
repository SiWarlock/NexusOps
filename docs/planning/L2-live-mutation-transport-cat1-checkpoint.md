# L2 — Live Mutation Transport: Category-1 Safety Checkpoint (for lead rule-vs-park)

> **Orchestrator-authored cat-1 checkpoint** for **L2 — the live mutation transport** (the UI's
> go-live moment: the existing `canSubmitIntent`-gated intent seam stops throwing `not-wired` and
> actually reaches the daemon's Action Gateway over the socket). Per the team protocol I do **NOT**
> author the L2 `/tdd` brief(s) or dispatch until the lead steers this surface. **The questions are
> laid out as OPTIONS — not decided.** The safety posture stays with the lead (and, for the go-live
> gate, likely the user).
>
> **Relationship to the existing Q1–Q7 rulings** (`docs/planning/intent-seam-cat1-safety-design.md`,
> away-ruled 2026-06-13): those settled the **seam's submit/approve/deny LOGIC + the approval-card
> rendering** — built + security-reviewed at **043** (the seam in isolation) and **044** (`GatewayModal`-real).
> Q1–Q7 are **durable — I consume, never re-open.** L2 is a **distinct surface**: it makes that
> already-gated, already-reviewed seam **LIVE over the wire.** So the L2 invariants are mostly **Q1–Q7
> RE-PINNED on the live transport path**, plus the transport-specific surface that L1's read transport
> (049–052) didn't cover.

---

## What L2 is (plain language)

Today the cockpit can **read** the daemon live (L1 complete: real projections on load + a subscribe
stream keeping it live) and it can **render** an approval card with the daemon's real risk/policy
(044/053/053b — the 044 [med] resolved both paths). But every **mutation** control is still inert:
`UdsGatewayPort.submit_action / approve / deny / preview_action` **throw `not-wired`** (pinned
`subscribe_mutation_methods_still_throw_not_wired`), and `canSubmitIntent` gates controls that can't
reach a mutation regardless.

**L2 wires the live mutation path:** the Rust transport crate gains the mutation RPCs, the Tauri host
gains typed mutation commands, the TS `UdsGatewayPort` replaces the `not-wired` throws with real
`invoke`s, and the `GatewayModal`/`DiffReview` submit/approve/deny **actually reach the daemon.** A
real human can then approve a real, daemon-risk-classified action and the daemon executes it. **The UI
still never mutates** — it submits a typed intent to the daemon's Action Gateway (the single mutator)
and renders what the daemon reports. So, as with the seam, every safety answer is "what does a *client
of an already-frozen single-mutator contract* do over a live socket?"

## Prerequisites (both land BEFORE this checkpoint is actioned)

- **053b ✅ LANDED (`ca42732`)** — the per-hunk `enrichHunkAction` real-risk swap; the **044 [med] is
  resolved on BOTH approval-card paths** (no fixture risk anywhere a human approves).
- **054 — the connection-state single-authority reconcile** (the 052 Finding; **in flight / landing**).
  **This is the hard pre-L2 gate:** L2 makes `canSubmitIntent` **load-bearing** (a real human approving
  a real mutation), so the gate must be single-authority + fail-safe-correct first. 054 collapses the
  two-writer race to one authority (the port) and suppresses a read-upgrade while the stream is
  degraded. **L2 must not be authored until 054 is sealed.**

## The frozen contract L2 is a (live) CLIENT of

- **Anchors:** §4.2 (3 laws — single-mutation-chokepoint · events-are-facts/UI-reads-projections ·
  reason-vs-execute) · §6.1 (GatewayPort method surface; daemon `daemon/src/ipc/methods.rs`) · §6.2
  (intent→policy→approval→execute→audit) · §6.4 (IPC framing + `IpcErrorCode`) · §11.1/§11.4 (the
  read-only/degraded gate) · §11.5 (Gateway card semantics) · §11.7 (honest degradation) · §15
  (INV-SEC-1) · forbidden #2 (never invent consequences).
- **Frozen mutation entrypoints (§6.1):** `submit_action(ActionRequest) → ActionAck{action_request_id,
  status}` · `preview_action(action_request_id) → ActionPreview` · `approve(approval_id, step_id?) →
  ActionAck` · `deny(approval_id, reason) → ActionAck`. The daemon mints `action_request_id`; the UI
  submits intents only, holds **no authoritative state**, never writes the DB.
- **The existing transport (L1, LESSON 20/21/22/23):** the `nexusops-gateway-uds` pure-Rust crate
  (codec + handshake + `ServerFrame::RpcResponse` demux) → the Tauri host's **NARROW typed read-only
  allowlist** (`gateway_get_projection`/`get_diff`/`get_capabilities`; **NO generic `gateway_call`**) →
  the TS `UdsGatewayPort` (parse-don't-trust; wire-rejection→plain data vs transport-fault→Error). L2
  extends each layer with the mutation surface.

---

## The questions (OPTIONS — the lead rules; the go-live gate likely → the user)

### Part A — invariants that RE-PIN Q1–Q7 on the live path (*classification: DETERMINED — ratify*)

Each follows from the already-ruled Q1–Q7 + the frozen contract; L2's job is to keep them true once the
path is **live**. Each → a pinned test in the L2 brief(s); `security-reviewer` REQUIRED.

- **L2-D1 (Q1 live).** The live transport is a **pure pass-through** — `submit_action`/`approve`/`deny`
  send a typed RPC to the daemon and render the ack; **no UI execution path** exists (INV-SEC-1). Pin:
  the TS port's mutation methods only `invoke` the typed mutation command + boundary-parse the ack —
  never branch into a local mutation.
- **L2-D2 (NEW — LESSON 21 extended to mutations).** The Tauri host gains **one typed command per
  mutation method** (`gateway_submit_action`/`approve`/`deny`/`preview_action`), each forwarding a typed
  payload to the daemon. **The registered set IS the allowlist — still NO generic `gateway_call`** (which
  would expose arbitrary daemon methods). The bridge **leaks nothing** (daemon `Value`/ack on success;
  `Wire{code}` verbatim §6.4; an `Internal` host-fault variant — no path/secret). Pin: the command set is
  the typed mutation allowlist; no arbitrary-method reach.
- **L2-D3 (Q2 live — now load-bearing).** Every live mutation control is gated by `canSubmitIntent`
  (fail-safe FALSE on unknown/degraded). **054's single-authority gate is the prerequisite.** Caveat
  (non-negotiable, state in the brief): the gate is **defense-in-depth, NEVER the sole guard — the
  daemon Gateway is the real chokepoint** (a UI-enable bug must still be rejected daemon-side).
- **L2-D4 (Q3 live).** **No optimistic "done."** A live in-flight intent renders the daemon-reported
  status (`ActionAck.status`); flips to done ONLY when the projection / `ActionResult` confirms. Pin: no
  "succeeded" render without a confirming projection/result.
- **L2-D5 (Q4/Q5 live).** The card renders the **daemon's** `PolicyDecision` + `ActionPreview`;
  Approve/Deny submit per the frozen contract; **never UI-derived risk / fabricated preview** (forbidden
  #2). 044/053/053b already pin the render-real-risk half; L2 makes the **submit** live.
- **L2-D6 (Q6 live).** Each §6.4 `IpcErrorCode` on the live path → its distinct §11.5 card
  (`fencing_conflict` never re-approvable #6; `internal_error` fail-closed #5; `precondition_stale`
  re-approvable; `policy_denied` deny). The **wire-rejection (plain data) vs transport-fault (Error)**
  classification (LESSON 16/22) must hold for **mutation** responses too. Pin: distinct rejection cards
  on the live path; fencing never collapses to re-approvable.
- **L2-D7 (Q7-A live + what stays OUT).** **No caching / no auto-retry** (Q7-A) — on disconnect
  `canSubmitIntent`→false; an in-flight intent either reached the daemon (the projection reflects it on
  reconnect) or failed (the user re-submits). The **`policy_grant` "always allow" standing-grant stays
  DISABLED** (its own cat-1 checkpoint — NOT L2). `submit_action_plan`/per-step `require_step_approval`
  status as of L2 = confirm scope (single-action submit is the L2 core; plan-level may be a follow-on).

### Part B — genuinely-OPEN L2 questions (the lead rules / parks; some → the user)

#### L2-O1 — Scope / sub-slicing of the live transport — *OPEN (orchestration + safety surface)*
- **Options:** **(A)** ONE cat-1 slice wiring the full mutation transport (crate RPCs + Tauri commands +
  TS port + enable the live submit) — a usable mutation path in one go, but the **largest** security-
  review surface, harder to bisect a safety regression. **(B)** layer it like L1 did (049 crate → 050
  bridge → 051 TS → 052): **slice A** = the crate mutation RPCs (pure Rust, security-reviewed in
  isolation) → **slice B** = the Tauri mutation commands + the TS `UdsGatewayPort` live wire (still
  consumer-disabled) → **slice C** = enable the live `GatewayModal`/`DiffReview` submit. Smaller,
  independently-reviewable, bisectable.
- **Orchestrator rec:** **(B) foundation-first**, mirroring the L1 precedent + the lead's prior seam
  Scope ruling ("(A) Foundation-first"). Each cat-1 sub-slice gets `security-reviewer`; the enable-live
  step (slice C) is the smallest, most-scrutinized surface.
- **Lead ruling:** **(B) foundation-first** — see LEAD RULINGS below.

#### L2-O2 — `preview_action` live with the submit transport, or a follow-on? — *OPEN*
- **Context:** `preview_action(action_request_id) → ActionPreview` is in the §6.1 mutation-intent surface
  but is **read-like** (previews consequences; does NOT execute). The `GatewayModal` renders the preview;
  today `preview_action` throws `not-wired`, so the modal shows the enrichment/passed-in preview.
- **Options:** **(A)** land live `preview_action` WITH the L2 transport (the modal fetches a real daemon
  preview when the live submit is enabled — consistent, no fabricated preview). **(B)** land
  `preview_action` separately (it's read-like — could even ride an L1-style read slice) and keep the
  modal's preview as-is until then.
- **Orchestrator rec:** **(A)** — couple the live preview to the live submit so the human approves
  against a real daemon preview at the moment the submit goes live (forbidden #2 / Q5 spirit).
- **Lead ruling:** **(A) live `preview_action` with the submit** — see LEAD RULINGS below.

#### L2-O3 — The go-live USER sign-off gate — *OPEN (likely → the user)*
- **Context:** L2 is the moment a real human can drive a **real, executed mutation** from the cockpit.
  Q1–Q7 + Part A are all locked/ratifiable invariants (away-authority OK). But **"do we enable the live
  mutation submit now"** is the load-bearing go-live posture — analogous to the daemon's P4 live-drive-
  loop "deep-dive first" gate.
- **Options:** **(A)** the lead ratifies Part A + rules Part B, and the **live-enable step (slice C)
  requires explicit USER sign-off** before it merges/goes live (the cat-1 rulings + this checkpoint →
  user return-review first). **(B)** the lead's away-authority covers the full L2 (Part A is all locked
  invariants; the user reviews post-hoc like the daemon cat-1 work). **(C)** stage live-enable behind a
  dev-only flag first (the live transport exists + is reviewed; the actual go-live is a separate
  user-gated flip).
- **Orchestrator rec:** **(A) or (C)** — the live-enable is the one step where a UI bug becomes a real
  mutation attempt (still daemon-gated, but it's the trust boundary going live). I lean: build +
  security-review the transport (slices A/B) under the lead's authority; **gate the live-enable (slice
  C) on explicit user sign-off.** The lead maps this to the user via `AskUserQuestion` if it agrees.
- **Lead ruling:** **(A) — slice C gated on EXPLICIT USER SIGN-OFF. 🔒 USER-RULED "Sign off before go-live" 2026-06-14.** See LEAD RULINGS below.

#### L2-O4 — `idempotency_key` / `fencing_token` formation on the live path — *DETERMINED-leaning, confirm*
- **Context:** `ActionRequest` carries `idempotency_key` + `fencing_token`. On the live path these become
  load-bearing (the daemon dedups + rejects stale-token mutations → a hard-conflict card, never
  auto-resolved — safety rule #6).
- **Options:** **(A)** the UI forms these per the frozen contract and passes them through opaquely — the
  daemon owns dedup + fencing; a stale-token rejection renders the `fencing_conflict` hard-conflict card
  (L2-D6). The UI never reasons about fencing locally. **(B)** the UI tracks/optimizes tokens locally.
- **Orchestrator rec:** **(A)** — pass-through per the frozen contract; the daemon is the fencing
  authority (Q1/§5.1). Pin: a stale-token live rejection → the never-auto-resolved hard-conflict card.
- **Lead ruling:** **(A) pass-through** — see LEAD RULINGS below.

---

## What stays OUT of L2 (parked, not dropped)

- **The `policy_grant` "always allow" standing-grant** — its OWN cat-1 checkpoint (same file-based rule).
  Stays disabled-pinned through L2.
- **Q7-(B)/(C) intent caching / retry** — PARKED-for-user (the lead's recorded lean: if ever added, **(C)
  manual resubmit, NEVER (B) auto-replay** — `idempotency_key` is dedup-safe but not consent-fresh).
- **`submit_action_plan` / per-step `require_step_approval`** — confirm at L2-D7; plan-level approval may
  be a Phase-8/HIQ follow-on (single-action submit is the L2 core).
- **The other-5-projection live deltas** (052 Q3 spread) — a projection-coverage slice, not L2.

## After the lead rules

I fold the rulings into the L2 `/tdd` brief(s)' "Cat-1 safety design" section (each invariant → a pinned
test), set the scope (L2-O1), the `preview_action` timing (L2-O2), and the go-live gate (L2-O3), and
dispatch WITH `security-reviewer` (invariant policy) on every L2 sub-slice. The rulings → user
return-review + next `/arch-finalize`.

---

## LEAD RULINGS — `ui-team-lead`

> Filled in-place by `ui-team-lead` 2026-06-14. Part A + the technical Part B (O1/O2/O4) = away-authority
> (each enforces a locked invariant or is precedent-determined); **L2-O3 = USER-RULED**.

Read the full checkpoint + **cross-verified Part A against the durable Q1–Q7 rulings**
(`intent-seam-cat1-safety-design.md`) independently — each Part-A invariant maps faithfully to an
already-ratified-and-logged Q-ruling, re-pinned on the **LIVE** transport path. Ratifying enforces locked
invariants (not a relaxation) → within away-authority. **Each invariant → a PINNED TEST; `security-reviewer`
REQUIRED on every L2 sub-slice** (invariant policy).

- **Part A (L2-D1…D7) — RATIFIED on the live path.** D1=Q1 (pure pass-through, no UI executor / INV-SEC-1) ·
  D2 (NEW: one typed Tauri command per mutation method = the allowlist, **NO generic `gateway_call`**; LESSON 21
  extended to mutations; the bridge leaks nothing — daemon `Value`/ack or verbatim `Wire{code}`, no path/secret) ·
  D3=Q2 (`canSubmitIntent` fail-safe FALSE; **054 single-authority is the prerequisite**; defense-in-depth,
  NEVER the sole guard — the daemon Gateway is the real chokepoint) · D4=Q3 (no optimistic "done") ·
  D5=Q4/Q5 (daemon's `PolicyDecision` + `ActionPreview`; never UI-derived/fabricated) · D6=Q6 (each
  `IpcErrorCode` → its distinct §11.5 card; `fencing_conflict` NEVER re-approvable [#6]; wire-rejection-vs-
  transport-fault holds for mutation responses too) · D7=Q7-A (no caching/auto-retry; `policy_grant` stays
  DISABLED; single-action scope — see below).
- **L2-O1 — RULE (B) foundation-first.** Slice A = crate mutation RPCs (pure Rust, reviewed in isolation) →
  slice B = Tauri mutation commands + the TS `UdsGatewayPort` live wire (consumer-DISABLED) → slice C = enable
  the live `GatewayModal`/`DiffReview` submit. The bisectable, independently-`security-reviewer`-able cat-1
  shape; mirrors L1 (049→052) + the prior seam Scope endorsement. Slice C is the smallest, most-scrutinized surface.
- **L2-O2 — RULE (A) live `preview_action` with the submit.** Couple the live preview to the live submit so the
  human approves against a REAL daemon preview the moment submit goes live (forbidden #2 / Q5 spirit). No window
  where a live submit is approved against a passed-in/stale preview.
- **L2-O3 — (A): slice C (the enable-live flip) is GATED ON EXPLICIT USER SIGN-OFF. 🔒 USER-RULED "Sign off
  before go-live" 2026-06-14.** Build + `security-reviewer` slices A/B under lead authority; **slice C does NOT
  merge / go live until the user explicitly signs off** — the lead brings it back via `AskUserQuestion` with the
  real-daemon verification surface (the [[visual-verification-gate]]: green tests ≠ looks right). This is the
  cockpit's trust-boundary-going-live moment.
- **L2-O4 — RULE (A) pass-through.** The UI forms `idempotency_key` + `fencing_token` per the frozen contract +
  passes them opaquely; the daemon owns dedup + fencing; a stale-token live rejection → the `fencing_conflict`
  never-auto-resolved hard-conflict card (rule #6 / L2-D6). The UI never reasons about fencing locally (Q1 /
  "holds no authoritative state").
- **Scope confirm (L2-D7):** single-action `submit_action` is the L2 core; `submit_action_plan` / per-step
  `require_step_approval` = a Phase-8/HIQ follow-on (NOT L2). `policy_grant` "always allow" stays disabled-pinned
  through L2 (its OWN cat-1 checkpoint — same file-based rule). Q7-(B)/(C) intent caching/retry = PARKED-for-user
  (lead lean: manual resubmit (C), NEVER auto-replay (B)).

**GATE:** L2 is **NOT authored until 054 seals** (the connection-state single-authority reconcile — the hard
pre-L2 gate that makes `canSubmitIntent` load-bearing-correct). After 054 seals: fold these into the L2 brief(s)'
"Cat-1 safety design" section (each invariant → a pinned test), Scope (B), `security-reviewer` every sub-slice;
**HOLD slice C for user sign-off.** All rulings → user return-review + next `/arch-finalize`.
