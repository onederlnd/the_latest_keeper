use crate::faction::{FactionDef, FactionState};
use crate::rite::Rite;
use std::collections::HashMap;

// Result of a completed burial — bundles the money/reputation fallout
// so callers (UI, save system) get one struct instead of loose values.
#[derive(Debug, Clone)]
pub struct BurialResult {
    pub payment: f32,
    pub reputation_deltas: HashMap<String, f32>,
}

pub struct Economy {
    pub stipend: f32,
    pub faction_state: FactionState,
}

impl Economy {
    pub fn new(faction_defs: &[FactionDef]) -> Self {
        Economy {
            stipend: 0.0,
            faction_state: FactionState::from_defs(faction_defs),
        }
    }

    pub fn settle_burial(
        &mut self,
        rite: &Rite,
        faction_defs: &[FactionDef],
        base_payment: f32,
        corpse_quality: f32,
    ) -> BurialResult {
        let payment = base_payment * (corpse_quality / 5.0) * (rite.base_quantity / 5.0);
        self.stipend += payment;

        let reputation_deltas = self.faction_state.apply_rite(rite, faction_defs);

        BurialResult {
            payment,
            reputation_deltas,
        }
    }
    pub fn can_afford(&self, cost: f32) -> bool {
        self.stipend >= cost
    }

    pub fn spend(&mut self, cost: f32) -> bool {
        if self.can_afford(cost) {
            self.stipend -= cost;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as Map;

    fn village() -> FactionDef {
        let mut reactions = Map::new();
        reactions.insert("holy".to_string(), 2.0);
        FactionDef {
            id: "village".into(),
            name: "The Village".into(),
            tag_reactions: reactions,
            starting_reputation: 0.0,
        }
    }

    #[test]
    fn higher_quality_rites_pay_more() {
        let defs = vec![village()];
        let mut economy = Economy::new(&defs);

        let mut thin = Rite::default();
        thin.base_quantity = 1.0;
        let thin_pay = economy.settle_burial(&thin, &defs, 10.0, 5.0).payment;
        assert_eq!(thin_pay, 2.0);

        let mut stacked = Rite::default();
        stacked.base_quantity = 5.0;
        let stacked_pay = economy.settle_burial(&stacked, &defs, 10.0, 5.0).payment;
        assert_eq!(stacked_pay, 10.0);

        let low_quality_pay = economy.settle_burial(&stacked, &defs, 10.0, 2.5).payment;
        assert_eq!(low_quality_pay, 5.0);
    }

    #[test]
    fn spend_fails_when_broke() {
        let defs = vec![village()];
        let mut economy = Economy::new(&defs);
        assert!(!economy.spend(50.0));
        economy.stipend = 100.0;
        assert!(economy.spend(50.0));
    }
}
