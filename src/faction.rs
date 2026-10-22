// src/faction.rs

use crate::rite::{Rite, Tag};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FactionDef {
    pub id: String,
    pub name: String,
    pub tag_reactions: HashMap<Tag, f32>,
    pub starting_reputation: f32,
}

pub struct FactionState {
    pub reputation: HashMap<String, f32>,
}

impl FactionState {
    pub fn from_defs(defs: &[FactionDef]) -> Self {
        let reputation = defs
            .iter()
            .map(|d| (d.id.clone(), d.starting_reputation))
            .collect();
        FactionState { reputation }
    }

    pub fn apply_rite(&mut self, rite: &Rite, defs: &[FactionDef]) -> HashMap<String, f32> {
        let mut deltas = HashMap::new();

        for def in defs {
            let mut delta = 0.0;
            for (tag, strength) in &rite.tags {
                if let Some(reaction) = def.tag_reactions.get(tag) {
                    delta += reaction * strength;
                }
            }
            if delta != 0.0 {
                *self.reputation.entry(def.id.clone()).or_insert(0.0) += delta;
                deltas.insert(def.id.clone(), delta);
            }
        }
        deltas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as Map;

    fn faction(id: &str, holy: f32, profane: f32) -> FactionDef {
        let mut reactions = Map::new();
        reactions.insert("holy".to_string(), holy);
        reactions.insert("profane".to_string(), profane);
        reactions.insert("bound".to_string(), 0.0);
        reactions.insert("preserved".to_string(), 0.0);
        FactionDef {
            id: id.into(),
            name: id.into(),
            tag_reactions: reactions,
            starting_reputation: 0.0,
        }
    }
    fn village() -> FactionDef {
        faction("village", 1.0, -1.0)
    }
    fn cult() -> FactionDef {
        faction("cult", -1.0, 1.0)
    }

    #[test]
    fn a_holy_rite_pleases_the_village_and_upsets_the_cult() {
        let defs = vec![village(), cult()];
        let mut state = FactionState::from_defs(&defs);

        let mut rite = Rite::default();
        rite.tags.insert("holy".to_string(), 2.0);

        let deltas = state.apply_rite(&rite, &defs);

        assert_eq!(deltas["village"], 2.0);
        assert_eq!(deltas["cult"], -2.0);
    }

    #[test]
    fn equal_holy_and_profane_cancel_for_both_factions() {
        let defs = vec![village(), cult()];
        let mut state = FactionState::from_defs(&defs);

        let mut rite = Rite::default();
        rite.tags.insert("holy".to_string(), 1.0);
        rite.tags.insert("profane".to_string(), 1.0);

        let deltas = state.apply_rite(&rite, &defs);

        assert!(deltas.is_empty());
        assert_eq!(state.reputation["village"], 0.0);
        assert_eq!(state.reputation["cult"], 0.0);
    }

    #[test]
    fn bound_and_preserved_do_not_move_reputation() {
        let defs = vec![village(), cult()];
        let mut state = FactionState::from_defs(&defs);

        let mut rite = Rite::default();
        rite.tags.insert("bound".to_string(), 3.0);
        rite.tags.insert("preserved".to_string(), 2.0);

        assert!(state.apply_rite(&rite, &defs).is_empty());
    }
}
