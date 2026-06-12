# Shared contract authority (`shared/`)

## Executive summary

`shared/` is a small Rust crate (`nexusops-shared`) that is the single source of truth for every value the daemon, the desktop UI, and the (sibling) Project Brain must agree on: status state machines, ID formats, the event envelope, concrete event payloads, and the IPC wire protocol. Instead of each language defining its own copy of these vocabularies, the Rust types are the native authority (ARCHITECTURE.md §5.0 "Option A", `ARCHITECTURE.md:129-137`), and a build tool emits one versioned JSON Schema file that is checked into the repo. TypeScript (Zod) and Python (Pydantic) consumers are *generated* from that schema, and tests fail loudly if anything drifts. Everything is "reject-unknown": an unrecognized status string, ID prefix, or extra JSON field is an error, never silently accepted — the contract-level half of the project's fail-closed security posture.

## Responsibilities

- **Owns** the canonical wire vocabulary: 10 audit actors, 9 frozen status machines (+1 deliberately held), 22 shared ID kinds + the prefixed-ULID format, 4 desktop-addendum objects, the §7.1 event envelope + its 4 enums, 5 concrete event payloads, and the full §6.1/§6.4 IPC GatewayPort surface.
- **Owns** the published interchange artifact: `shared/contracts/schema/nexusops-contract.schema.json`, stamped with `CONTRACT_VERSION` and byte-equality-gated by a test.
- **Owns** the cross-language verification harness (`shared/contracts/verify/`) that proves Rust == Pydantic == Zod value sets.
- **Is NOT** runtime logic: no DB access, no IO (except the `emit_schema` build tool), no state machines *executing* transitions — it defines value sets, not behavior (`shared/src/objects.rs:7` — "the freeze pins each object's type + identity, not its behavior").
- **Is NOT** the home of the Action Gateway's `ActionTypeCatalog` yet — §5.0 names it as contract surface (`ARCHITECTURE.md:130`) but it has not accreted into the crate (Phase 2 work).

## Key components

| Component | What it does | Where |
|-----------|--------------|-------|
| `CONTRACT_VERSION` | The frozen-contract version (`"0.14.0"`); stamped into the schema; bump history in doc comments | `shared/src/lib.rs:32` |
| `EXECUTION_PROFILE_STATUS_HELD` | Marker const making the *absence* of the 10th status machine deliberate, not forgotten | `shared/src/lib.rs:40-42` |
| `status_machine!` + 9 enums | Frozen snake_case status machines, each with `ALL` + `is_terminal()` | `shared/src/status.rs:15-131` |
| `Session` status (17 states) | Driver-agnostic session lifecycle the harness adapters map into | `shared/src/status.rs:42-52` |
| `ActionRequest` status (15 states) | The Gateway execution-lifecycle axis (split from the 10-state `Approval` axis) | `shared/src/status.rs:114-122` |
| `IdKind` (22 kinds) + `prefix()`/`from_prefix()` | Total, unique prefix→kind map; 16 platform-minted + 6 external native-valued kinds | `shared/src/ids.rs:15-103` |
| `minted_id!` → 16 newtypes | `<prefix><ULID>` newtypes with `new()`/`parse()` (fail-closed: wrong prefix or bad ULID → `IdError`) | `shared/src/ids.rs:127-203` |
| `SYSTEM_WORKSPACE_ID` | All-zero-ULID `ws_` sentinel for workspace-less System-actor bootstrap events (a reserved value, not a nullable column) | `shared/src/ids.rs:205-219` |
| `ActorType` (10 actors) | The canonical `Event.actor_type` audit enum (R-2) | `shared/src/actor.rs:15-26` |
| `DesktopObjectKind` (4) + `DeviceId`/`LocalRunnerId` | §5.3 desktop addendum; only Device/LocalRunner get newtypes; `rc_`/`eprj_` are prefix-only | `shared/src/objects.rs:15-122` |
| `EventEnvelope` + 4 enums | The §7.1 envelope (16 required + 12 optional fields, `deny_unknown_fields`); `SourceType`(15)/`Sensitivity`(5)/`Visibility`(4)/`RedactionStatus`(2) | `shared/src/event_envelope.rs:29-110` |
| Event payloads (5) | `SessionStarted`, `DeviceRegistered`, `LocalRunnerRegistered`, `AuditIntegrityViolation`, `SensitiveOutputRedacted` | `shared/src/events.rs:24-91` |
| IPC wire contract | Handshake (`HelloFrame`/`HelloAck`/`VersionSkewError`), `IpcErrorCode`(7), `ProjectionName`(10), RPC envelopes, subscribe/`ServerFrame` mux | `shared/src/ipc.rs:16-224` |
| `ContractBundle` + `emit_schema_json()` | One `schema_for!` bundling every contract type; deterministic, version-stamped output | `shared/src/schema.rs:32-105` |
| `emit_schema` binary | Regenerates the checked-in schema artifact | `shared/src/bin/emit_schema.rs:5-14` |
| Schema diff gate (test 9) | Byte-equality: generated schema must == checked-in file | `shared/tests/contract.rs:460-475` |
| 3-way verify harness | Regenerates schema, generates Pydantic (uvx) + Zod (npx), asserts identical enum value sets | `shared/contracts/verify/run.sh:9-13`, `shared/contracts/verify/verify.py:142-147` |

