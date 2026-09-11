use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Emitter, State, AppHandle, Manager};
use crate::models::instance::{Instance, LoaderType};
use crate::models::profile::Profile;
use crate::AppState;
use crate::core::launcher::start_minecraft;

#[tauri::command]
pub fn get_instances(state: State<AppState>) -> Result<Vec<Instance>, String> {
    let settings = state.settings.lock().unwrap();
    let instances_dir = PathBuf::from(&settings.launcher_dir).join("instances");

    if !instances_dir.exists() {
        return Ok(vec![]);
    }

    let mut instances = Vec::new();
    for entry in fs::read_dir(instances_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let json_path = path.join("instance.json");
            if json_path.exists() {
                let content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
                let instance: Instance = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                instances.push(instance);
            }
        }
    }

    Ok(instances)
}

#[tauri::command]
pub fn create_instance(
    app_handle: AppHandle,
    state: State<AppState>,
    name: String,
    mc_version: String,
    loader: String,
    loader_version: Option<String>,
    memory: u32,
    icon: Option<String>,
) -> Result<(), String> {
    let settings = state.settings.lock().unwrap();
    let instances_dir = PathBuf::from(&settings.launcher_dir).join("instances");

    if instances_dir.exists() {
        for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().is_dir() {
                let json_path = entry.path().join("instance.json");
                if json_path.exists() {
                    let content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
                    let existing: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                    if existing["name"].as_str() == Some(&name) {
                        return Err(format!("'{}' nevű instance már létezik!", name));
                    }
                }
            }
        }
    }

    let instance_dir = instances_dir.join(&name);
    let game_dir = instance_dir.join(".minecraft");
    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;

    let loader_type = match loader.to_lowercase().as_str() {
        "forge" => LoaderType::Forge,
        "fabric" => LoaderType::Fabric,
        "quilt" => LoaderType::Quilt,
        "neoforge" => LoaderType::NeoForge,
        _ => LoaderType::Vanilla,
    };

    let instance = Instance::new(
        name,
        mc_version,
        loader_type,
        loader_version,
        memory,
        game_dir.to_string_lossy().to_string(),
        icon,
    );

    let json_path = instance_dir.join("instance.json");
    let json_content = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    fs::write(json_path, json_content).map_err(|e| e.to_string())?;

    let _ = app_handle.emit("instances-updated", ());

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    }
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        // Skip logs and crash-reports to keep duplication clean & lightweight
        if name_str == "logs" || name_str == "crash-reports" {
            continue;
        }

        let target_path = dst.join(&file_name);
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target_path)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn duplicate_instance(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    source_id: String,
    name: String,
    mc_version: String,
    loader: String,
    loader_version: Option<String>,
    memory: u32,
    icon: Option<String>,
) -> Result<Instance, String> {
    let settings_dir = {
        let settings = state.settings.lock().unwrap();
        PathBuf::from(&settings.launcher_dir)
    };
    let instances_dir = settings_dir.join("instances");

    tokio::task::spawn_blocking(move || -> Result<Instance, String> {
        if !instances_dir.exists() {
            return Err("Instances directory does not exist".to_string());
        }

        // 1. Check if name is taken
        for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().is_dir() {
                let json_path = entry.path().join("instance.json");
                if json_path.exists() {
                    let content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
                    let existing: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                    if existing["name"].as_str() == Some(&name) {
                        return Err(format!("'{}' nevű instance már létezik!", name));
                    }
                }
            }
        }

        // 2. Find source instance
        let mut source_instance: Option<Instance> = None;
        for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().is_dir() {
                let json_path = entry.path().join("instance.json");
                if json_path.exists() {
                    let content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
                    if let Ok(loaded) = serde_json::from_str::<Instance>(&content) {
                        if loaded.id == source_id {
                            source_instance = Some(loaded);
                            break;
                        }
                    }
                }
            }
        }

        let source = source_instance.ok_or_else(|| "Forrás instance nem található".to_string())?;

        // 3. Create destination directory and copy game_dir contents (mods, saves, config, etc.)
        let new_instance_dir = instances_dir.join(&name);
        let new_game_dir = new_instance_dir.join(".minecraft");
        fs::create_dir_all(&new_game_dir).map_err(|e| e.to_string())?;

        let source_game_dir = PathBuf::from(&source.game_dir);
        if source_game_dir.exists() {
            let _ = copy_dir_recursive(&source_game_dir, &new_game_dir);
        }

        let loader_type = match loader.to_lowercase().as_str() {
            "forge" => LoaderType::Forge,
            "fabric" => LoaderType::Fabric,
            "quilt" => LoaderType::Quilt,
            "neoforge" => LoaderType::NeoForge,
            _ => LoaderType::Vanilla,
        };

        let chosen_icon = if icon.is_some() { icon } else { source.icon };

        let new_instance = Instance::new(
            name,
            mc_version,
            loader_type,
            loader_version,
            memory,
            new_game_dir.to_string_lossy().to_string(),
            chosen_icon,
        );

        let json_path = new_instance_dir.join("instance.json");
        let json_content = serde_json::to_string_pretty(&new_instance).map_err(|e| e.to_string())?;
        fs::write(json_path, json_content).map_err(|e| e.to_string())?;

        let _ = app_handle.emit("instances-updated", ());

        Ok(new_instance)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn update_instance(app_handle: AppHandle, state: State<AppState>, instance: Instance) -> Result<(), String> {
    let settings = state.settings.lock().unwrap();
    let instances_dir = PathBuf::from(&settings.launcher_dir).join("instances");

    for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let json_path = path.join("instance.json");
            if json_path.exists() {
                let content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
                let loaded: Instance = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                if loaded.id == instance.id {
                    let json_content = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
                    fs::write(json_path, json_content).map_err(|e| e.to_string())?;
                    let _ = app_handle.emit("instances-updated", ());
                    return Ok(());
                }
            }
        }
    }

    Err("Instance not found".to_string())
}

