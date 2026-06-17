//! 075d — the durable §15-redacted `ScrollbackStore` (the VT arc's safety tail).
//!
//! The production impl behind the 075c [`ScrollbackStore`](crate::terminal::ScrollbackStore) seam:
//! each session's VT scrollback is persisted to a **0600 sidecar file** in a **0700 dir**, with the
//! **§15 Redactor run over the PLAIN text BEFORE write** (USER ruling ① = A: persisted form is
//! redacted plain-text; formatting dropped — the live re-render stays formatted). Swapping it in for
//! the 075c no-op placeholder makes the `Replayed`-after-daemon-restart rung LIVE.
//!
//! **🔴 §15 redaction-before-persist (the load-bearing invariant).** The sidecar may NEVER hold an
//! unredacted secret — this binds the store exactly as it binds the event store (forbidden #3). The
//! structural guarantee: [`PersistedScrollback`] is the ONLY `Serialize` on-disk type and every field
//! is POST-redaction, so an unredacted value cannot reach the serialized bytes (the §16 co-located-
//! gate). `VtSnapshot` itself has NO `Serialize`. **Fail-closed:** if redaction does not cleanly
//! complete, NO sidecar is written (the scrollback is lost → `Relaunched`, which is safe — never
//! persist unredacted). Reuses the proven Redactor (LESSONS §13/§49); 0700/0600 fail-closed (§15 #11,
//! LESSON §43). A **substrate write** (the daemon recording its own observed output for its own
//! survival — LESSONS §10 family — §15-redacted but NOT a Gateway Action).
//!
//! **Accepted residual (the harness-transcript posture, §13 envelope):** redaction is best-effort
//! recall + 0600 + the local-machine trust boundary; unlike events there is no keychain-refs-only
//! primary backstop (an agent can echo a literal no-shape secret). Same posture as harness transcripts
//! (also best-effort-redacted + 0600) — precedent-consistent, an honest accepted limitation.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use nexusops_shared::event_envelope::RedactionStatus;
use nexusops_shared::ids::SessionId;

use crate::eventstore::Redactor;
use crate::terminal::{HeadlessVt, ScrollbackStore, VtSnapshot, DEFAULT_SCROLLBACK_CAPACITY};

/// The on-disk sidecar format version — a migration hook (independent of the in-memory
/// `VtSnapshot`'s own version). A sidecar with an unknown version loads as `None` (fail-safe).
const PERSIST_VERSION: u8 = 1;

/// Retention backstop bounds (USER ruling ③). Conservative defaults; tuning is a follow-up. Per
/// sidecar: ~`DEFAULT_SCROLLBACK_CAPACITY` rows × a generous per-row size. Total dir: a soft cap that
/// evicts oldest-first. Age TTL: a stale sidecar (no recovery in this long) is reaped.
const MAX_SIDECAR_BYTES: u64 = 4 * 1024 * 1024; // 4 MiB / session sidecar
const MAX_DIR_BYTES: u64 = 256 * 1024 * 1024; // 256 MiB total
const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60); // 7 days

/// The ONLY serializable on-disk scrollback type — **every String is POST-redaction** (the §15
/// structural guarantee: an unredacted secret cannot reach the serialized bytes). Daemon-internal (NOT
/// a `shared/` wire contract); `version` is the migration hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedScrollback {
    version: u8,
    rows: u16,
    cols: u16,
    /// the visible screen as REDACTED plain text (`\n`-joined lines).
    screen_text: String,
    /// the scrollback rows as REDACTED plain text, oldest→newest.
    scrollback_rows: Vec<String>,
}

/// The durable §15-redacted [`ScrollbackStore`]. Holds the sidecar dir (0700) + the injected Redactor.
/// Lives in `scrollback/` (NOT `terminal/`/`session/`) so it can hold the Redactor + do FS without
/// breaking the `session/` cat-1 boundary (which keeps only the `Arc<dyn ScrollbackStore>` trait object).
pub struct FileScrollbackStore {
    dir: PathBuf,
    redactor: Arc<dyn Redactor>,
}

impl FileScrollbackStore {
    /// Build the store, ensuring `dir` exists at **0700** (fail-closed — the `harden_codex_dirs`
    /// pattern, §15 #11). An `Err` means the caller must degrade (main.rs falls back to the no-op store
    /// → no `Replayed`, never an insecure sidecar dir).
    pub fn new(dir: PathBuf, redactor: Arc<dyn Redactor>) -> io::Result<Self> {
        harden_dir_0700(&dir)?;
        Ok(Self { dir, redactor })
    }

    fn sidecar_path(&self, session_id: &SessionId) -> PathBuf {
        self.dir.join(format!("{}.json", session_id.as_str()))
    }

