//! Codex status derivation (§5.1) — the pure `(prev, CodexSignal) -> Session` fold.
//!
//! **Safety #9 — derive status from the STRUCTURED event stream, NEVER PTY output bytes.**
//! [`derive_status`] takes only the typed [`CodexSignal`] (normalized from the rollout `event_msg`/
//! `response_item` records OR the `exec --json` `ThreadEvent` stream — `parse.rs`/`stream.rs`); it has
//! no access to the PTY. This is the pure core (the Claude `derive_status` mirror, LESSON §25); the
//! LIVE signal ingestion is the I/O seam at 3.3b/3.3c. The display-only #9 invariant is pinned
//! STRUCTURALLY by `daemon/tests/codex_adapter.rs::test_status_no_pty_scrape`, a forbidden-token grep
//! scoped to THIS file (so the sibling parser/stream I/O cannot false-alarm it).

use nexusops_shared::status::Session;

/// The semantic tool-call category (§9.1 — classified BY SEMANTICS, not hardcoded vendor names, so
/// the OSS `codex` names [`shell`/`local_shell`] and the desktop-build names [`exec_command`] both
/// map to the same kind). 3.3c's CAT-1 interception keys on this. Classification = [`super::parse::classify_tool`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodexToolKind {
    /// a shell/command execution (the dominant mutation tool)
    ShellExec,
    /// a file patch (`apply_patch` — the FS-mutation tool)
    FilePatch,
    /// an MCP tool call
    McpTool,
    /// anything else (request_user_input, write_stdin, update_plan, …) — non-mutation by default
    Other,
}

/// A typed, STRUCTURED signal the status derivation folds over — normalized from EITHER Codex
/// vocabulary (the rollout `event_msg`/`response_item` OR the `exec --json` `ThreadEvent`), NEVER raw
/// PTY output bytes (safety #9). The two parsers (`parse.rs`/`stream.rs`) produce the SAME
/// `CodexSignal` stream, so one `derive_status` serves both.
///
/// Kept EXHAUSTIVE (no `#[non_exhaustive]`) — daemon-internal; the exhaustive `match` in
/// [`derive_status`] is the compile-time reminder to map any future variant.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CodexSignal {
    /// the session started (`session_meta` / `thread.started`).
    SessionStarted,
    /// a turn began (`task_started` / `turn.started`) — the agent is working.
    TaskStarted,
    /// a tool is being invoked (`function_call`/`custom_tool_call`/`item.*`), classified by semantics.
    ToolCall(CodexToolKind),
    /// a turn completed cleanly (`task_complete` / `turn.completed`).
    TurnCompleted,
    /// a turn ended abnormally (`turn_aborted` / `turn.failed` / `error`).
    TurnAborted,
}

/// Derive the §5.1 `Session` state from the prior state + one structured [`CodexSignal`] (safety #9 —
/// STRUCTURED signals only, never PTY output bytes). Pure + total: a `(prev, signal)` fold.
///
/// **R-9 terminal-sink guard:** a terminal state is a SINK — no signal resurrects it (the prior state
/// is held). The full §5.1 legal-edge set is a §5.1-machine concern, not the adapter's.
///
/// **Turn-scoped, NON-terminal by design (3.3a, lead-ruled Q1):** a Codex session is MULTI-TURN, so a
/// turn boundary ≠ session death — `TurnCompleted`/`TurnAborted` both map to **`Idle`** (the session is
/// alive, awaiting the next turn), NEVER terminal `Failed`/`Completed` (mapping a turn abort → terminal
/// would R-9-sink a recoverable session forever). The **terminal** states (`Completed`/`Failed`/
/// `Killed`) derive from the **process-exit signal** (the codex child dying, observed by the
/// supervisor/launcher) — that arm lands with its live source in **3.3b/3.3c**, NOT this observe slice.
///
/// **Out of scope (no reliable signal in the rollout/stream):** `Thinking`/`RunningTests`/
/// `ChangesReady`/`WaitingOn*` (transcript-marker / approval-elicitation states — follow-ons);
/// `Creating`/`Stale`/`Archived` (daemon-lifecycle, not adapter-derived).
pub fn derive_status(prev: Session, signal: &CodexSignal) -> Session {
    if prev.is_terminal() {
        return prev; // R-9 terminal-sink: held, not transitioned
    }
    match signal {
        CodexSignal::SessionStarted => Session::Starting,
        CodexSignal::TaskStarted => Session::Active,
        CodexSignal::ToolCall(CodexToolKind::ShellExec) => Session::RunningCommand,
        CodexSignal::ToolCall(CodexToolKind::FilePatch) => Session::EditingFiles,
        // an MCP/other tool call is "working" but not a shell/file-edit → Active (no narrower §5.1 state).
        CodexSignal::ToolCall(CodexToolKind::McpTool | CodexToolKind::Other) => Session::Active,
        // a turn ending (clean OR abnormal) is a turn boundary, NOT session death → Idle (Q1).
        CodexSignal::TurnCompleted | CodexSignal::TurnAborted => Session::Idle,
    }
}
