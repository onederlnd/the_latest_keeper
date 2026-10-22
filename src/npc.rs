// src/npc.rs

use crate::clock::GameClock;
use macroquad::rand::gen_range;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DialogueTier {
    pub min_rep: f32,
    pub max_rep: f32,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Npc {
    pub name: String,
    #[serde(default)]
    pub id: String,
    pub position: (f32, f32),
    pub active_start: f32,
    pub active_end: f32,
    pub faction_id: String,
    pub sprite_id: String,
    #[serde(default)]
    pub zone_id: Option<String>,
    #[serde(default)]
    pub dialogue: Vec<DialogueTier>,
    #[serde(default)]
    pub map: Option<String>,
}

impl Npc {
    pub fn is_active(&self, clock: &GameClock) -> bool {
        clock.is_between(self.active_start, self.active_end)
    }

    pub fn line_for_reputation(&self, rep: f32) -> String {
        for tier in &self.dialogue {
            if rep >= tier.min_rep && rep < tier.max_rep {
                if let Some(line) = pick_line(&tier.lines) {
                    return format!("{}: \"{}\"", self.name, line);
                }
            }
        }
        format!("{}: \"Good day to you.\"", self.name)
    }

    pub fn distance_to(&self, point: (f32, f32)) -> f32 {
        ((self.position.0 - point.0).powi(2) + (self.position.1 - point.1).powi(2)).sqrt()
    }
}
fn pick_line(lines: &[String]) -> Option<&String> {
    if lines.is_empty() {
        None
    } else if lines.len() == 1 {
        lines.first()
    } else {
        let idx = gen_range(0, lines.len());
        lines.get(idx)
    }
}
