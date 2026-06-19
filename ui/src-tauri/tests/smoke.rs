//! Live smoke for the read-command bridge (P6.8 L1 slice 2/3) — `#[ignore]`, needs a RUNNING
//! daemon bound to the app-support `gateway.sock`. Run explicitly:
//!
//! ```sh
//! cargo test -p nexusops-ui --test smoke -- --ignored
//! ```
//!
//! It exercises the SAME transport path `gateway_get_capabilities` runs (the 049 crate's
//! `connect_and_call`). The full `invoke("gateway_get_capabilities")` path (frontend → the
//! #[tauri::command] → here) needs a running Tauri app, so it's the documented `pnpm tauri dev`
//! manual step (a Tauri command can't be driven from a plain integration test without an app).
//! The bridge's deterministic value-add (param marshaling + the ClientError→error mapping) is
//! unit-tested in `src/commands.rs`.

#[test]
#[ignore = "needs a running daemon bound to the app-support gateway.sock"]
fn smoke_get_capabilities_roundtrips() {
    let caps = nexusops_gateway_uds::connect_and_call("get_capabilities", serde_json::json!({}))
        .expect("live get_capabilities should round-trip against a running daemon");
    assert!(
        caps.get("contract_version").and_then(|v| v.as_str()).is_some(),
        "get_capabilities should return a Capabilities value with a string contract_version, got: {caps}"
    );
}
