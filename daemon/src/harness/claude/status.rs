//! Claude status derivation (§5.1) — the pure `(prev, ClaudeSignal) -> Session` fold.
//!
//! **Safety #9 — derive status from STRUCTURED signals, NEVER PTY output bytes.** [`derive_status`]
//! takes only the typed [`ClaudeSignal`] (hook events + the 3.4 process-exit event); it has no
//! access to the PTY. This is the pure core; the LIVE signal ingestion (the hook receiver +
//! transcript tailer + statusLine endpoint → these typed signals) is the I/O seam at 043 (hooks) +
//! P4 (drive loop). The display-only #9 invariant is pinned STRUCTURALLY by
//! `daemon/tests/claude_adapter.rs::test_status_derivation_takes_structured_signals_not_pty_bytes`,
//! a forbidden-token grep scoped to **this file** (so the future reader/receiver code in sibling
//! modules cannot false-alarm it).

use nexusops_shared::status::Session;

/// A typed, STRUCTURED signal the status derivation folds over — Claude hook events + the 3.4
/// process-exit event — NEVER raw PTY output bytes (safety #9). The LIVE ingestion (the hook
/// receiver + transcript tailer + statusLine endpoint → these typed signals) is the I/O seam that
/// lands with 043 (hooks) + P4 (drive loop); `derive_status` is the pure core, test-first now.
///
/// Kept EXHAUSTIVE (no `#[non_exhaustive]`) on purpose — it is daemon-internal, and the exhaustive
/// `match` in `derive_status` is the compile-time reminder to map any future variant.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ClaudeSignal {
    /// the session started (`SessionStart` hook): `source` = startup|resume|…, `model` if reported.
    SessionStart {
        source: String,
        model: Option<String>,
    },
    /// a tool is about to run (`PreToolUse` hook): `tool` = the `tool_name` (Bash/Write/Edit/…).
    PreToolUse { tool: String },
    /// a tool finished (`PostToolUse` hook).
    PostToolUse,
    /// a notification (`Notification` hook), pre-classified by the ingestion layer.
    Notification(NotificationKind),
    /// the agent yielded the turn (`Stop` hook).
    Stop,
    /// the PTY child exited (the 3.4 `TerminalProcessExited` event) — `exit_code` XOR `signal`.
    ProcessExited {
        exit_code: Option<i32>,
        signal: Option<String>,
    },
}

/// The kind of `Notification` hook (classified from the hook's `notification_type`/`message` by the
/// ingestion layer): a permission request, an input request, or other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotificationKind {
    Permission,
    InputNeeded,
    Other,
}

/// Derive the §5.1 `Session` state from the prior state + one structured [`ClaudeSignal`] (safety #9
/// — STRUCTURED signals only, never PTY output bytes). Pure + total: a `(prev, signal)` fold.
///
/// **R-9 terminal-sink guard:** a terminal state is a SINK — no signal resurrects it (the prior
/// state is held, never corrupted). The FULL §5.1 legal-edge set (non-terminal illegal edges) is a
/// **§5.1-machine concern, NOT the adapter's** — this guards only the adapter-relevant case.
///
/// **Deferred (Q2 — no reliable signal yet):** `RunningTests` (no command-sniffing — a test-runner
/// Bash command maps to `RunningCommand`), `Thinking`/`ChangesReady`/`WaitingOnExternalService`
/// (transcript-marker states — a follow-on once the JSONL parse lands). **Out of scope (daemon
/// lifecycle, not adapter-derived):** `Creating`/`Stale`/`Archived`/`Killed`.
pub fn derive_status(prev: Session, signal: &ClaudeSignal) -> Session {
    if prev.is_terminal() {
        return prev; // R-9 terminal-sink: held, not transitioned
    }
    match signal {
        ClaudeSignal::SessionStart { .. } => Session::Starting,
        ClaudeSignal::PreToolUse { tool } => match tool.as_str() {
            "Bash" => Session::RunningCommand,
            "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => Session::EditingFiles,
            _ => Session::Active,
        },
        ClaudeSignal::PostToolUse => Session::Active,
        ClaudeSignal::Notification(NotificationKind::Permission) => Session::WaitingOnPermission,
        ClaudeSignal::Notification(NotificationKind::InputNeeded) => Session::WaitingOnHumanInput,
        // an unclassified notification holds the prior state (no spurious transition).
        ClaudeSignal::Notification(NotificationKind::Other) => prev,
        ClaudeSignal::Stop => Session::Idle,
        // exit 0 with no terminating signal → Completed; EVERY other shape (nonzero code, a signal,
        // or the unknown `(None, None)`) → Failed (fail-closed — never a false Completed).
        ClaudeSignal::ProcessExited {
            exit_code,
            signal: exit_signal,
        } => {
            if *exit_code == Some(0) && exit_signal.is_none() {
                Session::Completed
            } else {
                Session::Failed
            }
        }
    }
}