## Interfaces & contracts

**To Rust consumers (daemon):** plain `use nexusops_shared::…` — the daemon imports the types natively (path dependency, `daemon/Cargo.toml:10`). Examples: the event writer uses `EventId` + `ProjectionDelta` (`daemon/src/runtime/writer.rs:16-17`), bootstrap uses `DeviceRegistered`/`LocalRunnerRegistered`/`CONTRACT_VERSION` (`daemon/src/bootstrap.rs:17-23`), every projector folds `EventEnvelope` (`daemon/src/projections/mod.rs:24`).

**To non-Rust consumers (UI, Brain):** the checked-in JSON Schema `shared/contracts/schema/nexusops-contract.schema.json` (1,064 lines; all value sets in `$defs`; `x-contract-version` stamped — line 1064 currently `"0.14.0"`). Consumers generate validators from it; they never import Rust.

**Key input→output contracts:**

- `IdKind::prefix() -> Option<&'static str>` — `Some` for the 16 minted kinds, `None` for the 6 external kinds (`shared/src/ids.rs:69-95`); `from_prefix` is the exact inverse (`shared/src/ids.rs:98-103`).
- `<MintedId>::parse(&str) -> Result<Self, IdError>` — rejects wrong prefix and malformed ULID body (`shared/src/ids.rs:151-155`).
- `protocol_in_range(u32) -> bool` — the daemon-side handshake skew check; `PROTOCOL_VERSION = 1`, supported range `[1,1]` (`shared/src/ipc.rs:16-26`). **Two version axes:** `PROTOCOL_VERSION` (wire handshake) and `CONTRACT_VERSION` (schema/codegen) move independently (`shared/src/ipc.rs:7-9`).
- `emit_schema_json() -> String` — deterministic; same source → byte-identical output including trailing newline (`shared/src/schema.rs:87-105`).
- Every struct crossing the wire carries `#[serde(deny_unknown_fields)]`; every enum is serde-closed — unknown values fail deserialization (pinned by `shared/tests/contract.rs:444-456` and `shared/tests/envelope.rs:85-91`).

## Data & state

The crate is stateless — its "state" is the frozen value sets themselves plus one checked-in artifact:

