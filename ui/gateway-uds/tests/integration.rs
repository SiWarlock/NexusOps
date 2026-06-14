//! Live-socket integration for the UDS read-client (P6.8 L1).
//!
//! `#[ignore]` by default — it needs a RUNNING daemon (`nexusopsd`) bound to the app-support
//! `gateway.sock`. Run explicitly against a live daemon:
//!
//! ```sh
//! cargo test -p nexusops-gateway-uds --test integration -- --ignored
//! ```
//!
//! The deterministic transport core (codec/handshake/demux/call) is fully covered by the
//! in-crate unit tests over a fake stream; this exercises the thin real-`UnixStream` adapter
//! (connect → handshake → call → drop) end-to-end against the real daemon.

use nexusops_gateway_uds::{connect_and_call, ClientError};

#[test]
#[ignore = "needs a running daemon bound to the app-support gateway.sock"]
fn connect_and_call_get_capabilities_live() {
    // get_capabilities is a no-param read RPC (methods.rs) — a safe liveness probe against a
    // real daemon: the result carries the daemon's protocol + contract version.
    match connect_and_call("get_capabilities", serde_json::json!({})) {
        Ok(caps) => {
            assert!(
                caps.get("contract_version").is_some(),
                "live get_capabilities should return a Capabilities value, got: {caps}"
            );
        }
        Err(ClientError::Io(e)) => {
            panic!("could not connect to the daemon (is nexusopsd running?): {e}");
        }
        Err(e) => panic!("live get_capabilities failed: {e}"),
    }
}
