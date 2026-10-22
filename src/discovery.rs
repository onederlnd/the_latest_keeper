use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discovery {
    pub id: String,
    pub zone_id: String,
    pub faction_id: String,
    pub announce_text: String,
}
