// src/zone.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_id: String,
    pub count: u32,
}

// A named location (workyard, morgue, cellar) whose storage is shared by
// every station inside it. Stations query their zone for inputs instead
// of holding private inventories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub storage: HashMap<String, ItemStack>,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub faction_id: Option<String>,
    pub unlocks_from: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ZoneDef {
    pub id: String,
    pub name: String,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub starting_stock: HashMap<String, u32>,
    #[serde(default)]
    pub faction_id: Option<String>,
    #[serde(default)]
    pub unlocks_from: Option<String>,
}

impl Zone {
    pub fn from_def(def: &ZoneDef) -> Self {
        let mut zone = Zone::new(
            &def.id,
            &def.name,
            def.position,
            def.size,
            def.faction_id.clone(),
            def.unlocks_from.clone(),
        );
        for (item_id, count) in &def.starting_stock {
            zone.deposit(item_id, *count);
        }
        zone
    }

    pub fn new(
        id: &str,
        name: &str,
        position: (f32, f32),
        size: (f32, f32),
        faction_id: Option<String>,
        unlocks_from: Option<String>,
    ) -> Self {
        Zone {
            id: id.to_string(),
            name: name.to_string(),
            storage: HashMap::new(),
            position,
            size,
            faction_id,
            unlocks_from,
        }
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        let (px, py) = point;
        let (zx, zy) = self.position;
        let (zw, zh) = self.size;
        px >= zx && px <= zx + zw && py >= zy && py <= zy + zh
    }

    pub fn deposit(&mut self, item_id: &str, count: u32) {
        self.storage
            .entry(item_id.to_string())
            .and_modify(|s| s.count += count)
            .or_insert(ItemStack {
                item_id: item_id.to_string(),
                count,
            });
    }

    // Removes up to `count` of an item. Returns how many were actually
    // removed (may be less than requested, or 0 if none available).
    pub fn withdraw(&mut self, item_id: &str, count: u32) -> u32 {
        let Some(stack) = self.storage.get_mut(item_id) else {
            return 0;
        };
        let taken = stack.count.min(count);
        stack.count -= taken;
        if stack.count == 0 {
            self.storage.remove(item_id);
        }
        taken
    }

    pub fn quantity_of(&self, item_id: &str) -> u32 {
        self.storage.get(item_id).map(|s| s.count).unwrap_or(0)
    }
}

// Holds every zone in the world, keyed by id, so systems can look up
// "the workyard" or "the morgue" without owning a Zone directly.

pub struct ZoneRegistry {
    pub zones: HashMap<String, Zone>,
}

impl ZoneRegistry {
    pub fn new() -> Self {
        ZoneRegistry {
            zones: HashMap::new(),
        }
    }

    pub fn register(&mut self, zone: Zone) {
        self.zones.insert(zone.id.clone(), zone);
    }

    pub fn get_mut(&mut self, id: &str) -> &mut Zone {
        self.zones
            .get_mut(id)
            .unwrap_or_else(|| panic!("missing zone: {id}"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_and_withdraw_round_trip() {
        let mut zone = Zone::new(
            "workyard",
            "Workyard",
            (0.0, 0.0),
            (100.0, 100.0),
            None,
            None,
        );
        zone.deposit("wood", 5);
        zone.deposit("wood", 3);
        assert_eq!(zone.quantity_of("wood"), 8);

        let taken = zone.withdraw("wood", 6);
        assert_eq!(taken, 6);
        assert_eq!(zone.quantity_of("wood"), 2);
    }

    #[test]
    fn withdraw_caps_at_available_amount() {
        let mut zone = Zone::new("morgue", "Morgue", (0.0, 0.0), (100.0, 100.0), None, None);
        zone.deposit("salt", 2);
        let taken = zone.withdraw("salt", 10);
        assert_eq!(taken, 2);
        assert_eq!(zone.quantity_of("salt"), 0);
    }
}
