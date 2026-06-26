import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config";

// Inherit the app config (React plugin + the @ui-kit alias + fs.allow) so tests
// resolve kit components exactly as the app does. Pure-logic tests run in the
// node env (fast); the .tsx render tests opt into jsdom via a per-file
// `// @vitest-environment jsdom` docblock.
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: "node",
      include: [
        "src/**/*.test.ts",
        "src/**/*.test.tsx",
        // ui-078: build-script unit tests (e.g. the prod-bundle gate's pure scanner).
        "scripts/**/*.test.mjs",
      ],
      globals: false,
    },
  }),
);
