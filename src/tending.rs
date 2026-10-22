use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TendTask {
    pub zone_id: String,
    #[serde(default)]
    pub faction_id: String,
    #[serde(default)]
    pub amount: f32,
    #[serde(default)]
    pub stipend_amount: Option<f32>,
    pub flavor_text: String,
}
