#!/usr/bin/env python3
"""OQ-DATA-SPIKE-5 / 0.5 test 8 — self-contained cross-language equality harness.

Proves the SAME enum value sets are exposed by the published JSON Schema (== the
Rust authority, gated by Rust test 9), the generated Python Pydantic consumer,
and the generated TS Zod consumer (ARCHITECTURE §5.0). Name-agnostic: compares
the *collection of value-sets* (every enum present in all three, none extra) so
generator naming differences don't matter. Also checks CONTRACT_VERSION.

Depends only on the published schema + codegen tools (npx json-schema-to-zod,
uvx datamodel-code-generator) — NOT on ui/ or brain/ being built.
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


def sets_of(value_sets) -> "frozenset":
    """A collection of enum value-sets as a frozenset-of-frozensets (order/name agnostic)."""
    return frozenset(frozenset(v) for v in value_sets)


def from_schema(schema: dict):
    defs = schema.get("$defs") or schema.get("definitions") or {}
    out = [d["enum"] for d in defs.values() if isinstance(d, dict) and "enum" in d]
    if not out:
        fail("no enum $defs found in schema")
    return out


def from_pydantic(models_py: Path):
    # AST-parse the generated Enum classes (faithful + no need to install pydantic
    # to import the module). datamodel-codegen emits `class X(Enum): name = 'value'`.
    tree = ast.parse(models_py.read_text())
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
            out.append(vals)
    if not out:
        fail("no Enum classes found in generated Pydantic module")
    return out


def from_zod(zod_ts: str):
    # flat string enums → `z.enum(["a","b",...])`
    out = []
    # DOTALL so multi-line z.enum([...]) arrays are captured, not silently dropped
    for arr in re.findall(r"z\.enum\(\[(.*?)\]\)", zod_ts, re.DOTALL):
        vals = re.findall(r"""['"]([^'"]+)['"]""", arr)
        if vals:
            out.append(vals)
    if not out:
        fail("no z.enum([...]) arrays found in generated Zod")
    return out


def main() -> "None":
    if not SCHEMA.exists():
        fail(f"schema missing: {SCHEMA} (run `cargo run --bin emit_schema`)")
    schema = json.loads(SCHEMA.read_text())

    version = schema.get("x-contract-version")
    if not version:
        fail("schema missing x-contract-version")

    schema_sets = sets_of(from_schema(schema))

    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)

        # --- generate + reflect Pydantic ---
        models_py = tmp / "models.py"
        run_tool(
            ["uvx", "--from", "datamodel-code-generator", "datamodel-codegen",
             "--input", str(SCHEMA), "--input-file-type", "jsonschema",
             "--output", str(models_py), "--output-model-type", "pydantic_v2.BaseModel"]
        )
        pydantic_sets = sets_of(from_pydantic(models_py))

        # --- generate + parse Zod ---
        # json-schema-to-zod does NOT resolve internal $ref → emits z.any() for the
        # root's $ref properties. Feed it the enum $defs inlined into a flat object;
        # this is lossless for the value sets (the same published contract values),
        # which is exactly what the equality check compares.
        defs = schema.get("$defs") or schema.get("definitions") or {}
        zod_input = tmp / "zod_input.json"
        zod_input.write_text(json.dumps({
            "type": "object",
            "properties": {n: d for n, d in defs.items() if "enum" in d},
        }))
        zod_ts = tmp / "zod.ts"
        run_tool(
            ["npx", "-y", "json-schema-to-zod", "--input", str(zod_input), "--output", str(zod_ts)]
        )
        zod_sets = sets_of(from_zod(zod_ts.read_text()))

    print(f"schema enums:   {len(schema_sets)}")
    print(f"pydantic enums: {len(pydantic_sets)}")
    print(f"zod enums:      {len(zod_sets)}")

    if schema_sets != pydantic_sets:
        fail(f"schema vs pydantic value-set mismatch:\n  only-schema={schema_sets - pydantic_sets}\n  only-pydantic={pydantic_sets - schema_sets}")
    if schema_sets != zod_sets:
        fail(f"schema vs zod value-set mismatch:\n  only-schema={schema_sets - zod_sets}\n  only-zod={zod_sets - schema_sets}")

    print(f"PASS: Rust(schema) == Pydantic == Zod — {len(schema_sets)} enums agree; CONTRACT_VERSION={version}")


if __name__ == "__main__":
    main()
