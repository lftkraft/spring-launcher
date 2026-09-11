use std::path::{Path, PathBuf};
use std::fs;
use crate::api::modrinth;
use crate::models::mod_meta::{ModrinthVersion, LocalMod, DependencyInfo, MrpackIndex, MrpackFile, ModpackProgress};
use crate::models::instance::{Instance, LoaderType};
use crate::AppState;
use tauri::{AppHandle, Emitter, State};
use zip::ZipArchive;
use data_encoding::BASE64;
use std::io::Read;

#[tauri::command]
pub async fn search_mods(
    query: String,
    facets: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<crate::models::mod_meta::ModrinthSearchResult, String> {
    modrinth::search_mods(&query, facets.as_deref(), limit, offset).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mod_versions(project_id: String, loader: Option<String>, game_version: Option<String>) -> Result<Vec<ModrinthVersion>, String> {
    let loaders = loader.map(|l| vec![l.to_lowercase()]);
    let game_versions = game_version.map(|v| vec![v]);
    modrinth::get_project_versions(&project_id, loaders, game_versions).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mod_dependencies(version_id: String) -> Result<Vec<DependencyInfo>, String> {
    let version = modrinth::get_version(&version_id).await.map_err(|e| e.to_string())?;
    let mut deps = Vec::new();

    for dep in version.dependencies {
        if dep.dependency_type == "required" || dep.dependency_type == "optional" {
            if let Some(project_id) = dep.project_id {
                if let Ok(project) = modrinth::get_project(&project_id).await {
                    deps.push(DependencyInfo {
                        project,
                        dependency_type: dep.dependency_type,
                    });
                }
            }
        }
    }

    Ok(deps)
}

#[tauri::command]
pub async fn install_mod(
    app: AppHandle,
    _instance_id: String,
    instance_path: String,
    version_id: String,
    project_type: Option<String>,
) -> Result<(), String> {
    let version = modrinth::get_version(&version_id).await.map_err(|e| e.to_string())?;
    let primary_file = version.files.iter().find(|f| f.primary).or(version.files.first())
        .ok_or("No files found for this version")?;

    let subfolder = match project_type.as_deref() {
        Some("resourcepack") => "resourcepacks",
        Some("datapack") => "datapacks",
        _ => "mods",
    };

    let target_dir = PathBuf::from(&instance_path).join(subfolder);
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    }

    let dest_path = target_dir.join(&primary_file.filename);

    // Download logic
    let response = reqwest::get(&primary_file.url).await.map_err(|e| e.to_string())?;
    let content = response.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&dest_path, &content).map_err(|e| e.to_string())?;

    // If it is a datapack, also copy into saves/*/datapacks so existing worlds can use it
    if subfolder == "datapacks" {
        let saves_dir = PathBuf::from(&instance_path).join("saves");
        if saves_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(saves_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let world_dp = entry.path().join("datapacks");
                        let _ = fs::create_dir_all(&world_dp);
                        let _ = fs::write(world_dp.join(&primary_file.filename), &content);
                    }
                }
            }
        }
    }

    app.emit("instances-updated", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_installed_mods(instance_path: String) -> Result<Vec<LocalMod>, String> {
    let mods_dir = PathBuf::from(&instance_path).join("mods");
    if !mods_dir.exists() {
        return Ok(Vec::new());
    }

    let mut installed_mods = Vec::new();
    if let Ok(entries) = fs::read_dir(mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let extension = path.extension().and_then(|s| s.to_str());

            if extension == Some("jar") || extension == Some("disabled") {
                let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                let enabled = extension == Some("jar");

                let mut name = filename.clone();
                let mut version = "Ismeretlen".to_string();
                let mut icon_data = None;

                if let Ok(file) = fs::File::open(&path) {
                    if let Ok(mut archive) = ZipArchive::new(file) {
                        // 1. Try common icon names
                        let icon_names = vec!["icon.png", "logo.png"];
                        for iname in icon_names {
                            if let Ok(mut icon_file) = archive.by_name(iname) {
                                let mut buffer = Vec::new();
                                if icon_file.read_to_end(&mut buffer).is_ok() {
                                    icon_data = Some(format!("data:image/png;base64,{}", BASE64.encode(&buffer)));
                                    break;
                                }
                            }
                        }

                        // 2. Read fabric.mod.json for metadata and specific icon path
                        let mut mod_json_content = String::new();
                        let mut custom_icon_path = None;

                        if let Ok(mut mod_json_file) = archive.by_name("fabric.mod.json") {
                            if mod_json_file.read_to_string(&mut mod_json_content).is_ok() {
                                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&mod_json_content) {
                                    if let Some(n) = json_val["name"].as_str() { name = n.to_string(); }
                                    if let Some(v) = json_val["version"].as_str() { version = v.to_string(); }
                                    if let Some(i) = json_val["icon"].as_str() { custom_icon_path = Some(i.to_string()); }
                                }
                            }
                        }

                        // 3. If we have a custom icon path from JSON and haven't found an icon yet
                        if let Some(icon_p) = custom_icon_path {
                            if icon_data.is_none() {
                                if let Ok(mut icon_file) = archive.by_name(&icon_p) {
                                    let mut buffer = Vec::new();
                                    if icon_file.read_to_end(&mut buffer).is_ok() {
                                        icon_data = Some(format!("data:image/png;base64,{}", BASE64.encode(&buffer)));
                                    }
                                }
                            }
                        }
                    }
                }

                installed_mods.push(LocalMod {
                    name,
                    filename,
                    version,
                    author: None,
                    description: None,
                    icon_path: icon_data,
                    enabled,
                });
            }
        }
    }

    Ok(installed_mods)
}

