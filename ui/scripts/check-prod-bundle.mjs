// ui-078 — the production-bundle gate (§10.6 hardening). Mechanically backstops the ui-075 cat-1
// go-live's prod-no-Mock guarantee: the dev MockGatewayPort (and its VITE_NEXUSOPS_MOCK build flag)
// must tree-shake OUT of a production `vite build`. That guarantee previously rested only on Vite's
// static dead-code elimination + the source unit pin (`main_default_path_uses_production_uds_no_mock`);
// this scans the BUILT chunks so a regression (a stray runtime env read, a side-effectful import, a
// bundler change) fails CI loudly instead of silently shipping a deceptive cockpit.
//
// Usage: `pnpm check:bundle` (= `vite build && node scripts/check-prod-bundle.mjs`). Exit 0 + an OK
// line when clean; exit 1 + the offending file(s) when a forbidden string is present.
// Note: `check:bundle` runs `vite build` (NOT the full `pnpm build` = `tsc --noEmit && vite build`) — it
// is the bundle gate, not a type-check substitute (CI runs `pnpm typecheck` as its own earlier step).
import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/** Strings that must NEVER appear in a production bundle: the dev Mock gateway + its build-env flag. */
export const FORBIDDEN = ["MockGatewayPort", "VITE_NEXUSOPS_MOCK"];

/** The built-output dir (relative to cwd = `ui/`, where the npm scripts run). */
export const DIST_DIR = "dist";

/**
 * Pure: the subset of `needles` that appear (SUBSTRING) in `text`. Substring (not word-boundary) so a
 * minifier-renamed occurrence (e.g. `MockGatewayPort$1`) still trips; returns ALL matches (not just the
 * first) so a second leak can't hide behind the first. Exported for unit-pinning (check-prod-bundle.test.mjs).
 *
 * Residual limitation (accepted): if tree-shaking FAILS *and* the minifier fully mangles the class name to
 * a short opaque identifier (e.g. `M`), the substring scan can't catch it — substring ≠ semantic/AST. The
 * LOAD-BEARING guarantee remains Vite's dead-code elimination (verified by build + this gate clean today);
 * this gate is the backstop for the common-case leak (a stray runtime env read / a side-effectful import),
 * not a proof of absence under adversarial mangling.
 */
export function scanForForbidden(text, needles) {
  return needles.filter((needle) => text.includes(needle));
}

/** Recursively collect every `*.js` file path under `dir`. */
function jsFilesUnder(dir) {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...jsFilesUnder(p));
    else if (entry.name.endsWith(".js")) out.push(p);
  }
  return out;
}

function main() {
  if (!existsSync(DIST_DIR)) {
    console.error(
      `check-prod-bundle: no ${DIST_DIR}/ — run \`vite build\` first (or \`pnpm check:bundle\`).`,
    );
    process.exit(1);
  }
  const offenders = [];
  for (const file of jsFilesUnder(DIST_DIR)) {
    const found = scanForForbidden(readFileSync(file, "utf8"), FORBIDDEN);
    if (found.length > 0) offenders.push({ file, found });
  }
  if (offenders.length > 0) {
    console.error(
      "check-prod-bundle: FAIL — the dev Mock leaked into the production bundle:",
    );
    for (const { file, found } of offenders) {
      console.error(`  ${file}: ${found.join(", ")}`);
    }
    console.error(
      `The dev Mock (${FORBIDDEN.join(" / ")}) must tree-shake out of prod (ui-075 prod-no-Mock).`,
    );
    process.exit(1);
  }
  console.log(
    `check-prod-bundle: OK — no ${FORBIDDEN.join(" / ")} in the production bundle.`,
  );
}

// Run the I/O gate ONLY when executed directly (`node scripts/check-prod-bundle.mjs`), never when the
// vitest imports `scanForForbidden` (then process.argv[1] is the vitest runner, so main() stays inert).
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main();
}
