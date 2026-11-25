#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
mod installed_owner;

fn main() {
    let options = agent_hud_host::HostOptions {
        // An ordinary public build has no fixed owner. A Windows installer
        // built for a specific location can explicitly delegate that copy.
        #[cfg(target_os = "windows")]
        canonical_executable: std::env::current_exe().ok().and_then(|running| {
            installed_owner::installed_owner(
                option_env!("AGENT_HUD_WINDOWS_INSTALLED_EXE"),
                &running,
            )
        }),
        #[cfg(not(target_os = "windows"))]
        canonical_executable: None,
    };
    agent_hud_host::run_with_options(tauri::generate_context!(), None, options);
}
