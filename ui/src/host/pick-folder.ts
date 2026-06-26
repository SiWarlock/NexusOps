// Native folder-picker seam over the Tauri dialog plugin (the cockpit "Add project" flow).
//
// A thin HOST adapter: returns the user-selected repo path for the `project.rescan` intent. The
// Tauri plugin is DYNAMIC-imported so this module is safe to reference from non-Tauri (jsdom/unit-test)
// contexts without loading the native plugin — the add-project container takes `pickFolder` as an
// INJECTABLE dependency (tests pass a fake; this real impl is the production default). The picker
// itself grants no FS access — it returns only the chosen path string (the daemon scans it, gated by
// the Gateway). Capability: `dialog:allow-open` only (src-tauri/capabilities/default.json).

/**
 * Open the native folder picker and return the selected absolute path, or `null` if the user
 * cancelled. Throws if the dialog can't be opened (a host fault — the caller degrades honestly).
 */
export async function pickFolder(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    directory: true,
    multiple: false,
    title: "Select a git repository to add",
  });
  // With { directory:true, multiple:false } the plugin returns `string | null`; narrow defensively
  // (an array would only arise under multiple:true) — never hand a non-string path downstream.
  return typeof picked === "string" ? picked : null;
}
