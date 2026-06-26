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
//! **086 — the 083 live-validation chain** (each a THIN wrapper over an EXISTING audited action via the
//! same `submit_action`/`connect_via_gh`; NO new mutation surface — the daemon gates + executes unchanged):
//! - `connect-gh --provider github --account <acct>`   → the `connect_via_gh` trigger (gh token → keychain)
//! - `connect --provider github --keychain-ref <ref> --account <acct>` → `integration.connect` (risk-2)
//! - `set-live-writes --connection <id> --enabled <true|false>`        → `integration.set_live_writes` (risk-2)
//! - `create-pr --project <id> --head <b> --base <b> --title <t> [--body <b>]` → `github.create_pr` (risk-3)
//! - `merge-pr --repo <id> --pr <n> --sha <head> --method <merge|squash|rebase>` → `github.merge_pr` (risk-3)
//! - `submit-review --repo <id> --pr <n> --sha <commit> --event <approve|request_changes|comment> [--body]` → `github.submit_review` (risk-3)
//!
//! The risk-2/3 actions land at `awaiting_approval` (the user runs `smoke approve <id>` — the per-action
//! approval gate the 083 validation exercises stays visible). End-to-end CHAIN: connect-gh → connect →
//! approve → set-live-writes → approve → create-pr → approve → merge-pr/submit-review → approve.
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
use nexusops_shared::ids::{ActionRequestId, ProjectId};
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

const USAGE: &str = "usage: nexusopsd smoke <sub> [args]\n  \
    create --project <id> [--prompt \"<text>\"] [--profile <id>]\n  \
    queue\n  \
    approve <approval_id>\n  \
    deny <approval_id> <reason...>\n  \
    kill <session_id>\n  \
    audit\n  \
    --- 083 live-validation chain (each SUBMITS; then `smoke approve <id>`) ---\n  \
    connect-gh --provider github --account <acct>\n  \
    connect --provider github --keychain-ref <ref> --account <acct>\n  \
    set-live-writes --connection <connection_id> --enabled <true|false>\n  \
    create-pr --project <project_id> --head <branch> --base <branch> --title <t> [--body <b>]\n  \
    merge-pr --repo <repo_id> --pr <n> --sha <head_sha> --method <merge|squash|rebase>\n  \
    submit-review --repo <repo_id> --pr <n> --sha <commit_id> --event <approve|request_changes|comment> [--body <b>]\n  \
    CHAIN: connect-gh → connect → approve → set-live-writes → approve → create-pr → approve → merge-pr → approve";

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
        // 086 — the 083 live-validation chain (each a THIN wrapper over an EXISTING audited action).
        "connect-gh" => cmd_connect_gh(rest),
        "connect" => {
            build_integration_connect_request(rest).and_then(|r| submit("integration.connect", r))
        }
        "set-live-writes" => build_set_live_writes_request(rest)
            .and_then(|r| submit("integration.set_live_writes", r)),
        "create-pr" => build_create_pr_request(rest).and_then(|r| submit("github.create_pr", r)),
        "merge-pr" => build_merge_pr_request(rest).and_then(|r| submit("github.merge_pr", r)),
        "submit-review" => {
            build_submit_review_request(rest).and_then(|r| submit("github.submit_review", r))
        }
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

// ---- 086: the 083 live-validation chain — pure `build_*_request` helpers (the cmd_kill pattern) ----
//
// Each helper assembles a typed `ActionRequest` for an action that ALREADY EXISTS (no new mutation
// surface) — the daemon's policy/approval/executor/keychain pipeline runs UNCHANGED; the CLI is just a
// local IPC client. The recorded `risk_level` is a placeholder (`Level0`) — the §6.3 catalog reconciles
// it to the authoritative risk at submit (recorded-not-trusted, LESSON §19), so a wrong recorded risk
// can't bypass anything. The requester is `User` (UI/IPC, PIN e). A missing/invalid REQUIRED arg → a
// typed `Err(String)` (fail-closed CLI parse — never a malformed/partial submit).

/// the value following a required `flag`, or a typed error.
fn required(rest: &[String], flag: &str) -> Result<String, String> {
    flag_value(rest, flag).ok_or_else(|| format!("missing required {flag}\n{USAGE}"))
}

/// a base `ActionRequest` for `action_type` (the shared boilerplate every build_* helper fills in).
/// Returns `Result` so the placeholder-timestamp parse propagates via `?` (the cmd_kill pattern) rather
/// than `.expect()` — never a panic on the (impossible) parse failure.
fn smoke_request(
    action_type: &str,
    project_id: Option<ProjectId>,
    resource_refs: Vec<ResourceRef>,
    inputs: serde_json::Value,
) -> Result<ActionRequest, String> {
    Ok(ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id,
        action_type: action_type.to_string(),
        requester_type: RequesterType::User, // UI/IPC (PIN e)
        requester_id: "smoke".to_string(),
        resource_refs,
        inputs,
        // recorded-not-trusted (§15/LESSON §19) — the catalog reconciles to the authoritative risk at submit.
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        // a placeholder — the daemon's `request::insert` stamps `created_at` from its Clock.
        created_at: Timestamp::parse("1970-01-01T00:00:00Z")
            .map_err(|e| format!("placeholder timestamp: {e}"))?,
        preview: None,
    })
}

