use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Milestone {
    pub id: String,
    #[serde(default)]
    pub zone_id: String,
    pub faction_id: String,
    pub threshold: f32,
    pub direction: String,
    #[serde(default)]
    pub secondary_faction_id: Option<String>,
    #[serde(default)]
    pub secondary_threshold: Option<f32>,
    #[serde(default)]
    pub secondary_direction: Option<String>,
    pub announce_text: String,
    pub memory_fragment: String,
}

impl Milestone {
    pub fn is_triggered<F>(&self, get_rep: F) -> bool
    where
        F: Fn(&str) -> f32,
    {
        let primary_rep = get_rep(&self.faction_id);
        let primary_ok = match self.direction.as_str() {
            "above" => primary_rep >= self.threshold,
            "below" => primary_rep <= self.threshold,
            _ => false,
        };
        if !primary_ok {
            return false;
        }
        match (
            &self.secondary_faction_id,
            self.secondary_threshold,
            &self.secondary_direction,
        ) {
            (Some(fid), Some(thresh), Some(dir)) => {
                let rep = get_rep(fid);
                match dir.as_str() {
                    "above" => rep >= thresh,
                    "below" => rep <= thresh,
                    _ => false,
                }
            }
            _ => true,
        }
    }
}
