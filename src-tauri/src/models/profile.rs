use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub uuid: String,
    pub profile_type: String, // "offline" or "microsoft"
    pub access_token: String,
}

impl Profile {
    pub fn offline(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            uuid: "offline".to_string(),
            profile_type: "offline".to_string(),
            access_token: "0".to_string(),
        }
    }
}