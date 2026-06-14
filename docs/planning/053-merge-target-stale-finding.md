# Finding — the verified merge target `ea0e93e` (0.30.0) is STALE; main moved to `abc4c57` (0.31.0)

> **Orchestrator-routed Finding** (escalation category #2 — broken premise on a load-bearing
> cross-track decision) for `ui-team-lead` → user. Surfaced at the 0.30.0 boundary-merge step.
> The merge was **aborted** (not committed); `track/ui` is clean at `0de9c8a`. Awaiting the lead's
> merge-target confirmation before re-merging.

## What happened

The lead's GO message verified the main idle tip as **`ea0e93e` (CONTRACT 0.30.0)** and gave it as
the merge target. When I executed the merge, the conflict resolved as predicted (only
`IMPLEMENTATION_PLAN.md`) — but a recalled daemon-orchestrator memory note flagged a newer state, so
I verified against git before committing.

## The evidence (git, this repo's object store)

- `git log -1 main` → **`abc4c57`** ("seal 4.1b-1 — restart session-recovery orchestration; …").
- `git merge-base --is-ancestor ea0e93e main` → **true** — `ea0e93e` is a STRICT ANCESTOR of main.
- `git log ea0e93e..main` → **3 commits ahead**: `475068b` (4.1b-1 C1 — restart session-recovery
  orchestration) · `1e68f20` (4.1b-1 C2 — the `SessionRecovered` observation event) · `abc4c57`
  (4.1b-1 seal).
- `git show abc4c57:…schema.json` → **`"x-contract-version": "0.31.0"`** (vs `ea0e93e` = 0.30.0).

So **the daemon landed 4.1b-1 (→ CONTRACT 0.31.0) in the ~minutes between the lead's verification and
my execution.** `ea0e93e`/0.30.0 was correct-when-verified but is now stale-by-3-commits. The
daemon-orch's current memory confirms 0.31.0 is the intended ui merge target ("ui = the active
main-merge target @0.31.0; team HELD idle for the ui's merge … main must stay stable").

## Recommendation — re-target the merge to `abc4c57` (0.31.0), the actual current tip

1. **`abc4c57` IS the current stable idle tip.** The daemon is HELD idle *after* the 4.1b-1 seal
   (per the daemon-orch note) — so 0.31.0 is the stable boundary, not 0.30.0.
2. **0.31.0 ⊇ 0.30.0.** It carries everything the ui L2 build needs (the typed `ApprovalQueueRow` +
   `PolicyDecision` enrichment, frozen at 0.30.0) PLUS the 4.1b-1 survival/recovery surface (the
   `SessionRecovered` event + restart-recovery), which the ui's provisional `RecoveryState`/
   `ResumeMode`/`RecoveryStatus` shadows reconcile against.
3. **One merge vs two.** Merging the stale 0.30.0 leaves the ui a daemon-round behind → another
   merge needed almost immediately. Merging the actual tip syncs the ui fully in one boundary merge.
4. The regen slice (sequence step 2) then targets **0.28→0.31** (survival types + `ApprovalQueueRow`
   + the fixture-enrichment→real swap) — a superset of the planned 0.28→0.30 regen, same shape.

## Ruling requested

Confirm the merge target: **`abc4c57` (0.31.0, recommended)** OR hold at `ea0e93e` (0.30.0) for a
reason I'm missing. On confirmation I re-merge fresh, reconcile the `IMPLEMENTATION_PLAN.md` union,
and proceed to the regen slice. **No merge committed until your confirm.**