/// `integration.connect` (risk-2, edges-029) — REGISTRATION-ONLY: inputs carry the keychain_ref POINTER,
/// NEVER a token (LESSON §49). The connection identity is the inputs (`requires_resource_refs=false`).
pub fn build_integration_connect_request(rest: &[String]) -> Result<ActionRequest, String> {
    let provider = required(rest, "--provider")?;
    let keychain_ref = required(rest, "--keychain-ref")?;
    let account = required(rest, "--account")?;
    smoke_request(
        "integration.connect",
        None,
        vec![],
        serde_json::json!({ "provider": provider, "keychain_ref": keychain_ref, "account": account }),
    )
}

/// `integration.set_live_writes` (risk-2, 083) — the live-writes governance flip. `--enabled` is parsed
/// to a JSON BOOL (fail-closed on a non-bool — a mis-parsed enable is the one CLI footgun worth pinning).
pub fn build_set_live_writes_request(rest: &[String]) -> Result<ActionRequest, String> {
    let connection_id = required(rest, "--connection")?;
    let enabled: bool = required(rest, "--enabled")?
        .parse()
        .map_err(|_| "--enabled must be `true` or `false`".to_string())?;
    smoke_request(
        "integration.set_live_writes",
        None,
        vec![],
        serde_json::json!({ "connection_id": connection_id, "enabled": enabled }),
    )
}

/// `github.create_pr` (risk-3, D-series) — the audited create TARGET is the envelope `project_id` (the
/// executor's `resolve_repo_target` reads `req.project_id`, LESSON §63); a Project resource_ref carries
/// the same id for the catalog `requires_resource_refs`. `--body` is optional.
pub fn build_create_pr_request(rest: &[String]) -> Result<ActionRequest, String> {
    let project = required(rest, "--project")?;
    // parse the project id at CLI-submit time → a clear `--project: <parse error>` here, NOT a confusing
    // execute-time "missing the audited project_id" from the daemon on a malformed id (the resolve target).
    let project_id = ProjectId::parse(&project).map_err(|e| format!("--project: {e}"))?;
    let head = required(rest, "--head")?;
    let base = required(rest, "--base")?;
    let title = required(rest, "--title")?;
    let mut inputs = serde_json::json!({ "head": head, "base": base, "title": title });
    if let Some(body) = flag_value(rest, "--body") {
        inputs["body"] = serde_json::Value::String(body);
    }
    smoke_request(
        "github.create_pr",
        Some(project_id.clone()),
        vec![ResourceRef {
            resource_type: ResourceType::Project,
            id: project_id.as_str().to_string(),
            uri: None,
        }],
        inputs,
    )
}

/// `github.merge_pr` (risk-3, D9/LESSON §60) — the audited target is the Repo resource_ref (`resolve_pr_
/// target`, LESSON §63/082); `--sha` is the anti-race head-pin (LESSON §60). `--pr` parses to a u64
/// (fail-closed on non-numeric — never a malformed pr_number).
pub fn build_merge_pr_request(rest: &[String]) -> Result<ActionRequest, String> {
    let repo = required(rest, "--repo")?;
    let pr_number: u64 = required(rest, "--pr")?
        .parse()
        .map_err(|_| "--pr must be a positive integer".to_string())?;
    let sha = required(rest, "--sha")?;
    let merge_method = required(rest, "--method")?;
    smoke_request(
        "github.merge_pr",
        None,
        vec![ResourceRef {
            resource_type: ResourceType::Repo,
            id: repo,
            uri: None,
        }],
        serde_json::json!({ "pr_number": pr_number, "sha": sha, "merge_method": merge_method }),
    )
}

/// `github.submit_review` (risk-3, D10/LESSON §61) — the Repo resource_ref is the audited target; `--sha`
/// is the reviewed-head pin (the executor's `commit_id`). `--body` optional (GitHub requires it for
/// request_changes/comment — the daemon enforces that; the CLI just forwards).
pub fn build_submit_review_request(rest: &[String]) -> Result<ActionRequest, String> {
    let repo = required(rest, "--repo")?;
    let pr_number: u64 = required(rest, "--pr")?
        .parse()
        .map_err(|_| "--pr must be a positive integer".to_string())?;
    let commit_id = required(rest, "--sha")?;
    let event = required(rest, "--event")?;
    let mut inputs =
        serde_json::json!({ "pr_number": pr_number, "commit_id": commit_id, "event": event });
    if let Some(body) = flag_value(rest, "--body") {
        inputs["body"] = serde_json::Value::String(body);
    }
    smoke_request(
        "github.submit_review",
        None,
        vec![ResourceRef {
            resource_type: ResourceType::Repo,
            id: repo,
            uri: None,
        }],
        inputs,
    )
}

// ---- 086: the subcommand arms (THIN — build → submit the EXISTING action) -----------------------

/// `connect-gh --provider github --account <acct>` → the EXISTING `connect_via_gh` IPC trigger (the
/// daemon sources the `gh` token → keychain; NO token printed). Prints the keychain_ref / gh_unavailable.
fn cmd_connect_gh(rest: &[String]) -> Result<String, String> {
    let provider = required(rest, "--provider")?;
    let account = required(rest, "--account")?;
    let result = call(
        "connect_via_gh",
        serde_json::json!({ "provider": provider, "account": account }),
    )?;
    Ok(format!("connect_via_gh →\n{}", pretty(&result)))
}

/// Submit a built `ActionRequest` via the GENERIC `submit_action` method (the cmd_kill pattern) — the
/// daemon's pipeline gates it (risk-2/3 → awaiting_approval; the user then `smoke approve <id>`).
fn submit(label: &str, req: ActionRequest) -> Result<String, String> {
    let params = serde_json::to_value(&req).map_err(|e| format!("encode {label}: {e}"))?;
    let result = call("submit_action", params)?;
    Ok(format!("{label} submitted →\n{}", pretty(&result)))
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
