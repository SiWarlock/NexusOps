#!/usr/bin/env python3
"""OQ-DATA-SPIKE-5 / 0.5 test 8 — self-contained cross-language equality harness.

Proves the SAME enum value sets are exposed by the published JSON Schema (== the
Rust authority, gated by Rust test 9), the generated Python Pydantic consumer,
and the generated TS Zod consumer (ARCHITECTURE §5.0). Name-agnostic: compares
the *collection of value-sets* (every enum present in all three, none extra) so
generator naming differences don't matter. Also checks CONTRACT_VERSION.

An enum value-set surfaces in TWO schema shapes, both treated identically here:
  - **flat** — `{"enum":[...]}` (schemars' default for a plain serde enum).
  - **const-union** — `{"oneOf":[{"const":"a"},{"const":"b"},...]}`, the shape
    schemars emits for a *per-variant-doc'd* enum (e.g. `MetricQuality`). A
    `oneOf`/`anyOf` whose members are OBJECTS is a TAGGED UNION (`ServerFrame`,
    `ActionError`) — NOT an enum, and is excluded (its inner `const` discriminants
    must not leak into the value-sets). See P4.0b-T / LESSON 29.

Depends only on the published schema + codegen tools (npx json-schema-to-zod,
uvx datamodel-code-generator) — NOT on ui/ or brain/ being built. The codegen
tool versions are PINNED below: the P4.0b-T Finding was precisely an unpinned
`datamodel-code-generator` auto-update that re-shaped `MetricQuality` and left
this gate silently RED for ~7 slices.
"""
import ast
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCHEMA = HERE.parent / "schema" / "nexusops-contract.schema.json"

# Pinned codegen tool versions (belt-and-suspenders against the auto-update class
# of regression — freeze hard, bump deliberately; LESSON 29). The version is
# verified live by `npm view json-schema-to-zod version` / `datamodel-codegen
# --version` during a deliberate bump.
DATAMODEL_CODEGEN_PIN = "datamodel-code-generator==0.63.0"
JSON_SCHEMA_TO_ZOD_PIN = "json-schema-to-zod@2.8.1"

_ZOD_FLAT_RE = re.compile(r"z\.enum\(\[(.*?)\]\)", re.DOTALL)
_ZOD_STR_RE = re.compile(r"""['"]([^'"]+)['"]""")
_ZOD_LITERAL_RE = re.compile(r"""z\.literal\(\s*['"]([^'"]+)['"]\s*\)""")


def fail(msg: str) -> "None":
    print(f"FAIL: {msg}")
    sys.exit(1)


def run_tool(cmd: list) -> "None":
    """Run a codegen tool, converting any failure into a structured FAIL line
    (never a raw traceback) so CI output stays readable."""
    try:
        subprocess.run(cmd, check=True, capture_output=True, text=True)
    except FileNotFoundError:
        fail(f"required tool not found: {cmd[0]} (needed for the 3-way verify)")
    except subprocess.CalledProcessError as e:
        fail(f"{cmd[0]} failed (exit {e.returncode}):\n{(e.stderr or e.stdout or '').strip()}")


# --- enum-value-set extraction (pure: return `(form, values)` records, no I/O,
#     no exit — `main()` does the non-empty validation so these stay testable) ---

def schema_enum_record(d):
    """Return `(form, values)` if `d` is an enum value-set def, else `None`.

    form is "flat" for `{"enum":[...]}` or "const_union" for an all-string-`const`
    `oneOf`/`anyOf`. A `oneOf`/`anyOf` with ANY non-string-`const` member (an
    object, a `$ref`, …) is a tagged union, not an enum → `None`."""
    if not isinstance(d, dict):
        return None
    if "enum" in d:
        return ("flat", list(d["enum"]))
    members = d.get("oneOf") or d.get("anyOf")
    if isinstance(members, list) and members:
        vals = []
        for m in members:
            if (
                isinstance(m, dict)
                and isinstance(m.get("const"), str)
                and m.get("type", "string") == "string"
                and "properties" not in m
            ):
                vals.append(m["const"])
            else:
                return None  # any object / $ref / non-string-const member ⇒ NOT an enum
        return ("const_union", vals)
    return None


def is_enum_like(d) -> bool:
    """True iff `d` is an enum value-set def (flat or const-union). The zod_input
    filter in `main()` shares this with `from_schema` via this one predicate so the
    two can't re-drift to different notions of "enum" — that divergence (one read
    `"enum" in d`, the other never fed MetricQuality) is exactly how the gate went
    dark (P4.0b-T / LESSON 29)."""
    return schema_enum_record(d) is not None


