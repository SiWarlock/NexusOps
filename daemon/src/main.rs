//! nexusopsd — the daemon runtime entry (§12 / §16).
//!
//! The `#[tokio::main]` production entry: resolve the app-support dir → `cold_start()` (1.6a:
//! pidlock → migrate → version-floor) → stand up the single write-actor → (L2: drainer/reaper
//! loops · L3: UDS accept-loop · L4: subscribe fanout) → block on a shutdown signal → graceful
//! drain + exit (the held `PidLock` releases on exit). This is the real production caller that
//! closes the 1.3/1.4/1.5/1.6a "ship the mechanism, wire the runtime at 1.6" reachability chain.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use nexusopsd::bootstrap::{cold_start, BootstrapConfig, DB_FILENAME};
use nexusopsd::clock::SystemClock;
use nexusopsd::eventstore::{JsonlMirror, PrefixRedactor};
use nexusopsd::gateway::executor::CatalogExecutor;
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use nexusopsd::ipc::current_euid;
use nexusopsd::runtime::{bind, spawn_accept_loop, spawn_drainer, spawn_reaper, WriteActor};
use nexusopsd::session::spawn_supervisor_task;

/// outbox drain cadence (§12) — deliver due rows a few times a minute.
const DRAINER_INTERVAL: Duration = Duration::from_secs(5);
/// lease reap cadence (§17) — free expired leases periodically.
const REAPER_INTERVAL: Duration = Duration::from_secs(30);
/// the local JSONL audit/debug mirror sink (§10.4) within the app-support dir.
const EVENTS_MIRROR_FILE: &str = "events.jsonl";
/// the GatewayPort UDS within the app-support dir (§6.4).
const SOCKET_FILE: &str = "gateway.sock";
/// max concurrent live GatewayPort connections (anti-DoS bound, §6.4).
const MAX_CONNECTIONS: usize = 64;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("nexusopsd: fatal: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = production_base_dir()?;
    let cfg = BootstrapConfig {
        base_dir: base_dir.clone(),
        idgen: Box::new(UlidGen),
        clock: Box::new(SystemClock),
        redactor: Box::new(PrefixRedactor),
    };
    let ctx = cold_start(cfg)?;
    eprintln!(
        "nexusopsd: started (contract {}, db user_version {})",
        ctx.version.contract_version, ctx.version.db_user_version
    );

    // keep the PidLock bound for the daemon lifetime (single-instance); the write-actor owns the
    // writable store (the sole mutation path). The drainer/reaper loops + (L3) the UDS accept-loop
    // run off the write-actor handle.
    let (_pidlock, store, _version) = ctx.into_parts();
    // the Action Gateway (the sole mutator) runs its pipeline ON the write-actor thread (forbidden
    // #2/#3). 2.2: CatalogPolicy (the §6.3 catalog-authoritative risk engine — risk-0 auto-allows,
    // 1-3 require approval, 4 require step-approval, uncatalogued fail-closed deny). 2.3:
    // CatalogExecutor (validates requires_resource_refs + dispatches by ExecutorKind to side-effect-
    // free per-namespace stubs — real adapters land Phase 3/5/7/8).
    let gateway = Gateway::new(Box::new(CatalogPolicy), Box::new(CatalogExecutor));
    let actor = WriteActor::spawn(store, Box::new(SystemClock), gateway);
    let handle = actor.handle();
    // the post-commit broadcast sender for the accept-loop's per-connection live subscribers (1.6d);
    // captured before `handle` is moved into the reaper below.
    let deltas = handle.delta_sender();

    // L2 — the outbox-drainer + lease-reaper interval loops, stopped by the shutdown watch.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mirror = Arc::new(JsonlMirror::new(base_dir.join(EVENTS_MIRROR_FILE)));
    let drainer = spawn_drainer(
        handle.clone(),
        mirror,
        DRAINER_INTERVAL,
        shutdown_rx.clone(),
    );
    let reaper = spawn_reaper(handle.clone(), REAPER_INTERVAL, shutdown_rx.clone());

    // P4.0a — the §10 opt-3 session supervisor (the live drive-loop spine), spawned like the
    // drainer/reaper, stopped by the shutdown watch. FakeHarness/FakePty-capable; in 4.0a it holds NO
    // live sessions — the cat-1 live launch + the INV-SEC-1 interception + the Gateway session.create
    // executor that DRIVES it are 4.0b. `_supervisor` (the SupervisorHandle) is that 4.0b driver entry
    // — wired + reachable here, not yet driven (the underscore binds it alive for the daemon lifetime).
    let (supervisor, _supervisor) = spawn_supervisor_task(shutdown_rx.clone());

    // L3 — bind the GatewayPort UDS (reclaiming a stale socket) + spawn the peer-auth'd accept-loop.
    let db_path = base_dir.join(DB_FILENAME);
    let listener = bind(&base_dir.join(SOCKET_FILE))?;
    eprintln!("nexusopsd: GatewayPort listening at {SOCKET_FILE}");
    let accept = spawn_accept_loop(
        listener,
        db_path,
        current_euid(),
        MAX_CONNECTIONS,
        deltas,
        handle, // the §6.1 mutation path → the Gateway pipeline on the write-actor
        shutdown_rx,
    );

    wait_for_shutdown().await;
    eprintln!("nexusopsd: shutdown signal received; draining + exiting");
    // stop the interval loops + accept-loop, then drain + close the writer.
    let _ = shutdown_tx.send(true);
    let _ = drainer.await;
    let _ = reaper.await;
    let _ = accept.await;
    let _ = supervisor.await; // drains its session actors (Kill + await each handle — no orphan task).
    actor.shutdown().await;
    // _pidlock drops here → the single-instance OS lock releases.
    Ok(())
}

/// Resolve the macOS app-support dir: `$HOME/Library/Application Support/NexusOps` (1.6a Q2 —
/// hand-rolled via `std::env`, macOS-only MVP; cold_start creates it exists-ok). A bundled
/// launchd/SMAppService integration is Phase 10 (§16).
fn production_base_dir() -> Result<PathBuf, std::io::Error> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home).join("Library/Application Support/NexusOps"))
}

/// Block until SIGTERM or SIGINT (Ctrl-C) arrives (1.6a Q5 — this signal set only this slice;
/// the §16 `prepare_for_update` intent is Phase 10).
async fn wait_for_shutdown() {
    use tokio::signal::unix::{signal, SignalKind};
    // expect() is correct fail-loud behavior on the entry path: a daemon that cannot install its
    // shutdown handlers at startup must abort, not run un-stoppable. NOT a runtime/request path.
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }
}
