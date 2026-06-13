# Intent Seam — Category-1 Safety Design Surface (for lead rule-vs-park)

> **Orchestrator-authored cat-1 safety checkpoint** for the UI's **FIRST mutation /
> intent-submission path** (the parked Decision-C work; the UI analog of the daemon's
> P4 live-drive-loop "deep-dive first" gate). **The lead rules each question IN-PLACE**
> (fill the `Lead ruling:` line, or reply per-question). I do **NOT** author the `/tdd`
> brief or dispatch until the lead steers. **The questions are laid out as OPTIONS — not
> decided.** The safety posture stays with the lead.
>
> Expectation (your stated posture): ratify the **6 DETERMINED** questions conservatively
> now (logged, user return-review); **rule-or-park the 1 OPEN**.

---

## What the seam is (plain language)

Today the UI can only **read** — every button that would *change* something is disabled
(`canSubmitIntent` is the fail-safe gate; the `GatewayModal` approve/deny buttons are
disabled). The **intent seam** is the wiring that lets the UI **ask the daemon to do
something** — submit an action, approve/deny a pending one. **The UI never performs the
change itself.** It sends a typed request to the daemon's Action Gateway (the single thing
allowed to mutate state) and renders what the daemon reports back. So every safety question
below is really "what does a *client of an already-frozen single-mutator contract* do?" —
and most answers follow from that contract.

## The frozen contract the seam is a CLIENT of

- **Anchors:** §4.2 (the 3 laws: single-mutation-chokepoint · events-are-facts-UI-reads-projections ·
  reason-vs-execute) [LOCKED] · §6.1 (GatewayPort method surface) · §6.2 (intent→policy→approval→execute→audit
  pipeline) · §6.4 (IPC framing + error codes) · §11.5 (Action Gateway & Brain UI card semantics) [LOCKED-O-3] ·
  §11.1 (read-only/degraded gate) · §11.7 (honest degradation) · §15 (INV-SEC-1) / §17 (the safety cards) ·
  forbidden #2 (never invent consequences).
- **Frozen IPC methods (§6.1, ARCHITECTURE ln226-234):** `submit_action(ActionRequest) → ActionAck{action_request_id, status}` ·
  `submit_action_plan(ActionPlan) → PlanAck` (O-3) · `preview_action → ActionPreview` · `approve` / `deny`.
  **Only `submit_*` are mutation entrypoints; the daemon mints `action_request_id`.** §6.1 ln234: "Agents/Brain/UI
  submit intents only … never the transport." §6.1 ln344: the UI "holds **NO authoritative state**; never writes the DB."
- **Frozen models (0.23.0, verified in the schema):**
  - `ActionRequest{action_request_id, action_type, requester_type, requester_id, resource_refs, inputs, risk_level, status, created_at, idempotency_key, fencing_token, preview, project_id}`
  - `Approval{approval_id, required_approver, status, scope, risk_level, action_request_id, decided_at, decided_by, expires_at, plan_id}`
  - `PolicyDecision{status, reasons, required_approvals, constraints, safer_alt}`
  - `ActionPreview{action_request_id, generated_at, risk_level, risk_reasons, summary, changed_resources, cannot_preview_reason}`
  - `ActionResult{action_request_id, status, created_resources, changed_resources, emitted_events, rollback_available, error}`
  - `ActionAck{action_request_id, status}` · `ServerFrame.rpc_response` (id-correlated) + `WireError(IpcErrorCode)`.
