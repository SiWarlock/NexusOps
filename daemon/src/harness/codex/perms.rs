//! Codex rollout-store perms hardening (§15 #11).
//!
//! Codex hardens its `auth.json`/`config.toml` to 0600 but leaves `~/.codex/sessions` 0755 and the
//! rollout files 0644 (research §2.3) — and the rollouts carry full transcripts (incl. command output
//! that may hold secrets). So the daemon hardens the store itself: pre-create `sessions` at 0700 +
//! launch the codex child under [`CODEX_CHILD_UMASK`] (0o077) so Codex's per-session 0644/0755 files
//! land 0600/0700. The §15 #11 invariant IS a function of this I/O succeeding (LESSON 30) → fail-closed.

use std::path::Path;

/// The umask applied to the spawned codex child (§15 #11): 0o077 → Codex's 0644/0755-created rollout
/// files/dirs land 0600/0700. The spawner applies it pre-exec at the real spawn (3.3c); the live
/// "Codex doesn't chmod the perms back" verify is HITL (research Open-Q #9).
pub const CODEX_CHILD_UMASK: u32 = 0o077;

/// Pre-create the codex rollout store (`<codex_home>/sessions`) at 0700 (§15 #11), FAIL-CLOSED. An
/// `Err` means the caller must refuse to launch (no un-hardened rollout store — the `write_settings()?`
/// fail-closed precedent, LESSON 30). Verifies the resulting mode (defense-in-depth: a hostile umask /
/// ACL could foil `set_permissions`).
pub fn harden_codex_dirs(codex_home: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let sessions = codex_home.join("sessions");
    // create_dir_all errors if the path exists but is NOT a directory (a file occupies it) → fail-closed.
    std::fs::create_dir_all(&sessions)?;
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o700))?;
    // re-read + verify the mode (set_permissions sets the ABSOLUTE mode, unaffected by the process
    // umask — so this guards a hostile ACL or a filesystem that doesn't honor POSIX modes, not umask).
    let mode = std::fs::metadata(&sessions)?.permissions().mode() & 0o777;
    if mode != 0o700 {
        return Err(std::io::Error::other(format!(
            "codex sessions dir mode {mode:o} != 0700 after harden (refusing to launch)"
        )));
    }
    Ok(())
}
