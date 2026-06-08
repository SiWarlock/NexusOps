import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
// Link the canonical NexusOps-ui-kit token layer (the §11.1 design system).
// styles.css is the kit's @import manifest of tokens/*.css; consumers link only it
// (it carries the global @media(prefers-reduced-motion) guard via motion.css).
import "../../NexusOps-ui-kit/styles.css";
// 6.5a Graphite Arc base/global theme — APPLIES the kit semantic tokens at the
// document root (dark --surface-window canvas + Geist + scrollbar chrome + the
// global .sr-only utility). Loads AFTER the kit tokens (so they resolve) and
// BEFORE focus.css (disjoint selectors; the focus ring stays last).
import "./theme/global.css";
// 6.5b Graphite Arc shell chrome + cockpit layout grid — paints the .shell region
// grid + region chrome (topbar/sidebar/main/dock/status/banner) from semantic
// tokens. After global.css (base canvas), before focus.css.
import "./theme/shell.css";
// 6.5c Graphite Arc view + component panels — themes the in-view panels + the
// banner/safety surfaces (severity color additive to glyph+label). After shell.css.
import "./theme/components.css";
// Global :focus-visible ring on every interactive control (§11.6) — uses the kit
// ring tokens; must load after the kit tokens so the custom properties resolve.
import "./a11y/focus.css";
import { Shell } from "./shell/Shell";

// 6.1b: mount the shell as the production entry point. <Shell/> instantiates the
// gateway-client (MockGatewayPort for now) and reads projections through the
// boundary validator — closing 6.1a's foundation reachability gap. The real
// UdsGatewayPort backs this once daemon 1.5 is live.
const rootEl = document.getElementById("root");
if (rootEl) {
  createRoot(rootEl).render(
    <StrictMode>
      <Shell />
    </StrictMode>,
  );
}
