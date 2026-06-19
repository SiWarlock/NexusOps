# L2-C — Go-Live Verification Surface (for the user's sign-off, via the lead)

> **🔒 L2-O3 USER-RULED.** The lead presents this to the user (via `AskUserQuestion`) to obtain the
> explicit go-live sign-off. **Slice C is NOT authored/built until the user signs off.** On the user's
> GO, the orchestrator dispatches slice C (the build spec is the HELD brief `docs/briefs/057-P6-L2C-enable-live-mutation-USER-GATED.md`).
>
> **State now:** the full L2 mutation transport is BUILT + security-reviewed + pushed (`track/ui`
> @ `c70a415`), but **DORMANT** — `mutationsEnabled=false`, so no production path reaches a live
> mutation (verified repo-wide). L2-C is the single switch that turns it on.

## What L2-C does (plain language)

Exactly one change: the production Shell constructs the gateway port `mutationsEnabled: true`
(`new UdsGatewayPort({ mutationsEnabled: true })`). That single flag lights up, together:
- **the transport** — the port's `submit_action`/`approve`/`deny`/`preview_action` stop throwing
  "not enabled" and `invoke` the live Tauri commands → the daemon's Action Gateway;
- **the controls** — the `GatewayModal` approve/deny + the `DiffReview` per-hunk submit enable (when
  connected), via `canSubmitIntent && mutationsEnabled`.

After it, **a real human can approve/deny/submit a real, daemon-risk-classified mutation from the
cockpit.** That is the trust boundary going live — which is why it is the user's call.

## What does NOT change (the safety frame the user is signing off against)

- **The daemon Action Gateway stays the single mutator + the INV-SEC-1 chokepoint.** The UI submits a
  typed intent; the daemon classifies risk, applies policy, gates approval, executes, and audits. The
  UI never mutates. A UI bug cannot bypass the daemon (the UI `canSubmitIntent` gate is
  defense-in-depth, never the sole guard).
- **`canSubmitIntent` stays fail-safe** (FALSE on any unknown/degraded; true only when positively
  confirmed connected + version-compatible — and now single-authority after 054, so an ad-hoc read can
  never mask a degraded stream).
- **The "Always allow" / `policy_grant` standing-grant stays DISABLED** — it is its OWN cat-1
  checkpoint, NOT enabled by L2-C. Every mutation remains a per-action human approval.

## The real-daemon verification surface (what to check before signing off)

A manual operator gate against a REAL daemon (no daemon runs in the ui worktree — the live render is a
cross-track operator step). Launch the cockpit against a running daemon and confirm:

1. **Real risk, not fixture, at approval.** The approval card (HIQ + per-hunk) shows the daemon's
   REAL `risk_level`/`policy_decision` from the served `ApprovalQueueRow` — not a UI-invented value
   (the 044 [med] is resolved on both paths @053/053b). The modal's risk number comes from the live
   `preview_action`.
2. **The live submit/approve/deny boundary.** A submit/approve/deny reaches the daemon Gateway; the
   daemon executes + records the audit event; the cockpit reflects the **daemon-confirmed** status —
   **no optimistic "done"** (status from the daemon ack/projection only, Q3).
3. **Rejections render the right card.** A daemon `WireError` shows its distinct §11.5 card — a
   `fencing_conflict` is the never-auto-resolved hard-conflict card (#6), NOT collapsed to re-approvable
   (the verbatim §6.4 code path, Q6).
4. **The standing-grant stays disabled.** The "Always allow" control remains disabled even live.
5. **Q1–Q7 honored live** (the durable cat-1 rulings, re-pinned on the live path at L2-A/B):
   pure-submitter (no UI execution), `canSubmitIntent` fail-safe gate, no optimistic-done, daemon-driven
   card (real risk/preview), verbatim §6.4 rejection cards, no intent caching/auto-retry.

## The enable-live plan (the build, on the user's GO)

The HELD `/tdd` brief `docs/briefs/057-P6-L2C-enable-live-mutation-USER-GATED.md` is the build spec:
the one-line flip + a Shell-integration pin (the production port is mutations-enabled; the controls
enable when connected; the standing-grant stays disabled; no optimistic-done re-asserted) +
`security-reviewer` REQUIRED (the full live path) + the real-daemon live verification above. **1 commit.**

## The decision (lead → user)

- **GO** → the orchestrator dispatches L2-C (the flip + the live verification); L2 goes COMPLETE; the
  cockpit can drive real mutations.
- **HOLD** → the transport stays dormant (`mutationsEnabled=false`); nothing live; revisit when ready.

Parked regardless of the decision: the `policy_grant` standing-grant (own cat-1), Q7-B/C intent
caching (parked-for-user), `submit_action_plan`/per-step, the other-5-projection live deltas.
