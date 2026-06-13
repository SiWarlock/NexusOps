#!/usr/bin/env bash
# 0.5 test 8 — cross-language equality (self-contained). Regenerates the schema
# from the Rust authority, then asserts Rust(schema) == generated Pydantic == Zod.
# Needs: cargo (toolchain), python3, uvx (datamodel-code-generator), npx
# (json-schema-to-zod). No ui/ or brain/ build required.
#
# The codegen tool versions are PINNED in verify.py (DATAMODEL_CODEGEN_PIN /
# JSON_SCHEMA_TO_ZOD_PIN) — the P4.0b-T Finding was an unpinned auto-update that
# left this gate silently RED for ~7 slices (LESSON 29).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

echo "[1/3] offline extractor self-tests (no network — fail fast)..."
python3 "$ROOT/shared/contracts/verify/test_verify.py"

echo "[2/3] regenerating schema from the Rust authority..."
cargo run --quiet --manifest-path "$ROOT/shared/Cargo.toml" --bin emit_schema >/dev/null

echo "[3/3] generating Pydantic + Zod, comparing value sets..."
python3 "$ROOT/shared/contracts/verify/verify.py"
