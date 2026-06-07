import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// 6.1a tests are pure-TS (contract layer + boundary + mock); no DOM is needed,
// so the node environment is sufficient. Component tests (6.1b+) will add jsdom.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "node",
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    globals: false,
  },
});
