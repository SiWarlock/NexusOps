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

use nexusops_shared::catalog::ExecutorKind;
use tokio::sync::watch;

use nexusopsd::bootstrap::{cold_start, BootstrapConfig, DB_FILENAME};
use nexusopsd::clock::SystemClock;
use nexusopsd::decisions::DecisionRegistry;
use nexusopsd::eventstore::{open_read_only, JsonlMirror, PrefixRedactor};
use nexusopsd::gateway::circuit_breaker::AuditBackboneBreaker;
use nexusopsd::gateway::executor::CatalogExecutor;
use nexusopsd::gateway::policy::AgentMutationPolicy;
use nexusopsd::gateway::session_executor::SessionExecutor;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use nexusopsd::integrity::{FileIntegrityAlarm, IntegrityAlarm};
use nexusopsd::ipc::current_euid;
use nexusopsd::runtime::recovery::run_restart_recovery;
use nexusopsd::runtime::{
    bind, spawn_accept_loop, spawn_drainer, spawn_reaper, WriteActor,
    WriteActorTelemetrySinkFactory, MAX_CONNECTIONS,
};
use nexusopsd::session::spawn_supervisor_task;
use nexusopsd::session::tmux::{
    select_survival_backend, tmux_probe, SurvivalBackend, SystemCommandRunner,
};
use nexusopsd::terminal::PortablePtySpawner;

/// outbox drain cadence (§12) — deliver due rows a few times a minute.
const DRAINER_INTERVAL: Duration = Duration::from_secs(5);
/// lease reap cadence (§17) — free expired leases periodically.
const REAPER_INTERVAL: Duration = Duration::from_secs(30);
/// the local JSONL audit/debug mirror sink (§10.4) within the app-support dir.
const EVENTS_MIRROR_FILE: &str = "events.jsonl";
/// the GatewayPort UDS within the app-support dir (§6.4).
const SOCKET_FILE: &str = "gateway.sock";
/// the §17 durable integrity-incident file within the app-support dir (call-2 — the off-DB alarm).
const INTEGRITY_INCIDENTS_FILE: &str = "integrity-incidents.jsonl";
// the general GatewayPort connection cap + the F2 intercept-wait reserved sub-bound are the canonical
// `nexusopsd::runtime::{MAX_CONNECTIONS, RESERVED_GENERAL}` (the accept loop derives the wait class
// from the `max_connections` it is passed — `wait_cap = MAX_CONNECTIONS − MAX_CONNECTIONS/4`).