def from_schema(schema):
    """Enum value-set records from the published JSON Schema $defs."""
    defs = schema.get("$defs") or schema.get("definitions") or {}
    out = []
    for d in defs.values():
        rec = schema_enum_record(d)
        if rec is not None:
            out.append(rec)
    return out


def from_pydantic(models_src: str):
    """Enum value-set records from generated Pydantic source (AST-parsed — faithful,
    and no need to install pydantic to import the module). datamodel-codegen emits
    `class X(Enum): name = 'value'` for BOTH the flat and const-union schema forms,
    so they're indistinguishable here (form="enum") — which is why self-health
    form-coverage is asserted on the schema + zod extractors, not this one."""
    tree = ast.parse(models_src)
    out = []
    for node in tree.body:
        if not isinstance(node, ast.ClassDef):
            continue
        # base may be `Enum` (Name) or `enum.Enum` (Attribute); `class X(str, Enum)`
        # still has the Name/Attribute `Enum` among its bases.
        is_enum = any(
            (isinstance(b, ast.Name) and b.id == "Enum")
            or (isinstance(b, ast.Attribute) and b.attr == "Enum")
            for b in node.bases
        )
        if not is_enum:
            continue
        vals = [
            s.value.value
            for s in node.body
            if isinstance(s, ast.Assign)
            and isinstance(s.value, ast.Constant)
            and isinstance(s.value.value, str)
        ]
        if vals:
            out.append(("enum", vals))
    return out


def _balanced_array(text: str, open_idx: int):
    """`text[open_idx]` is `[`; return `(inner, close_idx)` for the balanced `[...]`,
    skipping string literals so brackets inside `.describe(...)` strings (or nested
    `z.union([...])`) don't miscount. Returns `(None, len(text))` if unbalanced."""
    depth = 0
    quote = None
    esc = False
    i = open_idx
    while i < len(text):
        c = text[i]
        if quote is not None:
            if esc:
                esc = False
            elif c == "\\":
                esc = True
            elif c == quote:
                quote = None
        elif c in "\"'":
            quote = c
        elif c == "[":
            depth += 1
        elif c == "]":
            depth -= 1
            if depth == 0:
                return text[open_idx + 1:i], i
        i += 1
    return None, len(text)


def from_zod(zod_ts: str):
    """Enum value-set records from generated Zod source. Two forms:
      - flat → `z.enum(["a","b",...])`.
      - const-union → `z.any().superRefine(...)` wrapping `const schemas = [
        z.literal("a"), z.literal("b"), ... ]` (all `z.literal`). An object-union
        (`ServerFrame`/`ActionError`) uses the SAME wrapper but its `schemas[]`
        members are `z.object(...)` → excluded (inner `z.literal` discriminants
        must not leak)."""
    records = []
    # const-union superRefine arrays (object-union arrays excluded)
    marker = "const schemas = "
    idx = 0
    while True:
        m = zod_ts.find(marker, idx)
        if m == -1:
            break
        # search AFTER the marker text so `open_idx` is the array's own `[` (the
        # marker holds no `[`; this keeps `_balanced_array`'s `text[open_idx]=='['`
        # contract honest regardless of what precedes the array).
        open_idx = zod_ts.find("[", m + len(marker))
        if open_idx == -1:
            break
        content, close_idx = _balanced_array(zod_ts, open_idx)
        idx = close_idx + 1
        # substring test (not an AST parse) — safe for machine-generated output:
        # `.describe(...)` strings carry prose, not literal `z.object(` fragments.
        if content is None or "z.object(" in content:
            continue  # malformed, or a tagged object-union → not an enum value-set
        lits = _ZOD_LITERAL_RE.findall(content)
        if lits:
            records.append(("const_union", lits))
    # flat z.enum arrays. The current pinned generator never emits `z.enum` INSIDE a
    # `const schemas` array (it uses `z.literal`/`z.object` there) and never nests
    # `z.enum(...)` inside another `z.enum(...)` — so the non-greedy `_ZOD_FLAT_RE`
    # on the full text is sufficient; a JSON_SCHEMA_TO_ZOD_PIN bump must re-verify
    # both assumptions (the self-health gate would catch a misclassification).
    for arr in _ZOD_FLAT_RE.findall(zod_ts):
        vals = _ZOD_STR_RE.findall(arr)
        if vals:
            records.append(("flat", vals))
    return records


# --- record helpers ---

def value_sets(records):
    """The value-sets as a frozenset-of-frozensets (order/name/form agnostic) — the
    unit the cross-language equality check compares."""
    return frozenset(frozenset(vals) for _form, vals in records)


def forms_seen(records):
    return {form for form, _vals in records}


