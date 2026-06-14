//! Phase 3.3b — Codex rollout-store perms hardening (§15 #11). RED-first.
//! ARCHITECTURE §15 (#11 — Codex rollout files hardened: pre-create `~/.codex/sessions` 0700, files
//! 0600; Codex leaves them 0755/0644 → the daemon hardens). Foundation: docs/planning/0.3-codex-schema-research.md §2.3/§6.
//!
//! NON-cat-1: deterministic dir-precreate + umask VALUE; the live "Codex doesn't chmod back" check is HITL (Open-Q #9).

use std::os::unix::fs::PermissionsExt;

use nexusopsd::harness::codex::perms::{harden_codex_dirs, CODEX_CHILD_UMASK};

fn mode_of(path: &std::path::Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

// ---- #8 — harden_codex_dirs pre-creates ~/.codex/sessions at 0700 (§15 #11) ---------------------

#[test]
fn test_umask_dir_precreate_0700() {
    // spec(§15) — harden_codex_dirs(codex_home) pre-creates <home>/sessions at mode 0700 (so a
    // recovery scan / the child's rollout store is owner-only).
    let dir = tempfile::tempdir().unwrap();
    harden_codex_dirs(dir.path()).expect("harden ok");
    let sessions = dir.path().join("sessions");
    assert!(sessions.is_dir(), "sessions dir created");
    assert_eq!(mode_of(&sessions), 0o700, "sessions dir is owner-only 0700");
}

// ---- #9 — harden fails CLOSED on an un-securable store (§15 #11 / LESSON 30) --------------------

#[test]
fn test_umask_hardening_fail_closed() {
    // spec(§15) — the §15 #11 invariant IS a function of this I/O succeeding (LESSON 30) → fail-closed:
    // if `<home>/sessions` cannot be created/secured (here: a FILE already occupies that path), return
    // Err so the caller refuses to launch (no un-hardened rollout store; the write_settings()? precedent).
    let dir = tempfile::tempdir().unwrap();
    // a regular file at the `sessions` path → create_dir_all fails → harden returns Err.
    std::fs::write(dir.path().join("sessions"), b"not a dir").unwrap();
    assert!(
        harden_codex_dirs(dir.path()).is_err(),
        "a file-occupied sessions path → Err (fail-closed, no launch)"
    );
}

// ---- #10 — the child umask is 0077 (§15 #11) ---------------------------------------------------

#[test]
fn test_umask_value_0077() {
    // spec(§15) — the child umask is 0o077, so Codex's 0644/0755-created rollout files/dirs land
    // 0600/0700 (the spawner applies it pre-exec at the real spawn = 3.3c; the live "Codex doesn't
    // chmod back" verify is HITL Open-Q #9).
    assert_eq!(CODEX_CHILD_UMASK, 0o077);
}
