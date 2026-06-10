// Single import surface for the NexusOps-ui-kit components the shell uses.
//
// Keeps the @ui-kit alias path in ONE place so a re-hue / kit upgrade touches
// only this file (the kit's "re-hue touches primitives only" promise, §11.1).
// Tokens are linked separately via the kit styles.css (imported in main.tsx);
// components come from the kit .jsx sources through the @ui-kit Vite alias.
export { Button } from "@ui-kit/controls/Button";
export { IconButton } from "@ui-kit/controls/IconButton";
export { Badge } from "@ui-kit/badges/Badge";
export { HarnessBadge } from "@ui-kit/badges/HarnessBadge";
export { MetaChip } from "@ui-kit/badges/MetaChip";
export { ProfileBadge } from "@ui-kit/badges/ProfileBadge";
export { RiskBadge } from "@ui-kit/status/RiskBadge";
export { UsageMeter } from "@ui-kit/status/UsageMeter";
export { SessionRow } from "@ui-kit/objects/SessionRow";
export { GraphNode } from "@ui-kit/objects/GraphNode";
export { EvidenceChip } from "@ui-kit/objects/EvidenceChip";
export { DiffHunk } from "@ui-kit/objects/DiffHunk";
// NOTE: the kit StatusPill/AttentionMarker are NOT exported here on purpose —
// status rendering goes through the descriptor-bound wrappers in ui/src/status/
// (Lesson §6: single source = the descriptor table; kit fallback guarded).
