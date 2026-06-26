//! The read-command bridge (P6.8 L1 slice 2/3) — a NARROW typed allowlist exposing the daemon's
//! READ methods to the frontend. One #[tauri::command] per read method (get_projection / get_diff
//! / get_capabilities) — NEVER a generic gateway_call (which would let the frontend invoke any
//! daemon method incl. the L2 mutations). Each command marshals the params, calls the 049
//! nexusops-gateway-uds transport crate's `connect_and_call`, and returns the raw daemon JSON
//! `Value` (the TS layer Zod-parses it at 051) or a serializable, leak-free `GatewayCommandError`.
//! L2-B: the bridge now ALSO carries the 4 §6.1 MUTATION commands (submit_action / preview_action /
//! approve / deny) — one typed #[tauri::command] per method (the NARROW allowlist; STILL no generic
//! gateway_call). Each marshals params + calls the SAME `call_daemon` (verbatim §6.4 `Wire{code}` via
//! `map_client_error`). The live mutation path is gated OFF on the TS side (`mutationsEnabled=false`)
//! until L2-C — the UI still never mutates; the daemon Gateway is the INV-SEC-1 chokepoint.
//!
//! The TDD core is the two PURE fns (`map_client_error` + the param marshaling); the
//! #[tauri::command] wrappers + the socket connect are infra (the gated smoke test covers the
//! round-trip end-to-end against a real daemon).

use nexusops_gateway_uds::{connect_and_call, connect_and_subscribe, ClientError};
use nexusops_shared::actions::ActionRequest;
use nexusops_shared::ipc::{
    GetDiffParams, GetPrDiffParams, GetProjectionParams, IpcErrorCode, ProjectionName,
    ProjectionScope,
};
use serde::Serialize;
use serde_json::Value;
use std::ops::ControlFlow;
use tauri::ipc::Channel;

/// A serializable (→ JS), leak-free command error. `Wire` carries the verbatim §6.4 code so the
/// 051 TS `describeRejection` routes it; the other variants carry only structural info (sizes /
/// versions / an error Display string) — reads-only, so no daemon payload / path / secret crosses.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GatewayCommandError {
    /// the daemon rejected the read with a structured §6.4 code (verbatim wire value).
    Wire {
        code: String,
    },
    VersionSkew {
        supported_min: u32,
        supported_max: u32,
        client_protocol_version: u32,
    },
    FrameTooLarge {
        declared: usize,
        max: usize,
    },
    /// a transport/io fault (a structural Display string — no payload).
    Io {
        message: String,
    },
    /// a wire/frame-discipline violation.
    Protocol {
        message: String,
    },
    /// a (de)serialize fault at the boundary.
    Serde {
        message: String,
    },
    /// a bridge-INTERNAL fault (e.g. the blocking transport task failed to join) — a host
    /// runtime fault, NOT a daemon/wire error (kept distinct so the TS router never mistakes it
    /// for a §6.4 protocol/wire violation).
    Internal {
        message: String,
    },
}

/// The §6.4 snake_case wire string for an `IpcErrorCode` (e.g. `NotFound` → `"not_found"`), via the
/// frozen serde repr — so the code is carried VERBATIM, never hand-mapped (a hand map would drift).
fn wire_code_str(code: IpcErrorCode) -> String {
    // IpcErrorCode is a flat #[serde(rename_all="snake_case")] enum → serializes to a JSON string
    // today. The fallback fires only if it ever becomes a non-flat (tuple/struct) variant (→ a
    // JSON object); "internal_error" is itself a valid §6.4 code, so the TS router still handles
    // it. No panic either way.
    serde_json::to_value(code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "internal_error".to_string())
}

/// Map the 049 transport `ClientError` → the serializable frontend error. The §6.4 `Wire` code is
/// VERBATIM; the other variants carry only structural info (reads-only — nothing sensitive).
pub fn map_client_error(e: ClientError) -> GatewayCommandError {
    match e {
        ClientError::Wire(code) => GatewayCommandError::Wire {
            code: wire_code_str(code),
        },
        ClientError::VersionSkew {
            supported_min,
            supported_max,
            client_protocol_version,
        } => GatewayCommandError::VersionSkew {
            supported_min,
            supported_max,
            client_protocol_version,
        },
        ClientError::FrameTooLarge { declared, max } => {
            GatewayCommandError::FrameTooLarge { declared, max }
        }
        ClientError::Io(err) => GatewayCommandError::Io {
            message: err.to_string(),
        },
        ClientError::Protocol(message) => GatewayCommandError::Protocol { message },
        ClientError::Serde(err) => GatewayCommandError::Serde {
            message: err.to_string(),
        },
    }
}

