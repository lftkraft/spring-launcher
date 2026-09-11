use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentServer {
    pub id: String,
    pub server_address: String,
    pub server_name: String,
    pub instance_id: String,
    pub instance_name: String,
    pub instance_loader: String,
    pub instance_version: String,
    pub instance_icon: Option<String>,
    pub icon: Option<String>,
    pub last_played: Option<String>,
}
