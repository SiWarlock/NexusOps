import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { ServerFrame, WireError } from "./index";

// Read the FROZEN schema at test time — the §2.5-seam schema-snapshot for the
// provisional ServerFrame frame-mux (ARCHITECTURE.md §6.4). A daemon frame-shape
// change (a property added/removed on either variant) fails this loudly, the same
// way the generated-enum drift test fails on a value-set change.
const schemaPath = fileURLToPath(
  new URL(
    "../../../shared/contracts/schema/nexusops-contract.schema.json",
    import.meta.url,
  ),
);

describe("provisional ServerFrame (§6.4 frame-mux)", () => {
  it("serverframe_variant_fields_match_frozen_schema", () => {
    // spec(§6.4) — §2.5-seam: the provisional ServerFrame's two variant field-sets
    // must equal the frozen schema's ServerFrame.oneOf variant property sets,
    // keyed by the `frame_type` discriminant (rpc_response / subscription_push).
    const schema = JSON.parse(readFileSync(schemaPath, "utf8")) as {
      $defs: Record<
        string,
        {
          oneOf?: { properties: Record<string, { const?: string }> }[];
          properties?: Record<string, unknown>;
        }
      >;
    };
    const frozen = schema.$defs.ServerFrame!.oneOf!;
    const frozenByTag = new Map(
      frozen.map((v) => [
        v.properties.frame_type!.const!,
        Object.keys(v.properties).toSorted(),
      ]),
    );

    for (const variant of ServerFrame.options) {
      const tag = variant.shape.frame_type.value;
      const provisionalKeys = Object.keys(variant.shape).toSorted();
      expect(
        frozenByTag.get(tag),
        `provisional ServerFrame has a tag "${tag}" absent from the frozen schema`,
      ).toBeDefined();
      expect(provisionalKeys, `field drift in ServerFrame:${tag}`).toEqual(
        frozenByTag.get(tag),
      );
    }

    // Both directions: every frozen variant is present in the provisional union.
    expect(
      ServerFrame.options.map((v) => v.shape.frame_type.value).toSorted(),
    ).toEqual([...frozenByTag.keys()].toSorted());

    // WireError is a frozen §6.4 $def hand-modeled here (reachable via
    // rpc_response.error) — pin its field-set both directions too (== ["code"]).
    expect(Object.keys(WireError.shape).toSorted()).toEqual(
      Object.keys(schema.$defs.WireError!.properties!).toSorted(),
    );
  });

  it("serverframe_variant_id_type_matches_frozen_schema", () => {
    // spec(§6.4) — the field-set snapshot above pins NAMES, not TYPES. The two
    // variants type `id` DIFFERENTLY in the frozen schema: `rpc_response.id` is a
    // REQUIRED uint64 INTEGER (the RPC correlation id), while `subscription_push.id`
    // is a nullable + optional (id ∉ required) string. A numeric correlation id
    // modeled as `z.string()` silently passes the field-name snapshot but would
    // REJECT a real RPC frame at `.parse()` — pin the divergence behaviorally so a
    // type regression on the discriminant-adjacent `id` fails loudly.
    expect(
      ServerFrame.safeParse({ frame_type: "rpc_response", id: 7 }).success,
      "rpc_response.id is a uint64 integer (frozen schema) — numeric id must parse",
    ).toBe(true);
    expect(
      ServerFrame.safeParse({ frame_type: "rpc_response", id: "7" }).success,
      "rpc_response.id is an integer, not a string — a string id must be rejected",
    ).toBe(false);

    // subscription_push.id: string | null | absent (id ∉ required) all parse.
    const sub = { frame_type: "subscription_push", projection: "Session", kind: "upsert" };
    expect(ServerFrame.safeParse({ ...sub, id: "s1" }).success).toBe(true);
    expect(ServerFrame.safeParse({ ...sub, id: null }).success).toBe(true);
    expect(ServerFrame.safeParse(sub).success).toBe(true);
  });
});
