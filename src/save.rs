// src/save.rs

use crate::cellar::CellarRecord;
use crate::corpse::Corpse;
use crate::quest::QuestStatus;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveData {
    pub stipend: f32,
    pub reputation: HashMap<String, f32>,
    pub day_index: u32,
    pub time_of_day: f32,
    pub player_pos: (f32, f32),
    pub player_inventory: HashMap<String, u32>,
    pub zone_stock: HashMap<String, HashMap<String, u32>>,
    #[serde(default)]
    pub fired_beats: HashSet<String>,
    #[serde(default)]
    pub fired_milestones: HashSet<String>,
    #[serde(default)]
    pub memory_fragments: Vec<String>,
    #[serde(default)]
    pub keeper_number: u32,
    #[serde(default)]
    pub tenure_start_day: u32,
    #[serde(default)]
    pub cellar_records: Vec<CellarRecord>,
    pub discovered_zones: std::collections::HashSet<String>,
    #[serde(default)]
    pub last_tended: HashMap<String, u32>,
    #[serde(default)]
    pub fired_tutorial: std::collections::HashSet<String>,
    #[serde(default)]
    pub sacrifice_available: bool,
    #[serde(default)]
    pub pending_fragment: Option<String>,
    #[serde(default)]
    pub pending_sacrifice_faction: Option<String>,
    #[serde(default)]
    pub pending_village: Option<String>,
    #[serde(default)]
    pub next_corpse_id: u32,
    #[serde(default)]
    pub missed_days_this_tenure: u32,
    pub current_map: Option<String>,
    pub village_seeds: HashMap<String, u64>,
    #[serde(default)]
    pub hub_player_pos: (f32, f32),
    #[serde(default)]
    pub quest_status: HashMap<String, (QuestStatus, u32)>,
    #[serde(default)]
    pub pair_index: u32,
    #[serde(default)]
    pub is_cult_life: bool,
    #[serde(default)]
    pub chosen_themes: Vec<String>,
    #[serde(default)]
    pub last_delivery_day: Option<u32>,
    #[serde(default)]
    pub last_restock_day: Option<u32>,
    #[serde(default)]
    pub corpse: Option<Corpse>,
    #[serde(default)]
    pub burn_timer: Option<f32>,
}

pub fn save_game(data: &SaveData, path: &str) -> Result<(), String> {
    let text = ron::to_string(data).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

pub fn load_game(path: &str) -> Result<SaveData, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    ron::from_str(&text).map_err(|e| e.to_string())
}
