import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

// 6.1a foundation: minimal mount stub ONLY. The real shell (top bar, project
// switcher, sidebar, right-drawer stack, activity dock, status bar) + the
// daemon-connection indicator + global read-only degraded mode land in 6.1b and
// mount the gateway-client as the single daemon-access seam.
const rootEl = document.getElementById("root");
if (rootEl) {
  createRoot(rootEl).render(
    <StrictMode>
      <div>NexusOps — foundation (6.1a). Shell lands in 6.1b.</div>
    </StrictMode>,
  );
}
