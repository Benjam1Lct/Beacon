mod commands;
mod hardening;
mod secrets;
mod ssh;
mod store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::ssh_test_connection,
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::connect_profile,
            commands::harden_bootstrap,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