fn main() -> ExitCode {
    // C2 — the `nexusopsd hook <event>` subcommand: the live PreToolUse interception ingress (a
    // short-lived UDS client Claude spawns per tool-call). Dispatched BEFORE the daemon runtime — it
    // is a synchronous client, not the daemon; it must NOT start the write-actor / accept-loop.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("hook") {
        return nexusopsd::hook::run(args.get(2).map(String::as_str).unwrap_or(""));
    }
    // P4.0b-2-smoke (brief 053) — the `nexusopsd smoke <sub>` dev-client (the 0.1-HITL "see it work"
    // rig). Feature-gated `dev-client` (off in a release build). A synchronous UDS client like `hook`;
    // dispatched BEFORE the daemon runtime — it must NOT start the write-actor / accept-loop.
    #[cfg(feature = "dev-client")]
    if args.get(1).map(String::as_str) == Some("smoke") {
        return nexusopsd::smoke::run(&args);
    }
    // the daemon — build the tokio runtime + run.
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("nexusopsd: fatal: cannot build the tokio runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    match rt.block_on(run()) {
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
    // writable store (the sole mutation path). The drainer/reaper loops + the UDS accept-loop run off
    // the write-actor handle.
    let (_pidlock, store, _version) = ctx.into_parts();

    // ---- P4.0b-2 C2 (CAT-1) — the live drive loop: the atomic binding-flip ----
    // The binding condition flips from BY-CONSTRUCTION to ENFORCED-BY-THE-INTERCEPTION here: a live
    // agent becomes reachable (the registered SessionExecutor + the live PtyLauncher) ONLY TOGETHER
    // with the interception (AgentMutationPolicy + the per-session decision_sink registry + the §17
    // alarm) — never an un-intercepted-live window (the call-5 atomicity PIN; the inverted guard
    // `tests/session_executor.rs::test_live_session_create_has_interception` pins this co-residency).

    // the shutdown watch + the per-session decision_sink registry FIRST — the supervisor (cancel a
    // dead session's pending interceptions → Deny) and the accept-loop (the `intercept` wait + the
    // approve/deny resolve) both share the registry.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let registry = Arc::new(DecisionRegistry::new());

    // the §10 opt-3 session supervisor (spawned before the write-actor — it needs only the runtime +
    // the shutdown watch + the registry). Its `SupervisorHandle` DRIVES the SessionExecutor.
    let (supervisor, supervisor_handle) =
        spawn_supervisor_task(shutdown_rx.clone(), registry.clone());

    // the LIVE launcher — the real `ClaudeAdapter` via the `PortablePtySpawner` (the O-13 #10
    // enforcement surface + the env-hygiene/correlation, P4.0b-2 Option A). cwd = the daemon's project
    // dir (MVP: the current dir; per-project resolution from session.create's project_id is Phase 5).
    // hook_receiver = `<this binary> hook` — the PreToolUse interception ingress.
    let project_cwd = std::env::current_dir()?;
    let hook_receiver = format!("{} hook", std::env::current_exe()?.display());
    // P4.0c — the production telemetry sink factory (the live drive caller of the 044 emit path). Built
    // DEFERRED: the launcher (holding the factory) is consumed into the SessionExecutor → Gateway →
    // write-actor BELOW, BEFORE the `WriteHandle` exists — so the factory shares a cell the
    // `telemetry_slot` binds the handle into once the actor is up (well before any session.create →
    // make_sink). The launcher holds the factory as an opaque `Box<dyn TelemetrySinkFactory>` →
    // `session/` never imports `WriteHandle` (cat-1). The per-session sink appends `TelemetrySampled`
    // OBSERVATION events via the write-actor; the pump's live `UsageSource` ingress is the P4 follow-on.
    let (telemetry_factory, telemetry_slot) =
        WriteActorTelemetrySinkFactory::deferred(Arc::new(SystemClock));
    // P4.1b-2 — probe tmux ONCE at startup (Q2: the backend is fixed for the daemon's life) → select a
    // CONSISTENT survival backend (launcher ⇔ broker always agree, pin #3/#7). tmux present →
    // survivable sessions (`TmuxLauncher`, the `env`-wrapped O-13 spec) + survivor detection
    // (`TmuxBroker`, so `ReattachedLive` is reachable); absent → graceful-degrade to the non-surviving
    // `PtyLauncher` + `NoSurvivorBroker` (B2-achievable: resume/replay, NEVER reattach-live). The
    // telemetry factory threads into whichever launcher is selected.
    let tmux_available = tmux_probe(&SystemCommandRunner);
    let backend = select_survival_backend(
        tmux_available,
        Box::new(PortablePtySpawner),
        project_cwd,
        hook_receiver,
        Some(Box::new(telemetry_factory)),
    );
    eprintln!(
        "nexusopsd: survival backend = {:?} (tmux {})",
        backend.kind,
        if tmux_available {
            "present"
        } else {
            "absent — degraded"
        }
    );
    // the launcher → the SessionExecutor (write-actor); the broker → restart recovery (held below).
    let SurvivalBackend {
        kind: _,
        launcher,
        broker: survival_broker,
    } = backend;

    // the session.create/kill executor (the live launcher + the supervisor), REGISTERED under
    // `ExecutorKind::Session` in the production CatalogExecutor — this is the reachable session.create
    // dispatch home (the `Adjudication` INV-SEC-1 guard still runs BEFORE dispatch, R1-A).
    let mut catalog_exec = CatalogExecutor::new();
    catalog_exec.register(
        ExecutorKind::Session,
        Arc::new(SessionExecutor::new(launcher, supervisor_handle)),
    );

    // the production policy is now `AgentMutationPolicy` (the catalog-authoritative risk engine + the
    // O-13 agent-mutation deny-rules) — the daemon supervises live agents. The Action Gateway (the
    // sole mutator) runs its pipeline ON the write-actor thread (forbidden #2/#3).
    let gateway = Gateway::new(Box::new(AgentMutationPolicy), Box::new(catalog_exec));

    // the durable independent §17 integrity alarm (call-2) — an agent-mutation adjudication's
    // audit-WRITE-fault raises it on the off-DB channel; bound at the write-actor. Shared (`Arc`)
    // with the audit-backbone breaker below so BOTH the per-incident alarm AND the systemic-failure
    // alarm append to the same durable incident file.
    let alarm: Arc<dyn IntegrityAlarm> = Arc::new(FileIntegrityAlarm::new(
        base_dir.join(INTEGRITY_INCIDENTS_FILE),
    ));
    // P4.0b-2c (§17/§15 #5, RULED B) — the daemon-wide audit-backbone circuit-breaker: N-consecutive
    // (or a clearly-unrecoverable) audit-write fault latches it → the write-actor quiesce-and-refuses
    // every mutation (no audit-write attempted; reads stay live), until an operator restart. Kept as an
    // `Arc` so the §17/Phase-6 safety surface can read its latched state (`is_tripped()`).
    let audit_breaker = Arc::new(AuditBackboneBreaker::new(alarm.clone()));
    let actor = WriteActor::spawn_with_alarm_and_breaker(
        store,
        Box::new(SystemClock),
        gateway,
        alarm,
        audit_breaker,
    );
    let handle = actor.handle();
    // P4.0c — bind the write-actor handle into the telemetry factory's deferred slot NOW (the actor is
    // up; this is before the accept-loop below serves any session.create → make_sink). The per-session
    // sinks the launcher mints now reach the live write-actor.
    telemetry_slot.bind(handle.clone());
    // the post-commit broadcast sender for the accept-loop's per-connection live subscribers (1.6d).
    let deltas = handle.delta_sender();

    // the outbox-drainer + lease-reaper interval loops, stopped by the shutdown watch.
    let mirror = Arc::new(JsonlMirror::new(base_dir.join(EVENTS_MIRROR_FILE)));
    let drainer = spawn_drainer(
        handle.clone(),
        mirror,
        DRAINER_INTERVAL,
        shutdown_rx.clone(),
    );
    let reaper = spawn_reaper(handle.clone(), REAPER_INTERVAL, shutdown_rx.clone());

    // bind the GatewayPort UDS (reclaiming a stale socket) + spawn the peer-auth'd accept-loop — the
    // registry is threaded in for the live `intercept` transport + the approve/deny decision_sink.
    let db_path = base_dir.join(DB_FILENAME);

    // P4.1b-1 — the §16/§8.1 restart recovery: enumerate the sessions that were live at shutdown (from
    // the rebuilt `proj_session`) → `decide_resume` per session → emit the System-actor `SessionRecovered`
    // observation (Q1=(a), lead/user-ruled: the daemon re-materializing its OWN already-approved session,
    // NOT a Gateway Action). The PRODUCTION caller of `decide_resume` — closes its 4.1a Step-7.5
    // reachability. Runs post-write-actor + post-supervisor, BEFORE the accept-loop serves clients. The
    // §15 #8 profile is preserved from the committed `SessionStarted`. The REAL broker/launcher
    // re-materialization is 4.1b-2 (LIVE survival = HITL); here the recovery DECISION is made + audited.
    match open_read_only(&db_path) {
        Ok(conn) => {
            // pass the SELECTED survival broker (4.1b-2: `TmuxBroker` when tmux is present so a survivor
            // reaches `ReattachedLive`, else `NoSurvivorBroker` — graceful degrade).
            let recovered = run_restart_recovery(
                &conn,
                survival_broker.as_ref(),
                &handle,
                Arc::new(SystemClock),
            );
            if recovered > 0 {
                eprintln!("nexusopsd: restart recovery — {recovered} session(s) recovered");
            }
        }
        Err(e) => eprintln!("nexusopsd: restart recovery skipped (proj_session unavailable): {e}"),
    }

    let listener = bind(&base_dir.join(SOCKET_FILE))?;
    eprintln!("nexusopsd: GatewayPort listening at {SOCKET_FILE}");
    let accept = spawn_accept_loop(
        listener,
        db_path,
        current_euid(),
        MAX_CONNECTIONS,
        deltas,
        handle, // the §6.1 mutation path → the Gateway pipeline on the write-actor
        registry.clone(),
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
