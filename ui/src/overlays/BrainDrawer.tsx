import { Brain, Expand, X } from "lucide-react";
import { Badge, IconButton } from "../design-system/kit";
import { BrainPage } from "../views/brain/BrainPage";
import { Overlay } from "./Overlay";

/**
 * The Project Brain drawer (ported from kit-overlays.jsx BrainDrawer): a right
 * drawer hosting the co-pilot surface in drawer mode (no memory rail), with
 * Expand → the full Brain page. The co-pilot content itself is the Phase-8
 * display fixture (see BrainPage — flagged, composer disabled).
 */
export function BrainDrawer({
  onClose,
  onExpand,
}: {
  onClose: () => void;
  onExpand: () => void;
}) {
  return (
    <Overlay onClose={onClose} align="right" width={480} label="Project Brain drawer">
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          height: "100%",
          background: "var(--surface-canvas)",
          borderLeft: "1px solid var(--brain-line)",
        }}
        data-testid="brain-drawer"
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "10px 12px",
            borderBottom: "1px solid var(--border-default)",
            background: "var(--brain-surface)",
          }}
        >
          <span aria-hidden="true" style={{ color: "var(--brain-ink)", display: "inline-flex" }}>
            <Brain size={16} />
          </span>
          <span style={{ font: "var(--fw-semibold) var(--fs-sub) var(--font-sans)", color: "var(--text-primary)" }}>
            Project Brain
          </span>
          <Badge tone="brain" variant="dot" style={{ marginLeft: 4 }}>
            co-pilot
          </Badge>
          <span style={{ marginLeft: "auto", display: "flex", gap: 4 }}>
            <IconButton label="Expand to full page" onClick={onExpand}>
              <Expand size={15} />
            </IconButton>
            <IconButton label="Close" onClick={onClose}>
              <X size={15} />
            </IconButton>
          </span>
        </div>
        <div style={{ flex: 1, minHeight: 0 }}>
          <BrainPage drawer />
        </div>
      </div>
    </Overlay>
  );
}
