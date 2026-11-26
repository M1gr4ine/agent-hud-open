#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
#[path = "installed_owner.rs"]
mod owner;
#[cfg(not(target_os = "windows"))]
#[path = "installed_owner_other.rs"]
mod owner;

fn main() {
    let options = agent_hud_host::HostOptions {
        // An ordinary public build has no fixed owner. A Windows installer
        // built for a specific location can explicitly delegate that copy.
        canonical_executable: owner::canonical_executable(),
    };
    agent_hud_host::run_with_options(tauri::generate_context!(), None, options);
}
