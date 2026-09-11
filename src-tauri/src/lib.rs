mod models;
mod core;
mod utils;
mod api;

use std::sync::Mutex;
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;
use tauri::Manager;
use window_vibrancy::apply_acrylic;

pub struct AppState {
    pub settings: Mutex<models::settings::Settings>,
    pub running_instances: Mutex<HashMap<String, std::process::Child>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let settings_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SpringLauncher")
        .join("settings.json");

    let initial_settings = if settings_path.exists() {
        if let Ok(content) = fs::read_to_string(settings_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            models::settings::Settings::default()
        }
    } else {
        models::settings::Settings::default()
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "windows")]
            let _ = apply_acrylic(&window, Some((20, 20, 20, 10)));

            Ok(())
        })
        .manage(AppState {
            settings: Mutex::new(initial_settings),
            running_instances: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            // Instance
            core::instance::get_instances,
            core::instance::create_instance,
            core::instance::duplicate_instance,
            core::instance::update_instance,
            core::instance::delete_instance,
            core::instance::launch_instance,
            core::instance::kill_instance,
            core::instance::is_instance_running,

            // Servers
            core::server_manager::get_recent_servers,
            core::server_manager::add_recent_server,
            core::server_manager::remove_recent_server,
            core::server_manager::launch_instance_server,

            // Launcher
            core::launcher::get_minecraft_versions,
            core::launcher::get_loader_versions,
            core::launcher::download_instance,

            // Settings
            core::settings::get_settings,
            core::settings::save_settings,

            // Profile
            core::profile::get_profiles,
            core::profile::create_profile,
            core::profile::delete_profile,
            core::profile::login_microsoft,

            // Mods
            core::mod_manager::search_mods,
            core::mod_manager::get_mod_versions,
            core::mod_manager::get_mod_dependencies,
            core::mod_manager::install_mod,
            core::mod_manager::install_modpack_new_instance,
            core::mod_manager::install_modpack_existing_instance,
            core::mod_manager::get_installed_mods,
            core::mod_manager::delete_mod,
            core::mod_manager::toggle_mod,

            // Utils & Files
            utils::open_folder,
            utils::select_folder,
            utils::select_file,
            utils::get_screenshots,
            utils::get_screenshot_full,
            utils::files::list_directory,
            utils::files::read_text_file,
            utils::files::write_text_file,
            utils::files::delete_file_item,
            utils::files::create_file_item,
            utils::files::rename_file_item,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}