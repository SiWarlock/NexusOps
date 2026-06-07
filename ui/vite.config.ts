import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

// The canonical NexusOps-ui-kit lives as a SIBLING dir (../NexusOps-ui-kit).
// We consume it the §11.1 way: link its token styles.css (in main.tsx) and
// import its component .jsx sources via the `@ui-kit` alias — NOT the runtime
// global bundle (which fights TS-strict ES modules) and NOT a vendored copy
// (which would diverge from the canonical kit). server.fs.allow lets Vite read
// the sibling dir; @vitejs/plugin-react transforms the kit .jsx on the fly.
const uiKitComponents = fileURLToPath(
  new URL("../NexusOps-ui-kit/components", import.meta.url),
);
const uiKitRoot = fileURLToPath(new URL("../NexusOps-ui-kit", import.meta.url));

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@ui-kit": uiKitComponents,
    },
    // The kit .jsx sources live OUTSIDE this project and import `react` as a
    // peer; their dir has no node_modules. dedupe forces react / react-dom (and
    // the injected react/jsx-*-runtime subpaths, matched by package name) to
    // resolve from THIS project's root, so the single ui/ react copy backs both
    // app and kit. (Vite resolves deduped deps from `root`, not the importer.)
    dedupe: ["react", "react-dom"],
  },
  server: {
    fs: {
      allow: [".", uiKitRoot],
    },
  },
});
