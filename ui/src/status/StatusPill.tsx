import {
  StatusPill as KitStatusPill,
  STATUS as KIT_STATUS,
} from "@ui-kit/status/StatusPill";
import type { StatusKind } from "@ui-kit/status/StatusPill";
import { describeStatus } from "./descriptors";

// A thin wrapper: (machine, status) → 6.2a descriptor → kit StatusPill. Never
// re-declares a status's visual/label/rank at the call site (single source = the
// descriptor table). The kit renders the four channels (color + glyph + label +
// motion); we supply the visualKind + label.

// Derived from the kit's own STATUS map (StatusPill.jsx) — zero drift if the kit
// adds / renames / removes a visual kind.
const KIT_KINDS = new Set<string>(Object.keys(KIT_STATUS));

// §11.3 at the render layer: an unknown / non-kit visualKind maps to a VISIBLE
// degraded pill — NOT the kit's silent idle fallback (it does STATUS[x]||idle).
function toKitKind(visualKind: string): StatusKind {
  return (KIT_KINDS.has(visualKind) ? visualKind : "degraded") as StatusKind;
}

/**
 * The guarded (machine, status) → kit StatusKind mapping for callers that feed
 * kit components taking a raw kit `status` prop (e.g. SessionRow-shaped rows).
 * SAME single source as the pill: descriptor visualKind + the unknown→visible
 * guard (Lesson §6 — never the kit's silent ||idle fallback).
 */
export function kitKindFor(machine: string, status: string): StatusKind {
  return toKitKind(describeStatus(machine, status).visualKind);
}

// R-5 two-surface: Approval (decision axis) and ActionRequest (execution axis)
// render as two distinct surfaces — a visible surface-type chip disambiguates
// the same lifecycle word (e.g. awaiting_approval) across the two machines.
const SURFACE_LABEL: Record<string, string> = {
  Approval: "Approval",
  ActionRequest: "Action",
};

export function StatusPill({
  machine,
  status,
  size = "sm",
  emphasis,
}: {
  machine: string;
  status: string;
  size?: "xs" | "sm" | "md";
  emphasis?: "soft" | "solid";
}) {
  const descriptor = describeStatus(machine, status);
  const surface = SURFACE_LABEL[machine];
  // data-* + the surface chip live on OUR wrapper element — the kit component's
  // typed props are closed (no data-*/HTMLAttributes passthrough).
  return (
    <span
      className="status-pill"
      data-machine={machine}
      data-status={status}
      data-visual-kind={descriptor.visualKind}
      style={{ display: "inline-flex", alignItems: "center", gap: 4 }}
    >
      {surface ? (
        <span className="status-pill__surface">{surface}</span>
      ) : null}
      <KitStatusPill
        status={toKitKind(descriptor.visualKind)}
        label={descriptor.label}
        size={size}
        emphasis={emphasis}
      />
    </span>
  );
}