- **`shared/contracts/schema/nexusops-contract.schema.json`** — the published contract (title `nexusops-contract`, `x-contract-version: 0.14.0`), regenerated by `cargo run --bin emit_schema` after any contract change.
- **Status machines** (`shared/src/status.rs`): Session(17) · Task(17) · WorktreeGit(6, no terminals — the overlay axis carries `deleted`) · WorktreeOverlay(6) · PullRequest(11) · WorkflowInstance(12) · ProjectBrain(10) · Approval(10) · ActionRequest(15) · AgentTeam(9). Wire form is exact snake_case; stored daemon-side as `TEXT status`.
- **`EventEnvelope`** (`shared/src/event_envelope.rs:72-110`): required fields are non-`Option` (`event_id`, `seq` — the canonical total order, not `occurred_at` — `event_type`, `event_version`, timestamps, `workspace_id`, actor/source/correlation, `sensitivity`, `redaction_status`, `payload_json`, `schema_version`); optionals include typed `project_id`/`session_id`/`agent_team_id`/`causation_id`/`action_request_id`, `object_refs[]`, and the **reserved** R-10 fields `payload_hash`/`previous_event_hash` (`shared/src/event_envelope.rs:108-109`).
- **Identity placement convention:** `SessionStarted` identity lives on envelope typed columns (`shared/src/events.rs:16-21`); `DeviceRegistered`/`LocalRunnerRegistered` identities live **in the payload** because `dev_`/`lr_` are not envelope columns — the `object_refs` projector sources edges from the payload (`shared/src/events.rs:34-51`).
- **Content-free safety payloads:** `AuditIntegrityViolation {seq, reason}` and `SensitiveOutputRedacted {original_event_type, reason, detector}` deliberately carry structural metadata only — never the corrupt row or any byte of a diverted secret (`shared/src/events.rs:53-91`).

## Dependencies

- **Depends on:** nothing project-internal. External crates only: `serde`/`serde_json`, `schemars` 1.2, `ulid` 1.2 (`shared/Cargo.toml:12-16`). This is the innermost layer.
- **Used by:**
  - **daemon** — native import (`daemon/Cargo.toml:10`): event store writer, projections, bootstrap, IPC listener all consume these types directly (see [02-event-store.md](02-event-store.md), [04-projections.md](04-projections.md), [06-ipc.md](06-ipc.md), [07-daemon-runtime.md](07-daemon-runtime.md)).
  - **ui** — generated consumer: `ui/src/contracts/generated.ts` is produced from the schema artifact and pins its own `CONTRACT_VERSION` (`ui/src/contracts/generated.ts:10`); a UI-side test asserts it equals the schema's `x-contract-version` (`ui/src/contracts/generated.test.ts:57-58`). See [08-ui.md](08-ui.md).
  - **Project Brain (sibling repo)** — generates Pydantic from the same schema; the 3-way verify harness proves the generation path works without `brain/` being present (`shared/contracts/verify/verify.py:9-11`).

## How it works (flow)

```
Rust types (authority)          published artifact              generated consumers
shared/src/*.rs  ──schemars──▶  nexusops-contract.schema.json ──▶ TS Zod (ui)
      │                                │            └────────────▶ Py Pydantic (brain)
      │ test 9: byte-equality gate     │ x-contract-version
      └── cargo test ◀─────────────────┘
```

1. **Author/extend a type** in `shared/src/` — e.g. a new event payload added to `events.rs` with `deny_unknown_fields`, plus an `EVENT_TYPE` const (`shared/src/events.rs:64-69`).
2. **Register it in the bundle:** add a field to `ContractBundle` so `schema_for!` captures it under `$defs` (`shared/src/schema.rs:32-83`). Bump `CONTRACT_VERSION` (`shared/src/lib.rs:32` — the doc comment is a running changelog, 0.9.0 → 0.14.0).
3. **Regenerate:** `cargo run --bin emit_schema` writes the artifact; `emit_schema_json()` injects `title` + `x-contract-version` and guarantees a trailing newline so the diff gate is byte-stable (`shared/src/schema.rs:90-104`, `shared/src/bin/emit_schema.rs:8-13`).
4. **Gate:** test 9 reads the checked-in file and asserts exact byte equality with a fresh emission — forgetting step 3 fails CI with "schema drift — regenerate…" (`shared/tests/contract.rs:461-475`).
5. **Cross-language proof:** `shared/contracts/verify/run.sh` regenerates the schema, then `verify.py` generates a Pydantic module (uvx datamodel-code-generator) and a Zod module (npx json-schema-to-zod) into a tempdir and compares the three *collections of enum value-sets* as frozenset-of-frozensets — name-agnostic, so generator naming quirks don't matter (`shared/contracts/verify/verify.py:41-43,107-145`).
6. **Consumers regenerate** on their own cadence; their pinned-version tests catch staleness (see Gotchas).

