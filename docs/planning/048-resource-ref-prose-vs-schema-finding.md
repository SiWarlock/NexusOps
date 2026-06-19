# Finding — 6.3e resource_ref: daemon/brief PROSE vs the FROZEN 0.28.0 schema

> **Orchestrator-authored** for `ui-team-lead` (requested at 048 Step-2.5). The
> implementer surfaced this at 048 Step-2.5; I verified it independently against the
> frozen schema. **Bottom line up front: this is a DOC-PROSE imprecision, NOT a
> contract gap. The 0.28.0 ① freeze is sound + complete for 6.3e. ui-resolvable; NO
> daemon ②-mini contract packet needed — the only daemon-side action is a one-line §6.3
> PROSE tidy (informal→precise). No cat-1 ruling touched.**

## 1. What the gap is

It is **not** a missing contract field or semantic — 0.28.0 provides exactly what 6.3e
needs. It is a **prose-vs-schema wording mismatch** on the `ResourceRef` shape the ui
forms for a per-hunk `git.*` action.

- **The FROZEN schema (the §5.0 source-of-truth, `shared/contracts/schema/nexusops-contract.schema.json`) — CORRECT + COMPLETE:**
  `ResourceRef` `$def` = `{ type, id, uri? }`, `required: [type, id]`; `ResourceType`
  enum carries the lowercase value **`"file"`** (verified: `['project','repo','worktree','branch','file','diff',…]`).
  The ui's `intent-contracts.ts` already mirrors this exactly (`ResourceRef = z.object({ type: bundle.shape.ResourceType, id, uri? })`).
- **The informal PROSE (loose) — in two places:**
  - `daemon/CLAUDE.md` §6.3 cross-doc row: *"`ResourceRef{resource_type=File, id="{worktree_id}\x1f{file}\x1f…"}`"*
  - my brief 048 (now CORRECTED): same `resource_type:"File"` shorthand.
  Both used **Rust-variant shorthand** (`ResourceType::File`, which serdes to `"file"`)
  and a non-existent field name `resource_type`. Taken literally, `{resource_type:"File"}`
  **fails `ResourceRef.parse`** (wrong field + wrong-case value).

The substantive contract (the `\x1f`-delimited hunk-encoding convention, the `File`
resource type, hunk-precise `id`) is **fully present + unambiguous at 0.28.0**. The
gap is purely descriptive wording.

## 2. 6.3e's workaround

**None required** — 6.3e conforms to the frozen schema directly (the §5.0 "schema is
authoritative, prose is origin/informal" discipline). The implementer forms
`{ type: "file", id: "{wt}\x1f{file}\x1f{old_start},{old_lines},{new_start},{new_lines}" }`
and pins it with a dedicated conformance test (`resource_ref_type_is_frozen_lowercase_file`
+ the encoder ×4: exact/round-trip/U+001F/distinct-hunks). No provisional shape, no
UI-side bridge, no deferred sub-feature. The security-critical pin (submitted hunk ==
displayed hunk) is **strengthened** by conforming to the precise `type:"file"` form.

## 3. Resolution class

**ui-resolvable — DONE in 048. NO daemon ②-mini contract packet needed.**

- The 0.28.0 **contract** needs **zero change** (it is already correct + complete).
- The only daemon-side action is an **optional one-line PROSE tidy** in `daemon/CLAUDE.md`
  §6.3: `ResourceRef{resource_type=File, …}` → `ResourceRef{type:"file", …}` (or note
  the `ResourceType::File`→`"file"` serialization explicitly), so a future reader who
  takes the prose literally doesn't write a parse-failing `{resource_type:"File"}`. This
  is **daemon-track doc territory** (the ui track doesn't edit `daemon/CLAUDE.md`) — route
  it to the daemon track as a low-priority doc cleanup whenever it next runs; it is **not**
  a freeze, not blocking, not urgent.
- My brief 048 is **already corrected** (the ui-side artifact).

## 4. Cat-1 / safety implication

**None — no ruling touched.** This is a coverage/precision gap, exactly as you
predicted, not a safety posture. If anything it **reinforces** the §5.0 discipline (the
schema is the authority; the implementer correctly conformed to it over the loose prose)
and the cat-1 Q-rulings (the resource_ref correctness pin — submitted == displayed —
is the security-critical control and is now pinned against the precise frozen form).

## 5. One-line for the user (if surfaced)

> The ① 0.28.0 freeze is sound; a per-hunk-action resource_ref was described loosely in
> the daemon's own notes (`resource_type:"File"`) vs the precise frozen schema
> (`type:"file"`) — caught at ui Step-2.5, the UI conforms to the schema, no contract
> change or follow-up freeze needed; just a one-line daemon doc tidy.
