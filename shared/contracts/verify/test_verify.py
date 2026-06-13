#!/usr/bin/env python3
"""Offline, fixture-based unit tests for the §5.0 3-way verify extractors.

The deterministic RED-first surface for P4.0b-T: pins the extractor contract
(flat enum + const-union enum recognition; tagged object-union exclusion) using
inline fixtures — no cargo / uvx / npx / network needed. Plain `assert`, no
pytest dep. Run: `python3 test_verify.py` (exit 0 = all pass, 1 = any fail).

`run.sh` runs this BEFORE the network-dependent codegen step (fail-fast), so a
broken extractor is caught wherever the verify runs, not only nightly.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from verify import (  # noqa: E402  (path-insert must precede the import)
    form_coverage_ok,
    forms_seen,
    from_pydantic,
    from_schema,
    from_zod,
    value_sets,
)


def test_from_schema_extracts_flat_enum():  # spec(§5.0) — regression guard for the 35 already-paired enums
    recs = from_schema({"$defs": {"Status": {"enum": ["a", "b"]}}})
    assert value_sets(recs) == {frozenset({"a", "b"})}
    assert forms_seen(recs) == {"flat"}


def test_from_schema_extracts_const_union():  # spec(§5.0) — the root-cause gap (MetricQuality), schema side
    mq = {"oneOf": [
        {"const": "exact", "type": "string", "description": "authoritative"},
        {"const": "estimated", "type": "string", "description": "derived"},
        {"const": "unavailable", "type": "string", "description": "no metric"},
    ]}
    recs = from_schema({"$defs": {"MetricQuality": mq}})
    assert value_sets(recs) == {frozenset({"exact", "estimated", "unavailable"})}
    assert forms_seen(recs) == {"const_union"}


def test_from_schema_excludes_object_union():  # spec(§5.0) — ServerFrame/ActionError are NOT enums
    server_frame = {"oneOf": [
        {"type": "object", "properties": {"frame_type": {"const": "rpc_response", "type": "string"},
                                          "id": {"type": "integer"}}, "required": ["frame_type"]},
        {"type": "object", "properties": {"frame_type": {"const": "subscription_push", "type": "string"}}},
    ]}
    recs = from_schema({"$defs": {"ServerFrame": server_frame}})
    # `recs == []` is the complete guard: nothing extracted ⇒ the inner `frame_type`
    # discriminant const ("rpc_response") cannot leak into any value-set.
    assert recs == [], f"object-union must not be extracted (would leak discriminants), got {recs}"


def test_from_zod_extracts_flat_enum():  # spec(§5.0) — regression guard for z.enum([...])
    recs = from_zod('export default z.object({ "S": z.enum(["a","b"]).optional() })')
    assert value_sets(recs) == {frozenset({"a", "b"})}
    assert forms_seen(recs) == {"flat"}


def test_from_zod_extracts_const_union_superrefine():  # spec(§5.0) — the root-cause gap, Zod side
    zod = (
        'z.any().superRefine((x, ctx) => {\n'
        '  const schemas = [z.literal("exact").describe("authoritative"), '
        'z.literal("estimated").describe("derived"), '
        'z.literal("unavailable").describe("no metric")];\n'
        '  const passed = schemas.length;\n})'
    )
    recs = from_zod(zod)
    assert value_sets(recs) == {frozenset({"exact", "estimated", "unavailable"})}
    assert forms_seen(recs) == {"const_union"}


def test_from_zod_excludes_object_union_superrefine():  # spec(§5.0) — the dangerous over-match to guard
    # ServerFrame-shaped: schemas[] members are z.object(...) carrying inner z.literal discriminants.
    zod = (
        'z.any().superRefine((x, ctx) => {\n'
        '  const schemas = [z.object({ "frame_type": z.literal("rpc_response"), '
        '"error": z.union([z.any(), z.null()]).optional() }).strict(), '
        'z.object({ "frame_type": z.literal("subscription_push"), "kind": z.any() }).strict()];\n'
        '  const passed = schemas.length;\n})'
    )
    recs = from_zod(zod)
    # `recs == []` is the complete guard: nothing extracted ⇒ the inner
    # `z.literal("rpc_response")` discriminants cannot leak into any value-set.
    assert recs == [], f"object-union superRefine must not be extracted (would leak discriminants), got {recs}"


def test_from_pydantic_extracts_enum_class():  # spec(§5.0) — Pydantic extraction unchanged (the already-correct 36)
    src = (
        "class MetricQuality(Enum):\n"
        "    exact = 'exact'\n"
        "    estimated = 'estimated'\n"
        "    unavailable = 'unavailable'\n"
        "\n"
        "class Envelope(BaseModel):\n"
        "    seq: int\n"
    )
    recs = from_pydantic(src)
    assert value_sets(recs) == {frozenset({"exact", "estimated", "unavailable"})}


def test_self_health_detects_degenerate_run():  # spec(§5.0) / LESSON 29 — the dark-gate detector fires
    only_flat = [("flat", ["a", "b"]), ("flat", ["c", "d"])]
    only_const = [("const_union", ["exact", "estimated"])]
    both = [("flat", ["a", "b"]), ("const_union", ["exact", "estimated"])]
    assert form_coverage_ok(only_flat) is False  # a generator change hiding const-unions ⇒ loud FAIL
    assert form_coverage_ok(only_const) is False  # symmetric: losing the flat arm also fails
    assert form_coverage_ok(both) is True


def _main() -> "None":
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  ok   {t.__name__}")
        except Exception as e:  # noqa: BLE001 — a test runner surfaces any failure, not just AssertionError
            failed += 1
            print(f"  FAIL {t.__name__}: {type(e).__name__}: {e}")
    print(f"\n{len(tests) - failed}/{len(tests)} passed")
    if failed:
        sys.exit(1)


if __name__ == "__main__":
    _main()