The 26 tests across `shared/tests/contract.rs` (19) and `shared/tests/envelope.rs` (7) pin: every value set verbatim, terminal-state sets, the 22-ID prefix map (totality + uniqueness, `shared/tests/contract.rs:262-316`), mint/parse round-trips, reject-unknown, the held-not-frozen ExecutionProfile marker (`shared/tests/contract.rs:432-439`), the system-workspace sentinel (`shared/tests/contract.rs:579-595`), and the exact `CONTRACT_VERSION` string (`shared/tests/contract.rs:715-718`).

## Design decisions & rationale

- **Option A — Rust as native authority** (§5.0, `ARCHITECTURE.md:129-137`, `[LOCKED]`): no type in the trust core is generated; an external IDL generating *into* the daemon was rejected as inverting authority and producing bare types in the safety-critical module. Re-validated unchanged at the Phase-0-exit `/arch-finalize` re-run (`ARCHITECTURE.md:137`).
- **Schema as a first-class published artifact**, not a build byproduct — checked in, versioned, CI-diff-gated; the same drift-gate pattern as the Codex app-server schema (§5.0 point 2).
- **Reject-unknown end-to-end** (§5.0 point 4, §15): serde-closed enums → JSON-Schema `enum` → `z.enum` → Pydantic. This is why `DesktopObjectKind` uses plain `//` comments instead of `///` doc comments on variants — doc comments would make schemars emit a `oneOf`-of-`const` instead of a flat string `enum`, breaking uniform downstream codegen (`shared/src/objects.rs:21-22`).
- **Desktop IDs outside the frozen 22:** `DeviceId`/`LocalRunnerId` key off `DesktopObjectKind`, *not* new `IdKind` variants, so the LOCKED 22-ID set stays exactly 22 (`shared/src/objects.rs:58-61`; guarded by `shared/tests/contract.rs:561-576`).
- **`ProjectionName` is PascalCase on the wire, deliberately** — no `rename_all`, matching the UI's pinned `get_projection("Session")` literals and the §7 registry labels; an explicit "do not fix to snake_case" comment defends it (`shared/src/ipc.rs:103-108`). `UsageLedger` is canonical (the UI's provisional `Usage` reconciles to it).
- **`Approval` vs `ActionRequest` are two axes** (R-5): the human/policy decision lifecycle and the execution lifecycle are separate machines (`shared/src/status.rs:105-122`).
- **System-workspace sentinel over nullable column:** bootstrap events that predate any Workspace use a reserved all-zero-ULID `ws_` value so the §15 fail-closed envelope parse holds without schema change (`shared/src/ids.rs:205-210`).
- **The registry accretes per phase** — `events.rs` is explicitly *not* defined all-at-once; each phase adds payloads additively with a minor `CONTRACT_VERSION` bump caught by the schema gate (`shared/src/events.rs:3-8`).

## Gotchas & sharp edges

