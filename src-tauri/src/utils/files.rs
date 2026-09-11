use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub created: u64,
    pub modified: u64,
    pub item_count: Option<usize>,
}

#[command]
pub async fn list_directory(path: String) -> Result<Vec<FileInfo>, String> {
    let dir_path = Path::new(&path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err("Directory does not exist".to_string());
    }

    let mut files = Vec::new();
    let entries = fs::read_dir(dir_path).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        let path_buf = entry.path();

        let created = metadata.created()
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0);

        let modified = metadata.modified()
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0);

        let mut item_count = None;
        if metadata.is_dir() {
            if let Ok(sub_entries) = fs::read_dir(&path_buf) {
                item_count = Some(sub_entries.count());
            }
        }

        files.push(FileInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            path: path_buf.to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            created,
            modified,
            item_count,
        });
    }

    files.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(files)
}

#[command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[command]
pub async fn write_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|e| e.to_string())
}

#[command]
pub async fn delete_file_item(path: String) -> Result<(), String> {
    // A törlést egy külön szálra dobjuk (spawn_blocking), hogy ne fagyjon meg a UI
    tauri::async_runtime::spawn_blocking(move || {
        let p = Path::new(&path);
        if p.is_dir() {
            fs::remove_dir_all(p).map_err(|e| e.to_string())
        } else {
            fs::remove_file(p).map_err(|e| e.to_string())
        }
    }).await.map_err(|e| e.to_string())?
}

#[command]
pub async fn create_file_item(path: String, is_dir: bool) -> Result<(), String> {
    if is_dir {
        fs::create_dir_all(path).map_err(|e| e.to_string())
    } else {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, "").map_err(|e| e.to_string())
    }
}

#[command]
pub async fn rename_file_item(old_path: String, new_path: String) -> Result<(), String> {
    fs::rename(old_path, new_path).map_err(|e| e.to_string())
}