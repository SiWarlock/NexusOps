//! The read-command bridge (P6.8 L1 slice 2/3) — a NARROW typed allowlist exposing the daemon's
//! READ methods to the frontend. One #[tauri::command] per read method (get_projection / get_diff
//! / get_capabilities) — NEVER a generic gateway_call (which would let the frontend invoke any
//! daemon method incl. the L2 mutations). Each command marshals the params, calls the 049
//! nexusops-gateway-uds transport crate's `connect_and_call`, and returns the raw daemon JSON
//! `Value` (the TS layer Zod-parses it at 051) or a serializable, leak-free `GatewayCommandError`.
//! NON-cat-1 (reads only) — no submit_action/approve/deny; INV-SEC-1 stays daemon-side.
//!
//! The TDD core is the two PURE fns (`map_client_error` + the param marshaling); the
//! #[tauri::command] wrappers + the socket connect are infra (the gated smoke test covers the
//! round-trip end-to-end against a real daemon).

use nexusops_gateway_uds::{connect_and_call, ClientError};
use nexusops_shared::ipc::{
    GetDiffParams, GetProjectionParams, IpcErrorCode, ProjectionName, ProjectionScope,
};
use serde::Serialize;
use serde_json::Value;

/// A serializable (→ JS), leak-free command error. `Wire` carries the verbatim §6.4 code so the
/// 051 TS `describeRejection` routes it; the other variants carry only structural info (sizes /
/// versions / an error Display string) — reads-only, so no daemon payload / path / secret crosses.
#[derive(Debug, Serialize)]
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
pub async fn gateway_get_capabilities() -> Result<Value, GatewayCommandError> {
    call_daemon("get_capabilities", serde_json::json!({})).await
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
}
