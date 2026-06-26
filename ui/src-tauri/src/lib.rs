//! The NexusOps Tauri host. Wraps the Vite frontend and registers the NARROW typed command bridge —
//! one #[tauri::command] per daemon method (reads at L1; the 4 §6.1 MUTATIONS added at L2-B), NEVER a
//! generic gateway_call. The frontend invokes only the enumerated set; the bridge calls the 049
//! nexusops-gateway-uds transport crate. The L2 mutation path is gated OFF on the TS side
//! (`mutationsEnabled=false`) until L2-C — INV-SEC-1 stays daemon-side.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Native folder picker for the cockpit "Add project" flow (project.rescan inputs.path). Scoped
        // to `dialog:allow-open` (capabilities/default.json) — the picker returns only the chosen path;
        // the daemon Gateway is still the sole mutator (the path feeds a project.rescan intent).
        .plugin(tauri_plugin_dialog::init())
        // The complete command allowlist (registered = invokable; nothing else is exposed). Still NO
        // generic `gateway_call` — reads + subscribe + the 4 typed L2 mutation commands only (LESSON 21).
        .invoke_handler(tauri::generate_handler![
            commands::gateway_get_projection,
            commands::gateway_get_diff,
            commands::gateway_get_pr_diff,
            commands::gateway_get_capabilities,
            commands::gateway_subscribe,
            commands::gateway_submit_action,
            commands::gateway_preview_action,
            commands::gateway_approve,
            commands::gateway_deny,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
