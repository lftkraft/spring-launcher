use std::fs;
use std::path::PathBuf;
use tauri::State;
use crate::models::settings::Settings;
use crate::AppState;

fn get_settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SpringLauncher")
        .join("settings.json")
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    // Check if file exists, if not use state (default)
    let path = get_settings_path();
    if path.exists() {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let settings: Settings = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        // Update state as well
        let mut state_settings = state.settings.lock().unwrap();
        *state_settings = settings.clone();

        return Ok(settings);
    }

    let settings = state.settings.lock().unwrap();
    Ok(settings.clone())
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, new_settings: Settings) -> Result<(), String> {
    let path = get_settings_path();
    let parent = path.parent().unwrap();
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let content = serde_json::to_string_pretty(&new_settings).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;

    let mut settings = state.settings.lock().unwrap();
    *settings = new_settings;

    Ok(())
}