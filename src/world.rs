// src/world.rs
use crate::npc::Npc;
use crate::render;
use crate::zone::{Zone, ZoneDef, ZoneRegistry};
use macroquad::rand;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum MapId {
    Hub,
    Village(String),
}

pub struct VillageInstance {
    pub tilemap: render::TileMapDef,
    pub npcs: Vec<Npc>,
    pub zones: ZoneRegistry,
    pub exit_zone: (f32, f32, f32, f32),
    pub seed: u64,
}
/// Placeholder generator — fill in real layout/content later.
pub fn generate_village(zone_id: &str, seed: u64) -> VillageInstance {
    rand::srand(seed);
    let width = 50;
    let height = 60;
    let tile_size = 32.0;
    let origin = (
        -(width as f32 * tile_size) / 2.0,
        -(height as f32 * tile_size) / 2.0,
    );

    let mut zones = ZoneRegistry::new();

    let npcs: Vec<Npc> = crate::load_npcs()
        .into_iter()
        .filter(|n| n.map.as_deref() == Some(zone_id))
        .collect();

    if let Some((_, grounds_id, item_id)) = VILLAGE_STOCK.iter().find(|(v, _, _)| *v == zone_id) {
        let mut starting_stock = HashMap::new();
        starting_stock.insert(item_id.to_string(), 3);

        let name = match zone_id {
            "mournhollow" => "MourningGrounds".to_string(),
            other => {
                let mut c = other.chars();
                let first: String = c
                    .next()
                    .map(|f| f.to_uppercase().collect())
                    .unwrap_or_default();
                format!("{first}{} Grounds", c.as_str())
            }
        };

        zones.register(Zone::from_def(&ZoneDef {
            id: grounds_id.to_string(),
            name,
            position: (-40.0, 60.0),
            size: (80.0, 60.0),
            starting_stock,
            faction_id: None,
            unlocks_from: None,
        }));
    }

    let mut targets: Vec<(f32, f32)> = zones
        .zones
        .values()
        .filter(|z| !z.id.ends_with("_grounds"))
        .map(|z| (z.position.0 + z.size.0 / 2.0, z.position.1 + z.size.1 / 2.0))
        .collect();
    targets.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let path = format!("data/tilemap_{zone_id}.ron");

    let tilemap = if let Ok(data) = std::fs::read_to_string(&path) {
        ron::from_str(&data).unwrap_or_else(|_| {
            generate_tilemap(width, height, tile_size, origin, (0.0, 0.0), &targets)
        })
    } else {
        generate_tilemap(width, height, tile_size, origin, (0.0, 0.0), &targets)
    };

    rand::srand(macroquad::miniquad::date::now() as u64);

    VillageInstance {
        tilemap,
        npcs,
        zones,
        exit_zone: (-30.0, -30.0, 60.0, 60.0),
        seed,
    }
}

pub fn generate_tilemap(
    width: usize,
    height: usize,
    tile_size: f32,
    origin: (f32, f32),
    hub_world: (f32, f32),
    targets_world: &[(f32, f32)],
) -> render::TileMapDef {
    let mut grid = vec![vec!['G'; width]; height];

    let to_grid = |world: (f32, f32)| -> (isize, isize) {
        let col = ((world.0 - origin.0) / tile_size).floor() as isize;
        let row = ((world.1 - origin.1) / tile_size).floor() as isize;
        (
            col.clamp(0, width as isize - 1),
            row.clamp(0, height as isize - 1),
        )
    };
    let hub = to_grid(hub_world);
    for &target_world in targets_world {
        let target = to_grid(target_world);
        carve_path(&mut grid, hub, target, width, height);
    }

    let rows: Vec<String> = grid.iter().map(|r| r.iter().collect()).collect();

    let mut legend = HashMap::new();
    legend.insert('G', "grass".to_string());
    legend.insert('P', "path".to_string());

    render::TileMapDef {
        tile_size,
        origin,
        legend,
        rows,
    }
}

