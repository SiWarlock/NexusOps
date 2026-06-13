# FINDING — git diff copy-detection deferred (git2 0.21 limitation)

> **Status:** DEFERRED (R4, 2026-06-12). A §D-carry from edges-006 (carried since R2). Lead-confirmed defer
> + finding-doc (no in-lane path). Not a slice. Revisit at the P5/P7.1 phase-exit or when the gated
> git-CLI path lands.

## The requirement
The git diff read backend (`daemon/src/git/reads.rs` — `read_diff` / `read_file_hunks`) should ideally
surface **copied** files the way `git diff -C` / `--find-copies` does: a file copied from an existing one
shows as a `Copied` delta carrying its source path (analogous to how a `Renamed` delta carries `old_path`).
This is the natural sibling of the rename detection that edges-011 (`fcf3ba9`) already ships.

## The limitation (why it's not in-lane)
git2 **0.21** (the in-tree version) does **not** detect copies. The diff backend already uses
`DiffFindOptions` + `find_similar` with `renames(true)` for rename detection; turning on copy detection
(`copies(true)`) **detects none** in 0.21 — an **empirically-determined** limitation recorded in-code at
`daemon/src/git/reads.rs:245`:

> *"Copy detection stays OFF (git2 0.21 detects none)."*

So there is **no pure-git2 in-lane path** to copy detection with the current dependency. Rename detection —
the common, higher-value case — works and ships; copy detection is the gap.

## The only working alternatives (both out-of-lane)
1. **Shell out to the git-CLI** (`git diff -C --find-copies[-harder]`) and parse the porcelain. But git-CLI
   invocation in this codebase is **not** the read path — git2 owns hot structured **reads** (§9), and the
   git-CLI is reserved for **mutations as Gateway actions** (forbidden #6). A CLI-based *read* for copy
   detection would be a new pattern that belongs with the **gated wiring** (R1 — the executor seam +
   git namespace), not the pure in-lane read backend. → deferred with the wiring.
2. **Bump git2/libgit2** to a version whose `find_similar` actually detects copies, then flip `copies(true)`
   on. A dependency bump is a cross-cutting change to re-verify empirically (it may still not detect copies —
   libgit2's copy detection has historically been weaker than the CLI's). → a phase-exit evaluation, not an
   in-lane slice.

## Impact — LOW
No correctness loss. A copied file currently reports as a plain **`Added`** delta (with its real line
counts) — which is *accurate*, just less rich than `Copied{from: …}`. The diff is never wrong; it only
omits the copy-provenance signal. Rename detection (the frequent case) is unaffected and works.

## Recommendation — DEFER
- **Do not** add an in-lane copy-detection slice (no pure-git2 path exists in 0.21).
- **Revisit at the P5/P7.1 phase-exit**, two ways: (a) re-test `copies(true)` against whatever git2/libgit2
  the merged tree carries; (b) if the gated git-CLI git-namespace executor (R1) lands, evaluate a
  CLI-`-C`-parsed copy signal there.
- Keep the `reads.rs:245` "copies OFF" comment as the in-code marker; this doc is the durable rationale.

## Cross-refs
- `daemon/src/git/reads.rs:245` (the in-code determination) · edges-011 `fcf3ba9` (rename detection that
  copy detection would extend) · `docs/planning/edges-R1-routing-packet.md` (the gated git-CLI executor path).
