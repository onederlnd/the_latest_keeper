use crate::rite::Rite;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disposal {
    Burial,
    Burning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorpseState {
    Delivered,
    Autopsied,
    GraveDug,
    Fueled,
    Buried,
    Burned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corpse {
    pub id: u32,
    pub state: CorpseState,
    pub base_quality: f32,
    pub age_secs: f32,
    pub decay_rate: f32,
    #[serde(skip)]
    pub pending_rite: Option<Rite>,
    pub requested_tag: Option<String>,
}

impl Corpse {
    pub fn new(id: u32, base_quality: f32) -> Self {
        Corpse {
            id,
            state: CorpseState::Delivered,
            base_quality,
            age_secs: 0.0,
            decay_rate: 0.005,
            pending_rite: None,
            requested_tag: None,
        }
    }

    pub fn autopsy(&mut self) {
        if self.state == CorpseState::Delivered {
            self.state = CorpseState::Autopsied;
        }
    }
    pub fn dig_grave(&mut self) {
        if self.state == CorpseState::Autopsied {
            self.state = CorpseState::GraveDug;
        }
    }

    pub fn stoke(&mut self) {
        if self.state == CorpseState::Autopsied {
            self.state = CorpseState::Fueled;
        }
    }

    pub fn tick_decay(&mut self, dt: f32) {
        if !matches!(self.state, CorpseState::Buried | CorpseState::Burned) {
            self.age_secs += dt;
        }
    }

    pub fn current_quality(&self) -> f32 {
        (self.base_quality - self.decay_rate * self.age_secs).max(0.0)
    }

    pub fn assign_rite(&mut self, rite: Rite) {
        self.pending_rite = Some(rite);
    }
    pub fn dispose(&mut self) -> (Rite, Disposal) {
        let method = match self.state {
            CorpseState::GraveDug => Disposal::Burial,
            CorpseState::Fueled => Disposal::Burning,
            _ => panic!("corpse is not ready for disposal"),
        };
        let rite = self
            .pending_rite
            .take()
            .expect("no rite assigned before disposal");
        self.state = match method {
            Disposal::Burial => CorpseState::Buried,
            Disposal::Burning => CorpseState::Burned,
        };
        (rite, method)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_starts_at_delivery() {
        let mut corpse = Corpse::new(1, 5.0);
        corpse.tick_decay(100.0); // 100s * 0.005 = 0.5 lost
        assert!((corpse.current_quality() - 4.5).abs() < 0.001);
    }

    #[test]
    fn dispose_before_ready_panics() {
        let mut corpse = Corpse::new(1, 5.0);
        corpse.autopsy();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| corpse.dispose()));
        assert!(result.is_err());
    }

    #[test]
    fn burial_flow_returns_rite() {
        let mut corpse = Corpse::new(1, 5.0);
        corpse.autopsy();
        corpse.dig_grave();
        let mut rite = Rite::default();
        rite.tags.insert("holy".to_string(), 1.0);
        corpse.assign_rite(rite);

        let (returned, method) = corpse.dispose();
        assert_eq!(corpse.state, CorpseState::Buried);
        assert_eq!(method, Disposal::Burial);
        assert_eq!(returned.tags["holy"], 1.0);
    }

    #[test]
    fn burning_flow_ends_burned() {
        let mut corpse = Corpse::new(1, 5.0);
        corpse.autopsy();
        corpse.stoke();
        corpse.assign_rite(Rite::default());
        let (_, method) = corpse.dispose();
        assert_eq!(corpse.state, CorpseState::Burned);
        assert_eq!(method, Disposal::Burning);
    }
}
