use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthProject {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: Option<String>,
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    pub downloads: i32,
    pub followers: i32,
    pub client_side: String,
    pub server_side: String,
    pub project_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthSearchHit {
    pub project_id: String,
    pub slug: String,
    pub author: String,
    pub title: String,
    pub description: String,
    pub categories: Option<Vec<String>>,
    pub display_categories: Option<Vec<String>>,
    pub versions: Vec<String>,
    pub downloads: i32,
    pub follows: i32,
    pub icon_url: Option<String>,
    pub project_type: String,
    pub client_side: String,
    pub server_side: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub changelog: Option<String>,
    pub dependencies: Vec<ModrinthDependency>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<ModrinthFile>,
    pub date_published: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthSearchResult {
    pub hits: Vec<ModrinthSearchHit>,
    pub total_hits: i32,
    pub offset: Option<i32>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalMod {
    pub name: String,
    pub filename: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub icon_path: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyInfo {
    pub project: ModrinthProject,
    pub dependency_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MrpackIndex {
    #[serde(rename = "formatVersion")]
    pub format_version: u32,
    pub game: String,
    #[serde(rename = "versionId")]
    pub version_id: Option<String>,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<MrpackFile>,
    #[serde(default)]
    pub dependencies: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MrpackFile {
    pub path: String,
    #[serde(default)]
    pub hashes: std::collections::HashMap<String, String>,
    pub env: Option<MrpackEnv>,
    pub downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    pub file_size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MrpackEnv {
    pub client: Option<String>,
    pub server: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackProgress {
    pub step: String,
    pub current: usize,
    pub total: usize,
    pub file_name: String,
    pub percent: f32,
    #[serde(default)]
    pub speed: f64,
    #[serde(default)]
    pub total_bytes: u64,
    #[serde(default)]
    pub current_bytes: u64,
    #[serde(default)]
    pub remaining_time: String,
}