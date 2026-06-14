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

use nexusops_gateway_uds::{connect_and_call, connect_and_subscribe, ClientError};
use nexusops_shared::ipc::ProjectionName;
use std::ops::ControlFlow;

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

#[test]
#[ignore = "needs a running daemon + a live mutation to emit a Session delta; exercises the persistent subscribe connection"]
fn connect_and_subscribe_session_live() {
    // 052 — the dedicated persistent subscribe connection against the real daemon. This blocks on
    // the push stream until the daemon closes it on lag (no read timeout, by design) OR the sink
    // breaks; an operator drives a Session change to observe a delta. Counts the deltas seen, stops
    // after the first (Break) so the test terminates without needing the daemon to lag-close.
    let mut seen = 0;
    let result = connect_and_subscribe(ProjectionName::Session, |_delta| {
        seen += 1;
        ControlFlow::Break(()) // stop after the first delta (the live probe; no operator wait loop)
    });
    match result {
        Ok(()) => {
            // Ok = a clean end (the sink broke after a delta, OR the daemon closed on lag). With a
            // live Session mutation, `seen` should be ≥1; without one it ends on the daemon's close.
            println!("subscribe live: clean end, deltas observed = {seen}");
        }
        Err(ClientError::Io(e)) => {
            panic!("could not connect to the daemon (is nexusopsd running?): {e}");
        }
        Err(e) => panic!("live subscribe failed: {e}"),
    }
}
