#!/usr/bin/env node
// Regenerate the UI's Zod contract layer FROM the frozen shared schema.
//
// Reads  shared/contracts/schema/nexusops-contract.schema.json  (READ-ONLY)
// Writes ui/src/contracts/generated.ts                          (checked-in artifact)
//
// Mirrors shared/contracts/verify/run.sh + verify.py: json-schema-to-zod does
// NOT resolve internal $ref (it would emit z.any() for the root's $ref props),
// so we inline the enum $defs into a flat object before generating. This is
// lossless for the value sets — the same published contract values — which is
// exactly what the drift test (src/contracts/generated.test.ts) pins.
// ARCHITECTURE.md §5.0: generated, drift-caught Zod consumer of the Rust authority.
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const schemaPath = fileURLToPath(
  new URL(
    "../../shared/contracts/schema/nexusops-contract.schema.json",
    import.meta.url,
  ),
);
const outPath = fileURLToPath(
  new URL("../src/contracts/generated.ts", import.meta.url),
);
const binPath = fileURLToPath(
  new URL("../node_modules/.bin/json-schema-to-zod", import.meta.url),
);

const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
const defs = schema["$defs"] ?? schema.definitions ?? {};

const properties = {};
const required = [];
for (const [name, def] of Object.entries(defs)) {
  if (def && Array.isArray(def.enum)) {
    properties[name] = def;
    required.push(name);
  }
}
if (required.length === 0) {
  console.error("gen-contracts: no enum $defs found in schema");
  process.exit(1);
}

const version = schema["x-contract-version"];
if (!version) {
  console.error("gen-contracts: schema missing x-contract-version");
  process.exit(1);
}

const tmp = mkdtempSync(join(tmpdir(), "nexusops-zodgen-"));
try {
  const flatPath = join(tmp, "flat.json");
  // `required` on every key so json-schema-to-zod does not wrap members in
  // .optional() — we want bare z.enum(...) so index.ts can read .shape.<Name>.
  writeFileSync(
    flatPath,
    JSON.stringify({ type: "object", properties, required }),
  );

  const rawPath = join(tmp, "raw.ts");
  execFileSync(binPath, ["--input", flatPath, "--output", rawPath], {
    stdio: ["ignore", "ignore", "inherit"],
  });
  const generated = readFileSync(rawPath, "utf8").trim();

  const banner =
    [
      "// GENERATED FILE — DO NOT EDIT BY HAND.",
      `// Source of truth: shared/contracts/schema/nexusops-contract.schema.json (FROZEN, x-contract-version ${version}).`,
      "// Regenerate with: pnpm gen:contracts   (ARCHITECTURE.md §5.0 — generated, drift-caught Zod consumer).",
      "// The drift test (src/contracts/generated.test.ts) fails if this file drifts from the frozen schema.",
      "",
    ].join("\n") + "\n";

  const versionConst = `\n\nexport const CONTRACT_VERSION = ${JSON.stringify(version)};\n`;

  writeFileSync(outPath, banner + generated + versionConst);
  console.log(
    `gen-contracts: wrote ${outPath} from ${required.length} enum $defs (CONTRACT_VERSION=${version})`,
  );
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
