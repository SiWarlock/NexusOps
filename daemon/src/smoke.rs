//! The `nexusopsd smoke` dev-client subcommand (P4.0b-2-smoke / brief 053) — the authorized 0.1-HITL
//! "see it work" rig for the live INV-SEC-1 drive loop. A short-lived synchronous UDS client (the
//! `hook`-subcommand precedent): handshake-first, then ONE RPC, print the result. **Feature-gated
//! behind `dev-client`** (production hygiene — the smoke client is not needed in a release build); the
//! runbook builds `--features dev-client`. NEVER starts the write-actor / accept-loop — it submits
//! intents over the same GatewayPort the ui would, and the daemon adjudicates.
//!
//! Subcommands (the runbook `docs/runbooks/smoke-harness-live-drive-loop.md` drives these):
//! - `create --project <id> [--prompt "<text>"] [--profile <id>]` → `session.create` (launch a real claude)
//! - `queue`                       → `get_projection ApprovalQueue` (the pending gated tool calls)
//! - `approve <approval_id>`       → `approve` (the blocked agent tool runs)
//! - `deny <approval_id> <reason>` → `deny` (the blocked agent tool is denied)
//! - `kill <session_id>`           → `submit_action` a `session.kill` (stop the supervised session)
//! - `audit`                       → `get_projection AuditTrail` (the recorded event trail)
//!
//! It is a dev tool — errors print a clear message + a non-zero exit. Fail-closed isn't required here
//! (it only submits intents; the daemon is the adjudicator + the sole mutator).

use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::ipc::{HelloFrame, RpcRequest, ServerFrame, PROTOCOL_VERSION};
use nexusops_shared::status::ActionRequest as ActionRequestStatus;
use nexusops_shared::time::Timestamp;

use crate::ipc::{read_frame, write_frame, IpcError};

/// the GatewayPort UDS file within the app-support dir (mirrors `main.rs` / `hook.rs`).
const SOCKET_FILE: &str = "gateway.sock";

/// the dev-client read deadline — session.create/approve/deny/kill/queue/audit all resolve promptly
/// (none ride the daemon's 5-min interception wait, which only the `hook` subcommand drives). A
/// generous bound so a busy launch still completes; a read past it → Err → a clear failure message.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

const USAGE: &str = "usage: nexusopsd smoke <create|queue|approve|deny|kill|audit> [args]\n  \
    create --project <id> [--prompt \"<text>\"] [--profile <id>]\n  \
    queue\n  \
    approve <approval_id>\n  \
    deny <approval_id> <reason...>\n  \
    kill <session_id>\n  \
    audit";

