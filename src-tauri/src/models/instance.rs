use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LoaderType {
    Vanilla,
    Forge,
    Fabric,
    Quilt,
    NeoForge,
}

impl Default for LoaderType {
    fn default() -> Self {
        Self::Vanilla
    }
}

impl ToString for LoaderType {
    fn to_string(&self) -> String {
        match self {
            Self::Vanilla => "vanilla".to_string(),
            Self::Forge => "forge".to_string(),
            Self::Fabric => "fabric".to_string(),
            Self::Quilt => "quilt".to_string(),
            Self::NeoForge => "neoforge".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub loader: LoaderType,
    pub loader_version: Option<String>,
    pub memory: u32,
    pub java_path: Option<String>,
    pub game_dir: String,
    pub icon: Option<String>, // Base64 encoded icon
    pub created: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    #[serde(default)]
    pub playtime: u64, // Másodpercben
}

impl Instance {
    pub fn new(
        name: String,
        mc_version: String,
        loader: LoaderType,
        loader_version: Option<String>,
        memory: u32,
        game_dir: String,
        icon: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            mc_version,
            loader,
            loader_version,
            memory,
            java_path: None,
            game_dir,
            icon,
            created: Utc::now(),
            last_played: None,
            playtime: 0,
        }
    }
}