    /// Run the §15 Redactor over one plain-text string. `Some(masked)` ONLY when redaction cleanly
    /// completed (`status == Redacted` AND no quarantine signal); `None` → the caller fails closed
    /// (no write). This is the §15 gate: the returned (masked) text is the ONLY thing that reaches the
    /// serialized struct.
    fn redact_clean(&self, text: &str) -> Option<String> {
        let outcome = self.redactor.redact(text);
        // INTENTIONALLY stricter than the event-store gate (`redact_row` checks only `!= Redacted`):
        // a `Redacted + quarantine.is_some()` outcome (a "redacted but can't-bound" note) also
        // fail-closes here — the survival scrollback degrades to `Relaunched` (safe) rather than risk
        // persisting a value the Redactor itself flagged. Do not relax to match `redact_row`.
        if matches!(outcome.status, RedactionStatus::Redacted) && outcome.quarantine.is_none() {
            Some(outcome.payload_json)
        } else {
            None
        }
    }

    /// Atomic, fail-closed sidecar write: a temp file in the SAME 0700 dir (same-filesystem rename),
    /// created + `set_permissions(0600)` + re-read-verified (§15 #11 defense-in-depth) BEFORE the
    /// content is written, then `fsync` + atomic `rename` over the final path (a crash mid-write can't
    /// corrupt an existing sidecar; a leftover `.tmp` is ignored by `load`).
    fn write_atomic(&self, session_id: &SessionId, bytes: &[u8]) -> io::Result<()> {
        use std::io::Write as _;
        use std::os::unix::fs::PermissionsExt;

        let final_path = self.sidecar_path(session_id);
        let tmp_path = self.dir.join(format!("{}.json.tmp", session_id.as_str()));
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;
        // 0600 ABSOLUTE (set_permissions is umask-immune — guards a hostile umask/ACL), then VERIFY.
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600))?;
        let mode = std::fs::metadata(&tmp_path)?.permissions().mode() & 0o777;
        if mode != 0o600 {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(io::Error::other(format!(
                "scrollback sidecar temp mode {mode:o} != 0600 (refusing to write)"
            )));
        }
        // on any failure past this point, remove the temp so a stale `.tmp` never accumulates (it would
        // be ignored by `load` regardless — it's never named `<sid>.json` — but this keeps the dir tidy;
        // consistent with the mode-mismatch cleanup above).
        if let Err(e) = (|| -> io::Result<()> {
            f.write_all(bytes)?;
            f.sync_all()?;
            Ok(())
        })() {
            drop(f);
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }
        drop(f);
        if let Err(e) = std::fs::rename(&tmp_path, &final_path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }
        Ok(())
    }

    /// Startup orphan-sweep (075d retention ③): remove any sidecar whose `session_id` has no matching
    /// `proj_session` row (the session was never recorded, or its row was pruned), plus any leftover
    /// `.tmp` from a crashed write. Best-effort (a read error skips the sweep — never fatal). Called on
    /// the CONCRETE store at startup (before erasing to the trait object), with the live session id set.
    pub fn sweep_orphans(&self, known: &HashSet<SessionId>) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match path.extension().and_then(|e| e.to_str()) {
                Some("tmp") => {
                    // a leftover temp from a crashed write — never a valid sidecar; sweep it.
                    let _ = std::fs::remove_file(&path);
                }
                Some("json") => {
                    let keep = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(|stem| SessionId::parse(stem).ok())
                        .is_some_and(|sid| known.contains(&sid));
                    if !keep {
                        // no matching session (or an unparseable name) → orphan; remove.
                        let _ = std::fs::remove_file(&path);
                    }
                }
                _ => {}
            }
        }
    }

    /// Retention backstop (075d ③) with production bounds.
    pub fn enforce_backstop(&self) {
        self.enforce_backstop_with(MAX_SIDECAR_BYTES, MAX_DIR_BYTES, MAX_AGE);
    }

    /// The parameterized backstop (tests pass tight bounds): remove each sidecar over `max_sidecar_bytes`
    /// OR older than `max_age`; then, if the dir total still exceeds `max_dir_bytes`, evict oldest-mtime
    /// sidecars until under. Best-effort; `.json` only (skips `.tmp`). Deletions only → no §15 surface.
    pub fn enforce_backstop_with(
        &self,
        max_sidecar_bytes: u64,
        max_dir_bytes: u64,
        max_age: Duration,
    ) {
        let now = SystemTime::now();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let mut survivors: Vec<(PathBuf, u64, SystemTime)> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let size = meta.len();
            let mtime = meta.modified().unwrap_or(now);
            let too_old = now
                .duration_since(mtime)
                .map(|age| age > max_age)
                .unwrap_or(false);
            if size > max_sidecar_bytes || too_old {
                let _ = std::fs::remove_file(&path);
                continue;
            }
            survivors.push((path, size, mtime));
        }
        // dir cap — evict oldest first until the total is under the cap.
        let mut total: u64 = survivors.iter().map(|(_, s, _)| *s).sum();
        if total > max_dir_bytes {
            survivors.sort_by_key(|(_, _, mtime)| *mtime);
            for (path, size, _) in &survivors {
                if total <= max_dir_bytes {
                    break;
                }
                // only count the freed bytes if the delete actually succeeded — a failed remove must
                // NOT deflate `total` (that would halt eviction with the dir still over the cap).
                if std::fs::remove_file(path).is_ok() {
                    total -= size;
                }
            }
        }
    }
}

