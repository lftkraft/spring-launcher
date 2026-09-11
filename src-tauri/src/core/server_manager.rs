use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use tauri::{AppHandle, Manager, State};

use crate::models::server::RecentServer;
use crate::models::profile::Profile;
use crate::models::instance::Instance;
use crate::AppState;
use crate::core::launcher::start_minecraft;

fn decompress_if_gzip(data: &[u8]) -> Vec<u8> {
    if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        if decoder.read_to_end(&mut decompressed).is_ok() {
            return decompressed;
        }
    }
    data.to_vec()
}

/// Robustly extracts (name, ip, icon) from Minecraft's servers.dat file
pub fn parse_servers_dat(file_path: &Path) -> Vec<(String, String, Option<String>)> {
    let mut results = Vec::new();
    let raw = match fs::read(file_path) {
        Ok(b) => b,
        Err(_) => return results,
    };

    let bytes = decompress_if_gzip(&raw);
    let len = bytes.len();

    let read_u16 = |buf: &[u8], p: usize| -> Option<u16> {
        if p + 2 <= buf.len() {
            Some(u16::from_be_bytes([buf[p], buf[p + 1]]))
        } else {
            None
        }
    };

    // Find "servers" list needle: Tag 9 (List), name length 7, "servers"
    let needle = b"\x09\x00\x07servers";
    let mut servers_list_idx = None;
    for i in 0..len.saturating_sub(needle.len() + 5) {
        if &bytes[i..i + needle.len()] == needle {
            servers_list_idx = Some(i + needle.len());
            break;
        }
    }

    if let Some(mut idx) = servers_list_idx {
        if idx + 5 <= len {
            let elem_type = bytes[idx];
            idx += 1;
            let count = i32::from_be_bytes([bytes[idx], bytes[idx + 1], bytes[idx + 2], bytes[idx + 3]]);
            idx += 4;

            if elem_type == 10 && count > 0 {
                for _ in 0..count {
                    let mut name = None;
                    let mut ip = None;
                    let mut icon = None;

                    while idx < len {
                        let tag_type = bytes[idx];
                        idx += 1;
                        if tag_type == 0 {
                            // TAG_End
                            break;
                        }

                        if let Some(n_len) = read_u16(&bytes, idx) {
                            idx += 2;
                            let n_len = n_len as usize;
                            if idx + n_len > len {
                                break;
                            }
                            let tag_name = String::from_utf8_lossy(&bytes[idx..idx + n_len]).to_string();
                            idx += n_len;

                            match tag_type {
                                1 => idx += 1, // Byte
                                2 => idx += 2, // Short
                                3 => idx += 4, // Int
                                4 => idx += 8, // Long
                                5 => idx += 4, // Float
                                6 => idx += 8, // Double
                                7 => { // Byte array
                                    if idx + 4 <= len {
                                        let arr_len = i32::from_be_bytes([bytes[idx], bytes[idx + 1], bytes[idx + 2], bytes[idx + 3]]) as usize;
                                        idx += 4 + arr_len;
                                    }
                                }
                                8 => { // String
                                    if let Some(s_len) = read_u16(&bytes, idx) {
                                        idx += 2;
                                        let s_len = s_len as usize;
                                        if idx + s_len <= len {
                                            let s_val = String::from_utf8_lossy(&bytes[idx..idx + s_len]).to_string();
                                            idx += s_len;
                                            if tag_name == "name" {
                                                name = Some(s_val);
                                            } else if tag_name == "ip" {
                                                ip = Some(s_val);
                                            } else if tag_name == "icon" {
                                                icon = Some(s_val);
                                            }
                                        }
                                    }
                                }
                                9 => { // List
                                    if idx + 5 <= len {
                                        let sub_type = bytes[idx];
                                        let sub_count = i32::from_be_bytes([bytes[idx + 1], bytes[idx + 2], bytes[idx + 3], bytes[idx + 4]]) as usize;
                                        idx += 5;
                                        if sub_type == 8 {
                                            for _ in 0..sub_count {
                                                if let Some(sl) = read_u16(&bytes, idx) {
                                                    idx += 2 + sl as usize;
                                                }
                                            }
                                        }
                                    }
                                }
                                10 => { // Compound - skip simple nested
                                    let mut depth = 1;
                                    while idx < len && depth > 0 {
                                        let sub_tag = bytes[idx];
                                        idx += 1;
                                        if sub_tag == 0 {
                                            depth -= 1;
                                        }
                                    }
                                }
                                11 => { // Int array
                                    if idx + 4 <= len {
                                        let arr_len = i32::from_be_bytes([bytes[idx], bytes[idx + 1], bytes[idx + 2], bytes[idx + 3]]) as usize;
                                        idx += 4 + arr_len * 4;
                                    }
                                }
                                12 => { // Long array
                                    if idx + 4 <= len {
                                        let arr_len = i32::from_be_bytes([bytes[idx], bytes[idx + 1], bytes[idx + 2], bytes[idx + 3]]) as usize;
                                        idx += 4 + arr_len * 8;
                                    }
                                }
                                _ => break,
                            }
                        } else {
                            break;
                        }
                    }

                    if let Some(server_ip) = ip {
                        let server_name = name.unwrap_or_else(|| "Minecraft Szerver".to_string());
                        results.push((server_name, server_ip, icon));
                    }
                }
            }
        }
    }

    results
}