fn carve_path(
    grid: &mut [Vec<char>],
    start: (isize, isize),
    target: (isize, isize),
    width: usize,
    height: usize,
) {
    let mut current = start;
    let dx0 = target.0 - current.0;
    let dy0 = target.1 - current.1;
    let base_distance = dx0.unsigned_abs() + dy0.unsigned_abs();
    let mut budget: i64 = base_distance as i64 + base_distance as i64 / 3 + 4;

    let primary_is_x = dx0.unsigned_abs() >= dy0.unsigned_abs();

    loop {
        if current.0 >= 0
            && current.1 >= 0
            && (current.0 as usize) < width
            && (current.1 as usize) < height
        {
            grid[current.1 as usize][current.0 as usize] = 'P';
        }
        if current == target {
            break;
        }
        let dx = target.0 - current.0;
        let dy = target.1 - current.1;
        let remaining: i64 = (dx.unsigned_abs() + dy.unsigned_abs()) as i64;

        let wander_choice = if remaining <= 4 {
            0
        } else if remaining <= 10 {
            10
        } else {
            6
        };

        if budget <= remaining {
            if dx != 0 {
                current.0 += dx.signum();
            } else {
                current.1 += dy.signum();
            }
        } else if wander_choice > 0 && rand::gen_range(0, wander_choice) == 0 {
            if primary_is_x {
                current.1 += if rand::gen_range(0, 2) == 0 { 1 } else { -1 };
            } else {
                current.0 += if rand::gen_range(0, 2) == 0 { 1 } else { -1 };
            }
        } else if dx != 0 && (dy == 0 || rand::gen_range(0, 2) == 0) {
            current.0 += dx.signum();
        } else if dy != 0 {
            current.1 += dy.signum();
        }

        current.0 = current.0.clamp(0, width as isize - 1);
        current.1 = current.1.clamp(0, height as isize - 1);
        budget -= 1;
    }
}

pub const VILLAGE_STOCK: [(&str, &str, &str); 8] = [
    ("mournhollow", "mournhollow_grounds", "mourning_wreath"),
    ("ashmourn", "ashmourn_grounds", "widows_ash"),
    ("heartwarden", "heartwarden_grounds", "loveknot"),
    ("heartforge", "heartforge_grounds", "forgechain"),
    ("penncross", "penncross_grounds", "penitent_stone"),
    ("guiltpyre", "guiltpyre_grounds", "pyre_coal"),
    ("hopewell", "hopewell_grounds", "hopewell_bloom"),
    ("hopeforsworn", "hopeforsworn_grounds", "forsworn_husk"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_village_gets_a_grounds_zone_with_its_reagent() {
        for (village, grounds, item) in VILLAGE_STOCK {
            let v = generate_village(village, 1);
            let z = v
                .zones
                .zones
                .get(grounds)
                .unwrap_or_else(|| panic!("{village} has no {grounds}"));
            assert!(z.storage.contains_key(item), "{grounds} missing {item}");
        }
    }

    #[test]
    fn stock_ids_exist_in_data_files() {
        let items: Vec<crate::rite::Reagent> =
            ron::from_str(&std::fs::read_to_string("data/items.ron").unwrap()).unwrap();
        let zones: Vec<ZoneDef> =
            ron::from_str(&std::fs::read_to_string("data/zones.ron").unwrap()).unwrap();
        for (village, _, item) in VILLAGE_STOCK {
            assert!(
                items.iter().any(|r| r.id == item),
                "{item} not in items.ron"
            );
            assert!(
                zones.iter().any(|z| z.id == village),
                "{village} not in zones.ron"
            );
        }
    }
    #[test]
    fn faction_tend_tasks_are_keyed_to_grounds_zones() {
        let tasks = crate::load_tending();
        for t in tasks.iter().filter(|t| !t.faction_id.is_empty()) {
            assert!(
                VILLAGE_STOCK.iter().any(|&(_, g, _)| g == t.zone_id),
                "{} is not a village grounds zone",
                t.zone_id
            );
        }
        for (_, grounds, _) in VILLAGE_STOCK {
            assert!(
                tasks.iter().any(|t| t.zone_id == grounds),
                "no tend task for {grounds}"
            );
        }
    }
}