impl ScrollbackStore for FileScrollbackStore {
    fn save(&self, session_id: &SessionId, snapshot: &VtSnapshot) {
        // 1. derive the PLAIN text (restore the screen; the scrollback rows are already plain).
        let vt = HeadlessVt::from_snapshot(snapshot);
        let (rows, cols) = vt.size();
        let screen_plain = vt.screen_contents();

        // 2. §15 gate — redact EACH piece; ANY non-clean redaction → FAIL-CLOSED (no write). The
        //    redacted text is the ONLY thing that reaches `PersistedScrollback`.
        let Some(screen_text) = self.redact_clean(&screen_plain) else {
            eprintln!(
                "nexusopsd: scrollback save skipped for {} — screen redaction not clean (fail-closed, §15)",
                session_id.as_str()
            );
            return;
        };
        let mut scrollback_rows = Vec::with_capacity(snapshot.scrollback_text().len());
        for row in snapshot.scrollback_text() {
            let Some(redacted) = self.redact_clean(row) else {
                eprintln!(
                    "nexusopsd: scrollback save skipped for {} — scrollback redaction not clean (fail-closed, §15)",
                    session_id.as_str()
                );
                return;
            };
            scrollback_rows.push(redacted);
        }

        // 3. build the post-redaction-only struct + atomically write the 0600 sidecar (fail-closed —
        //    a serialize/write error drops the snapshot; the scrollback is lost → Relaunched, safe).
        let persisted = PersistedScrollback {
            version: PERSIST_VERSION,
            rows,
            cols,
            screen_text,
            scrollback_rows,
        };
        let json = match serde_json::to_string(&persisted) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("nexusopsd: scrollback save skipped — serialize failed: {e}");
                return;
            }
        };
        if let Err(e) = self.write_atomic(session_id, json.as_bytes()) {
            eprintln!(
                "nexusopsd: scrollback save failed (write) for {}: {e}",
                session_id.as_str()
            );
        }
    }

    fn load(&self, session_id: &SessionId) -> Option<VtSnapshot> {
        // absent / unreadable → None (fail-safe). A leftover `.tmp` is never named `<sid>.json`.
        let bytes = std::fs::read(self.sidecar_path(session_id)).ok()?;
        // corrupt / partial / unknown-version → None (no panic).
        let persisted: PersistedScrollback = serde_json::from_slice(&bytes).ok()?;
        if persisted.version != PERSIST_VERSION {
            return None;
        }
        // reconstruct from the redacted PLAIN text (ruling ① = A: re-render is plain). The capacity is
        // the shared producer default so every persisted row fits; `max(len)` defends a malformed
        // oversized sidecar.
        let capacity = persisted
            .scrollback_rows
            .len()
            .max(DEFAULT_SCROLLBACK_CAPACITY);
        let mut vt = HeadlessVt::from_plain(
            persisted.rows,
            persisted.cols,
            capacity,
            &persisted.screen_text,
            &persisted.scrollback_rows,
        );
        Some(vt.snapshot())
    }

    fn evict(&self, session_id: &SessionId) {
        // best-effort + idempotent (a missing sidecar is fine — `remove_file` errs but we ignore it).
        let _ = std::fs::remove_file(self.sidecar_path(session_id));
    }
}

/// Pre-create `dir` at 0700 (§15 #11), FAIL-CLOSED — the `harden_codex_dirs` pattern: `create_dir_all`
/// → `set_permissions(0700)` → re-read + verify the mode (guards a hostile umask/ACL).
fn harden_dir_0700(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    let mode = std::fs::metadata(dir)?.permissions().mode() & 0o777;
    if mode != 0o700 {
        return Err(io::Error::other(format!(
            "scrollback dir mode {mode:o} != 0700 after harden (refusing to use it)"
        )));
    }
    Ok(())
}