#[tauri::command]
pub fn toggle_mod(instance_path: String, filename: String, enable: bool) -> Result<(), String> {
    let mods_dir = PathBuf::from(&instance_path).join("mods");
    let current_path = mods_dir.join(&filename);

    if !current_path.exists() {
        return Err("Fájl nem található".to_string());
    }

    let new_filename = if enable {
        filename.replace(".disabled", ".jar")
    } else {
        filename.replace(".jar", ".disabled")
    };

    let new_path = mods_dir.join(new_filename);
    fs::rename(current_path, new_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_mod(instance_path: String, filename: String) -> Result<(), String> {
    let mod_path = PathBuf::from(&instance_path).join("mods").join(filename);
    if mod_path.exists() {
        fs::remove_file(mod_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn process_mrpack(
    app: &AppHandle,
    game_dir: &Path,
    mrpack_bytes: Vec<u8>,
) -> Result<MrpackIndex, String> {
    let _ = app.emit("modpack-install-progress", ModpackProgress {
        step: "Csomag kicsomagolása...".to_string(),
        current: 0,
        total: 100,
        file_name: "modrinth.index.json".to_string(),
        percent: 5.0,
        speed: 0.0,
        total_bytes: 0,
        current_bytes: 0,
        remaining_time: "Számítás...".to_string(),
    });

    let cursor = std::io::Cursor::new(mrpack_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| format!("Hibás ZIP archívum: {}", e))?;

    // 1. Read modrinth.index.json
    let index: MrpackIndex = {
        let mut index_file = archive.by_name("modrinth.index.json")
            .map_err(|e| format!("Nem található modrinth.index.json: {}", e))?;
        let mut content = String::new();
        index_file.read_to_string(&mut content).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| format!("Hibás modrinth.index.json formátum: {}", e))?
    };

    // 2. Extract overrides & client-overrides
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();
        let target_subpath = if let Some(s) = name.strip_prefix("overrides/") {
            Some(s)
        } else if let Some(s) = name.strip_prefix("client-overrides/") {
            Some(s)
        } else {
            None
        };

        if let Some(subpath) = target_subpath {
            if subpath.is_empty() {
                continue;
            }
            let out_path = game_dir.join(subpath);
            if file.is_dir() {
                let _ = fs::create_dir_all(&out_path);
            } else {
                if let Some(parent) = out_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let mut outfile = fs::File::create(&out_path).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            }
        }
    }

    // 3. Filter client files
    let client_files: Vec<MrpackFile> = index.files.iter().filter(|f| {
        if let Some(env) = &f.env {
            if env.client.as_deref() == Some("unsupported") {
                return false;
            }
        }
        !f.downloads.is_empty()
    }).cloned().collect();

    let total_files = client_files.len();
    if total_files == 0 {
        let _ = app.emit("modpack-install-progress", ModpackProgress {
            step: "Kész!".to_string(),
            current: 0,
            total: 0,
            file_name: "".to_string(),
            percent: 100.0,
            speed: 0.0,
            total_bytes: 0,
            current_bytes: 0,
            remaining_time: "0 mp".to_string(),
        });
        return Ok(index);
    }

    // 4. Concurrently download all client files
    let total_bytes: u64 = client_files.iter().map(|f| f.file_size.unwrap_or(0)).sum();
    let total_bytes_arc = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(total_bytes));
    let downloaded_bytes = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let completed_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let start_instant = std::time::Instant::now();
    let http_client = reqwest::Client::new();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(8));

    let mut tasks = Vec::new();
    for f in client_files {
        let url = f.downloads[0].clone();
        let path = game_dir.join(&f.path);
        let sem = semaphore.clone();
        let client = http_client.clone();
        let counter = completed_counter.clone();
        let dl_bytes = downloaded_bytes.clone();
        let tot_bytes_arc = total_bytes_arc.clone();
        let app_handle = app.clone();
        let file_name = f.path.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.map_err(|e| e.to_string())?;
            if let Some(p) = path.parent() {
                let _ = fs::create_dir_all(p);
            }
            let resp = client.get(&url).header("User-Agent", "SpringLauncher/1.0.0").send().await.map_err(|e| e.to_string())?;
            let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
            let b_len = bytes.len() as u64;
            fs::write(&path, bytes).map_err(|e| e.to_string())?;

            let done = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let b_done = dl_bytes.fetch_add(b_len, std::sync::atomic::Ordering::SeqCst) + b_len;
            let mut tot_b = tot_bytes_arc.load(std::sync::atomic::Ordering::Relaxed);
            if b_done > tot_b {
                tot_bytes_arc.store(b_done, std::sync::atomic::Ordering::Relaxed);
                tot_b = b_done;
            }

            let elapsed = start_instant.elapsed().as_secs_f64();
            let speed_mb_s = if elapsed > 0.05 {
                (b_done as f64 / 1_048_576.0) / elapsed
            } else {
                0.0
            };

            let remaining_time = if speed_mb_s > 0.01 && tot_b > b_done {
                let rem_bytes = tot_b - b_done;
                let rem_secs = (rem_bytes as f64 / (speed_mb_s * 1_048_576.0)) as u64;
                if rem_secs >= 3600 {
                    format!("{} óra {} p", rem_secs / 3600, (rem_secs % 3600) / 60)
                } else if rem_secs >= 60 {
                    format!("{} p {} mp", rem_secs / 60, rem_secs % 60)
                } else {
                    format!("{} mp", rem_secs)
                }
            } else if done >= total_files {
                "0 mp".to_string()
            } else {
                "Számítás...".to_string()
            };

            let pct = (done as f32 / total_files as f32) * 90.0 + 10.0;
            let _ = app_handle.emit("modpack-install-progress", ModpackProgress {
                step: format!("Modok letöltése ({}/{})", done, total_files),
                current: done,
                total: total_files,
                file_name,
                percent: pct,
                speed: speed_mb_s,
                total_bytes: tot_b,
                current_bytes: b_done,
                remaining_time,
            });
            Ok::<(), String>(())
        }));
    }

    for t in tasks {
        if let Ok(res) = t.await {
            if let Err(e) = res {
                log::warn!("Modpack fájl letöltési hiba: {}", e);
            }
        }
    }

    let final_b = downloaded_bytes.load(std::sync::atomic::Ordering::Relaxed);
    let _ = app.emit("modpack-install-progress", ModpackProgress {
        step: "Telepítés befejezve!".to_string(),
        current: total_files,
        total: total_files,
        file_name: "".to_string(),
        percent: 100.0,
        speed: 0.0,
        total_bytes: if total_bytes > 0 { total_bytes } else { final_b },
        current_bytes: if total_bytes > 0 { total_bytes } else { final_b },
        remaining_time: "0 mp".to_string(),
    });

    Ok(index)
}