/// Marshal `get_projection` params == the daemon's `GetProjectionParams{name,scope?}` (methods.rs).
pub fn get_projection_params(
    name: ProjectionName,
    scope: Option<ProjectionScope>,
) -> Result<Value, GatewayCommandError> {
    serde_json::to_value(GetProjectionParams { name, scope }).map_err(|e| {
        GatewayCommandError::Serde {
            message: e.to_string(),
        }
    })
}

/// Marshal `get_diff` params == the daemon's `GetDiffParams{worktree_id,file}`.
pub fn get_diff_params(worktree_id: String, file: String) -> Result<Value, GatewayCommandError> {
    serde_json::to_value(GetDiffParams { worktree_id, file }).map_err(|e| {
        GatewayCommandError::Serde {
            message: e.to_string(),
        }
    })
}

/// Marshal `get_pr_diff` params == the daemon's `GetPrDiffParams{repo_id,pr_number,file}` (D7).
pub fn get_pr_diff_params(
    repo_id: String,
    pr_number: u64,
    file: Option<String>,
) -> Result<Value, GatewayCommandError> {
    serde_json::to_value(GetPrDiffParams {
        repo_id,
        pr_number,
        file,
    })
    .map_err(|e| GatewayCommandError::Serde {
        message: e.to_string(),
    })
}

// ── the L2-B mutation param marshaling (== the daemon's methods.rs shapes; L2-D1/O4 opaque) ────

/// Marshal `submit_action` params: the `ActionRequest` serialized AS-IS (its idempotency_key /
/// fencing_token / resource_refs ride OPAQUELY to the daemon, which owns dedup+fencing — L2-O4).
pub fn submit_action_params(request: ActionRequest) -> Result<Value, GatewayCommandError> {
    serde_json::to_value(request).map_err(|e| GatewayCommandError::Serde {
        message: e.to_string(),
    })
}

/// Marshal `preview_action` params == the daemon's `{action_request_id}` (methods.rs:441).
pub fn preview_action_params(action_request_id: String) -> Value {
    serde_json::json!({ "action_request_id": action_request_id })
}

/// Marshal `approve` params == the daemon's `{approval_id, step_id?}` (methods.rs:212; step_id is
/// accepted-but-RESERVED in 2.1c). This Rust marshal OMITS the key when `None`; the TS caller sends
/// `stepId: null` — both map to the daemon's `Option<String>` `None` (Tauri deserializes JSON null → None).
pub fn approve_params(approval_id: String, step_id: Option<String>) -> Value {
    match step_id {
        Some(step) => serde_json::json!({ "approval_id": approval_id, "step_id": step }),
        None => serde_json::json!({ "approval_id": approval_id }),
    }
}

/// Marshal `deny` params == the daemon's `{approval_id, reason}` (methods.rs:227).
pub fn deny_params(approval_id: String, reason: String) -> Value {
    serde_json::json!({ "approval_id": approval_id, "reason": reason })
}

/// Marshal `session.create` params == the daemon's flat `{project_id (required), initial_prompt?,
/// execution_profile_id?}` (methods.rs:340 build_session_create_request). Absent optionals are
/// OMITTED (not null), matching the daemon's manual `.get()` param handling; the daemon mints the id.
pub fn create_session_params(
    project_id: String,
    initial_prompt: Option<String>,
    execution_profile_id: Option<String>,
) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("project_id".to_string(), Value::String(project_id));
    if let Some(prompt) = initial_prompt {
        m.insert("initial_prompt".to_string(), Value::String(prompt));
    }
    if let Some(profile) = execution_profile_id {
        m.insert("execution_profile_id".to_string(), Value::String(profile));
    }
    Value::Object(m)
}

