// @vitest-environment jsdom
// Wiring guard for the 6.5a base theme — the DETERMINISTIC part only (Lesson §10):
// it catches gross absence (theme not imported / sr-only not applied), but it does
// NOT verify appearance. The acceptance is the rendered visual check (dark canvas +
// Geist + scrollbars), not these assertions.
import { describe, it, expect, afterEach } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { cleanup, render, screen } from "@testing-library/react";
import { TopBar } from "../shell/TopBar";

afterEach(cleanup);

describe("6.5a theme base — wiring guard (NOT the visual gate)", () => {
  it("main_imports_global_theme_after_kit_tokens", () => {
    // vitest runs with cwd = the ui/ package root.
    const mainSrc = readFileSync(resolve(process.cwd(), "src/main.tsx"), "utf8");
    // the theme stylesheet is wired into the production entry point...
    expect(mainSrc).toContain("./theme/global.css");
    // ...AFTER the kit token manifest (tokens defined first, then applied)
    const kitIdx = mainSrc.indexOf("NexusOps-ui-kit/styles.css");
    const themeIdx = mainSrc.indexOf("./theme/global.css");
    expect(kitIdx).toBeGreaterThanOrEqual(0);
    expect(themeIdx).toBeGreaterThan(kitIdx);
  });

  it("topbar_accessible_names_use_global_sr_only", () => {
    render(
      <TopBar
        projects={[]}
        counts={{}}
        onOpenSettings={() => {}}
        onBack={() => {}}
        onForward={() => {}}
        canBack={false}
        canForward={false}
      />,
    );
    // the back/forward accessible names are visually-hidden via the global
    // .sr-only utility (the SR_ONLY inline-style swap is behavior-preserving)
    expect(screen.getByText("Back").className).toBe("sr-only");
    expect(screen.getByText("Forward").className).toBe("sr-only");
  });
});
