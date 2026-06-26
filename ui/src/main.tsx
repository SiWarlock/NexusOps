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
// xterm.js base stylesheet for the §6.4 Session Terminal well (6.3d). Global, like
// the other CSS above — keeps TerminalDisplay import-free of CSS (test-clean).
import "@xterm/xterm/css/xterm.css";
import { Shell } from "./shell/Shell";
import { MockGatewayPort } from "./gateway-client/mock";
import type { GatewayPort } from "./gateway-client/types";

/**
 * ui-075 — the dev-shell visual-gate seam. A BUILD-TIME env-gated Mock-injection branch: when
 * `VITE_NEXUSOPS_MOCK` is EXACTLY `"1"` the entry injects a `MockGatewayPort` (a daemon-free shell whose
 * PR-mutation controls render ENABLED — the visual gate's pixel-check surface, no live daemon needed);
 * any other value / unset (the default / every production build) returns `undefined` → the Shell falls
 * back to the production `UdsGatewayPort`. The guard is an EXPLICIT ALLOWLIST (`=== "1"`), not a
 * truthiness check: a string env value like `"0"`/`"false"` is JS-truthy, so a truthiness guard would
 * INVERT a "disable" attempt into a Mock-in-prod leak — the allowlist fails CLOSED to the production port.
 * `import.meta.env.VITE_*` is a build-time LITERAL (Vite inlines it), so a clean production build (env
 * unset) statically evaluates this to `return undefined` and dead-code-eliminates the Mock branch; the
 * load-bearing guarantee is this fail-safe default + the allowlist (a bundle-grep gate is a Step-9
 * hardening follow-on). Exported so it's unit-testable without mounting the app.
 */
export function resolveEntryGateway(): GatewayPort | undefined {
  return import.meta.env.VITE_NEXUSOPS_MOCK === "1" ? new MockGatewayPort() : undefined;
}

// 6.1b: mount the shell as the production entry point. <Shell/> instantiates the
// gateway-client and reads projections through the boundary validator. As of the L1
// read-swap (051) the production default is the real UdsGatewayPort — the initial
// load reads REAL daemon data over the 050 invoke bridge (the MockGatewayPort is now
// the injectable test/dev seam). The live subscribe stream + recovery land in 052.
// ui-075: the gateway prop is the env-gated dev seam (undefined in production → UdsGatewayPort).
const rootEl = document.getElementById("root");
if (rootEl) {
  createRoot(rootEl).render(
    <StrictMode>
      <Shell gateway={resolveEntryGateway()} />
    </StrictMode>,
  );
}
