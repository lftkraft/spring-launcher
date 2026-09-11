use std::fs;
use std::path::PathBuf;
use tauri::{State, AppHandle};
use crate::models::profile::Profile;
use crate::AppState;
use crate::api::microsoft;

#[tauri::command]
pub fn get_profiles(state: State<AppState>) -> Result<Vec<Profile>, String> {
    let settings = state.settings.lock().unwrap();
    let profiles_dir = PathBuf::from(&settings.launcher_dir).join("profiles");
    if !profiles_dir.exists() { return Ok(vec![]); }
    let mut profiles = Vec::new();
    for entry in fs::read_dir(profiles_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let profile: Profile = serde_json::from_str(&content).map_err(|e| e.to_string())?;
            profiles.push(profile);
        }
    }
    Ok(profiles)
}

#[tauri::command]
pub async fn login_microsoft(app_handle: AppHandle, state: State<'_, AppState>) -> Result<Profile, String> {
    let profile = microsoft::login_with_webview(app_handle).await.map_err(|e| e.to_string())?;
    let settings = state.settings.lock().unwrap();
    let profiles_dir = PathBuf::from(&settings.launcher_dir).join("profiles");
    fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    let file_path = profiles_dir.join(format!("{}.json", profile.id));
    fs::write(file_path, serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(profile)
}

#[tauri::command]
pub fn create_profile(state: State<AppState>, name: String) -> Result<Profile, String> {
    let settings = state.settings.lock().unwrap();
    let profiles_dir = PathBuf::from(&settings.launcher_dir).join("profiles");
    fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    let profile = Profile::offline(name);
    let file_path = profiles_dir.join(format!("{}.json", profile.id));
    fs::write(file_path, serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(profile)
}

#[tauri::command]
pub fn delete_profile(state: State<AppState>, id: String) -> Result<(), String> {
    let settings = state.settings.lock().unwrap();
    let profiles_dir = PathBuf::from(&settings.launcher_dir).join("profiles");
    let file_path = profiles_dir.join(format!("{}.json", id));
    if file_path.exists() { fs::remove_file(file_path).map_err(|e| e.to_string())?; Ok(()) }
    else { Err("Profil nem található".to_string()) }
}