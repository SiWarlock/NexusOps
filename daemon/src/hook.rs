//! The `nexusopsd hook` subcommand — the live INV-SEC-1 interception INGRESS (P4.0b-2 C2).
//!
//! Claude spawns this per `PreToolUse` tool-call (the 042/043 generated per-session settings wire the
//! hook command to `<nexusopsd> hook <event>`). It reads the tool-call payload on stdin, tags it with
//! the daemon session (`NEXUSOPS_SESSION_ID`, set in the spawned env so the daemon correlates the
//! interception to the session — the `cancel_session` key), routes it to the daemon over the UDS
//! `intercept` method, and translates the verdict to Claude's `PreToolUse` hook output.
//!
//! **FAIL-CLOSED (the 043 posture):** ANY error — no daemon, unreachable socket, parse failure, an
//! unexpected frame — → **DENY** (block the tool). The daemon-side receiver deny (`CoverageGap` etc.)
//! + the launch `permissions.deny` baseline are the PRIMARY controls; this conduit defaults to deny.
//!
//! **0.1-HITL (flagged):** the exact `PreToolUse` hook output grammar Claude honors is validated by
//! the authorized CLI smoke harness (the user runs it with their account); the deny default is the
//! load-bearing safety property regardless.
//!
//! **3.3c (brief 066) — the Codex variant (`--harness codex`):** [`run_codex`] reads Codex's
//! `PreToolUse` stdin, NORMALIZES it into the SAME `intercept` RPC params the Claude path sends
//! ([`crate::harness::codex::intercept`]), reuses the SAME [`send_intercept`] transport (the daemon
//! adjudication loop is harness-agnostic — swap only the I/O envelope), and emits Codex's `PreToolUse`
//! output. FAIL-CLOSED identically — any error → DENY.

use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use nexusops_shared::ipc::{HelloFrame, RpcRequest, ServerFrame, PROTOCOL_VERSION};

use crate::harness::codex::intercept::{
    codex_verdict_output, normalize_to_intercept_params, parse_codex_payload,
};
use crate::ipc::{read_frame, write_frame, IpcError};

/// the GatewayPort UDS file within the app-support dir (mirrors `main.rs`).
const SOCKET_FILE: &str = "gateway.sock";

/// the hook's CLIENT-side read deadline — bounds each read so a stalled/deadlocked daemon can't block
/// the agent's `PreToolUse` hook FOREVER (the daemon's 5-min `APPROVAL_WAIT` ticks INSIDE the daemon;
/// this is the hook's own fail-closed timeout for the verdict frame). > the approval wait + a margin
/// so a legitimately-pending human approval is NOT cut short; a read past it → Err → Deny (fail-closed).
const HOOK_READ_TIMEOUT: Duration = Duration::from_secs(360);

/// Run the hook subcommand for `event`. Only `PreToolUse` adjudicates (the interception); every other
/// hook event (`SessionStart`/`Stop`/`Notification`/`statusline`) is a no-op allow for the MVP — the
/// statusLine telemetry ingestion is P4.0c.
pub fn run(event: &str) -> ExitCode {
    if event != "PreToolUse" {
        return ExitCode::SUCCESS;
    }
    match adjudicate() {
        // a daemon Allow → permit the tool; a daemon Deny OR any error → block (fail-closed).
        Ok(true) => emit_allow(),
        Ok(false) | Err(_) => emit_deny(),
    }
}

/// Run the Codex hook subcommand (`nexusopsd hook --harness codex <event>`, brief 066). Only
/// `PreToolUse` adjudicates; every other Codex hook event is a no-op allow (the MVP). The verdict is
/// translated to Codex's `PreToolUse` output ([`codex_verdict_output`] — FAIL-CLOSED: any error /
/// non-allow → deny, identical to the Claude default).
pub fn run_codex(event: &str) -> ExitCode {
    if event != "PreToolUse" {
        return ExitCode::SUCCESS;
    }
    // the Codex output IS the verdict translation — `codex_verdict_output` maps Ok(true)→allow,
    // Ok(false)/Err→deny. Print it + exit 0 (the deny rides stdout, the Claude `emit_deny` precedent).
    println!("{}", codex_verdict_output(&adjudicate_codex()));
    ExitCode::SUCCESS
}

/// Read the `PreToolUse` payload, route it to the daemon's `intercept`, return whether it was ALLOWED.
/// Any I/O / protocol error propagates → the caller blocks (fail-closed).
fn adjudicate() -> std::io::Result<bool> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let mut payload: serde_json::Value =
        serde_json::from_str(&input).map_err(std::io::Error::other)?;

    // tag with the DAEMON session id (the correlation key) — overwrite the payload's `session_id`
    // (Claude's internal id) with the daemon's, so the daemon's registry keys on the session it tracks.
    let session_id = std::env::var("NEXUSOPS_SESSION_ID").map_err(|_| {
        std::io::Error::other("NEXUSOPS_SESSION_ID unset (not a daemon-launched session)")
    })?;
    payload["session_id"] = serde_json::Value::String(session_id);

    send_intercept(payload)
}

