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
use nexusopsd::git::cli::SystemGitCli;
use nexusopsd::git::executor::GitExecutor;
use nexusopsd::idgen::UlidGen;
use nexusopsd::integrations::executor::{GithubExecutor, LinearExecutor};
use nexusopsd::integrations::github_write::OctocrabGithubWriteClient;
use nexusopsd::integrations::linear_write::LinearGraphqlWriteClient;
use nexusopsd::ipc::current_euid;
use nexusopsd::project::executor::ProjectExecutor;
use nexusopsd::runtime::{
    bind, spawn_accept_loop, spawn_drainer, spawn_git_watcher, spawn_reaper, WriteActor,
};
use nexusopsd::session::spawn_supervisor_task;

/// outbox drain cadence (§12) — deliver due rows a few times a minute.
const DRAINER_INTERVAL: Duration = Duration::from_secs(5);
/// lease reap cadence (§17) — free expired leases periodically.
const REAPER_INTERVAL: Duration = Duration::from_secs(30);
/// git-watcher cadence (§7.2) — refresh each worktree's live-read git-axis cache periodically. Git
/// reads are local + cheap but not free; keep the cadence generous (the reaper cadence).
const GIT_WATCHER_INTERVAL: Duration = Duration::from_secs(30);
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
    // CatalogExecutor (validates requires_resource_refs + dispatches by ExecutorKind to a registered
    // handler else a side-effect-free per-namespace stub). R1a registers NO handler → every action
    // stubs (production behavior unchanged; the 4.0b-1 binding condition holds — no live executor);
    // the first real handler registers at the cat-1 4.0b-2 (+ edges' Project/Git/Github/Linear P5/P7).
    // (The session-executor type name is deliberately absent here — `tests/session_executor.rs`
    // #test_no_reachable_live_caller greps this file for it to prove no production session.create
    // caller is wired; the literal lands when 4.0b-2 actually registers it.)
    // P5.1 (edges-019) — register the first real edges Gateway-mutator handler: the project.rescan
    // executor (ExecutorKind::Project). Detection is read-only; it emits ProjectRescanned in-txn
    // through the §15 gate (SystemClock stamps scanned_at). The remaining edges namespaces
    // (Git/Github/Linear) register here as their wiring slices land (Wave B-D).
    let mut catalog_executor = CatalogExecutor::new();
    catalog_executor.register(
        nexusops_shared::catalog::ExecutorKind::Project,
        Arc::new(ProjectExecutor::new(Box::new(SystemClock))),
    );
    // P5.2 (edges-020) — the first real edges FS/git MUTATION handler: git.create_worktree via the git
    // CLI (forbidden #6 — never git2 for mutations). git.status/git.diff/git.create_branch delegate to
    // the inner stub (reads via the read path; create_branch → edges-021). risk-2 (approval-gated).
    catalog_executor.register(
        nexusops_shared::catalog::ExecutorKind::Git,
        Arc::new(GitExecutor::new(Box::new(SystemGitCli))),
    );
    // P7.1 (edges-023) — the first real edges EXTERNAL-NETWORK mutator: github.create_pr/_draft via
    // octocrab. 3a: the SYNC executor drives the async write-client via the CAPTURED tokio Handle
    // (`Handle::current()` is valid HERE — `run()` is async; `execute()` later `block_on`s it on the
    // write-actor's dedicated std::thread, where `Handle::current()` would panic). The
    // OctocrabGithubWriteClient takes an injected octocrab handle; auth bootstrap (gh-token/Device Flow)
    // is deferred → a default unauthenticated handle for now (a real create needs the deferred auth
    // slice). The proj_pull_request projector folding PullRequestSynced is a follow-on slice.
    catalog_executor.register(
        nexusops_shared::catalog::ExecutorKind::Github,
        Arc::new(GithubExecutor::new(
            Box::new(OctocrabGithubWriteClient::new(octocrab::Octocrab::default())),
            tokio::runtime::Handle::current(),
            Box::new(SystemClock),
        )),
    );
    // P7.1 (edges-024) — the second edges EXTERNAL-NETWORK mutator: linear.link_issue/create_issue via
    // the Linear GraphQL write client. Same 3a captured-Handle/block_on/timeout mechanism. The
    // LinearGraphqlWriteClient takes an injected reqwest::Client + endpoint + api_key; auth bootstrap
    // (OAuth/PKCE) is deferred → an empty key for now (a real mutation → 401→AuthFailed→Failed,
    // fail-closed-correct). Linear success emits NO domain event (Q1); ActionSucceeded is the record.
    catalog_executor.register(
        nexusops_shared::catalog::ExecutorKind::Linear,
        Arc::new(LinearExecutor::new(
            Box::new(LinearGraphqlWriteClient::new(
                reqwest::Client::new(),
                "https://api.linear.app/graphql".to_string(),
                String::new(),
            )),
            tokio::runtime::Handle::current(),
            Box::new(SystemClock),
        )),
    );
    let gateway = Gateway::new(Box::new(CatalogPolicy), Box::new(catalog_executor));
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
    // §7.2 git-watcher (edges-026): refresh each proj_worktree's live-read git-axis cache on an
    // interval (the drainer/reaper precedent). It enumerates proj_worktree over a read-only WAL conn +
    // issues a NON-event `refresh_worktree_status` per worktree through the write-actor.
    let git_watcher = spawn_git_watcher(
        handle.clone(),
        db_path.clone(),
        GIT_WATCHER_INTERVAL,
        shutdown_rx.clone(),
    );
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
    let _ = git_watcher.await;
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
