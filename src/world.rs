// src/world.rs

use crate::npc::Npc;
use crate::render;
use crate::village_layout::door_anchor;
use crate::village_layout::{self, Layout, VillageProp};
use crate::zone::DoorSide;
use crate::zone::{Zone, ZoneDef, ZoneKind, ZoneRegistry};
use macroquad::rand::{self, RandGenerator};
use std::collections::HashMap;

const STRUCTURES: &[(ZoneKind, &str, u32, (f32, f32))] = &[
    (ZoneKind::Store, "Store", 1, (140.0, 100.0)),
    (ZoneKind::Home, "Home", 5, (100.0, 80.0)),
];

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
    pub props: Vec<VillageProp>,
}

fn place_structures(
    zones: &mut ZoneRegistry,
    village: &str,
    rng: &RandGenerator,
    layout: &mut Layout,
) {
    for &(kind, label, count, (w, h)) in STRUCTURES {
        for i in 0..count {
            let pose = layout
                .try_place(zones, rng, w, h)
                .unwrap_or_else(|| panic!("could not place {label} {i} in {village}"));
            let id = format!("{village}_{}_{i}", label.to_lowercase());
            let mut zone = Zone::new(&id, label, pose.pos, (w, h), None, None);
            zone.kind = kind;
            zone.door = Some((pose.side, pose.off));
            zones.register(zone);
        }
    }
}
fn sibling(id: &str) -> Option<&'static str> {
    crate::THEMES.iter().find_map(|t| {
        if t.1 == id {
            Some(t.2)
        } else if t.2 == id {
            Some(t.1)
        } else {
            None
        }
    })
}

pub fn seed_for(id: &str, cache: &HashMap<String, VillageInstance>) -> u64 {
    sibling(id)
        .and_then(|s| cache.get(s))
        .map(|i| i.seed)
        .unwrap_or_else(|| rand::gen_range(0, u64::MAX))
}

pub fn generate_village(zone_id: &str, seed: u64) -> VillageInstance {
    let rng = RandGenerator::new();
    rng.srand(seed);
    let mut layout = village_layout::build_roads(&rng);

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

        let pose = layout
            .try_place(&zones, &rng, 80.0, 60.0)
            .expect("could not place grounds");
        let mut grounds = Zone::from_def(&ZoneDef {
            id: grounds_id.to_string(),
            name,
            position: pose.pos,
            size: (80.0, 60.0),
            starting_stock,
            faction_id: None,
            unlocks_from: None,
            kind: ZoneKind::Grounds,
            door: None,
        });
        grounds.door = Some((pose.side, pose.off));
        zones.register(grounds);
    }

    place_structures(&mut zones, zone_id, &rng, &mut layout);
    let tilemap = layout.tilemap();
    let props = layout.scatter_props(&zones, &rng);

    let mut inst = VillageInstance {
        tilemap,
        npcs,
        zones,
        exit_zone: (-30.0, -30.0, 60.0, 60.0),
        seed,
        props,
    };
    if is_cult(zone_id) {
        flip_x(&mut inst);
    }
    inst
}

pub fn generate_tilemap(
    width: usize,
    height: usize,
    tile_size: f32,
    origin: (f32, f32),
    hub_world: (f32, f32),
    targets_world: &[(f32, f32)],
) -> render::TileMapDef {
    // Public entry point (hub etc.): seed a local generator from the global one.
    let rng = RandGenerator::new();
    rng.srand(rand::gen_range(0u64, u64::MAX));
    generate_tilemap_with(
        &rng,
        width,
        height,
        tile_size,
        origin,
        hub_world,
        targets_world,
    )
}

pub fn generate_tilemap_with(
    rng: &RandGenerator,
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
        carve_path(&mut grid, hub, target, width, height, rng);
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
        solids: vec![],
    }
}

fn carve_path(
    grid: &mut [Vec<char>],
    start: (isize, isize),
    target: (isize, isize),
    width: usize,
    height: usize,
    rng: &RandGenerator,
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
        } else if wander_choice > 0 && rng.gen_range(0, wander_choice) == 0 {
            if primary_is_x {
                current.1 += if rng.gen_range(0, 2) == 0 { 1 } else { -1 };
            } else {
                current.0 += if rng.gen_range(0, 2) == 0 { 1 } else { -1 };
            }
        } else if dx != 0 && (dy == 0 || rng.gen_range(0, 2) == 0) {
            current.0 += dx.signum();
        } else if dy != 0 {
            current.1 += dy.signum();
        }

        current.0 = current.0.clamp(0, width as isize - 1);
        current.1 = current.1.clamp(0, height as isize - 1);
        budget -= 1;
    }
}
fn is_cult(zone_id: &str) -> bool {
    crate::THEMES.iter().any(|t| t.2 == zone_id)
}

