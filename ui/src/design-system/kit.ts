// Single import surface for the NexusOps-ui-kit components the shell uses.
//
// Keeps the @ui-kit alias path in ONE place so a re-hue / kit upgrade touches
// only this file (the kit's "re-hue touches primitives only" promise, §11.1).
// Tokens are linked separately via the kit styles.css (imported in main.tsx);
// components come from the kit .jsx sources through the @ui-kit Vite alias.
import type { CSSProperties, ReactElement, ReactNode } from "react";
import { Badge as KitBadge } from "@ui-kit/badges/Badge";

export { Button } from "@ui-kit/controls/Button";
export { IconButton } from "@ui-kit/controls/IconButton";

// Typing shim: the kit Badge.d.ts types its props via React.HTMLAttributes
// (default-import React), which doesn't resolve `children` under this tsconfig.
// Runtime is the kit component untouched; only the type surface is restated.
export const Badge = KitBadge as (props: {
  tone?:
    | "neutral"
    | "accent"
    | "brain"
    | "teal"
    | "success"
    | "attention"
    | "caution"
    | "warning"
    | "danger"
    | "review"
    | "slate";
  variant?: "soft" | "solid" | "outline" | "dot";
  mono?: boolean;
  icon?: ReactNode;
  size?: "xs" | "sm" | "md";
  children?: ReactNode;
  style?: CSSProperties;
  title?: string;
}) => ReactElement;
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
//
// DisplayStatusPill is the ONE sanctioned exception: a kit pill for DISPLAY-ONLY
// states that have no frozen (machine,status) contract (e.g. integration
// connected / profile health — provisional display fixtures). Callers MUST pass
// an explicit kit kind + label; frozen-enum statuses still go through the
// descriptor wrapper, never this.
export { StatusPill as DisplayStatusPill } from "@ui-kit/status/StatusPill";