#[tauri::command]
pub async fn delete_instance(app_handle: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let settings_dir = {
        let settings = state.settings.lock().unwrap();
        PathBuf::from(&settings.launcher_dir)
    };
    let instances_dir = settings_dir.join("instances");

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        if !instances_dir.exists() {
            return Err("Instances directory does not exist".to_string());
        }

        for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                let json_path = path.join("instance.json");
                if json_path.exists() {
                    let content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
                    if let Ok(instance) = serde_json::from_str::<Instance>(&content) {
                        if instance.id == id {
                            fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
                            let _ = app_handle.emit("instances-updated", ());
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err("Instance not found".to_string())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn launch_instance(
    app_handle: AppHandle,
    id: String,
    profile: Profile,
    server: Option<String>,
) -> Result<(), String> {
    let _ = app_handle.emit("launcher-debug", format!("launch_instance:start id={id} profile={} ({}) server={:?}", profile.name, profile.profile_type, server));
    eprintln!("[launch_instance] start id={id} profile={} ({}) server={:?}", profile.name, profile.profile_type, server);
    let settings_dir = {
        let state: State<AppState> = app_handle.state();
        let settings = state.settings.lock().unwrap();
        PathBuf::from(&settings.launcher_dir)
    };

    let instances_dir = settings_dir.join("instances");

    let mut instance = None;
    for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let json_path = path.join("instance.json");
            if json_path.exists() {
                let content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
                let loaded: Instance = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                if loaded.id == id {
                    instance = Some(loaded);
                    break;
                }
            }
        }
    }

    let mut instance = instance.ok_or("Instance not found".to_string())?;
    let _ = app_handle.emit("launcher-debug", format!("launch_instance:instance-found name={} loader={} mc={}", instance.name, instance.loader.to_string(), instance.mc_version));
    eprintln!("[launch_instance] instance-found name={} loader={} mc={}", instance.name, instance.loader.to_string(), instance.mc_version);
    instance.last_played = Some(chrono::Utc::now());

    let instance_dir = instances_dir.join(&instance.name);
    let json_path = instance_dir.join("instance.json");
    let json_content = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    fs::write(json_path, json_content).map_err(|e| e.to_string())?;

    let _ = app_handle.emit("launcher-debug", "launch_instance:calling-start_minecraft");
    eprintln!("[launch_instance] calling start_minecraft");
    let result = start_minecraft(app_handle, instance, profile, server).await;
    if let Err(ref err) = result {
        eprintln!("[launch_instance] start_minecraft failed: {err}");
    } else {
        eprintln!("[launch_instance] start_minecraft returned ok");
    }
    result
}

#[tauri::command]
pub fn kill_instance(state: State<AppState>, id: String) -> Result<(), String> {
    let mut running = state.running_instances.lock().unwrap();
    if let Some(mut child) = running.remove(&id) {
        let _ = child.kill();
        Ok(())
    } else {
        Err("Instance is not running".to_string())
    }
}

#[tauri::command]
pub fn is_instance_running(state: State<AppState>, id: String) -> Result<bool, String> {
    let running = state.running_instances.lock().unwrap();
    Ok(running.contains_key(&id))
}