- **ExecutionProfile is the deliberately missing 10th status machine.** Held for 0.5b pending the cat-4 SDK-vs-PTY ruling + the ≥2026-06-15 credit-pool drain; the hold is itself a tested contract (`shared/src/lib.rs:34-42`, `shared/tests/contract.rs:432-439`). Don't "helpfully" add it.
- **`occurred_at`/`recorded_at` are plain `String`** (`shared/src/event_envelope.rs:78-79`) — valid RFC3339 in practice but no `Timestamp` newtype / schema `format:date-time`. Known carry-forward, consumer-slice 2.1 (`IMPLEMENTATION_PLAN.md:49`). Likewise `seq` lacks a schema `minimum:1` annotation though the writer enforces it (`IMPLEMENTATION_PLAN.md:50`).
- **`workflow_run_id` and `approval_id` are untyped `Option<String>`** on the envelope (`shared/src/event_envelope.rs:98-99`) — neither is in the LOCKED 22; `workflow_run_id` has an open reconcile (≡ `wfi_` vs new kind → would be a LOCKED-22 escalation; `IMPLEMENTATION_PLAN.md:51`).
- **Drift: event-type names are duplicated daemon-side as bare string literals.** `AuditIntegrityViolation`/`SensitiveOutputRedacted` have a single `EVENT_TYPE` home (`shared/src/events.rs:68,90`), but `"SessionStarted"` appears as a literal in `daemon/src/runtime/writer.rs:241` and four projectors (`daemon/src/projections/session.rs:23`, `activity.rs:27`, `graph.rs:44`, `object_refs.rs:46`), and `"DeviceRegistered"`/`"LocalRunnerRegistered"` are local consts in `daemon/src/bootstrap.rs:33,35` plus literals in `audit.rs:67-68`/`object_refs.rs:54,59`. Acknowledged dedup carry-forward (`shared/src/events.rs:66-67`, `docs/team-handoffs/003-…md:62`).
- **Drift sentinel currently red on the UI side:** `ui/src/contracts/generated.ts:10` pins `CONTRACT_VERSION = "0.12.0"` while the schema is at 0.14.0; the UI test asserting equality with `x-contract-version` (`ui/src/contracts/generated.test.ts:57-58`) fails until the UI regenerates — which is exactly the mechanism working as designed (regeneration is queued for the ui↔daemon integration slice). Details in [08-ui.md](08-ui.md).
- **`GetProjectionParams.scope` is accepted but NOT enforced** — the daemon returns the full table regardless of `scope.project_id` in MVP; don't build clients assuming scoping works (`shared/src/ipc.rs:160-173`).
- **The §5.0 contract gates are not in CI yet** — no `.github/workflows/`; test 9 + the 3-way verify run via local `cargo test`/`run.sh` only (carry-forward, `IMPLEMENTATION_PLAN.md` "Wire the §5.0 contract gates into CI").
- **`ServerFrame` reserves the Terminal-Channel tag space** — no PTY-frame variant exists; JSON-base64 vs binary is an explicit Phase-3 decision (`shared/src/ipc.rs:215-224`).
- **The 3-way verify compares enum value-sets only** (frozenset-of-frozensets) — it proves vocabulary equality, not struct-shape equality, and needs network-capable `uvx`/`npx` (`shared/contracts/verify/verify.py:41-48`).
- **`is_deferred()` is `true` only for `RemoteClient`** — the local desktop-host Device became MVP-live by user ruling (Option A, 2026-06-10), with the iOS multi-device dimension still deferred (`shared/src/objects.rs:42-44`, `shared/tests/contract.rs:413-421`).

## Connects to

- **[02-event-store.md](02-event-store.md)** — the daemon's writer persists `EventEnvelope`-shaped rows and enforces `RedactionStatus::Redacted` at the single-writer gate (`shared/src/event_envelope.rs:49-55` defines the values; the gate lives in `daemon::eventstore`).
- **[03-redaction.md](03-redaction.md)** — `RedactionStatus`, `redaction_engine_version` provenance (`shared/src/event_envelope.rs:105-107`), and the `SensitiveOutputRedacted` divert payload (`shared/src/events.rs:71-91`) are the contract half of redaction-before-persist.
- **[04-projections.md](04-projections.md)** — every projector folds `EventEnvelope` + the typed payloads (`daemon/src/projections/mod.rs:24`); `ProjectionName` (`shared/src/ipc.rs:108-120`) is the closed catalog both sides reject-unknown on.
- **[06-ipc.md](06-ipc.md)** — the entire `shared/src/ipc.rs` surface (handshake, RPC envelopes, `ServerFrame` mux, error codes) is what the daemon's UDS listener serves.
- **[07-daemon-runtime.md](07-daemon-runtime.md)** — bootstrap consumes `DeviceRegistered`/`LocalRunnerRegistered`/`SYSTEM_WORKSPACE_ID` + the protocol-range consts (`daemon/src/bootstrap.rs:17-23`).
- **[08-ui.md](08-ui.md)** — the generated-Zod consumer side: `ui/src/contracts/generated.ts` + its version-pin drift test.