/// Run the `smoke` subcommand. `args` is the full process argv (`args[1] == "smoke"`,
/// `args[2]` = the subcommand, `args[3..]` = its arguments).
pub fn run(args: &[String]) -> ExitCode {
    let sub = args.get(2).map(String::as_str).unwrap_or("");
    let rest: &[String] = if args.len() > 3 { &args[3..] } else { &[] };
    let result = match sub {
        "create" => cmd_create(rest),
        "queue" => cmd_get_projection("ApprovalQueue", "pending approvals"),
        "approve" => cmd_approve(rest),
        "deny" => cmd_deny(rest),
        "kill" => cmd_kill(rest),
        "audit" => cmd_get_projection("AuditTrail", "audit trail"),
        "" => Err(format!("missing subcommand\n{USAGE}")),
        other => Err(format!("unknown smoke subcommand '{other}'\n{USAGE}")),
    };
    match result {
        Ok(out) => {
            println!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nexusopsd smoke: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `create --project <id> [--prompt "<text>"] [--profile <id>]` — launch a supervised session, with an
/// optional `initial_prompt` that drives a real `claude` to make tool calls (the Option-G dev-drive).
fn cmd_create(rest: &[String]) -> Result<String, String> {
    let project = flag_value(rest, "--project")
        .ok_or_else(|| format!("create requires --project <id>\n{USAGE}"))?;
    let mut params = serde_json::json!({ "project_id": project });
    if let Some(prompt) = flag_value(rest, "--prompt") {
        params["initial_prompt"] = serde_json::Value::String(prompt);
    }
    if let Some(profile) = flag_value(rest, "--profile") {
        params["execution_profile_id"] = serde_json::Value::String(profile);
    }
    let result = call("session.create", params)?;
    Ok(format!("session.create →\n{}", pretty(&result)))
}

/// `queue` / `audit` — read a projection table and print its rows.
fn cmd_get_projection(name: &str, label: &str) -> Result<String, String> {
    let rows = call("get_projection", serde_json::json!({ "name": name }))?;
    Ok(format!("{label} ({name}):\n{}", pretty(&rows)))
}

/// `approve <approval_id>` — resolve a gated approval (the blocked agent tool then runs).
fn cmd_approve(rest: &[String]) -> Result<String, String> {
    let id = rest
        .first()
        .ok_or_else(|| format!("approve requires <approval_id>\n{USAGE}"))?;
    let result = call("approve", serde_json::json!({ "approval_id": id }))?;
    Ok(format!("approved {id} →\n{}", pretty(&result)))
}

/// `deny <approval_id> <reason...>` — deny a gated approval (the blocked agent tool is blocked).
fn cmd_deny(rest: &[String]) -> Result<String, String> {
    let id = rest
        .first()
        .ok_or_else(|| format!("deny requires <approval_id> <reason>\n{USAGE}"))?;
    let reason = rest.get(1..).map(|r| r.join(" ")).unwrap_or_default();
    if reason.is_empty() {
        return Err(format!("deny requires a <reason>\n{USAGE}"));
    }
    let result = call(
        "deny",
        serde_json::json!({ "approval_id": id, "reason": reason }),
    )?;
    Ok(format!("denied {id} →\n{}", pretty(&result)))
}

/// `kill <session_id>` — submit a `session.kill` action (risk-0, NaturalResourceRef-keyed on the
/// target session) via the generic `submit_action` method (there is no dedicated `session.kill` IPC
/// method — the executor handles the action type). The typed `ActionRequest` round-trips through serde
/// so the wire shape always matches the daemon's parser.
fn cmd_kill(rest: &[String]) -> Result<String, String> {
    let session_id = rest
        .first()
        .ok_or_else(|| format!("kill requires <session_id>\n{USAGE}"))?;
    let req = ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "session.kill".to_string(),
        requester_type: RequesterType::User, // UI/IPC (PIN e)
        requester_id: "smoke".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::Session,
            id: session_id.clone(),
            uri: None,
        }],
        inputs: serde_json::json!({}),
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        // a placeholder — the daemon's `request::insert` stamps `created_at` from its Clock.
        created_at: Timestamp::parse("1970-01-01T00:00:00Z")
            .map_err(|e| format!("placeholder timestamp: {e}"))?,
        preview: None,
    };
    let params = serde_json::to_value(&req).map_err(|e| format!("encode session.kill: {e}"))?;
    let result = call("submit_action", params)?;
    Ok(format!("session.kill {session_id} →\n{}", pretty(&result)))
}

// ---- the UDS call mechanism (the `hook.rs` precedent) -------------------------------------------

/// One synchronous GatewayPort call: connect → §6.4 handshake-first → one [`RpcRequest`] → read the
/// [`ServerFrame`] → the result value (or a clear error string from a `WireError` / a bad frame).
fn call(method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    let path = socket_path()?;
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        format!(
            "cannot connect to the daemon at {} ({e}) — is nexusopsd running?",
            path.display()
        )
    })?;
    // bound every read so a stalled daemon can't hang the dev-client forever (the `hook.rs`
    // precedent propagates this rather than swallowing it).
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .map_err(|e| format!("set read timeout: {e}"))?;

    // §6.4 handshake-first (the daemon rejects an RPC before a Hello).
    let hello = HelloFrame {
        protocol_version: PROTOCOL_VERSION,
        client_kind: "nexusops-smoke".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    write_frame(&mut stream, &to_vec(&hello)?).map_err(ipc_err)?;
    let _ack = read_frame(&mut stream).map_err(ipc_err)?; // HelloAck — a successful read means we're in.

    let req = RpcRequest {
        method: method.to_string(),
        params,
        id: 1,
    };
    write_frame(&mut stream, &to_vec(&req)?).map_err(ipc_err)?;
    let resp = read_frame(&mut stream).map_err(ipc_err)?;

    let frame: ServerFrame =
        serde_json::from_slice(&resp).map_err(|e| format!("bad response frame: {e}"))?;
    match frame {
        ServerFrame::RpcResponse(r) => {
            if let Some(err) = r.error {
                return Err(format!("daemon rejected the request: {:?}", err.code));
            }
            // a success with no `result` (rare; e.g. a future no-body ack) → print `null`, not an error.
            Ok(r.result.unwrap_or(serde_json::Value::Null))
        }
        other => Err(format!(
            "unexpected server frame (not an RPC response): {other:?}"
        )),
    }
}

/// the value following `flag` in `args`, if present (`--project proj_x` → `Some("proj_x")`).
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Resolve the daemon's GatewayPort UDS path (the same app-support dir `main.rs` binds).
fn socket_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join("Library/Application Support/NexusOps")
        .join(SOCKET_FILE))
}

fn to_vec<T: serde::Serialize>(v: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(v).map_err(|e| format!("encode: {e}"))
}

fn ipc_err(e: IpcError) -> String {
    format!("transport: {e}")
}

fn pretty(v: &serde_json::Value) -> String {
    // serializing an already-parsed `serde_json::Value` is infallible; the fallback is belt-and-braces.
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}
