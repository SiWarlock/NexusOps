import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { validators, Session, ActorType, IdKind, CONTRACT_VERSION } from "./index";

// Read the FROZEN schema at test time (no cargo / no codegen needed) — this is
// the TS mirror of the Rust CI schema-diff gate (ARCHITECTURE.md §5.0 pt 2).
const schemaPath = fileURLToPath(
  new URL(
    "../../../shared/contracts/schema/nexusops-contract.schema.json",
    import.meta.url,
  ),
);
const schema = JSON.parse(readFileSync(schemaPath, "utf8")) as {
  $defs: Record<string, { enum?: string[] }>;
  "x-contract-version": string;
};
const enumDefs = Object.entries(schema.$defs).filter(
  ([, d]) => Array.isArray(d.enum),
);

describe("generated zod contract layer", () => {
  it("generated_zod_accepts_every_canonical_enum_value", () => {
    for (const [name, def] of enumDefs) {
      const v = validators[name];
      expect(v, `missing generated validator for ${name}`).toBeDefined();
      for (const member of def.enum!) {
        expect(
          () => v!.parse(member),
          `${name} should accept canonical value "${member}"`,
        ).not.toThrow();
      }
    }
  });

  it("generated_zod_rejects_unknown_enum_value", () => {
    expect(() => Session.parse("bogus")).toThrow();
    expect(() => ActorType.parse("hacker")).toThrow();
    expect(() => IdKind.parse("not_an_id_kind")).toThrow();
  });

  it("generated_zod_member_sets_equal_frozen_schema", () => {
    // Same set of value-sets present (no extra, none missing).
    expect(Object.keys(validators).toSorted()).toEqual(
      enumDefs.map(([n]) => n).toSorted(),
    );
    // Each generated enum's members === the frozen $defs enum (set equality).
    for (const [name, def] of enumDefs) {
      const generated = new Set(validators[name]!.options as readonly string[]);
      const frozen = new Set(def.enum!);
      expect(generated, `member drift in ${name}`).toEqual(frozen);
    }
  });

  it("generated_contract_version_matches_frozen_schema", () => {
    // The capabilities-reported contract_version must not drift from the
    // frozen schema's x-contract-version (§5.0 version gate).
    expect(CONTRACT_VERSION).toBe(schema["x-contract-version"]);
  });
});
