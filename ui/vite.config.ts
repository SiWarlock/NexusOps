import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// App build config. Test config lives in vitest.config.ts (Phase 6.1a: the
// real shell + dev server wiring lands in 6.1b; this stays minimal for now).
export default defineConfig({
  plugins: [react()],
});
