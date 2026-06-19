# ui track — post-6.3e direction (for the lead → user)

> **Orchestrator-authored** decision surface for `ui-team-lead` to map to the user
> (escalation category 4 — a load-bearing track-direction call; the prior pause/route
> decisions were user-ruled, and routing the next cross-track packet is "the USER's
> later call" per the resume handoff). **The ui track has reached a clean milestone.**

## Where the track is (clean milestone)

The **high-value item the user routed the unblock packet for is DONE.** After the
`main@26c87a3` → `track/ui` boundary merge (CONTRACT 0.28.0):

- **047** `fbd6adc` — regen ui-Zod 0.23.0→0.28.0 + the `get_diff` read surface (non-safety).
- **048** `bd07349` — **6.3e proper (cat-1)**: the per-hunk stage/unstage/discard wiring over the intent seam; `security-reviewer` PASS (0 crit/0 high; Q1–Q7 PASS); visual gate PASS; the resource_ref security pin (submitted == displayed) adversarially verified.
- **fixup** `74d26e6` — fixture fidelity (origin-char strip; the visual gate caught a mock-vs-daemon divergence).

**297 ui tests green; preflight clean; CONTRACT 0.28.0.** `track/ui` is unpushed since
the boundary merge (round close-out pushes it). The cat-1 rulings (Q1–Q7) held; one
low-priority cross-track doc-precision Finding logged (`048-resource-ref-prose-vs-schema-finding.md`
— NOT a contract gap).

## The situation: the high-value tail is again cross-track-blocked

The 6-tail was **6.3e / 7.2 / 8.2.** With **6.3e done**, the remaining high-value work is
blocked on new cross-track packets (the daemon/edges must freeze them — the user's call,
exactly as 6.3e was):

- **7.2 Full PR Review Workspace** — needs **Phase-7 PR-data** (octocrab GitHub + Linear read+link; the §7.2 PR SoT projection). The edges track owns Phase-7; not on `track/ui`.
- **8.2 Project Brain drawer** — needs **Phase-8 Brain contracts** (the stdio-MCP sidecar + the §11.5 Brain-card/plan/evidence contracts). Not landed.

The buildable **in-lane** work is **lower-value**:
- **6.6** §18 graph-render benchmark (own cadence; modest).
- **6.7** §18 diff-open benchmark — **only the render-half** is buildable now (the full git2-read→render budget needs the live `UdsGatewayPort` transport, which is parked).
- **Deferred polish:** the concurrent-submit UI guard (benign — daemon `idempotency_key` dedups), the parked 044/046 nits (`refreshPreview` unmount guard, degraded-vs-loading token, etc.).

## The options (for the user)

**A — Continue lower-value in-lane** (6.6 graph bench + the deferred polish nits).
*Pro:* keeps the track productive; small, safe, ships. *Con:* lower value; the
high-value tail stays blocked; defers the real question.

**B — Clean-pause the ui track** (the prior pattern — the high-value in-lane runway is
exhausted). *Pro:* honest; the daemon/edges stay on their critical paths; spin down
the ui team until a packet is routed. *Con:* the track idles. **This is what the user
ruled last time this exact situation occurred** (the 6-tail pause).

**C — Route the next cross-track packet NOW** (7.2 Phase-7 PR-data OR 8.2 Phase-8 Brain).
*Pro:* unblocks the next high-value tail. *Con:* needs the daemon/edges to freeze it
first (a cross-track sequencing decision the user controls); the ui track then waits
for that freeze + merge (like the 6.3e cycle).

## Orchestrator recommendation

**The honest read: this mirrors the prior pause (the high-value in-lane runway is
exhausted; the rest is cross-track-gated).** My lean is **B (clean-pause) OR C (route
the next packet)** over A — the in-lane polish is low-value and A defers the real
sequencing question. Between B and C: **C if the user wants to keep the ui track on the
critical path** (route 7.2 or 8.2 to the daemon/edges now), **B if the daemon/edges
should stay on their current work** and the ui resumes later. **A (a small 6.6 bench +
polish) is a reasonable "keep it warm" middle** if the user wants one more in-lane round
before deciding. The cat-1 rulings + the 0.28.0 contract are durable across any choice.

**If close-out (B):** I run `/orchestrate-end` (Carry-forward triage incl. the
`enrichHunkAction`→real-projection swap + concurrent-submit guard + worktree_id source +
the resource_ref conformance pairing + the daemon §6.3 prose-tidy; the Log entry; tick
6.3 complete; push `track/ui`) + a resume handoff; the lead runs the team spin-down.
