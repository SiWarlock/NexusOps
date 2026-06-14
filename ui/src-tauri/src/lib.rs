//! The NexusOps Tauri host (P6.8 L1 slice 2/3). Wraps the Vite frontend and registers the
//! NARROW read-command bridge (one #[tauri::command] per daemon READ method — never a generic
//! gateway_call). The frontend invokes only the enumerated reads; the bridge calls the 049
//! nexusops-gateway-uds transport crate. NON-cat-1 (reads only) — INV-SEC-1 stays daemon-side.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // The complete read-command allowlist (registered = invokable; nothing else is exposed).
        .invoke_handler(tauri::generate_handler![
            commands::gateway_get_projection,
            commands::gateway_get_diff,
            commands::gateway_get_capabilities,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