// ── the #[tauri::command] read allowlist (registered in lib.rs run()) ─────────────────────────

#[tauri::command]
pub async fn gateway_get_projection(
    name: ProjectionName,
    scope: Option<ProjectionScope>,
) -> Result<Value, GatewayCommandError> {
    let params = get_projection_params(name, scope)?;
    call_daemon("get_projection", params).await
}

#[tauri::command]
pub async fn gateway_get_diff(
    worktree_id: String,
    file: String,
) -> Result<Value, GatewayCommandError> {
    let params = get_diff_params(worktree_id, file)?;
    call_daemon("get_diff", params).await
}

#[tauri::command]
pub async fn gateway_get_pr_diff(
    repo_id: String,
    pr_number: u64,
    file: Option<String>,
) -> Result<Value, GatewayCommandError> {
    let params = get_pr_diff_params(repo_id, pr_number, file)?;
    call_daemon("get_pr_diff", params).await
}

#[tauri::command]
pub async fn gateway_get_capabilities() -> Result<Value, GatewayCommandError> {
    call_daemon("get_capabilities", serde_json::json!({})).await
}

// ── the #[tauri::command] L2 MUTATION allowlist (registered in lib.rs; STILL no gateway_call) ──
// One typed command per §6.1 mutation method (L2-D2). Each marshals + calls the SAME call_daemon —
// so the verbatim §6.4 Wire{code} rides through map_client_error identically to the reads (L2-D6). The
// UI never mutates: a command SENDS a typed intent; the daemon Gateway is the INV-SEC-1 chokepoint
// (L2-D1 pure pass-through). The live path is gated OFF on the TS side until L2-C.

#[tauri::command]
pub async fn gateway_submit_action(request: ActionRequest) -> Result<Value, GatewayCommandError> {
    let params = submit_action_params(request)?;
    call_daemon("submit_action", params).await
}

#[tauri::command]
pub async fn gateway_preview_action(
    action_request_id: String,
) -> Result<Value, GatewayCommandError> {
    call_daemon("preview_action", preview_action_params(action_request_id)).await
}

#[tauri::command]
pub async fn gateway_approve(
    approval_id: String,
    step_id: Option<String>,
) -> Result<Value, GatewayCommandError> {
    call_daemon("approve", approve_params(approval_id, step_id)).await
}

#[tauri::command]
pub async fn gateway_deny(
    approval_id: String,
    reason: String,
) -> Result<Value, GatewayCommandError> {
    call_daemon("deny", deny_params(approval_id, reason)).await
}

#[tauri::command]
pub async fn gateway_create_session(
    project_id: String,
    initial_prompt: Option<String>,
    execution_profile_id: Option<String>,
) -> Result<Value, GatewayCommandError> {
    call_daemon(
        "session.create",
        create_session_params(project_id, initial_prompt, execution_profile_id),
    )
    .await
}

/// A frame streamed to the frontend over the subscribe `Channel` (052). A tagged enum so the TS
/// AsyncIterable yields-vs-ends-vs-errors UNAMBIGUOUSLY (§11.7): `Delta` carries the raw daemon
/// `ProjectionDelta` (the TS `parseDelta`-validates it at the boundary — parse-don't-trust); `Closed`
/// is the daemon's clean lag-close (→ the iterable ends → the Shell recovery reconnects); `Error`
/// carries the leak-free §6.4 `GatewayCommandError` (→ the iterable surfaces it). NOT a shared
/// contract — a ui-host-local marshaling type.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscriptionEvent {
    Delta { delta: Value },
    Closed,
    Error { error: GatewayCommandError },
}