#[tauri::command]
pub async fn install_modpack_new_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    version_id: String,
    instance_name: String,
    memory: Option<u32>,
    icon: Option<String>,
) -> Result<Instance, String> {
    // 1. Get version details
    let version = modrinth::get_version(&version_id).await.map_err(|e| e.to_string())?;
    let primary_file = version.files.iter()
        .find(|f| f.filename.ends_with(".mrpack"))
        .or_else(|| version.files.iter().find(|f| f.primary))
        .or_else(|| version.files.first())
        .ok_or_else(|| "Nem található .mrpack fájl ehhez a verzióhoz".to_string())?;

    let _ = app.emit("modpack-install-progress", ModpackProgress {
        step: "Modpack csomag letöltése...".to_string(),
        current: 0,
        total: 100,
        file_name: primary_file.filename.clone(),
        percent: 2.0,
        speed: 0.0,
        total_bytes: primary_file.size.max(0) as u64,
        current_bytes: 0,
        remaining_time: "Számítás...".to_string(),
    });

    let resp = reqwest::get(&primary_file.url).await.map_err(|e| e.to_string())?;
    let mrpack_bytes = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();

    // 2. Read modrinth.index.json first to know loader & version
    let cursor = std::io::Cursor::new(&mrpack_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| e.to_string())?;
    let index: MrpackIndex = {
        let mut index_file = archive.by_name("modrinth.index.json").map_err(|e| e.to_string())?;
        let mut s = String::new();
        index_file.read_to_string(&mut s).map_err(|e| e.to_string())?;
        serde_json::from_str(&s).map_err(|e| e.to_string())?
    };

    let mc_version = index.dependencies.get("minecraft")
        .cloned()
        .or_else(|| version.game_versions.first().cloned())
        .unwrap_or_else(|| "1.21.1".to_string());

    let (loader_type, loader_version) = if let Some(v) = index.dependencies.get("fabric-loader") {
        (LoaderType::Fabric, Some(v.clone()))
    } else if let Some(v) = index.dependencies.get("quilt-loader") {
        (LoaderType::Quilt, Some(v.clone()))
    } else if let Some(v) = index.dependencies.get("neoforge") {
        (LoaderType::NeoForge, Some(v.clone()))
    } else if let Some(v) = index.dependencies.get("forge") {
        (LoaderType::Forge, Some(v.clone()))
    } else {
        let l_str = version.loaders.first().map(|s| s.as_str()).unwrap_or("vanilla");
        match l_str {
            "fabric" => (LoaderType::Fabric, None),
            "quilt" => (LoaderType::Quilt, None),
            "neoforge" => (LoaderType::NeoForge, None),
            "forge" => (LoaderType::Forge, None),
            _ => (LoaderType::Vanilla, None),
        }
    };

    // 3. Setup instance paths
    let (instances_dir, ram) = {
        let settings = state.settings.lock().unwrap();
        (
            PathBuf::from(&settings.launcher_dir).join("instances"),
            memory.unwrap_or(settings.default_memory),
        )
    };

    // Sanitize or avoid duplicate name
    let mut final_name = instance_name.trim().to_string();
    if final_name.is_empty() {
        final_name = index.name.clone();
    }
    let mut target_dir = instances_dir.join(&final_name);
    let mut counter = 1;
    while target_dir.exists() {
        final_name = format!("{} ({})", instance_name.trim(), counter);
        target_dir = instances_dir.join(&final_name);
        counter += 1;
    }

    let game_dir = target_dir.join(".minecraft");
    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;

    let instance = Instance::new(
        final_name,
        mc_version,
        loader_type,
        loader_version,
        ram,
        game_dir.to_string_lossy().to_string(),
        icon,
    );

    let json_path = target_dir.join("instance.json");
    let json_content = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    fs::write(json_path, json_content).map_err(|e| e.to_string())?;

    // 4. Process overrides & files
    process_mrpack(&app, &game_dir, mrpack_bytes).await?;

    let _ = app.emit("instances-updated", ());
    Ok(instance)
}

