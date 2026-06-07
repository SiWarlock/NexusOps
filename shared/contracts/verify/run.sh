#!/usr/bin/env bash
# 0.5 test 8 — cross-language equality (self-contained). Regenerates the schema
# from the Rust authority, then asserts Rust(schema) == generated Pydantic == Zod.
# Needs: cargo (toolchain), python3, uvx (datamodel-code-generator), npx
# (json-schema-to-zod). No ui/ or brain/ build required.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

echo "[1/2] regenerating schema from the Rust authority..."
cargo run --quiet --manifest-path "$ROOT/shared/Cargo.toml" --bin emit_schema >/dev/null

echo "[2/2] generating Pydantic + Zod, comparing value sets..."
python3 "$ROOT/shared/contracts/verify/verify.py"
