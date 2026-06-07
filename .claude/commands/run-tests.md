---
description: Run tests by class. cwd-aware. Usage: /run-tests [unit|integration|all]
allowed-tools: Bash
argument-hint: "[unit|integration|all]"
---

Run tests by class. **cwd-aware** — runs the right test runner for whichever code area you're in.

Argument: `$ARGUMENTS` — see the mapping table(s) below. Default: `unit`.

## Step 0 — Detect mode

```bash
case "$(pwd)" in
  */ui|*/ui/*) MODE=the Tauri desktop UI (TS frontend + thin Rust host) ;;
  *)           MODE=the Rust daemon (trust core) ;;
esac
```

Announce the detected mode before running.

---

## the Rust daemon (trust core) mode mapping

| Argument | Command |
|---|---|
| (empty / `unit`) | `cargo test --lib` |
| `integration` | `cargo test --test '*'` |
| `all` | `cargo test` |
| <other class / marker> | `cargo test <name> -- --nocapture` |

## the Tauri desktop UI (TS frontend + thin Rust host) mode mapping

| Argument | Command |
|---|---|
| (empty / `unit`) | `pnpm vitest run` |
| `integration` / `e2e` | `pnpm test:integration` |
| `all` | `pnpm test:all` |

If an argument names a class that belongs to the *other* mode, **ERROR** with a clear message naming the expected cwd.

---

<!-- ▼ EXAMPLE BLOCK [id=test-class-discipline-notes]: test-class discipline notes — OPTIONAL. Some test classes
     need preconditions (a live external dependency, an env var, a slow browser).
     The source project documented things like: "the live-attack class needs a
     reachable target + a bearer env var, else it skips with a clear message;"
     "the visual-smoke class is slow — run per-PR, not per-commit." Add the
     project's own per-class discipline notes here, or delete this block. ▼ -->
**Per-class discipline (NexusOps):**

- **daemon `unit`** (`cargo test --lib`) — pure trust-core logic: Action risk-classification, the typed event-log append/projection, intent validation. No PTY, no git, no network. The per-commit default.
- **daemon `integration`** (`cargo test --test '*'`) — exercises real side-effecting adapters: rusqlite event store, portable-pty session lifecycle, `git2` working-tree ops, keyring access. Tests that touch the keyring or spawn PTYs are serialized and may be slow; run per-slice, not on every red-green flip. The octocrab/network paths skip with a clear message unless a token env var (e.g. `NEXUSOPS_GH_TOKEN`) is present — never hit live GitHub in CI by default.
- **ui `unit`** (`pnpm vitest run`) — React component + Zod projection-decode tests against a mocked Tauri IPC bridge. Fast; per-commit default.
- **ui `e2e`/`integration`** (`pnpm test:integration`) — Tauri-driver drives the real desktop shell against a running daemon. Slow + requires the Tauri toolchain (and a built host binary); run per-PR / per-slice, not per-commit. Skips with a clear message if `tauri-driver` or the daemon socket isn't reachable.
- **Reminder:** the daemon is the single audited mutator — never let a UI test mutate state directly; UI `e2e` asserts on projections the daemon emits.
<!-- ▲ END EXAMPLE BLOCK [id=test-class-discipline-notes] ▲ -->

## Output

Report:
- Mode (which code area)
- Test count + class
- Pass / fail counts
- First ~20 lines of any failure
- Total duration
