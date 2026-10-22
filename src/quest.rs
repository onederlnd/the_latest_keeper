use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum QuestKind {
    Fetch { item_id: String, count: u32 },
    Timing { zone_id: String, by_day: u32 },
    Tag { tag: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestDef {
    pub id: String,
    pub npc_id: String,
    pub kind: QuestKind,
    pub offer_text: String,
    pub turn_in_text: String,
    pub reward_rep: f32,
    pub reward_stipend: f32,
    pub reward_item: Option<(String, u32)>,
    pub cooldown_days: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuestStatus {
    Accepted,
    Ready,
    Done { day: u32 },
}
