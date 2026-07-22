mod caddy;
mod commands;
mod docker;
mod files;
mod hardening;
mod monitor;
mod secrets;
mod ssh;
mod store;
mod terminal;

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
            commands::fetch_metrics,
            commands::docker_list,
            commands::docker_action,
            commands::docker_logs,
            commands::deploy_app,
            commands::install_docker,
            commands::caddy_status,
            commands::install_caddy,
            commands::apply_routes,
            commands::check_routes,
            commands::list_dir,
            commands::read_file,
            commands::open_ssh_terminal,
            commands::harden_bootstrap,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