/// Scans latest.log for "Connecting to <ip>, <port>" lines
fn scan_latest_log_for_servers(log_path: &Path) -> Vec<(String, DateTime<Utc>)> {
    let mut entries = Vec::new();
    if !log_path.exists() {
        return entries;
    }

    let mod_time: DateTime<Utc> = fs::metadata(log_path)
        .and_then(|m| m.modified())
        .map(DateTime::from)
        .unwrap_or_else(|_| Utc::now());

    if let Ok(content) = fs::read_to_string(log_path) {
        for line in content.lines() {
            if let Some(pos) = line.find("Connecting to ") {
                let after = &line[pos + "Connecting to ".len()..];
                let parts: Vec<&str> = after.split(',').collect();
                if let Some(host_part) = parts.get(0) {
                    let host = host_part.trim();
                    let port = parts.get(1).map(|p| p.trim()).unwrap_or("25565");
                    let addr = if port == "25565" {
                        host.to_string()
                    } else {
                        format!("{}:{}", host, port)
                    };
                    if !host.is_empty() && !entries.iter().any(|(a, _)| a == &addr) {
                        entries.push((addr, mod_time));
                    }
                }
            }
        }
    }

    entries
}

fn get_recent_servers_json_path(launcher_dir: &str) -> PathBuf {
    PathBuf::from(launcher_dir).join("recent_servers.json")
}

fn load_persisted_recent_servers(launcher_dir: &str) -> Vec<RecentServer> {
    let path = get_recent_servers_json_path(launcher_dir);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(list) = serde_json::from_str::<Vec<RecentServer>>(&content) {
                return list;
            }
        }
    }
    Vec::new()
}

fn save_persisted_recent_servers(launcher_dir: &str, list: &[RecentServer]) {
    let path = get_recent_servers_json_path(launcher_dir);
    if let Ok(content) = serde_json::to_string_pretty(list) {
        let _ = fs::write(path, content);
    }
}

#[tauri::command]
pub async fn get_recent_servers(state: State<'_, AppState>) -> Result<Vec<RecentServer>, String> {
    let launcher_dir = {
        let settings = state.settings.lock().unwrap();
        settings.launcher_dir.clone()
    };

    let mut persisted = load_persisted_recent_servers(&launcher_dir);

    // Scan instances directory
    let instances_dir = PathBuf::from(&launcher_dir).join("instances");
    let mut instances: Vec<Instance> = Vec::new();
    if instances_dir.exists() {
        if let Ok(entries) = fs::read_dir(&instances_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let json_p = p.join("instance.json");
                    if json_p.exists() {
                        if let Ok(c) = fs::read_to_string(&json_p) {
                            if let Ok(inst) = serde_json::from_str::<Instance>(&c) {
                                instances.push(inst);
                            }
                        }
                    }
                }
            }
        }
    }

    // Process each instance's logs & servers.dat
    for inst in &instances {
        let mc_dir = PathBuf::from(&inst.game_dir);

        // 1. servers.dat
        let servers_dat_path = mc_dir.join("servers.dat");
        let parsed_servers = if servers_dat_path.exists() {
            parse_servers_dat(&servers_dat_path)
        } else {
            Vec::new()
        };

        // 2. latest.log
        let latest_log_path = mc_dir.join("logs").join("latest.log");
        let logged_servers = scan_latest_log_for_servers(&latest_log_path);

        // Merge logged servers
        for (addr, log_date) in logged_servers {
            let pretty_name = parsed_servers
                .iter()
                .find(|(_, ip, _)| ip == &addr)
                .map(|(n, _, _)| n.clone())
                .unwrap_or_else(|| {
                    if addr.contains('.') {
                        addr.split('.').next().unwrap_or(&addr).to_uppercase()
                    } else {
                        "Minecraft Szerver".to_string()
                    }
                });

            let icon = parsed_servers
                .iter()
                .find(|(_, ip, _)| ip == &addr)
                .and_then(|(_, _, icon)| icon.clone());

            let id = format!("{}_{}", inst.id, addr);
            if let Some(existing) = persisted.iter_mut().find(|s| s.id == id || (s.server_address == addr && s.instance_id == inst.id)) {
                if existing.last_played.is_none() {
                    existing.last_played = Some(log_date.to_rfc3339());
                }
                if existing.icon.is_none() && icon.is_some() {
                    existing.icon = icon;
                }
            } else {
                persisted.push(RecentServer {
                    id,
                    server_address: addr,
                    server_name: pretty_name,
                    instance_id: inst.id.clone(),
                    instance_name: inst.name.clone(),
                    instance_loader: inst.loader.to_string(),
                    instance_version: inst.mc_version.clone(),
                    instance_icon: inst.icon.clone(),
                    icon,
                    last_played: Some(log_date.to_rfc3339()),
                });
            }
        }

        // Merge any other saved servers from servers.dat
        for (name, addr, icon) in parsed_servers {
            let id = format!("{}_{}", inst.id, addr);
            if !persisted.iter().any(|s| s.server_address == addr && s.instance_id == inst.id) {
                persisted.push(RecentServer {
                    id,
                    server_address: addr,
                    server_name: name,
                    instance_id: inst.id.clone(),
                    instance_name: inst.name.clone(),
                    instance_loader: inst.loader.to_string(),
                    instance_version: inst.mc_version.clone(),
                    instance_icon: inst.icon.clone(),
                    icon,
                    last_played: inst.last_played.map(|t| t.to_rfc3339()),
                });
            }
        }
    }

    // Sort by last_played descending
    persisted.sort_by(|a, b| {
        let a_time = a.last_played.as_deref().unwrap_or("");
        let b_time = b.last_played.as_deref().unwrap_or("");
        b_time.cmp(a_time)
    });

    save_persisted_recent_servers(&launcher_dir, &persisted);
    Ok(persisted)
}

