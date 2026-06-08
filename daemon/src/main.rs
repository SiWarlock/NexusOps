//! nexusopsd — the daemon runtime entry (§12 / §16).
//!
//! The `#[tokio::main]` production entry: resolve the app-support dir → `cold_start()` (1.6a:
//! pidlock → migrate → version-floor) → stand up the single write-actor → (L2: drainer/reaper
//! loops · L3: UDS accept-loop · L4: subscribe fanout) → block on a shutdown signal → graceful
//! drain + exit (the held `PidLock` releases on exit). This is the real production caller that
//! closes the 1.3/1.4/1.5/1.6a "ship the mechanism, wire the runtime at 1.6" reachability chain.

use std::path::PathBuf;
use std::process::ExitCode;

use nexusopsd::bootstrap::{cold_start, BootstrapConfig};
use nexusopsd::clock::SystemClock;
use nexusopsd::eventstore::PrefixRedactor;
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::WriteActor;

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
        base_dir,
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
    // writable store (the sole mutation path). L2 spawns the drainer/reaper loops + L3 the UDS
    // accept-loop off this handle.
    let (_pidlock, store, _version) = ctx.into_parts();
    let actor = WriteActor::spawn(store, Box::new(SystemClock));

    wait_for_shutdown().await;
    eprintln!("nexusopsd: shutdown signal received; draining + exiting");
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
