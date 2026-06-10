import { useEffect, useRef, type CSSProperties, type ReactNode } from "react";

/**
 * Overlay shell (ported from kit-overlays.jsx Overlay): scrim + blur, three
 * alignments (center modal · top palette · right drawer), click-outside and
 * Escape close, dialog semantics. Focus moves into the panel on open and
 * returns to the previously-focused element on unmount (§11.6).
 */
export function Overlay({
  onClose,
  align = "center",
  width = 480,
  label,
  children,
}: {
  onClose: () => void;
  align?: "center" | "top" | "right";
  width?: number;
  label: string;
  children: ReactNode;
}) {
  const panelRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const prev = document.activeElement as HTMLElement | null;
    // focus the panel (or its first autofocus-able input) so keyboard users
    // land inside the dialog
    const auto = panelRef.current?.querySelector<HTMLElement>("[data-autofocus]");
    (auto ?? panelRef.current)?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      prev?.focus?.();
    };
  }, [onClose]);

  const pos: CSSProperties =
    align === "center"
      ? { alignItems: "center", justifyContent: "center" }
      : align === "top"
        ? { alignItems: "flex-start", justifyContent: "center", paddingTop: "12vh" }
        : { alignItems: "stretch", justifyContent: "flex-end" };

  return (
    <div
      onClick={onClose}
      data-testid="overlay-scrim"
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 300,
        display: "flex",
        background: "var(--scrim)",
        backdropFilter: "blur(2px)",
        ...pos,
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={label}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        style={{
          width: align === "right" ? width : "100%",
          maxWidth: align === "right" ? undefined : width,
          margin: align === "right" ? 0 : "0 16px",
          animation:
            align === "right" ? "cp-slide-in 0.24s var(--ease-out)" : "cp-pop-in 0.18s var(--ease-out)",
          outline: "none",
        }}
      >
        {children}
      </div>
    </div>
  );
}