- **§6.4 codes → 3 DISTINCT §11.5 cards (frozen, 2.4 away-ruled, ARCHITECTURE ln243):** `fencing_conflict` =
  never-auto-resolved hard-conflict (rule #6) · `precondition_stale` = re-approvable · `internal_error` =
  fail-closed integrity alert (rule #5) · `policy_denied` = deny. **Collapsing them breaks #6 at the UI.**

---

## The 7 questions (OPTIONS — the lead rules)

### Q1 — Does the UI execute, or only submit? — *classification: DETERMINED*
- **Options:** **(A)** pure intent-submitter — the UI calls `submit_action`/`approve`/`deny` and never executes a mutation itself. **(B)** the UI executes some "low-risk" mutations directly to feel snappier.
- **Determining anchor:** §4.2 law 1 ("agents/Brain/UI submit *intents* only; only the Gateway executes and only the daemon writes the DB. Enforced + tested as INV-SEC-1") + §6.1 ln344 ("holds no authoritative state").
- **Answer it forces:** **(A).** (B) is forbidden by INV-SEC-1 — there is no UI execution path.
- **Orchestrator rec:** ratify **(A)**.
- **Lead ruling:** _________________________________________________

### Q2 — What gates the intent controls? — *classification: DETERMINED*
- **Options:** **(A)** every intent control is gated by `canSubmitIntent` (true ONLY when positively confirmed connected + version-compatible; FALSE on any unknown/degraded). **(B)** controls enabled by default, disabled only on a known-bad connection.
- **Determining anchor:** §11.1 read-only/degraded gate + Lesson §4 ("`canSubmitIntent` FALSE on unknown; true only confirmed connected + version-compatible … defense-in-depth — never the sole mutation guard; the daemon Gateway is").
- **Answer it forces:** **(A)** — fail-safe FALSE. (B) inverts the fail-safe default.
- **Orchestrator rec:** ratify **(A)**.
- **Lead ruling:** _________________________________________________

### Q3 — Optimistic-render-before-audit: may the UI show an un-audited mutation as DONE? — *classification: DETERMINED*
- **Options:** **(A)** NO optimistic "done" — an in-flight intent renders as "submitted/pending/executing" per the daemon's reported status (`ActionAck.status`), and flips to "done" ONLY when the daemon's confirmed state (the projection / `ActionResult`) reflects it. **(B)** optimistically render the mutation as done on submit, reconcile if the daemon disagrees.
- **Determining anchor:** §4.2 law 2 ("events are facts; the UI reads projections") + §6.1 ln344 ("the UI holds NO authoritative state; never writes the DB") + §11.7 (honest degradation). The UI cannot hold state that contradicts the audited projection.
- **Answer it forces:** **(A).** (B) would show un-audited state as real — exactly what law 2 + "no authoritative state" forbid. *(This is the one you flagged as a possible OPEN; the architecture actually settles the safety invariant. The in-flight "pending" PRESENTATION — spinner vs inline vs a tray — is a settle-able UX detail, NOT a safety posture; I'll handle it.)*
- **Orchestrator rec:** ratify **(A)**.
- **Lead ruling:** _________________________________________________

### Q4 — What does approve/deny submit, and what drives the card? — *classification: DETERMINED*
- **Options:** **(A)** the card renders the daemon's `PolicyDecision` (status/reasons/required_approvals/constraints/safer_alt) + the `ActionPreview`; Approve/Deny submit an `Approval` against the `action_request_id`/`approval_id` per the frozen contract. **(B)** the UI computes its own approval requirement / risk and submits a free-form decision.
- **Determining anchor:** §6.2 (the intent→**policy→approval**→execute pipeline) + the frozen `Approval` / `PolicyDecision` models + §11.5 card semantics.
- **Answer it forces:** **(A)** — the policy/approval requirement is the daemon's; the UI renders it and submits the human's Approve/Deny. (B) would duplicate/contradict the daemon's policy engine.
- **Orchestrator rec:** ratify **(A)**.
- **Lead ruling:** _________________________________________________

### Q5 — ActionPreview: daemon-provided or UI-invented? — *classification: DETERMINED*
- **Options:** **(A)** render the daemon's `ActionPreview` consequences only; show an honest "pending" / the `cannot_preview_reason` until the daemon previews; NEVER fabricate "what will happen." **(B)** the UI synthesizes a plausible consequence preview when the daemon's is slow/absent.
- **Determining anchor:** forbidden #2 ("never invent consequences") + the `ActionPreview` model (`summary`, `changed_resources`, `cannot_preview_reason`) + §11.5. *(The current `GatewayModal` stub already states this: "the consequence preview needs the daemon's ActionPlan preview contract — an honest pending note, never invented consequences.")*
- **Answer it forces:** **(A).**
- **Orchestrator rec:** ratify **(A)**.
- **Lead ruling:** _________________________________________________

### Q6 — How does the UI render a REJECTED mutation (the §6.4 codes)? — *classification: DETERMINED*
- **Options:** **(A)** each frozen `IpcErrorCode` → its mandated §11.5 card: `fencing_conflict` → the never-auto-resolved hard-conflict card (#6); `internal_error` → the fail-closed integrity alert (#5); `precondition_stale` → the re-approvable flow; `policy_denied` → the deny render. **(B)** a single generic "action failed" surface for all rejection codes.
- **Determining anchor:** §6.4 ln243 (the 3 DISTINCT cards, 2.4 away-ruled — "collapsing them would render fencing as re-approvable → break rule #6") + §17. The fencing/internal_error cards already exist (6.4d-2, security-reviewer PASS); `precondition_stale` is the parked re-approvable carry-forward.
- **Answer it forces:** **(A).** (B) collapses fencing into re-approvable → breaks #6.
- **Orchestrator rec:** ratify **(A)**.
- **Lead ruling:** _________________________________________________

### Q7 — Local intent caching / auto-retry across a disconnect? — *classification: **OPEN***
- **Options:**
  - **(A)** **NO caching / auto-retry** [conservative default]. On disconnect `canSubmitIntent`→false; an in-flight intent either already reached the daemon (the projection reflects its outcome on reconnect) or failed (the UI surfaces the failure; the user re-submits when reconnected).
  - **(B)** **cache + AUTO-retry** the in-flight intent on reconnect, deduped by the daemon's `idempotency_key`.
  - **(C)** **cache + a MANUAL "retry" affordance** — the UI holds the failed intent and offers the user a one-click resubmit (no automatic replay).
- **Why it's OPEN (no anchor forces it):** the daemon's `idempotency_key` makes a resubmit **dedup-safe** (so double-execution isn't the risk), but **whether the UI should hold and replay an in-flight intent across a reconnect is a trust/UX posture the daemon did not settle.** The tradeoff: convenience (B/C) vs the risk of **surfacing/replaying a now-stale intent the user may no longer want** (e.g. they'd have changed their mind during the outage) — and (B) auto-acting on the user's behalf after a gap is the sharper trust question.
- **Orchestrator rec:** rule **(A)** for the seam now (the fail-safe — no held mutation state, consistent with "UI holds no authoritative state"); **PARK (B)/(C)** as a user-facing enhancement decision. If you'd rather not rule even (A), parking the whole question is fine — the seam ships with no caching either way (A is also just "do nothing extra").
- **Lead ruling:** _________________________________________________

---

## Scope of the first slice (orchestration, NOT safety — your call or mine)
- **(A) Foundation-first [orchestrator rec].** Slice 1 = the seam only: the mutation methods on `GatewayPort` + `MockGatewayPort` + the `ServerFrame.rpc_response` id-correlation/demux + a `canSubmitIntent`-gated submit-intent seam (a hook + the request/ack/verdict types). **No UI consumer yet** (`GatewayModal` stays disabled). Slice 2 wires `GatewayModal`-real. *Pro:* the INV-SEC-1 seam is built + security-reviewed in isolation before any UI touches it; small, independently reviewable slices. *Con:* the seam is exposed-ahead-of-consumer for one slice.
- **(B) Seam + GatewayModal-real together.** One bigger slice = a working approval path. *Pro:* a usable mutation path in one go. *Con:* bundles the safety-critical card with the seam — larger security-review surface, harder to bisect a safety regression.

## After you rule
I fold the rulings into the `/tdd` brief's "Cat-1 safety design" section (each invariant → a pinned test), set the scope, and dispatch with the `security-reviewer` (invariant policy). The §17 `precondition_stale` re-approvable treatment stays a parked carry-forward for the permission-card/preview slice (not the seam).

---

## LEAD RULINGS — `ui-team-lead`, away-authority, 2026-06-13 (logged for USER return-review)

Verified each determining anchor against ARCHITECTURE.md independently. **Q1–Q6 RATIFIED (A)** — each is locked by the §4.2 laws / INV-SEC-1 / the frozen §6.4·§11.5 contract; ratifying enforces locked invariants (not a relaxation), so it is within away-authority. **Each invariant → a PINNED TEST in the brief** (these are the seam's safety contract, not prose). `security-reviewer` REQUIRED (invariant policy).

- **Q1 — RATIFY (A).** Pure intent-submitter; **no UI execution path** exists (INV-SEC-1 / §4.2 law 1 / §6.1 "no authoritative state"). (B) is an INV-SEC-1 bypass — forbidden. Pin: a test that the seam exposes only `submit_*`/`approve`/`deny`, never an executor.
- **Q2 — RATIFY (A).** `canSubmitIntent` fail-safe **FALSE on any unknown/degraded**; true only positively-confirmed connected + version-compatible (§11.1 / Lesson §4). Pin the fail-safe default. **Caveat (non-negotiable):** `canSubmitIntent` is **defense-in-depth, NEVER the sole guard — the daemon Gateway is the real chokepoint** (a UI-enable bug must still be rejected daemon-side). State that in the brief.
- **Q3 — RATIFY (A).** **NO optimistic "done."** In-flight renders as the daemon-reported status (`ActionAck.status`); flips to done ONLY when the projection / `ActionResult` confirms (§4.2 law 2 / "no authoritative state" / §11.7). (B) shows un-audited state as real — forbidden. The pending-PRESENTATION (spinner/inline/tray) is your non-safety UX detail. Pin: no "succeeded" render without a confirming projection/result.
- **Q4 — RATIFY (A).** The card renders the **daemon's** `PolicyDecision` + `ActionPreview`; Approve/Deny submit an `Approval` per the frozen contract. The UI **NEVER computes its own risk / approval-requirement** — the daemon's policy engine is authoritative (§6.2 / the 2.2 catalog-authoritative-risk ruling). Pin: the card's risk/approval-requirement is read from `PolicyDecision`, never UI-derived.
- **Q5 — RATIFY (A).** Render the daemon's `ActionPreview` consequences only; honest pending / `cannot_preview_reason`; **NEVER fabricate "what will happen"** (forbidden #2). Pin: no synthesized consequences.
- **Q6 — RATIFY (A).** Each `IpcErrorCode` → its mandated §11.5 card; **`fencing_conflict` stays the never-auto-resolved hard-conflict card (rule #6)** — no generic collapse (B breaks #6 at the UI). `internal_error` → fail-closed integrity alert (rule #5). `precondition_stale` re-approvable stays the **parked carry-forward** (permission-card/preview slice, NOT the seam). Pin: distinct rejection cards; fencing is never re-approvable.

- **Q7 (OPEN) — RULE (A) FOR THE SEAM NOW + PARK (B)/(C) FOR THE USER.**
  - **(A) ruled for the seam:** no caching / no auto-retry — the fail-safe do-nothing baseline; no held mutation state (consistent with "UI holds no authoritative state"); zero added risk; forward-compatible (adding (C) later is non-breaking). The seam ships with (A). Ruling the *absence of a speculative feature* is within away-authority.
  - **PARK (B)/(C) for the user** as a product/UX enhancement decision (cache + replay a disconnected intent). **My recorded lean to steer that decision:** if ANY retry is ever added it must be **(C) — a MANUAL, explicit user resubmit — NOT (B) auto-replay.** `idempotency_key` makes a resubmit *dedup-safe* but **not consent-fresh**: auto-acting on the user's behalf after a connectivity gap (replaying a possibly-stale mutation intent the user may no longer want) is against the human-gated-mutation spirit (INV-SEC-1's whole point is a human in the loop per mutation). (B) is the sharp trust concern; (C) preserves explicit consent. **Not decided — parked for the user.**

- **Scope — ENDORSE (A) Foundation-first** (your orchestration call; I endorse for the cleaner safety-review surface). Build + `security-reviewer` the INV-SEC-1 seam in **isolation** (GatewayModal stays disabled), wire `GatewayModal`-real in slice 2. Smaller, independently-reviewable, bisectable safety regressions — the right shape for a cat-1 surface; the one-slice exposed-ahead-of-consumer is the benign 040/041 pattern.

**Proceed:** fold these into the brief's "Cat-1 safety design" section (each invariant → a pinned test), Scope (A), dispatch WITH `security-reviewer`. Note in the brief that Q7-(B)/(C) is PARKED-for-user (not dropped) + my manual-not-auto lean. All rulings → user return-review + next `/arch-finalize`.