def form_coverage_ok(records) -> bool:
    """The dark-gate detector (LESSON 29): a non-degenerate extraction surfaced BOTH
    a flat enum AND a const-union enum. A generator change that hides const-unions
    (the P4.0b-T Finding) drops one arm → False → the gate refuses to pass green.

    PRECONDITION: the live schema genuinely has both forms (today: 35 flat + 1
    const-union). If a future schema ever loses ALL of one form (every enum becomes
    const-union, or MetricQuality is removed leaving no const-union), this returns
    False on a correct run — that's the intended human prompt to re-check the gate's
    premise, not a tooling regression; relax this predicate then (accepted tradeoff,
    Step-2.5 APPROVED)."""
    f = forms_seen(records)
    return "flat" in f and "const_union" in f


def main() -> "None":
    if not SCHEMA.exists():
        fail(f"schema missing: {SCHEMA} (run `cargo run --bin emit_schema`)")
    schema = json.loads(SCHEMA.read_text())

    version = schema.get("x-contract-version")
    if not version:
        fail("schema missing x-contract-version")

    schema_records = from_schema(schema)
    if not schema_records:
        fail("no enum value-sets (flat or const-union $defs) found in schema")
    schema_sets = value_sets(schema_records)

    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)

        # --- generate + reflect Pydantic ---
        models_py = tmp / "models.py"
        run_tool(
            ["uvx", "--from", DATAMODEL_CODEGEN_PIN, "datamodel-codegen",
             "--input", str(SCHEMA), "--input-file-type", "jsonschema",
             "--output", str(models_py), "--output-model-type", "pydantic_v2.BaseModel"]
        )
        pydantic_records = from_pydantic(models_py.read_text())
        if not pydantic_records:
            fail("no Enum classes found in generated Pydantic module")
        pydantic_sets = value_sets(pydantic_records)

        # --- generate + parse Zod ---
        # json-schema-to-zod does NOT resolve internal $ref → emits z.any() for the
        # root's $ref properties. Feed it the enum $defs inlined into a flat object;
        # this is lossless for the value sets (the same published contract values).
        # The filter is `is_enum_like` (NOT a bare `"enum" in d`) so const-union
        # enums (MetricQuality) ARE fed to the generator — otherwise they never
        # reach Zod and the count silently undershoots (the P4.0b-T dark gate).
        defs = schema.get("$defs") or schema.get("definitions") or {}
        zod_input = tmp / "zod_input.json"
        zod_input.write_text(json.dumps({
            "type": "object",
            "properties": {n: d for n, d in defs.items() if is_enum_like(d)},
        }))
        zod_ts = tmp / "zod.ts"
        run_tool(
            ["npx", "-y", JSON_SCHEMA_TO_ZOD_PIN, "--input", str(zod_input), "--output", str(zod_ts)]
        )
        zod_records = from_zod(zod_ts.read_text())
        if not zod_records:
            fail("no enum value-sets (z.enum or const-union superRefine) found in generated Zod")
        zod_sets = value_sets(zod_records)

    print(f"schema enums:   {len(schema_sets)}")
    print(f"pydantic enums: {len(pydantic_sets)}")
    print(f"zod enums:      {len(zod_sets)}")

    # --- self-health (LESSON 29): a green run must prove BOTH extraction arms fired ---
    # datamodel-codegen flattens both forms to `class X(Enum)`, so pydantic can't
    # express the distinction — form-coverage is asserted on schema + zod (the two
    # extractors that DID go dark). With both proving a const-union surfaced, the
    # full equality below transitively proves the const-union enum round-trips all
    # three; no brittle hard-coded value-set needed.
    for label, recs in (("schema", schema_records), ("zod", zod_records)):
        if not form_coverage_ok(recs):
            fail(
                f"§5.0 gate self-health FAILED: the {label} extractor is degenerate — "
                f"forms seen = {sorted(forms_seen(recs))}, expected BOTH 'flat' and 'const_union'. "
                f"A generator change that hides const-union enums (the P4.0b-T dark-gate class) "
                f"lands here: the gate refuses to pass green without exercising every arm. See LESSON 29."
            )

    if schema_sets != pydantic_sets:
        fail(f"schema vs pydantic value-set mismatch:\n  only-schema={schema_sets - pydantic_sets}\n  only-pydantic={pydantic_sets - schema_sets}")
    if schema_sets != zod_sets:
        fail(f"schema vs zod value-set mismatch:\n  only-schema={schema_sets - zod_sets}\n  only-zod={zod_sets - schema_sets}")

    print(f"PASS: Rust(schema) == Pydantic == Zod — {len(schema_sets)} enums agree; CONTRACT_VERSION={version}")


if __name__ == "__main__":
    main()
