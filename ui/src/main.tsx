import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
// Link the canonical NexusOps-ui-kit token layer (the §11.1 design system).
// styles.css is the kit's @import manifest of tokens/*.css; consumers link only it.
import "../../NexusOps-ui-kit/styles.css";
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