/// Read Codex's `PreToolUse` stdin → NORMALIZE into the SAME `intercept` params the Claude path sends
/// (brief 066, PIN-1: the `harness=codex` discriminator is stamped by `normalize_to_intercept_params`,
/// never from the agent's stdin) → route via the SAME [`send_intercept`] transport. Any error
/// (malformed stdin / `NEXUSOPS_SESSION_ID` unset / transport) propagates → the caller denies.
fn adjudicate_codex() -> std::io::Result<bool> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let payload = parse_codex_payload(&input)
        .map_err(|_| std::io::Error::other("malformed Codex PreToolUse payload"))?;
    let session_id = std::env::var("NEXUSOPS_SESSION_ID").map_err(|_| {
        std::io::Error::other("NEXUSOPS_SESSION_ID unset (not a daemon-launched session)")
    })?;
    send_intercept(normalize_to_intercept_params(&payload, &session_id))
}

/// The SHARED `intercept` transport (harness-agnostic, brief 066): connect the GatewayPort UDS,
/// handshake, send the `intercept` RPC with `params`, read the verdict frame → whether ALLOWED. Every
/// read is bounded by [`HOOK_READ_TIMEOUT`] (a stalled daemon can't hang the agent's hook forever); any
/// I/O / protocol error propagates → the caller blocks (fail-closed). Both the Claude and Codex
/// adjudicators feed their normalized `params` through this ONE path.
fn send_intercept(payload: serde_json::Value) -> std::io::Result<bool> {
    let mut stream = UnixStream::connect(socket_path()?)?;
    // bound every read so a stalled daemon can't hang the agent's hook forever (the hook fails closed
    // on its own timeout — independent of the daemon's internal APPROVAL_WAIT).
    stream.set_read_timeout(Some(HOOK_READ_TIMEOUT))?;
    // §6.4 handshake-first.
    let hello = HelloFrame {
        protocol_version: PROTOCOL_VERSION,
        client_kind: "nexusops-hook".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    write_frame(&mut stream, &to_vec(&hello)?).map_err(ipc_to_io)?;
    let _ack = read_frame(&mut stream).map_err(ipc_to_io)?; // HelloAck — a successful read means we're in.

    // the intercept RPC (the daemon routes it through the Gateway → the verdict / the decision_sink wait).
    let req = RpcRequest {
        method: "intercept".to_string(),
        params: payload,
        id: 1,
    };
    write_frame(&mut stream, &to_vec(&req)?).map_err(ipc_to_io)?;
    let resp = read_frame(&mut stream).map_err(ipc_to_io)?;

    // the response is a ServerFrame::RpcResponse wrapping `{decision: "allow"|"deny", reason?}`.
    let frame: ServerFrame = serde_json::from_slice(&resp).map_err(std::io::Error::other)?;
    let allow = match frame {
        ServerFrame::RpcResponse(r) => {
            r.result
                .as_ref()
                .and_then(|v| v.get("decision"))
                .and_then(|d| d.as_str())
                == Some("allow")
        }
        // any other frame (a subscription push / terminal output / an error response) → not an allow.
        _ => false,
    };
    Ok(allow)
}

/// Resolve the daemon's GatewayPort UDS path (the same app-support dir `main.rs` binds).
fn socket_path() -> std::io::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home)
        .join("Library/Application Support/NexusOps")
        .join(SOCKET_FILE))
}

fn to_vec<T: serde::Serialize>(v: &T) -> std::io::Result<Vec<u8>> {
    serde_json::to_vec(v).map_err(std::io::Error::other)
}

fn ipc_to_io(e: IpcError) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

/// Emit Claude's `PreToolUse` hook output PERMITTING the tool (the daemon adjudicated Allow). Exit 0.
fn emit_allow() -> ExitCode {
    println!(
        "{}",
        serde_json::json!({
            "hookSpecificOutput": { "hookEventName": "PreToolUse", "permissionDecision": "allow" }
        })
    );
    ExitCode::SUCCESS
}

/// Emit Claude's `PreToolUse` hook output DENYING the tool — the fail-closed default (a daemon Deny OR
/// any error). The exact grammar is 0.1-HITL-validated; the deny is the load-bearing safety property.
fn emit_deny() -> ExitCode {
    println!(
        "{}",
        serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": "NexusOps: denied by policy (or interception unavailable — fail-closed)"
            }
        })
    );
    ExitCode::SUCCESS
}
