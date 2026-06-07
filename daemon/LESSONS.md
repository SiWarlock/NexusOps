# LESSONS.md — NexusOps (the Rust daemon (trust core))

> Full prose for every lesson logged during work in `daemon/`. The compact index lives in `daemon/CLAUDE.md` "Lessons logged" table.
>
> **Lesson numbers are stable IDs.** New lessons get the next sequential number. Numbers may be referenced from code comments, commit messages, and cross-references between lessons. **Don't reorder; don't reuse a deleted number's slot.**
>
> **Lessons start at §1.** Each code area has its own lesson sequence — lessons don't carry across code areas.

---

## Lesson format

```markdown
## <a id="N"></a>N. <Short topic> — <one-line rule>

**Date:** YYYY-MM-DD.
**Source slice:** <slice-id or commit hash>.

<2-5 paragraphs explaining: what was discovered, why it matters, how to
apply the rule, what edge cases are still open. Cite file:line references
where applicable.>

**Rule:** <one-sentence summary, same as the heading subtitle>.
```

---

## <a id="1"></a>1. Broken cargo/rustc proxies — `rustup default stable` won't fix them; repoint the shims

**Date:** 2026-06-07.
**Source slice:** 0.4 (`OQ-DATA-SPIKE-3` env-finding; cleared under ③).

During Phase-0 the daemon-track build was blocked: `~/.cargo/bin/{cargo,rustc}` (and 11 other proxies, 13 total) were **broken dangling symlinks** pointing at a non-existent `/Users/nozzins` path — the artifact of a moved home directory. The real stable toolchain (1.93.0) was intact at `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin`.

The non-obvious part: **`rustup default stable` did NOT fix it.** `rustup` only (re)creates *missing* proxies; it leaves *existing-but-broken* symlinks untouched. The obvious first move silently no-ops. The working fix was to repoint every broken proxy at the local `rustup` binary directly — `ln -sf rustup ~/.cargo/bin/<proxy>` for each of the 13 — after which the plain shims resolve (no PATH workaround needed). Verify with `cargo --version && rustc --version` (expect 1.93.0), `cargo clippy`, and a real `cargo build`.

Edge cases still open: if the home dir moves again the same breakage recurs; a `rustup self uninstall` + reinstall would also fix it but is heavier. Prefer the targeted shim-repoint.

**Rule:** When `~/.cargo/bin` shims are *broken* (dangling symlinks, e.g. after a home-dir move), `rustup default stable` does **not** repair them (rustup only recreates *missing* proxies) — repoint each broken proxy to the local `rustup` binary (`ln -sf rustup ~/.cargo/bin/<proxy>`), then verify with the plain shims.

## <a id="2"></a>2. The wire value is the contract; the SoT propagation pattern is §5.0 — follow it for every contract addition

**Date:** 2026-06-07.
**Source slice:** 0.5 (shared contract freeze, `OQ-DATA-SPIKE-5`).

The 0.5 freeze settled two conventions every future contract surface (event-type registry, GatewayPort schema, action-type catalog, any new enum/ID) must follow.

**(a) The serialized wire/`TEXT` value is the contract — not the in-language identifier.** Enums serialize to exact snake_case strings via `#[serde(rename_all = "snake_case")]`; each language uses its idiomatic identifier (Rust `PascalCase` variants, TS string-literal unions, Python `Enum`) but the *string on the wire / in the `TEXT status` column* is what's frozen. A round-trip test (`test_every_state_machine_value_present_and_serializes`) pins this — never assume the identifier and the wire value match. Closed-enum / reject-unknown holds end-to-end (serde closed enums → JSON-Schema `enum` → `z.enum` → Pydantic): unknown values are rejected at every boundary, which is the fail-closed posture (§15), not optional.

**(b) The source-of-truth propagation is fixed by `ARCHITECTURE.md §5.0` (Option A):** Rust `shared` crate = native authority (newtypes for IDs, serde-closed enums) → `schemars` emits a **first-class, versioned, diff-gated** JSON Schema artifact (`shared/contracts/schema/`) → TS Zod + Python Pydantic are **generated** from that artifact → a self-contained 3-way value-set equality harness proves they agree. Do **not** hand-author a consumer's types or invert the authority (an external IDL/codegen-into-the-trust-core was rejected — it generates bare types in the safety-critical module and fights the newtype posture). Every new contract addition extends the Rust authority and regenerates the artifact; the diff-gate catches drift.

**Rule:** Freeze the *wire value* (snake_case `TEXT`), not the identifier — pin it with a round-trip test; and author every contract in Rust (`shared/`) per §5.0, regenerating the published schema + consumers (never hand-write a consumer or invert the authority).