#[tauri::command]
pub async fn add_recent_server(state: State<'_, AppState>, server: RecentServer) -> Result<(), String> {
    let launcher_dir = {
        let settings = state.settings.lock().unwrap();
        settings.launcher_dir.clone()
    };

    let mut list = load_persisted_recent_servers(&launcher_dir);
    list.retain(|s| s.id != server.id && !(s.server_address == server.server_address && s.instance_id == server.instance_id));
    list.insert(0, server);
    save_persisted_recent_servers(&launcher_dir, &list);
    Ok(())
}

#[tauri::command]
pub async fn remove_recent_server(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let launcher_dir = {
        let settings = state.settings.lock().unwrap();
        settings.launcher_dir.clone()
    };

    let mut list = load_persisted_recent_servers(&launcher_dir);
    list.retain(|s| s.id != id);
    save_persisted_recent_servers(&launcher_dir, &list);
    Ok(())
}

#[tauri::command]
pub async fn launch_instance_server(
    app_handle: AppHandle,
    instance_id: String,
    profile: Profile,
    server_address: String,
) -> Result<(), String> {
    let launcher_dir = {
        let state: State<AppState> = app_handle.state();
        let settings = state.settings.lock().unwrap();
        settings.launcher_dir.clone()
    };

    let instances_dir = PathBuf::from(&launcher_dir).join("instances");
    let mut found_instance: Option<Instance> = None;

    if instances_dir.exists() {
        for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let p = entry.path();
            if p.is_dir() {
                let json_path = p.join("instance.json");
                if json_path.exists() {
                    if let Ok(c) = fs::read_to_string(&json_path) {
                        if let Ok(inst) = serde_json::from_str::<Instance>(&c) {
                            if inst.id == instance_id {
                                found_instance = Some(inst);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    let mut instance = found_instance.ok_or_else(|| "Instance nem található".to_string())?;
    instance.last_played = Some(Utc::now());

    // Update instance.json
    let inst_json_path = instances_dir.join(&instance.name).join("instance.json");
    if let Ok(content) = serde_json::to_string_pretty(&instance) {
        let _ = fs::write(inst_json_path, content);
    }

    // Update recent_servers.json timestamp
    let mut list = load_persisted_recent_servers(&launcher_dir);
    if let Some(entry) = list.iter_mut().find(|s| s.instance_id == instance_id && s.server_address == server_address) {
        entry.last_played = Some(Utc::now().to_rfc3339());
    } else {
        list.insert(0, RecentServer {
            id: format!("{}_{}", instance_id, server_address),
            server_address: server_address.clone(),
            server_name: server_address.clone(),
            instance_id: instance_id.clone(),
            instance_name: instance.name.clone(),
            instance_loader: instance.loader.to_string(),
            instance_version: instance.mc_version.clone(),
            instance_icon: instance.icon.clone(),
            icon: None,
            last_played: Some(Utc::now().to_rfc3339()),
        });
    }
    save_persisted_recent_servers(&launcher_dir, &list);

    // Launch with server parameter
    start_minecraft(app_handle, instance, profile, Some(server_address)).await
}
