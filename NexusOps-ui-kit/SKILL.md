---
name: nexusops-design
description: Use this skill to generate well-branded interfaces and assets for NexusOps (the "Graphite Arc" desktop control plane for AI engineering agents), either for production or throwaway prototypes/mocks/etc. Contains essential design guidelines, colors, type, fonts, assets, and UI kit components for prototyping.
user-invocable: true
---

Read the README.md file within this skill, and explore the other available files.
If creating visual artifacts (slides, mocks, throwaway prototypes, etc), copy assets out and create static HTML files for the user to view. If working on production code, you can copy assets and read the rules here to become an expert in designing with this brand.
If the user invokes this skill without any other guidance, ask them what they want to build or design, ask some questions, and act as an expert designer who outputs HTML artifacts _or_ production code, depending on the need.

## Quick map
- `styles.css` — link this one file; it `@import`s every token + the webfonts (Geist / Geist Mono via Google Fonts).
- `tokens/` — `surfaces.css` (graphite ramp), `color.css` (Graphite Arc taxonomy: accent azure, brain violet, teal, + rationed status hues), `typography.css`, `space.css`, `motion.css`, `status.css` (four-channel status + glyphs).
- `components/<group>/` — React primitives: controls (Button, IconButton), status (StatusPill, AttentionMarker, RiskBadge, UsageMeter), badges (Badge, HarnessBadge, ProfileBadge, MetaChip), objects (SessionRow, GraphNode, DiffHunk, EvidenceChip). Read each `*.prompt.md` for usage.
- `ui_kits/control-plane/index.html` — full interactive desktop recreation to copy patterns from.
- `foundations/*.html` — specimen cards for type, color, spacing, the attention ladder, iconography.

## Non-negotiables when designing for NexusOps
- **Dark graphite cockpit.** Neutrals carry structure; saturated hue only ever carries *meaning*. Reference semantic tokens (`--surface-card`, `--text-primary`, `--state-waiting-human`, `--risk-high`), not raw primitives.
- **Attention-first.** Organize any view around what needs a human. Sort by the attention ladder (waiting-on-human → failed → running → idle). Amber is the loudest color and is rationed.
- **Status is never color alone** — always color + glyph + text label (use `StatusPill`).
- **Voice:** calm, exact, operational, sentence case, UPPERCASE micro-eyebrows for sections; mono for all IDs/paths/SHAs/numbers; no emoji, no hype.
- **Icons:** Lucide line icons (CDN) for objects/actions; geometric Unicode glyphs for status markers.
- **Restraint:** tight mechanical radii, 4px grid, compact controls, motion only for live-pulse + attention-beacon, depth reserved for overlays.

When in doubt, open `ui_kits/control-plane/index.html` and match its density, spacing, and component usage exactly.