/// Subscribe to a projection's live delta stream (052) — the dedicated persistent connection. Spawns
/// the blocking [`connect_and_subscribe`] off the async runtime (the 050 `spawn_blocking` precedent),
/// sending each delta over the Tauri `Channel`; on the daemon's lag-close → `Closed`, on a transport
/// fault → `Error` (distinct, §11.7). If the frontend drops the `Channel` (`send` errors), the sink
/// returns `Break` → the blocking read loop ends + the stream is dropped (NO leaked subscription
/// thread — the teardown/recovery path). Typed-narrow allowlist (registered in `lib.rs`); still NO
/// generic `gateway_call` / mutation command (LESSON 21 — reads-only, INV-SEC-1 daemon-side).
#[tauri::command]
pub async fn gateway_subscribe(
    projection: ProjectionName,
    on_event: Channel<SubscriptionEvent>,
) -> Result<(), GatewayCommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = connect_and_subscribe(projection, |delta| {
            // a ProjectionDelta is plain serde-derive data over JSON-compatible fields → serializing
            // it is infallible; expect (over a null/empty fallback) so a structural fault surfaces as
            // an Internal join-error, never a silent `delta: null` masquerading as a live delta.
            let value =
                serde_json::to_value(&delta).expect("a ProjectionDelta always serializes to JSON");
            match on_event.send(SubscriptionEvent::Delta { delta: value }) {
                Ok(()) => ControlFlow::Continue(()),
                // the frontend dropped the channel → stop reading (no leaked subscription thread).
                Err(_) => ControlFlow::Break(()),
            }
        });
        // signal the stream end DISTINCTLY: a clean close (lag/EOF) vs a transport error. A failed
        // send here just means the frontend already went away — nothing to do.
        match result {
            Ok(()) => {
                let _ = on_event.send(SubscriptionEvent::Closed);
            }
            Err(e) => {
                let _ = on_event.send(SubscriptionEvent::Error {
                    error: map_client_error(e),
                });
            }
        }
    })
    .await
    .map_err(|e| GatewayCommandError::Internal {
        message: format!("subscribe task failed to join: {e}"),
    })
}

