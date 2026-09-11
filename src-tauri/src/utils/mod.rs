pub mod files;

use std::process::Command;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotInfo {
    pub name: String,
    pub path: String,
    pub created: u64,
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(path).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn select_folder() -> Result<Option<String>, String> {
    Ok(FileDialog::new().pick_folder().map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn select_file(title: String, filter_name: String, filter_ext: String) -> Result<Option<String>, String> {
    Ok(FileDialog::new().set_title(&title).add_filter(&filter_name, &[&filter_ext]).pick_file().map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn get_screenshot_full(path: String) -> Result<String, String> {
    let bytes = fs::read(path).await.map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", data_encoding::BASE64.encode(&bytes)))
}

#[tauri::command]
pub async fn get_screenshots(game_dir: String) -> Result<Vec<ScreenshotInfo>, String> {
    let screenshot_dir = std::path::Path::new(&game_dir).join("screenshots");
    if !screenshot_dir.exists() { return Ok(vec![]); }

    let mut screenshots = Vec::new();
    let mut entries = fs::read_dir(screenshot_dir).await.map_err(|e| e.to_string())?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ["png", "jpg", "jpeg"].contains(&extension.to_lowercase().as_str()) {
                let metadata = entry.metadata().await.map_err(|e| e.to_string())?;
                let created = metadata.modified().map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
                }).unwrap_or(0);

                screenshots.push(ScreenshotInfo {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: path.to_string_lossy().to_string(),
                    created: created * 1000,
                });
            }
        }
    }

    screenshots.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(screenshots)
}