#[tauri::command]
pub async fn install_modpack_existing_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    version_id: String,
) -> Result<(), String> {
    // 1. Find existing instance
    let instances_dir = {
        let settings = state.settings.lock().unwrap();
        PathBuf::from(&settings.launcher_dir).join("instances")
    };

    let mut found_game_dir: Option<PathBuf> = None;
    if instances_dir.exists() {
        for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().is_dir() {
                let json_path = entry.path().join("instance.json");
                if json_path.exists() {
                    if let Ok(content) = fs::read_to_string(&json_path) {
                        if let Ok(inst) = serde_json::from_str::<Instance>(&content) {
                            if inst.id == instance_id {
                                found_game_dir = Some(PathBuf::from(inst.game_dir));
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    let game_dir = found_game_dir.ok_or_else(|| "A kiválasztott instance nem található".to_string())?;

    // 2. Fetch version and download mrpack
    let version = modrinth::get_version(&version_id).await.map_err(|e| e.to_string())?;
    let primary_file = version.files.iter()
        .find(|f| f.filename.ends_with(".mrpack"))
        .or_else(|| version.files.iter().find(|f| f.primary))
        .or_else(|| version.files.first())
        .ok_or_else(|| "Nem található .mrpack fájl ehhez a verzióhoz".to_string())?;

    let _ = app.emit("modpack-install-progress", ModpackProgress {
        step: "Modpack csomag letöltése...".to_string(),
        current: 0,
        total: 100,
        file_name: primary_file.filename.clone(),
        percent: 2.0,
        speed: 0.0,
        total_bytes: primary_file.size.max(0) as u64,
        current_bytes: 0,
        remaining_time: "Számítás...".to_string(),
    });

    let resp = reqwest::get(&primary_file.url).await.map_err(|e| e.to_string())?;
    let mrpack_bytes = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();

    process_mrpack(&app, &game_dir, mrpack_bytes).await?;

    let _ = app.emit("instances-updated", ());
    Ok(())
}