/// Run the BLOCKING 049 `connect_and_call` off the async runtime (it uses a std `UnixStream`) and
/// map any `ClientError` → the serializable command error.
async fn call_daemon(method: &'static str, params: Value) -> Result<Value, GatewayCommandError> {
    tauri::async_runtime::spawn_blocking(move || connect_and_call(method, params))
        .await
        // a join failure is a host runtime fault (the task panicked/was cancelled), NOT a wire
        // violation → Internal, never Protocol.
        .map_err(|e| GatewayCommandError::Internal {
            message: format!("bridge task failed to join: {e}"),
        })?
        .map_err(map_client_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind_of(e: &GatewayCommandError) -> String {
        serde_json::to_value(e).unwrap()["kind"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn client_error_maps_wire_to_kind_wire_verbatim_code() {
        // spec(§6.4) — Wire(NotFound) → {kind:"wire", code:"not_found"} (the verbatim §6.4 code so
        // the 051 TS describeRejection routes it; never hand-mapped/collapsed).
        let v = serde_json::to_value(map_client_error(ClientError::Wire(IpcErrorCode::NotFound)))
            .unwrap();
        assert_eq!(v["kind"], "wire");
        assert_eq!(v["code"], "not_found");
    }

    #[test]
    fn client_error_maps_each_variant_to_distinct_kind() {
        // spec(§6.4/§11.7) — each ClientError variant → a DISTINCT serialized kind; none collapse.
        let cases = [
            map_client_error(ClientError::Wire(IpcErrorCode::PolicyDenied)),
            map_client_error(ClientError::VersionSkew {
                supported_min: 1,
                supported_max: 1,
                client_protocol_version: 2,
            }),
            map_client_error(ClientError::FrameTooLarge {
                declared: 99,
                max: 10,
            }),
            map_client_error(ClientError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "refused",
            ))),
            map_client_error(ClientError::Protocol("bad frame".to_string())),
            map_client_error(ClientError::Serde(
                serde_json::from_str::<Value>("{").unwrap_err(),
            )),
        ];
        let kinds: Vec<String> = cases.iter().map(kind_of).collect();
        let unique: std::collections::HashSet<&String> = kinds.iter().collect();
        assert_eq!(
            unique.len(),
            kinds.len(),
            "every variant must map to a distinct kind"
        );
        for expected in [
            "wire",
            "version_skew",
            "frame_too_large",
            "io",
            "protocol",
            "serde",
        ] {
            assert!(
                kinds.iter().any(|k| k == expected),
                "missing kind {expected}"
            );
        }
    }

    #[test]
    fn get_projection_params_match_daemon() {
        // spec(§6.1/§5.0) — the marshaled params round-trip into the daemon's frozen
        // GetProjectionParams{name,scope?} (deny_unknown_fields → an exact field-set match).
        let params = get_projection_params(ProjectionName::Session, None).unwrap();
        assert_eq!(params["name"], "Session");
        let parsed: GetProjectionParams = serde_json::from_value(params).unwrap();
        assert!(matches!(parsed.name, ProjectionName::Session));
    }

    #[test]
    fn get_diff_params_match_daemon() {
        // spec(§6.1) — == the daemon's frozen GetDiffParams{worktree_id,file}.
        let params = get_diff_params("wt_1".to_string(), "a.ts".to_string()).unwrap();
        let parsed: GetDiffParams = serde_json::from_value(params).unwrap();
        assert_eq!(parsed.worktree_id, "wt_1");
        assert_eq!(parsed.file, "a.ts");
    }

    #[test]
    fn get_pr_diff_params_match_daemon() {
        // spec(§6.1) — == the daemon's frozen GetPrDiffParams{repo_id,pr_number,file} (D7).
        let params =
            get_pr_diff_params("repo_1".to_string(), 101, Some("a.ts".to_string())).unwrap();
        let parsed: GetPrDiffParams = serde_json::from_value(params).unwrap();
        assert_eq!(parsed.repo_id, "repo_1");
        assert_eq!(parsed.pr_number, 101);
        assert_eq!(parsed.file.as_deref(), Some("a.ts"));
        // the whole-changeset form (file=None) marshals to an explicit null field, not a missing key.
        let whole = get_pr_diff_params("repo_1".to_string(), 101, None).unwrap();
        assert_eq!(whole["file"], serde_json::Value::Null);
    }

    #[test]
    fn create_session_params_match_daemon() {
        // spec(§6.1) — flat session.create params {project_id (required), initial_prompt?,
        // execution_profile_id?}; absent optionals are OMITTED (not null), matching the daemon's
        // build_session_create_request (project_id is the catalog resource_ref; the daemon mints the id).
        let p = create_session_params("proj_1".to_string(), Some("hi".to_string()), None);
        assert_eq!(p["project_id"], "proj_1");
        assert_eq!(p["initial_prompt"], "hi");
        assert!(p.get("execution_profile_id").is_none());
        // absent prompt → omitted (not null)
        let bare = create_session_params("proj_2".to_string(), None, None);
        assert_eq!(bare["project_id"], "proj_2");
        assert!(bare.get("initial_prompt").is_none());
    }

    #[test]
    fn subscription_event_maps_delta_close_error_distinctly() {
        // spec(§11.7) — the 3 Channel payload variants serialize to DISTINCT kinds (delta/closed/
        // error) so the TS iterable yields-vs-ends-vs-errors unambiguously; none collapse, the error
        // carries the verbatim §6.4 code (leak-free — kind/code only, the same shape map_client_error
        // produces for the single-shot reads).
        let delta = serde_json::to_value(SubscriptionEvent::Delta {
            delta: serde_json::json!({ "projection": "Session", "kind": "upsert" }),
        })
        .unwrap();
        let closed = serde_json::to_value(SubscriptionEvent::Closed).unwrap();
        let err = serde_json::to_value(SubscriptionEvent::Error {
            error: map_client_error(ClientError::Wire(IpcErrorCode::NotFound)),
        })
        .unwrap();

        assert_eq!(delta["kind"], "delta");
        assert_eq!(closed["kind"], "closed");
        assert_eq!(err["kind"], "error");
        let kinds = [
            delta["kind"].as_str().unwrap(),
            closed["kind"].as_str().unwrap(),
            err["kind"].as_str().unwrap(),
        ];
        let unique: std::collections::HashSet<&str> = kinds.iter().copied().collect();
        assert_eq!(unique.len(), 3, "the 3 payload variants must be distinct");
        // the nested error is the verbatim §6.4 code (the 050 wire-code path), nothing else leaked.
        assert_eq!(err["error"]["kind"], "wire");
        assert_eq!(err["error"]["code"], "not_found");
    }

    // ─── L2-B — the mutation bridge param marshaling (the TDD core; the commands are infra) ────
    // The 4 mutation commands marshal params == the daemon's methods.rs shapes + reuse call_daemon +
    // map_client_error (the verbatim §6.4 code is shared with the reads). The wire is built here but
    // gated OFF on the TS side (mutationsEnabled=false) until L2-C.

    fn sample_action_request() -> nexusops_shared::actions::ActionRequest {
        use nexusops_shared::actions::{RequesterType, ResourceRef, ResourceType, RiskLevel};
        use nexusops_shared::ids::ActionRequestId;
        use nexusops_shared::status::ActionRequest as ActionRequestStatus;
        use nexusops_shared::time::Timestamp;
        nexusops_shared::actions::ActionRequest {
            action_request_id: ActionRequestId::new(),
            project_id: None,
            action_type: "git.stage_hunk".to_string(),
            requester_type: RequesterType::User,
            requester_id: "ui".to_string(),
            resource_refs: vec![ResourceRef {
                resource_type: ResourceType::File,
                id: "wt_1\u{1f}a.ts\u{1f}1,1,1,1".to_string(),
                uri: None,
            }],
            inputs: serde_json::json!({}),
            risk_level: RiskLevel::Level2,
            idempotency_key: Some("idem-abc".to_string()),
            fencing_token: Some(42),
            status: ActionRequestStatus::Submitted,
            preview: None,
            created_at: Timestamp::parse("2026-06-14T00:00:00Z").unwrap(),
        }
    }

    #[test]
    fn submit_action_params_match_daemon() {
        // spec(§6.1/L2-D1/O4) — submit_action_params serializes the ActionRequest AS-IS; it round-trips
        // back into the daemon's frozen ActionRequest (no field dropped/reshaped — opaque pass-through;
        // idempotency_key/fencing_token ride to the daemon, which owns dedup+fencing).
        let req = sample_action_request();
        let params = submit_action_params(req.clone()).unwrap();
        assert_eq!(params["action_type"], "git.stage_hunk");
        assert_eq!(params["idempotency_key"], "idem-abc"); // rides opaquely (L2-O4)
        assert_eq!(params["fencing_token"], 42);
        let parsed: nexusops_shared::actions::ActionRequest =
            serde_json::from_value(params).unwrap();
        assert_eq!(parsed, req);
    }

    #[test]
    fn approve_deny_preview_params_match_daemon() {
        // spec(§6.1) — the marshal fns produce the EXACT daemon-expected param shapes (methods.rs):
        // approve={approval_id,step_id?} · deny={approval_id,reason} · preview={action_request_id}.
        let approve_some = approve_params("appr_1".to_string(), Some("step_2".to_string()));
        assert_eq!(approve_some["approval_id"], "appr_1");
        assert_eq!(approve_some["step_id"], "step_2");
        let approve_none = approve_params("appr_1".to_string(), None);
        assert_eq!(approve_none["approval_id"], "appr_1");
        assert!(
            approve_none.get("step_id").is_none(),
            "step_id absent (not null) when None"
        );
        let deny = deny_params("appr_9".to_string(), "no".to_string());
        assert_eq!(deny["approval_id"], "appr_9");
        assert_eq!(deny["reason"], "no");
        let preview = preview_action_params("act_xyz".to_string());
        assert_eq!(preview["action_request_id"], "act_xyz");
    }

    #[test]
    fn mutation_commands_reuse_verbatim_wire_code() {
        // spec(L2-D6/§6.4) — map_client_error (shared with the reads) carries the §6.4 code VERBATIM on
        // the mutation path: fencing_conflict / precondition_stale stay DISTINCT (→ the §11.5 cards —
        // a collapsed code would break #6's never-auto-resolved hard-conflict at L2-C).
        let fc = serde_json::to_value(map_client_error(ClientError::Wire(
            IpcErrorCode::FencingConflict,
        )))
        .unwrap();
        assert_eq!(fc["kind"], "wire");
        assert_eq!(fc["code"], "fencing_conflict");
        let ps = serde_json::to_value(map_client_error(ClientError::Wire(
            IpcErrorCode::PreconditionStale,
        )))
        .unwrap();
        assert_eq!(ps["code"], "precondition_stale");
    }
}