fn flip_x(inst: &mut VillageInstance) {
    for z in inst.zones.zones.values_mut() {
        z.position.0 = -(z.position.0 + z.size.0);
        let w = z.size.0;
        z.door = z.door.take().map(|(side, off)| match side {
            DoorSide::Left => (DoorSide::Right, off),
            DoorSide::Right => (DoorSide::Left, off),
            DoorSide::Bottom => (DoorSide::Bottom, w - off),
            DoorSide::Top => (DoorSide::Top, w - off),
        });
    }
    for n in inst.npcs.iter_mut() {
        n.position.0 = -n.position.0;
    }
    for row in inst.tilemap.rows.iter_mut() {
        *row = row.chars().rev().collect();
    }
    let (x, y, w, h) = inst.exit_zone;
    inst.exit_zone = (-(x + w), y, w, h);
    for p in inst.props.iter_mut() {
        p.pos.0 = -p.pos.0;
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
    #[test]
    fn village_stock_items_exist() {
        let data = std::fs::read_to_string("data/items.ron").unwrap();
        let reagents: Vec<crate::rite::Reagent> = ron::from_str(&data).unwrap();
        for (_, _, item) in VILLAGE_STOCK {
            assert!(reagents.iter().any(|r| r.id == *item), "missing {item}");
        }
    }
    #[test]
    fn tending_zones_match_village_stock() {
        let data = std::fs::read_to_string("data/tending.ron").unwrap();
        let tasks: Vec<crate::tending::TendTask> = ron::from_str(&data).unwrap();
        for (_, grounds, item) in VILLAGE_STOCK {
            let t = tasks
                .iter()
                .find(|t| t.zone_id == *grounds)
                .unwrap_or_else(|| panic!("no tend task for {grounds}"));
            assert_eq!(t.restock_item, *item, "{grounds} restocks the wrong item");
        }
    }
    #[test]
    fn every_village_has_paths() {
        for (village, _, _) in VILLAGE_STOCK {
            let v = generate_village(village, 1);
            assert!(
                v.tilemap.rows.iter().any(|r| r.contains('P')),
                "{village} has no path"
            );
        }
    }
    #[test]
    fn structures_do_not_overlap() {
        for (village, _, _) in VILLAGE_STOCK {
            for seed in 0..20 {
                let v = generate_village(village, seed);
                let zs: Vec<_> = v.zones.zones.values().collect();
                for (i, a) in zs.iter().enumerate() {
                    for b in &zs[i + 1..] {
                        assert!(
                            !a.overlaps(b.position.0, b.position.1, b.size.0, b.size.1, 0.0),
                            "{village} seed {seed}: {} overlaps {}",
                            a.id,
                            b.id
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn cult_is_mirror_of_village() {
        for (_, v, c) in crate::THEMES {
            let a = generate_village(v, 7);
            let b = generate_village(c, 7);
            for za in a.zones.zones.values() {
                let zb = &b.zones.zones[&za.id.replacen(v, c, 1)];
                assert_eq!(zb.position.0, -(za.position.0 + za.size.0));
                assert_eq!(zb.position.1, za.position.1);
            }
            let flipped: Vec<String> = a
                .tilemap
                .rows
                .iter()
                .map(|r| r.chars().rev().collect())
                .collect();
            assert_eq!(flipped, b.tilemap.rows);
        }
    }
    #[test]
    fn every_door_opens_onto_a_path() {
        for (village, _, _) in VILLAGE_STOCK {
            for seed in 0..20 {
                let v = generate_village(village, seed);
                let t = &v.tilemap;
                for z in v.zones.zones.values() {
                    if let Some((ax, ay)) = door_anchor(z) {
                        let c = ((ax - t.origin.0) / t.tile_size).floor() as usize;
                        let r = ((ay - t.origin.1) / t.tile_size).floor() as usize;
                        assert_eq!(
                            t.rows[r].chars().nth(c),
                            Some('P'),
                            "{village} seed {seed}: {} door not on a path",
                            z.id
                        );
                    }
                }
            }
        }
    }
}
