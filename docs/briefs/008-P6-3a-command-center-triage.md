# /tdd brief — command_center_triage

## Feature
The **Command Center triage view**: items partitioned into **needs-attention / working / settled** (via the 6.2a triage buckets), with a distinct **Changes-ready** grouping, each item rendered via the 6.2b `StatusPill`/`AttentionMarker` and sorted within-group by `compareByAttention`. Mounts as the shell's default content view, rendering from fixture projections through the gateway boundary. First sub-slice of Phase 6.3 (core screens).

## Use case + traceability
- **Task ID:** P6.3a (decomposition of 6.3: **6.3a Command Center** → 6.3b Graph+list-fallback → 6.3c Sessions → 6.3d Terminal → 6.3e Code/Diff)
- **Architecture sections:** `ARCHITECTURE.md §11.2` (Command Center triage — needs-attention/working/settled incl. a Changes-ready grouping), `§11.3` (attention ordering / sort), `§5.2` (attention ordering per PRD), `§4.2` (renders from projections), `§11` (screen mounts in the shell content pane).
- **Related context:** 6.2a model (`ui/src/status/` — `triageBucket`, `needsMyAttention`, `compareByAttention`); 6.2b wrappers (`StatusPill`/`AttentionMarker`); 6.1b shell (the content pane to mount into); fixtures (`proj_session`/`proj_pull_request`/`proj_approval_queue`/etc via the gateway-client). Deterministic core = the **grouping + sort** (pure logic); rendering is render-tested. ExecutionProfile items excluded (held 0.5b).

## Acceptance criteria
- [ ] `groupForCommandCenter(items)` partitions items into **needs-attention / working / settled** by the 6.2a `triageBucket`, and extracts a **Changes-ready** group (items whose status is `changes_ready`).
- [ ] Within each group, items are sorted by `compareByAttention` (higher rank first).
- [ ] The Command Center view renders each group (with its items via `StatusPill`/`AttentionMarker`), mounts as the **default content view** in the shell (replacing the 6.1b placeholder), and reads **only through the gateway boundary** (no invented state — forbidden-pattern #2).
- [ ] Each empty group renders an explicit **empty state** (not absent).
- [ ] **Reachable from** `Shell → (content pane) → CommandCenter → gateway-client`. `/preflight` clean.

## Wiring / entry point (Step 7.5)
`Shell` content pane → `<CommandCenter/>` → `gateway-client` boundary → fixtures. Replaces the 6.1b placeholder content pane — names the real landing view. Confirm at Step 7.5 it's the default view + reads through the boundary.

## Files expected to touch
**New:** `ui/src/views/command/CommandCenter.tsx`, `ui/src/views/command/group.ts` (pure `groupForCommandCenter` + within-group sort), `ui/src/views/command/{group.test.ts, CommandCenter.test.tsx}`.
**Modified:** `ui/src/shell/Shell.tsx` (mount `<CommandCenter/>` in the content pane). Flag anything beyond at Step 2.5.

## RED test outline (Step 2)
**`views/command/group.test.ts`:**
1. **`groups_items_into_triage_buckets`** — items partition into needs-attention/working/settled per `triageBucket` (rank {4,5}/{2,3}/{0,1}). **[load-bearing]**
2. **`changes_ready_extracted_as_its_own_group`** — items with status `changes_ready` (across machines) surface in the Changes-ready group.
3. **`within_group_sorted_by_attention`** — each group's items are `compareByAttention`-ordered.

**`views/command/CommandCenter.test.tsx` (jsdom):**
4. **`renders_groups_from_projection`** — renders fixture items via the gateway boundary; rendered set === fixture set (no invented state). **[load-bearing — forbidden #2]**
5. **`empty_group_shows_empty_state`** — an empty triage group renders an explicit empty state.

## Cross-doc invariant impact
- **Model field changes:** none (consumes 6.2a + fixtures). **Orchestrator rows:** none expected.

## Things to flag at Step 2.5
1. **Changes-ready grouping.** Default vote: a **distinct labeled group** surfaced prominently (its items are already rank-4/needs-attention, so it's a highlighted cluster, not a separate bucket from the partition). Confirm vs a 4th independent bucket.
2. **Item sources.** Default vote: the projections the fixtures expose (sessions + tasks + approvals + PRs); exclude ExecutionProfile (held). Confirm the set.
3. **Command Center as default content view.** Default vote: yes — it's the landing view in the shell content pane. Confirm.
4. **Scope = triage groups only.** Default vote: 6.3a = the three triage groups + Changes-ready; the **Human Input Queue, capacity meters, and live event feed are deferred** (the activity dock already carries the event feed; HIQ is a later slice). Confirm.

## Dependencies + sequencing
- **Depends on:** 6.2a (`b32c3c0`) + 6.2b (`e2cebbc`) + 6.1b shell (`39a87c6`).
- **Blocks:** the rest of 6.3 (Graph/Sessions/Terminal/Code-Diff); the §25 demo triage surface.

## Estimated commit count
**1.** Cohesive triage view (grouping logic + render + shell mount). No safety invariant → security-reviewer NOT required; code-quality every-slice.

## Lessons-logged candidates anticipated
- **Convention candidate** — Command Center triage groups are derived purely from the 6.2a `triageBucket`/`compareByAttention` (single source); screens never re-derive attention. (Likely folds into `ui/LESSONS.md §5`.)

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd command_center_triage`.
1. Read this brief; Q1 (Changes-ready grouping) + Q4 (scope) are the ones to confirm.
2. Step 2.5 — test design + answers. Wait for the magic-words reply.
3. Step 7.5 — name `Shell → CommandCenter → gateway-client` as the entry point.
4. Step 9 — commit-message-first.
