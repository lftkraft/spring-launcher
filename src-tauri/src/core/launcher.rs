use std::path::{Path, PathBuf};
use std::fs;
use std::io::{Read, BufReader, BufRead};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State, AppHandle};
use crate::models::instance::{Instance, LoaderType};
use crate::models::profile::Profile;
use crate::AppState;
use futures::StreamExt;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use zip::ZipArchive;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DownloadProgress {
    pub total_files: u64,
    pub current_files: u64,
    pub total_bytes: u64,
    pub current_bytes: u64,
    pub percentage: f32,
    pub speed: f64,
    pub remaining_time: String,
    pub status: String,
    pub current_file_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionData {
    downloads: Option<ClientDownloads>,
    libraries: Vec<Library>,
    mainClass: String,
    assetIndex: Option<AssetIndexInfo>,
    minecraftArguments: Option<String>,
    arguments: Option<serde_json::Value>,
    java_version: Option<JavaVersionInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct JavaVersionInfo {
    component: Option<String>,
    major_version: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ClientDownloads { client: DownloadInfo }
#[derive(Debug, Deserialize, Serialize, Clone)]
struct DownloadInfo {
    url: String,
    sha1: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    path: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
struct Library {
    downloads: Option<LibraryDownloads>,
    name: String,
    url: Option<String>,
    natives: Option<std::collections::HashMap<String, String>>,
    rules: Option<Vec<serde_json::Value>>
}
#[derive(Debug, Deserialize, Serialize)]
struct LibraryDownloads {
    artifact: Option<DownloadInfo>,
    classifiers: Option<std::collections::HashMap<String, DownloadInfo>>
}
#[derive(Debug, Deserialize, Serialize)]
struct AssetIndexInfo { id: String, url: String }
#[derive(Debug, Deserialize, Serialize)]
struct AssetIndex { objects: std::collections::HashMap<String, AssetObject> }
#[derive(Debug, Deserialize, Serialize)]
struct AssetObject { hash: String, size: u64 }

fn check_rules(rules: &Option<Vec<serde_json::Value>>) -> bool {
    if let Some(rules) = rules {
        let mut allow = false;
        for rule in rules {
            let action = rule["action"].as_str().unwrap_or("allow");
            let mut matches = true;
            if let Some(os) = rule.get("os") {
                if let Some(name) = os["name"].as_str() {
                    let current_os = if cfg!(target_os = "windows") { "windows" } else if cfg!(target_os = "macos") { "osx" } else { "linux" };
                    if name != current_os { matches = false; }
                }
                if let Some(arch) = os["arch"].as_str() {
                    let current_arch = if cfg!(target_arch = "x86_64") {
                        "x64"
                    } else if cfg!(target_arch = "aarch64") {
                        "arm64"
                    } else if cfg!(target_arch = "x86") {
                        "x86"
                    } else {
                        "unknown"
                    };
                    if arch == "x86" && current_arch != "x86" { matches = false; }
                    else if arch == "arm64" && current_arch != "arm64" { matches = false; }
                    else if (arch == "x64" || arch == "x86_64") && current_arch != "x64" { matches = false; }
                }
            }
            if let Some(features) = rule.get("features").and_then(|f| f.as_object()) {
                for (_k, _v) in features {
                    matches = false;
                }
            }
            if matches { allow = action == "allow"; }
        }
        allow
    } else { true }
}

fn get_lib_path(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 { return name.to_string(); }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let filename = if parts.len() > 3 {
        format!("{}-{}-{}.jar", artifact, version, parts[3])
    } else {
        format!("{}-{}.jar", artifact, version)
    };
    format!("{}/{}/{}/{}", group, artifact, version, filename)
}

#[tauri::command]
pub async fn get_minecraft_versions() -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let res = client.get("https://launchermeta.mojang.com/mc/game/version_manifest.json").send().await.map_err(|e| e.to_string())?.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
    Ok(res["versions"].as_array().unwrap().iter().filter(|v| v["type"] == "release").map(|v| v["id"].as_str().unwrap().to_string()).collect())
}

#[tauri::command]
pub async fn get_loader_versions(loader: String, mc_version: String) -> Result<Vec<String>, String> {
    if loader == "fabric" {
        let res = reqwest::get("https://meta.fabricmc.net/v2/versions/loader").await.map_err(|e| e.to_string())?.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
        return Ok(res.as_array().unwrap().iter().map(|v| v["version"].as_str().unwrap().to_string()).collect());
    } else if loader == "forge" {
        let url = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
        let res = reqwest::get(url).await.map_err(|e| e.to_string())?.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
        let promos = res["promos"].as_object().ok_or("Forge promos not found")?;

        let mut versions = Vec::new();
        let latest_key = format!("{}-latest", mc_version);
        let recommended_key = format!("{}-recommended", mc_version);

        if let Some(v) = promos.get(&recommended_key).and_then(|v| v.as_str()) {
            versions.push(v.to_string());
        }
        if let Some(v) = promos.get(&latest_key).and_then(|v| v.as_str()) {
            let ver_str = v.to_string();
            if !versions.contains(&ver_str) {
                versions.push(ver_str);
            }
        }
        return Ok(versions);
    } else if loader == "quilt" {
        let res = reqwest::get("https://meta.quiltmc.org/v3/versions/loader")
            .await.map_err(|e| e.to_string())?
            .json::<serde_json::Value>()
            .await.map_err(|e| e.to_string())?;
        if let Some(arr) = res.as_array() {
            let vers: Vec<String> = arr.iter().filter_map(|v| v["version"].as_str().map(|s| s.to_string())).collect();
            return Ok(vers);
        }
    } else if loader == "neoforge" {
        let url = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
        let res = reqwest::get(url)
            .await.map_err(|e| e.to_string())?
            .json::<serde_json::Value>()
            .await.map_err(|e| e.to_string())?;
        if let Some(versions) = res["versions"].as_array() {
            let prefix = if mc_version.starts_with("1.") {
                &mc_version[2..]
            } else {
                &mc_version
            };
            let mut list: Vec<String> = versions.iter()
                .filter_map(|v| v.as_str())
                .filter(|v| v.starts_with(prefix))
                .map(|s| s.to_string())
                .collect();
            list.reverse();
            return Ok(list);
        }
    }
    Ok(vec![])
}

#[derive(Clone)]
struct DownloadTask {
    url: String,
    path: PathBuf,
    expected_size: u64,
}

#[tauri::command]
pub async fn download_instance(window: tauri::Window, instance: Instance) -> Result<(), String> {
    let _ = window.emit("launcher-debug", format!("download_instance:start id={} name={} loader={} mc={}", instance.id, instance.name, instance.loader.to_string(), instance.mc_version));
    eprintln!("[download_instance] start id={} name={} loader={} mc={}", instance.id, instance.name, instance.loader.to_string(), instance.mc_version);

    let _ = window.emit("dl-progress", DownloadProgress {
        total_files: 0,
        current_files: 0,
        total_bytes: 0,
        current_bytes: 0,
        percentage: 0.0,
        speed: 0.0,
        remaining_time: "Számítás...".into(),
        status: "Verzió információk ellenőrzése...".into(),
        current_file_name: format!("{}.json", instance.mc_version),
    });

    let client = reqwest::Client::builder().user_agent("SpringLauncher/1.0").tcp_nodelay(true).build().map_err(|e| e.to_string())?;
    let mc_dir = PathBuf::from(&instance.game_dir);

    // 1. Download Mojang Manifest
    let manifest: serde_json::Value = client.get("https://launchermeta.mojang.com/mc/game/version_manifest.json").send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let version_info = manifest["versions"].as_array().unwrap().iter().find(|v| v["id"] == instance.mc_version).ok_or("Verzió nem található")?;
    let mojang_json_bytes = client.get(version_info["url"].as_str().unwrap()).send().await.map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string())?;
    let mojang_data: VersionData = serde_json::from_slice(&mojang_json_bytes).map_err(|e| e.to_string())?;

    let v_dir = mc_dir.join("versions").join(&instance.mc_version);
    fs::create_dir_all(&v_dir).ok();
    fs::write(v_dir.join(format!("{}.json", instance.mc_version)), &mojang_json_bytes).ok();

    let _ = window.emit("dl-progress", DownloadProgress {
        total_files: 0,
        current_files: 0,
        total_bytes: 0,
        current_bytes: 0,
        percentage: 5.0,
        speed: 0.0,
        remaining_time: "Számítás...".into(),
        status: "Fájlok és könyvtárak feloldása...".into(),
        current_file_name: "Indexek beolvasása".into(),
    });

    let mut tasks: Vec<DownloadTask> = Vec::new();

    // Add Client JAR
    if let Some(d) = &mojang_data.downloads {
        let p = mc_dir.join("versions").join(&instance.mc_version).join(format!("{}.jar", instance.mc_version));
        if !p.exists() {
            tasks.push(DownloadTask {
                url: d.client.url.clone(),
                path: p,
                expected_size: d.client.size,
            });
        }
    }

    // Add Mojang libraries
    for lib in &mojang_data.libraries {
        if !check_rules(&lib.rules) { continue; }
        if let Some(d) = &lib.downloads {
            if let Some(art) = &d.artifact {
                let p = mc_dir.join("libraries").join(get_lib_path(&lib.name));
                if !p.exists() {
                    tasks.push(DownloadTask {
                        url: art.url.clone(),
                        path: p,
                        expected_size: art.size,
                    });
                }
            }
        }
    }

    // 2. Mod Loader Logic
    if instance.loader == LoaderType::Fabric {
        let loader_ver = instance.loader_version.as_ref().ok_or("Nincs Fabric verzió")?;
        let _ = window.emit("launcher-debug", format!("download_instance:fabric-loader-ver={loader_ver}"));
        eprintln!("[download_instance] fabric loader_ver={loader_ver}");
        let fabric_url = format!("https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json", instance.mc_version, loader_ver);
        if let Ok(res) = client.get(fabric_url).send().await {
            if let Ok(fabric_data) = res.json::<VersionData>().await {
                fs::create_dir_all(&mc_dir).ok();
                fs::write(mc_dir.join("loader_profile.json"), serde_json::to_string(&fabric_data).unwrap()).ok();
                let _ = window.emit("launcher-debug", format!("download_instance:fabric-profile-written libs={}", fabric_data.libraries.len()));
                eprintln!("[download_instance] fabric profile written libs={}", fabric_data.libraries.len());
                for lib in fabric_data.libraries {
                    let p = mc_dir.join("libraries").join(get_lib_path(&lib.name));
                    if !p.exists() {
                        let (url, size) = if let Some(u) = &lib.url {
                            (format!("{}{}", u, get_lib_path(&lib.name)), 0)
                        } else if let Some(art) = lib.downloads.and_then(|d| d.artifact) {
                            (art.url, art.size)
                        } else {
                            (String::new(), 0)
                        };
                        if !url.is_empty() {
                            tasks.push(DownloadTask {
                                url,
                                path: p,
                                expected_size: size,
                            });
                        }
                    }
                }
            }
        }
    } else if instance.loader == LoaderType::Forge {
        let loader_ver = instance.loader_version.as_ref().ok_or("Nincs Forge verzió")?;
        let _ = window.emit("launcher-debug", format!("download_instance:forge-loader-ver={loader_ver}"));
        eprintln!("[download_instance] forge loader_ver={loader_ver}");
        let forge_full_ver = format!("{}-{}", instance.mc_version, loader_ver);
        let forge_url = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{0}/forge-{0}-installer.jar", forge_full_ver);
        let installer_path = mc_dir.join(format!("forge-{}-installer.jar", forge_full_ver));

        if let Ok(resp) = client.get(&forge_url).send().await {
            if let Ok(installer_bytes) = resp.bytes().await {
                fs::write(&installer_path, &installer_bytes).ok();
                if let Ok(mut archive) = ZipArchive::new(std::io::Cursor::new(&installer_bytes)) {
                    if let Ok(mut version_file) = archive.by_name("version.json") {
                        let mut version_content = String::new();
                        if version_file.read_to_string(&mut version_content).is_ok() {
                            fs::create_dir_all(&mc_dir).ok();
                            fs::write(mc_dir.join("loader_profile.json"), &version_content).ok();
                            if let Ok(forge_data) = serde_json::from_str::<VersionData>(&version_content) {
                                for lib in forge_data.libraries {
                                    let p = mc_dir.join("libraries").join(get_lib_path(&lib.name));
                                    if !p.exists() {
                                        let (url, size) = if let Some(u) = &lib.url {
                                            (format!("{}{}", u, get_lib_path(&lib.name)), 0)
                                        } else if let Some(art) = lib.downloads.and_then(|d| d.artifact) {
                                            (art.url, art.size)
                                        } else {
                                            (format!("https://maven.minecraftforge.net/{}", get_lib_path(&lib.name)), 0)
                                        };
                                        tasks.push(DownloadTask {
                                            url,
                                            path: p,
                                            expected_size: size,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if instance.loader == LoaderType::NeoForge {
        let loader_ver = instance.loader_version.as_ref().ok_or("Nincs NeoForge verzió")?;
        let _ = window.emit("launcher-debug", format!("download_instance:neoforge-loader-ver={loader_ver}"));
        eprintln!("[download_instance] neoforge loader_ver={loader_ver}");
        let neoforge_url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{0}/neoforge-{0}-installer.jar", loader_ver);
        let installer_path = mc_dir.join(format!("neoforge-{}-installer.jar", loader_ver));

        if !installer_path.exists() {
            if let Ok(resp) = client.get(&neoforge_url).send().await {
                if let Ok(installer_bytes) = resp.bytes().await {
                    fs::write(&installer_path, &installer_bytes).ok();
                }
            }
        }

        if installer_path.exists() {
            if let Ok(installer_bytes) = fs::read(&installer_path) {
                if let Ok(mut archive) = ZipArchive::new(std::io::Cursor::new(&installer_bytes)) {
                    if let Ok(mut version_file) = archive.by_name("version.json") {
                        let mut version_content = String::new();
                        if version_file.read_to_string(&mut version_content).is_ok() {
                            fs::create_dir_all(&mc_dir).ok();
                            let lp = mc_dir.join("loader_profile.json");
                            if !lp.exists() {
                                fs::write(&lp, &version_content).ok();
                            }
                            if let Ok(nf_data) = serde_json::from_str::<VersionData>(&version_content) {
                                for lib in nf_data.libraries {
                                    let lib_rel_path = lib.downloads.as_ref()
                                        .and_then(|d| d.artifact.as_ref())
                                        .and_then(|a| a.path.clone())
                                        .unwrap_or_else(|| get_lib_path(&lib.name));
                                    let p = mc_dir.join("libraries").join(&lib_rel_path);
                                    if !p.exists() {
                                        let (url, size) = if let Some(u) = &lib.url {
                                            (format!("{}{}", u, lib_rel_path), 0)
                                        } else if let Some(art) = lib.downloads.and_then(|d| d.artifact) {
                                            (art.url, art.size)
                                        } else {
                                            (format!("https://maven.neoforged.net/releases/{}", lib_rel_path), 0)
                                        };
                                        tasks.push(DownloadTask {
                                            url,
                                            path: p,
                                            expected_size: size,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Assets
    if let Some(asset_info) = &mojang_data.assetIndex {
        let index_path = mc_dir.join("assets").join("indexes").join(format!("{}.json", asset_info.id));
        if !index_path.exists() {
            fs::create_dir_all(index_path.parent().unwrap()).ok();
            let b = client.get(&asset_info.url).send().await.map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string())?;
            fs::write(&index_path, b).ok();
        }
        if let Ok(asset_content) = fs::read(&index_path) {
            if let Ok(asset_index) = serde_json::from_slice::<AssetIndex>(&asset_content) {
                for obj in asset_index.objects.values() {
                    let p = mc_dir.join("assets").join("objects").join(&obj.hash[..2]).join(&obj.hash);
                    if !p.exists() {
                        tasks.push(DownloadTask {
                            url: format!("https://resources.download.minecraft.net/{}/{}", &obj.hash[..2], obj.hash),
                            path: p,
                            expected_size: obj.size,
                        });
                    }
                }
            }
        }
    }

    // 4. Execution with Real-Time Progress and Speed Tracking
    let total_files = tasks.len() as u64;
    let total_known_bytes: u64 = tasks.iter().map(|t| t.expected_size).sum();
    let total_bytes_arc = Arc::new(AtomicU64::new(total_known_bytes));
    let current_files = Arc::new(AtomicU64::new(0));
    let current_bytes = Arc::new(AtomicU64::new(0));
    let start_instant = Instant::now();
    let last_emit_instant = Arc::new(Mutex::new(Instant::now()));

    if total_files > 0 {
        let mut stream = futures::stream::iter(tasks).map(|task| {
            let client = client.clone();
            let cf = Arc::clone(&current_files);
            let cb = Arc::clone(&current_bytes);
            let tb = Arc::clone(&total_bytes_arc);
            let win = window.clone();
            let lei = Arc::clone(&last_emit_instant);
            async move {
                if let Some(parent) = task.path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(res) = client.get(&task.url).send().await {
                    if res.status().is_success() {
                        if let Ok(b) = res.bytes().await {
                            let bytes_len = b.len() as u64;
                            if fs::write(&task.path, b).is_ok() {
                                let f_done = cf.fetch_add(1, Ordering::SeqCst) + 1;
                                let b_done = cb.fetch_add(bytes_len, Ordering::SeqCst) + bytes_len;

                                let mut tot_b = tb.load(Ordering::Relaxed);
                                if b_done > tot_b {
                                    tb.store(b_done, Ordering::Relaxed);
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
                                } else if f_done >= total_files {
                                    "0 mp".to_string()
                                } else {
                                    "Számítás...".to_string()
                                };

                                let percentage = if tot_b > 0 {
                                    ((b_done as f32 / tot_b as f32) * 100.0).min(99.0)
                                } else {
                                    ((f_done as f32 / total_files as f32) * 100.0).min(99.0)
                                };

                                let should_emit = {
                                    let mut last = lei.lock().unwrap();
                                    if last.elapsed() > std::time::Duration::from_millis(80) || f_done == total_files {
                                        *last = Instant::now();
                                        true
                                    } else {
                                        false
                                    }
                                };

                                if should_emit {
                                    let file_name = task.path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                    let _ = win.emit("dl-progress", DownloadProgress {
                                        total_files,
                                        current_files: f_done,
                                        total_bytes: tot_b,
                                        current_bytes: b_done,
                                        percentage,
                                        speed: speed_mb_s,
                                        remaining_time,
                                        status: format!("Fájlok letöltése ({}/{})", f_done, total_files),
                                        current_file_name: file_name,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }).buffer_unordered(25);

        while let Some(_) = stream.next().await {}
    }

    // Post-download: if Forge, ensure forge client jar is generated
    if instance.loader == LoaderType::Forge {
        if let Some(loader_ver) = &instance.loader_version {
            let forge_full_ver = format!("{}-{}", instance.mc_version, loader_ver);
            let client_jar_path = mc_dir.join("libraries").join("net").join("minecraftforge").join("forge").join(&forge_full_ver).join(format!("forge-{}-client.jar", forge_full_ver));
            let installer_path = mc_dir.join(format!("forge-{}-installer.jar", forge_full_ver));

            if !client_jar_path.exists() && installer_path.exists() {
                let _ = window.emit("dl-progress", DownloadProgress {
                    total_files: 100,
                    current_files: 92,
                    total_bytes: 100,
                    current_bytes: 92,
                    percentage: 92.0,
                    speed: 0.0,
                    remaining_time: "Pár másodperc...".into(),
                    status: "Forge modloader konfigurálása...".into(),
                    current_file_name: "Kliens könyvtárak építése...".into(),
                });
                eprintln!("[download_instance] running forge installClient...");

                // Ensure dummy launcher_profiles.json exists
                let launcher_profiles = mc_dir.join("launcher_profiles.json");
                if !launcher_profiles.exists() {
                    fs::write(&launcher_profiles, "{\"profiles\":{}}").ok();
                }

                let req_java = get_required_java_version(&instance, Some(&mojang_data));
                let (java_cmd, _) = find_java(&instance, req_java).unwrap_or_else(|_| ("java.exe".into(), req_java));
                let output = std::process::Command::new(&java_cmd)
                    .args(&["-jar", &installer_path.to_string_lossy(), "--installClient", &mc_dir.to_string_lossy()])
                    .current_dir(&mc_dir)
                    .output();

                match output {
                    Ok(out) => {
                        eprintln!("[download_instance] forge installer exit: {:?}", out.status);
                        if !out.status.success() {
                            eprintln!("[download_instance] forge installer error: {}", String::from_utf8_lossy(&out.stderr));
                        }
                    }
                    Err(e) => eprintln!("[download_instance] failed to run forge installer: {}", e),
                }

                // Clean up installer jar
                let _ = fs::remove_file(&installer_path);
            }
        }
    }

    // Post-download: if NeoForge, ensure client jar is generated & libraries extracted
    if instance.loader == LoaderType::NeoForge {
        if let Some(loader_ver) = &instance.loader_version {
            let patched_client = mc_dir.join("libraries").join("net").join("neoforged").join("minecraft-client-patched").join(loader_ver).join(format!("minecraft-client-patched-{}.jar", loader_ver));
            let installer_path = mc_dir.join(format!("neoforge-{}-installer.jar", loader_ver));

            if !patched_client.exists() && installer_path.exists() {
                let _ = window.emit("dl-progress", DownloadProgress {
                    total_files: 100,
                    current_files: 92,
                    total_bytes: 100,
                    current_bytes: 92,
                    percentage: 92.0,
                    speed: 0.0,
                    remaining_time: "Pár másodperc...".into(),
                    status: "NeoForge modloader konfigurálása...".into(),
                    current_file_name: "Kliens könyvtárak építése és patch-elés...".into(),
                });
                eprintln!("[download_instance] running neoforge installClient...");

                // Ensure dummy launcher_profiles.json exists
                let launcher_profiles = mc_dir.join("launcher_profiles.json");
                if !launcher_profiles.exists() {
                    fs::write(&launcher_profiles, "{\"profiles\":{}}").ok();
                }

                let req_java = get_required_java_version(&instance, Some(&mojang_data));
                let (java_cmd, _) = find_java(&instance, req_java).unwrap_or_else(|_| ("java.exe".into(), req_java));
                let output = std::process::Command::new(&java_cmd)
                    .args(&["-jar", &installer_path.to_string_lossy(), "--installClient", &mc_dir.to_string_lossy()])
                    .current_dir(&mc_dir)
                    .output();

                match output {
                    Ok(out) => {
                        eprintln!("[download_instance] neoforge installer exit: {:?}", out.status);
                        if !out.status.success() {
                            eprintln!("[download_instance] neoforge installer error: {}", String::from_utf8_lossy(&out.stderr));
                        } else {
                            // Update loader_profile.json with generated version JSON
                            let gen_json = mc_dir.join("versions").join(format!("neoforge-{}", loader_ver)).join(format!("neoforge-{}.json", loader_ver));
                            if gen_json.exists() {
                                if let Ok(content) = fs::read_to_string(&gen_json) {
                                    let _ = fs::write(mc_dir.join("loader_profile.json"), content);
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("[download_instance] failed to run neoforge installer: {}", e),
                }

                // Clean up installer jar
                let _ = fs::remove_file(&installer_path);
            }
        }
    }

    let final_bytes = current_bytes.load(Ordering::Relaxed);
    let final_tot = total_bytes_arc.load(Ordering::Relaxed);
    let reported_tot = if final_tot > 0 { final_tot } else { final_bytes.max(1) };
    let reported_cur = if final_bytes > 0 { final_bytes } else { reported_tot };
    let _ = window.emit("dl-progress", DownloadProgress {
        total_files: total_files.max(1),
        current_files: total_files.max(1),
        total_bytes: reported_tot,
        current_bytes: reported_cur,
        percentage: 100.0,
        speed: 0.0,
        remaining_time: "0 mp".into(),
        status: "Kész! Indítás...".into(),
        current_file_name: "Minden fájl naprakész".into(),
    });
    Ok(())
}

pub async fn start_minecraft(app_handle: AppHandle, instance: Instance, profile: Profile, server: Option<String>) -> Result<(), String> {
    let _ = app_handle.emit("launcher-debug", format!("start_minecraft:start id={} name={} loader={} mc={}", instance.id, instance.name, instance.loader.to_string(), instance.mc_version));
    eprintln!("[start_minecraft] start id={} name={} loader={} mc={}", instance.id, instance.name, instance.loader.to_string(), instance.mc_version);
    let mc_dir = PathBuf::from(&instance.game_dir);

    let mut classpath = Vec::new();
    let mut main_class = String::new();
    let mut loader_data: Option<VersionData> = None;

    let loader_profile_path = mc_dir.join("loader_profile.json");
    if loader_profile_path.exists() {
        if let Ok(c) = fs::read_to_string(&loader_profile_path) {
            if let Ok(data) = serde_json::from_str::<VersionData>(&c) {
                main_class = data.mainClass.clone();
                let has_jvm_args = data
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("jvm"))
                    .and_then(|j| j.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                let has_game_args = data
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("game"))
                    .and_then(|j| j.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                let _ = app_handle.emit(
                    "launcher-debug",
                    format!("start_minecraft:loader_profile_parsed jvm_args={} game_args={}", has_jvm_args, has_game_args),
                );
                eprintln!("[start_minecraft] loader_profile_parsed jvm_args={} game_args={}", has_jvm_args, has_game_args);
                for lib in &data.libraries {
                    let lib_rel_path = lib.downloads.as_ref()
                        .and_then(|d| d.artifact.as_ref())
                        .and_then(|a| a.path.clone())
                        .unwrap_or_else(|| get_lib_path(&lib.name));
                    let p = mc_dir.join("libraries").join(&lib_rel_path);
                    if p.exists() { classpath.push(p.to_string_lossy().to_string()); }
                }
                loader_data = Some(data);
                let _ = app_handle.emit("launcher-debug", "start_minecraft:loader_profile_loaded");
                eprintln!("[start_minecraft] loader_profile_loaded");
            }
        }
    }

    let mojang_json = mc_dir.join("versions").join(&instance.mc_version).join(format!("{}.json", instance.mc_version));
    let mojang_content = fs::read_to_string(mojang_json).map_err(|e| format!("Minecraft JSON hiba: {}", e))?;
    let mojang_data: VersionData = serde_json::from_str(&mojang_content).map_err(|e| e.to_string())?;
    let _ = app_handle.emit("launcher-debug", format!("start_minecraft:mojang-main-class={}", mojang_data.mainClass));
    eprintln!("[start_minecraft] mojang mainClass={}", mojang_data.mainClass);

    let required_java = get_required_java_version(&instance, Some(&mojang_data));
    let (java_path, selected_java_version) = find_java(&instance, required_java)?;
    let _ = app_handle.emit("launcher-debug", format!("start_minecraft:java-path={java_path} (v{selected_java_version})"));
    eprintln!("[start_minecraft] java_path={java_path} (v{selected_java_version})");

    if main_class.is_empty() { main_class = mojang_data.mainClass.clone(); }

    for lib in &mojang_data.libraries {
        if check_rules(&lib.rules) {
            let lib_rel_path = lib.downloads.as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.path.clone())
                .unwrap_or_else(|| get_lib_path(&lib.name));
            let p = mc_dir.join("libraries").join(&lib_rel_path);
            if p.exists() { classpath.push(p.to_string_lossy().to_string()); }
        }
    }

    classpath.push(mc_dir.join("versions").join(&instance.mc_version).join(format!("{}.jar", instance.mc_version)).to_string_lossy().to_string());
    let classpath_str = classpath.join(if cfg!(target_os = "windows") { ";" } else { ":" });
    let _ = app_handle.emit("launcher-debug", format!("start_minecraft:classpath-count={}", classpath.len()));
    eprintln!("[start_minecraft] classpath_count={}", classpath.len());

    let mut final_args = Vec::new();
    final_args.push(format!("-Xmx{}M", instance.memory));

    let process_jvm_arg = |s: &str| -> String {
        s.trim()
         .replace("-DFabricMcEmu= ", "-DFabricMcEmu=")
         .replace("${library_directory}", &mc_dir.join("libraries").to_string_lossy())
         .replace("${natives_directory}", &mc_dir.join("natives").to_string_lossy())
         .replace("${launcher_name}", "SpringLauncher")
         .replace("${launcher_version}", "1.0")
         .replace("${classpath_separator}", if cfg!(target_os = "windows") { ";" } else { ":" })
    };

    let mut cp_added = false;
    let mut native_path_added = false;

    let mut jvm_args_raw: Vec<String> = Vec::new();
    // Base Mojang JVM arguments
    if let Some(args_obj) = &mojang_data.arguments {
        if let Some(jvm) = args_obj.get("jvm").and_then(|v| v.as_array()) {
            for arg in jvm {
                if let Some(s) = arg.as_str() {
                    jvm_args_raw.push(s.to_string());
                } else if let Some(obj) = arg.as_object() {
                    if check_rules(&obj.get("rules").and_then(|r| r.as_array()).cloned()) {
                        if let Some(value) = obj.get("value") {
                            if let Some(s) = value.as_str() { jvm_args_raw.push(s.to_string()); }
                            else if let Some(arr) = value.as_array() {
                                for item in arr { if let Some(s) = item.as_str() { jvm_args_raw.push(s.to_string()); } }
                            }
                        }
                    }
                }
            }
        }
    }

    // Loader specific JVM arguments (e.g. NeoForge -DlibraryDirectory, --add-opens, etc.)
    if let Some(ld) = &loader_data {
        if let Some(args_obj) = &ld.arguments {
            if let Some(jvm) = args_obj.get("jvm").and_then(|v| v.as_array()) {
                for arg in jvm {
                    if let Some(s) = arg.as_str() {
                        jvm_args_raw.push(s.to_string());
                    } else if let Some(obj) = arg.as_object() {
                        if check_rules(&obj.get("rules").and_then(|r| r.as_array()).cloned()) {
                            if let Some(value) = obj.get("value") {
                                if let Some(s) = value.as_str() { jvm_args_raw.push(s.to_string()); }
                                else if let Some(arr) = value.as_array() {
                                    for item in arr { if let Some(s) = item.as_str() { jvm_args_raw.push(s.to_string()); } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for arg in jvm_args_raw {
        let p = process_jvm_arg(&arg);

        // Filter out --sun-misc-unsafe-memory-access if Java version is < 24
        if selected_java_version < 24 && p.starts_with("--sun-misc-unsafe-memory-access=") {
            eprintln!("[start_minecraft] skipping {} on Java {}", p, selected_java_version);
            continue;
        }

        if p.contains("${classpath}") {
            final_args.push(p.replace("${classpath}", &classpath_str));
            cp_added = true;
        } else {
            if p.starts_with("-Djava.library.path=") {
                native_path_added = true;
            }
            if p == "-cp" {
                cp_added = true;
            }
            if !final_args.contains(&p) {
                final_args.push(p);
            }
        }
    }

    if !native_path_added {
        final_args.push(format!("-Djava.library.path={}", mc_dir.join("natives").to_string_lossy()));
    }
    if !cp_added {
        final_args.push("-cp".into());
        final_args.push(classpath_str.clone());
    }

    final_args.push(main_class);
    let _ = app_handle.emit("launcher-debug", format!("start_minecraft:final-args-count={}", final_args.len()));
    eprintln!("[start_minecraft] final_args_count={}", final_args.len());
    eprintln!("[start_minecraft] launching java");

    let process_arg = |s: &str| -> String {
        s.replace("${auth_player_name}", &profile.name).replace("${version_name}", &instance.mc_version)
         .replace("${game_directory}", &mc_dir.to_string_lossy()).replace("${assets_root}", &mc_dir.join("assets").to_string_lossy())
         .replace("${assets_index_name}", &mojang_data.assetIndex.as_ref().map(|a| a.id.clone()).unwrap_or_default())
         .replace("${auth_uuid}", &profile.uuid).replace("${auth_access_token}", &profile.access_token)
         .replace("${user_type}", "mojang").replace("${version_type}", "release").replace("${user_properties}", "{}")
         .replace("${clientid}", "0").replace("${auth_xuid}", "0")
    };

    // Add loader-specific game arguments (e.g. Forge --launchTarget forge_client)
    if let Some(ld) = &loader_data {
        if let Some(args_obj) = &ld.arguments {
            if let Some(game) = args_obj.get("game").and_then(|v| v.as_array()) {
                for arg in game {
                    if let Some(s) = arg.as_str() {
                        final_args.push(process_arg(s.trim()));
                    } else if let Some(obj) = arg.as_object() {
                        if check_rules(&obj.get("rules").and_then(|r| r.as_array()).cloned()) {
                            if let Some(value) = obj.get("value") {
                                if let Some(s) = value.as_str() { final_args.push(process_arg(s.trim())); }
                                else if let Some(arr) = value.as_array() {
                                    for item in arr { if let Some(s) = item.as_str() { final_args.push(process_arg(s.trim())); } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(args_obj) = &mojang_data.arguments {
        if let Some(game) = args_obj.get("game").and_then(|v| v.as_array()) {
            let _ = app_handle.emit("launcher-debug", format!("start_minecraft:game-args-source={}", game.len()));
            eprintln!("[start_minecraft] game args source count={}", game.len());
            for arg in game {
                if let Some(s) = arg.as_str() {
                    final_args.push(process_arg(s.trim()));
                } else if let Some(obj) = arg.as_object() {
                    if check_rules(&obj.get("rules").and_then(|r| r.as_array()).cloned()) {
                        if let Some(value) = obj.get("value") {
                            if let Some(s) = value.as_str() { final_args.push(process_arg(s.trim())); }
                            else if let Some(arr) = value.as_array() {
                                for item in arr { if let Some(s) = item.as_str() { final_args.push(process_arg(s.trim())); } }
                            }
                        }
                    }
                }
            }
        } else {
            let _ = app_handle.emit("launcher-debug", "start_minecraft:mojang-game-args-missing");
            eprintln!("[start_minecraft] mojang game args missing");
        }
    } else if let Some(mc_args) = &mojang_data.minecraftArguments {
        let _ = app_handle.emit("launcher-debug", "start_minecraft:legacy-minecraftArguments");
        eprintln!("[start_minecraft] legacy minecraftArguments");
        for a in mc_args.split_whitespace() { final_args.push(process_arg(a)); }
    }

    if let Some(ref srv) = server {
        let clean_srv = srv.trim();
        if !clean_srv.is_empty() {
            let parts: Vec<&str> = instance.mc_version.split('.').collect();
            let minor: u32 = parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);
            let major: u32 = parts.get(0).and_then(|m| m.parse().ok()).unwrap_or(1);

            if major > 1 || (major == 1 && minor >= 20) {
                final_args.push("--quickPlayMultiplayer".into());
                final_args.push(clean_srv.to_string());
            } else {
                let srv_parts: Vec<&str> = clean_srv.split(':').collect();
                let host = srv_parts[0];
                final_args.push("--server".into());
                final_args.push(host.to_string());
                if let Some(port) = srv_parts.get(1) {
                    final_args.push("--port".into());
                    final_args.push(port.to_string());
                }
            }
        }
    }

    let _ = app_handle.emit("launcher-debug", format!("start_minecraft:final-args={:?}", final_args));
    eprintln!("[start_minecraft] final_args={:?}", final_args);
    let mut child = std::process::Command::new(&java_path)
        .args(&final_args).current_dir(&mc_dir).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()
        .map_err(|e| e.to_string())?;
    let _ = app_handle.emit("launcher-debug", "start_minecraft:java-spawned");
    eprintln!("[start_minecraft] java spawned");

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let app_handle_out = app_handle.clone();
    let app_handle_err = app_handle.clone();

    // Prepare log directory and file
    let log_dir = mc_dir.join("logs").join("launcher");
    fs::create_dir_all(&log_dir).ok();
    let log_file_path = log_dir.join("latest.log");
    let log_file = fs::OpenOptions::new().create(true).write(true).truncate(true).open(&log_file_path).ok();
    let log_file_mutex = Arc::new(Mutex::new(log_file));
    let log_file_mutex_err = Arc::clone(&log_file_mutex);

    let console_label = format!("console-{}", instance.id);
    let _ = tauri::WebviewWindowBuilder::new(&app_handle, &console_label, tauri::WebviewUrl::App("/console".into()))
        .title(format!("{} - Konzol", instance.name)).build();

    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                eprintln!("[minecraft-stdout] {}", l);
                let _ = app_handle_out.emit("minecraft-log", l.clone());
                if let Ok(mut f) = log_file_mutex.lock() {
                    if let Some(file) = f.as_mut() {
                        use std::io::Write;
                        let _ = writeln!(file, "{}", l);
                    }
                }
            }
        }
    });

    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line {
                eprintln!("[minecraft-stderr] {}", l);
                let err_msg = format!("[ERROR] {}", l);
                let _ = app_handle_err.emit("minecraft-log", err_msg.clone());
                if let Ok(mut f) = log_file_mutex_err.lock() {
                    if let Some(file) = f.as_mut() {
                        use std::io::Write;
                        let _ = writeln!(file, "{}", err_msg);
                    }
                }
            }
        }
    });

    let state: State<AppState> = app_handle.state();
    state.running_instances.lock().unwrap().insert(instance.id.clone(), child);

    Ok(())
}

fn infer_required_java_version(mc_version: &str) -> u32 {
    let clean_v = mc_version.trim();
    if clean_v.starts_with("26.") || clean_v.starts_with("25.") || clean_v.starts_with("26w") || clean_v.starts_with("25w") {
        return 25;
    }
    if clean_v.starts_with("1.") {
        let parts: Vec<&str> = clean_v.split('.').collect();
        if parts.len() >= 2 {
            if let Ok(minor) = parts[1].parse::<u32>() {
                if minor >= 21 {
                    return 21;
                }
                if minor == 20 {
                    if parts.len() >= 3 {
                        if let Ok(patch) = parts[2].parse::<u32>() {
                            if patch >= 5 {
                                return 21;
                            }
                        }
                    }
                    return 17;
                }
                if minor >= 18 {
                    return 17;
                }
                if minor == 17 {
                    return 17;
                }
                return 8;
            }
        }
    }
    21
}

fn get_required_java_version(instance: &Instance, mojang_data: Option<&VersionData>) -> u32 {
    if let Some(data) = mojang_data {
        if let Some(jv) = &data.java_version {
            if let Some(major) = jv.major_version {
                return major;
            }
        }
    }
    let version_json_path = PathBuf::from(&instance.game_dir)
        .join("versions")
        .join(&instance.mc_version)
        .join(format!("{}.json", instance.mc_version));
    if version_json_path.exists() {
        if let Ok(content) = fs::read_to_string(&version_json_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(major) = data.get("javaVersion").and_then(|j| j.get("majorVersion")).and_then(|m| m.as_u64()) {
                    return major as u32;
                }
            }
        }
    }
    infer_required_java_version(&instance.mc_version)
}

fn parse_java_version_str(v_str: &str) -> Option<u32> {
    let s = v_str.trim().trim_matches('"');
    if s.starts_with("1.") {
        let rest = &s[2..];
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        return num.parse().ok();
    }
    let num: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse().ok()
}

fn detect_java_version(java_exe: &Path) -> Option<u32> {
    // 1. Try reading the 'release' file (typically in jdk_root/release, two levels up from bin/java.exe)
    if let Some(bin_dir) = java_exe.parent() {
        if let Some(jdk_dir) = bin_dir.parent() {
            let rel = jdk_dir.join("release");
            if rel.exists() {
                if let Ok(content) = fs::read_to_string(&rel) {
                    for line in content.lines() {
                        if line.starts_with("JAVA_VERSION=") {
                            let val = line.trim_start_matches("JAVA_VERSION=").trim_matches('"');
                            if let Some(v) = parse_java_version_str(val) {
                                return Some(v);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Try parsing from directory name / path
    let path_lower = java_exe.to_string_lossy().to_lowercase();
    for token in ["jdk-", "jdk", "jre-", "jre", "java-", "java"] {
        if let Some(pos) = path_lower.find(token) {
            let after = &path_lower[pos + token.len()..];
            if after.starts_with("1.") {
                if let Some(v) = parse_java_version_str(after) {
                    return Some(v);
                }
            }
            let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = digits.parse::<u32>() {
                if v > 0 {
                    return Some(v);
                }
            }
        }
    }

    // 3. Fallback: run `java.exe -version`
    if let Ok(output) = std::process::Command::new(java_exe).arg("-version").output() {
        let text = format!("{}\n{}", String::from_utf8_lossy(&output.stderr), String::from_utf8_lossy(&output.stdout));
        for line in text.lines() {
            if let Some(pos) = line.find("version \"") {
                let rest = &line[pos + 9..];
                if let Some(end) = rest.find('"') {
                    if let Some(v) = parse_java_version_str(&rest[..end]) {
                        return Some(v);
                    }
                }
            }
        }
    }

    None
}

fn scan_installed_javas() -> Vec<(PathBuf, u32)> {
    let mut candidates = Vec::new();
    let mut searched_roots = Vec::new();

    if cfg!(target_os = "windows") {
        searched_roots.extend(vec![
            PathBuf::from("C:\\Program Files\\Java"),
            PathBuf::from("C:\\Program Files\\Eclipse Adoptium"),
            PathBuf::from("C:\\Program Files\\OpenJDK"),
            PathBuf::from("C:\\Program Files\\Microsoft"),
            PathBuf::from("C:\\Program Files\\BellSoft"),
            PathBuf::from("C:\\Program Files\\Zulu"),
            PathBuf::from("C:\\Program Files (x86)\\Java"),
            PathBuf::from("C:\\Program Files (x86)\\Eclipse Adoptium"),
            PathBuf::from("C:\\Program Files (x86)\\OpenJDK"),
        ]);
        if let Some(home) = dirs::home_dir() {
            searched_roots.push(home.join(".jdks"));
            searched_roots.push(home.join("AppData").join("Local").join("Programs").join("Eclipse Adoptium"));
            searched_roots.push(home.join("AppData").join("Local").join("Programs").join("Java"));
            searched_roots.push(home.join("AppData").join("Roaming").join(".minecraft").join("runtime"));
        }
    } else if cfg!(target_os = "macos") {
        searched_roots.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        if let Some(home) = dirs::home_dir() {
            searched_roots.push(home.join("Library/Java/JavaVirtualMachines"));
            searched_roots.push(home.join(".sdkman/candidates/java"));
        }
    } else {
        searched_roots.extend(vec![
            PathBuf::from("/usr/lib/jvm"),
            PathBuf::from("/usr/java"),
        ]);
        if let Some(home) = dirs::home_dir() {
            searched_roots.push(home.join(".sdkman/candidates/java"));
        }
    }

    for var in ["JAVA_HOME", "JAVA_25_HOME", "JAVA_21_HOME", "JAVA_17_HOME", "JAVA_8_HOME"] {
        if let Ok(jh) = std::env::var(var) {
            searched_roots.push(PathBuf::from(jh));
        }
    }

    let mut found_paths = std::collections::HashSet::new();

    for root in searched_roots {
        if !root.exists() { continue; }
        let check_folder = |dir: &Path, found: &mut std::collections::HashSet<PathBuf>, cand: &mut Vec<(PathBuf, u32)>| {
            let exe_name = if cfg!(target_os = "windows") { "java.exe" } else { "java" };
            let direct_bin = dir.join("bin").join(exe_name);
            let mac_bin = dir.join("Contents").join("Home").join("bin").join(exe_name);
            let target_exe = if direct_bin.exists() {
                Some(direct_bin)
            } else if mac_bin.exists() {
                Some(mac_bin)
            } else {
                None
            };
            if let Some(exe) = target_exe {
                let canonical = exe.canonicalize().unwrap_or(exe.clone());
                if found.insert(canonical) {
                    if let Some(ver) = detect_java_version(&exe) {
                        cand.push((exe, ver));
                    }
                }
            }
        };

        check_folder(&root, &mut found_paths, &mut candidates);
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    check_folder(&entry.path(), &mut found_paths, &mut candidates);
                }
            }
        }
    }

    candidates
}

pub fn find_java(instance: &Instance, required_major: u32) -> Result<(String, u32), String> {
    eprintln!("[find_java] checking instance java_path={:?}, required_major={}", instance.java_path, required_major);

    // 1. Check user-specified java_path on the instance
    if let Some(p) = &instance.java_path {
        let mut path = p.clone();
        if path.ends_with("javaw.exe") { path = path.replace("javaw.exe", "java.exe"); }
        let pb = PathBuf::from(&path);
        if pb.exists() {
            let ver = detect_java_version(&pb).unwrap_or(required_major);
            eprintln!("[find_java] Using user-specified java: {} (v{})", path, ver);
            return Ok((path, ver));
        }
    }

    // 2. Scan installed Javas
    let candidates = scan_installed_javas();
    eprintln!("[find_java] Discovered {} Java installations:", candidates.len());
    for (p, v) in &candidates {
        eprintln!("  - Java {}: {}", v, p.display());
    }

    // 3. Match strategy
    // a. Exact match
    if let Some((best_path, v)) = candidates.iter().find(|(_, v)| *v == required_major) {
        let p_str = best_path.to_string_lossy().to_string();
        eprintln!("[find_java] Selected EXACT match Java {}: {}", v, p_str);
        return Ok((p_str, *v));
    }

    // b. Compatible matches
    if required_major >= 25 {
        if let Some((best_path, v)) = candidates.iter().filter(|(_, v)| *v >= 25).min_by_key(|(_, v)| *v) {
            let p_str = best_path.to_string_lossy().to_string();
            eprintln!("[find_java] Selected compatible Java >=25 (v{}): {}", v, p_str);
            return Ok((p_str, *v));
        }
    } else if required_major == 21 {
        if let Some((best_path, v)) = candidates.iter().filter(|(_, v)| *v >= 21).min_by_key(|(_, v)| *v) {
            let p_str = best_path.to_string_lossy().to_string();
            eprintln!("[find_java] Selected compatible Java >=21 (v{}): {}", v, p_str);
            return Ok((p_str, *v));
        }
    } else if required_major == 17 {
        if let Some((best_path, v)) = candidates.iter().filter(|(_, v)| *v >= 17).min_by_key(|(_, v)| *v) {
            let p_str = best_path.to_string_lossy().to_string();
            eprintln!("[find_java] Selected compatible Java >=17 (v{}): {}", v, p_str);
            return Ok((p_str, *v));
        }
    } else if required_major == 8 {
        if let Some((best_path, v)) = candidates.iter().find(|(_, v)| *v == 8) {
            let p_str = best_path.to_string_lossy().to_string();
            eprintln!("[find_java] Selected Java 8: {}", p_str);
            return Ok((p_str, *v));
        }
    }

    // c. Error if no compatible version found
    if !candidates.is_empty() {
        let list: Vec<String> = candidates.iter().map(|(p, v)| format!("Java {} ({})", v, p.display())).collect();
        return Err(format!(
            "A Minecraft {} futtatásához Java {} szükséges, de nem található kompatibilis verzió a gépeden!\nTalált verziók:\n{}\nKérlek telepíts Java {}-öt!",
            instance.mc_version, required_major, list.join("\n"), required_major
        ));
    }

    // d. Fallback to system java.exe
    eprintln!("[find_java] Fallback to system 'java.exe'");
    Ok(("java.exe".into(), required_major))
}
