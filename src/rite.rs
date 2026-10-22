// src/rite.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Tag = String;

pub const THEME_TAGS: [&str; 4] = ["grief", "love", "guilt", "hope"];

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reagent {
    pub id: String,
    pub name: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Default)]
pub struct Rite {
    pub tags: HashMap<Tag, f32>,
    pub base_quantity: f32,
    pub reconciled: bool,
}

pub fn resolve_rite(reagents: &[Reagent]) -> Rite {
    let mut rite = Rite::default();

    for reagent in reagents {
        for tag in &reagent.tags {
            *rite.tags.entry(tag.clone()).or_insert(0.0) += 1.0;
        }
    }

    let total_weight: f32 = rite.tags.values().sum();
    let distinct_tags = rite.tags.len().max(1) as f32;
    rite.base_quantity = (total_weight / distinct_tags).min(5.0);

    rite.reconciled = THEME_TAGS.iter().any(|theme| {
        let has = |side: &str| {
            reagents
                .iter()
                .any(|r| r.tags.iter().any(|t| t == theme) && r.tags.iter().any(|t| t == side))
        };
        has("holy") && has("profane")
    });

    rite
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacking_the_same_tag_increases_its_strength() {
        let salt = Reagent {
            id: "salt".into(),
            name: "Salt".into(),
            tags: vec!["preserved".into()],
        };
        let candle = Reagent {
            id: "candle".into(),
            name: "Prayer candle".into(),
            tags: vec!["holy".into(), "preserved".into()],
        };

        let rite = resolve_rite(&[salt, candle]);

        assert_eq!(rite.tags["preserved"], 2.0);
        assert_eq!(rite.tags["holy"], 1.0);
    }

    fn reagent(id: &str, tags: &[&str]) -> Reagent {
        Reagent {
            id: id.into(),
            name: id.into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
        }
    }

    #[test]
    fn same_theme_on_both_sides_is_reconciled() {
        let bell = reagent("bell", &["holy", "grief"]);
        let ash = reagent("ash", &["profane", "grief"]);
        assert!(resolve_rite(&[bell, ash]).reconciled);
    }

    #[test]
    fn different_themes_are_not_reconciled() {
        let bell = reagent("bell", &["holy", "grief"]);
        let forge = reagent("locket", &["profane", "love", "bound"]);
        assert!(!resolve_rite(&[bell, forge]).reconciled);
    }

    #[test]
    fn one_side_only_is_not_reconciled() {
        let bell = reagent("bell", &["holy", "grief"]);
        let ring = reagent("ring", &["holy", "love", "bound"]);
        assert!(!resolve_rite(&[bell, ring]).reconciled);
    }

    #[test]
    fn every_theme_has_a_reconcilable_pair_in_items_ron() {
        let data = std::fs::read_to_string("data/items.ron").unwrap();
        let reagents: Vec<Reagent> = ron::from_str(&data).unwrap();

        for theme in THEME_TAGS {
            let side = |tag: &str| {
                reagents
                    .iter()
                    .find(|r| r.tags.iter().any(|t| t == theme) && r.tags.iter().any(|t| t == tag))
                    .unwrap_or_else(|| panic!("no {tag} reagent for theme {theme}"))
                    .clone()
            };
            let rite = resolve_rite(&[side("holy"), side("profane")]);
            assert!(rite.reconciled, "{theme} pair is not reconciled");
        }
    }

    #[test]
    fn items_use_only_known_tags() {
        let data = std::fs::read_to_string("data/items.ron").unwrap();
        let reagents: Vec<Reagent> = ron::from_str(&data).unwrap();
        let known = [
            "holy",
            "profane",
            "preserved",
            "bound",
            "grief",
            "love",
            "guilt",
            "hope",
        ];
        for r in &reagents {
            for t in &r.tags {
                assert!(known.contains(&t.as_str()), "{} has unknown tag {t}", r.id);
            }
        }
    }
}
