// src/main.rs

mod action;
mod cellar;
mod clock;
mod corpse;
mod discovery;
mod economy;
mod faction;
mod milestone;
mod npc;
mod progression;
mod quest;
mod render;
mod render_helpers;
mod rite;
mod save;
mod story;
mod tending;
mod tutorial;
mod world;
mod zone;

use crate::npc::Npc;
use crate::zone::ZoneRegistry;

use action::{ActionKind, ActionStatus, TimedAction};
use cellar::CellarRecord;
use clock::GameClock;
use corpse::{Corpse, CorpseState, Disposal};
use discovery::Discovery;
use economy::Economy;
use faction::FactionDef;
use macroquad::prelude::*;
use macroquad::rand;
use milestone::Milestone;
use progression::Tier;
use quest::{QuestDef, QuestKind, QuestStatus};
use render::{AssetStore, GameCamera};
use render_helpers::{
    draw_grave_markers, draw_progress_bar, draw_temple, point_in_rect, wrap_text,
};
use rite::{resolve_rite, Reagent};
use save::{load_game, save_game, SaveData};
use std::collections::hash_map;
use std::collections::HashMap;
use story::StoryBeat;
use tending::TendTask;
use tutorial::TutorialStep;
use world::{generate_tilemap, generate_village, MapId, VillageInstance, VILLAGE_STOCK};
use zone::Zone;

enum MenuChoice {
    NewGame,
    Continue,
    Quit,
}

enum ActiveWindow {
    None,
    Inventory,
    GameMap,
    Cellar,
    Sacrifice,
    Dialogue,
    ThemePicker,
    QuestLog,
}

#[derive(Clone, Copy, PartialEq)]
enum QuestMarker {
    Available,
    Pending,
    Ready,
}

struct UseableSlot {
    label: &'static str,
    hint: &'static str,
}

const AUTOPSY_SECS: f32 = 15.0;
const DIG_SECS: f32 = 20.0;
const STOKE_SECS: f32 = 20.0;
const INTERACT_REACH: f32 = 40.0;
const DISPOSAL_WEIGHT: f32 = 0.5;
const THEMES: [(&str, &str, &str); 4] = [
    ("grief", "mournhollow", "ashmourn"),
    ("love", "heartwarden", "heartforge"),
    ("guilt", "penncross", "guiltpyre"),
    ("hope", "hopewell", "hopeforsworn"),
];
const USEABLES: [UseableSlot; 4] = [
    UseableSlot {
        label: "Hoe",
        hint: "1",
    },
    UseableSlot {
        label: "Shovel",
        hint: "2",
    },
    UseableSlot {
        label: "Spell A",
        hint: "3",
    },
    UseableSlot {
        label: "Spell B",
        hint: "4",
    },
];
fn menu_button(rect: (f32, f32, f32, f32), label: &str, enabled: bool) -> bool {
    let hover = enabled && point_in_rect(mouse_position(), rect);
    let bg = if !enabled {
        Color::new(0.1, 0.1, 0.1, 0.6)
    } else if hover {
        Color::new(0.2, 0.5, 0.2, 0.85)
    } else {
        Color::new(0.15, 0.15, 0.15, 0.68)
    };
    draw_rectangle(rect.0, rect.1, rect.2, rect.3, bg);
    draw_rectangle_lines(
        rect.0,
        rect.1,
        rect.2,
        rect.3,
        2.0,
        if enabled { WHITE } else { GRAY },
    );
    let d = measure_text(label, None, 24, 1.0);
    draw_text(
        label,
        rect.0 + (rect.2 - d.width) / 2.0,
        rect.1 + rect.3 / 2.0 + d.height / 2.0,
        24.0,
        if enabled { WHITE } else { GRAY },
    );
    hover && is_mouse_button_pressed(MouseButton::Left)
}

fn target_zone(themes: &[String], pair: u32, cult: bool) -> Option<&'static str> {
    let theme = themes.get(pair as usize)?;
    THEMES
        .iter()
        .find(|t| t.0 == theme.as_str())
        .map(|t| if cult { t.2 } else { t.1 })
}
fn quest_marker(
    npc_id: &str,
    quest_defs: &[QuestDef],
    quest_status: &HashMap<String, (QuestStatus, u32)>,
    inventory: &HashMap<String, u32>,
) -> Option<QuestMarker> {
    let q = quest_defs
        .iter()
        .filter(|q| q.npc_id == npc_id)
        .find(|q| !matches!(quest_status.get(&q.id), Some((QuestStatus::Done { .. }, _))))?;

    match quest_status.get(&q.id).map(|(s, _)| s) {
        None => Some(QuestMarker::Available),
        Some(QuestStatus::Accepted) => {
            let have_item = match &q.kind {
                QuestKind::Fetch { item_id, count } => {
                    inventory.get(item_id).copied().unwrap_or(0) >= *count
                }
                _ => false,
            };
            if have_item {
                Some(QuestMarker::Ready)
            } else {
                Some(QuestMarker::Pending)
            }
        }
        Some(QuestStatus::Ready) => Some(QuestMarker::Ready),
        Some(QuestStatus::Done { .. }) => None,
    }
}

fn quest_objective(
    q: &QuestDef,
    inventory: &HashMap<String, u32>,
    reagent_defs: &HashMap<String, Reagent>,
) -> String {
    match &q.kind {
        QuestKind::Fetch { item_id, count } => {
            let name = reagent_defs
                .get(item_id)
                .map(|r| r.name.as_str())
                .unwrap_or(item_id.as_str());
            let have = inventory.get(item_id).copied().unwrap_or(0).min(*count);
            format!("Bring {count} {name} ({have}/{count})")
        }
        QuestKind::Tag { tag } => format!("Perform a rite with a {tag} reagent"),
        QuestKind::Timing { zone_id, by_day } => {
            format!("Be at {zone_id} by day {}", by_day + 1)
        }
    }
}

fn quest_reward_text(q: &QuestDef, reagent_defs: &HashMap<String, Reagent>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if q.reward_stipend > 0.0 {
        parts.push(format!("{:.0} stipend", q.reward_stipend));
    }
    if q.reward_rep != 0.0 {
        parts.push(format!("{:+.1} rep", q.reward_rep));
    }
    if let Some((id, n)) = &q.reward_item {
        let name = reagent_defs
            .get(id)
            .map(|r| r.name.as_str())
            .unwrap_or(id.as_str());
        parts.push(format!("{n} {name}"));
    }
    if parts.is_empty() {
        "Reward: none".to_string()
    } else {
        format!("Reward: {}", parts.join(", "))
    }
}

fn next_quest_in_days(
    npc_id: &str,
    quest_defs: &[QuestDef],
    quest_status: &HashMap<String, (QuestStatus, u32)>,
    today: u32,
) -> Option<u32> {
    quest_defs
        .iter()
        .filter(|q| q.npc_id == npc_id && q.cooldown_days > 0)
        .filter_map(|q| match quest_status.get(&q.id) {
            Some((QuestStatus::Done { day }, _)) => {
                Some((day + q.cooldown_days).saturating_sub(today))
            }
            _ => None,
        })
        .min()
}
fn locked_village_hint(zone_id: &str) -> &'static str {
    match zone_id {
        "mournhollow" => "Something quiet waits past the mist out of the north.",
        "ashmourn" => "Smoke, somewhere south. Not from here.",
        "heartwarden" => "A warmth in the distance that isn't the sun.",
        "heartforge" => "The clang of metal on metal, far off.",
        "penncross" => "Somewhere, a record is being kept of you.",
        "guiltpyre" => "The smell of old paper burning, faint.",
        "hopewell" => "A green so stubborn it shouldn't be there yet.",
        "hopeforsworn" => "A path that seems to lead nowhere in particular.",
        _ => "There's something out there, still out of reach",
    }
}

fn weekday_label(w: clock::Weekday) -> &'static str {
    match w {
        clock::Weekday::Moon => "Moonday",
        clock::Weekday::Tyr => "Tyrsday",
        clock::Weekday::Woden => "Wodensday",
        clock::Weekday::Thor => "Thorsday",
        clock::Weekday::Freyja => "Freyjasday",
        clock::Weekday::Sun => "Sunday",
    }
}

// Theme/Discovery Tiers
fn load_tiers() -> Vec<Tier> {
    let data = std::fs::read_to_string("data/progression.ron")
        .expect("failed to read data/progression.ron");
    ron::from_str(&data).expect("failed to parse data/progression.ron")
}

fn slot_of(themes: &[String], zone_id: &str) -> Option<usize> {
    let theme = THEMES.iter().find(|t| t.1 == zone_id || t.2 == zone_id)?.0;
    themes.iter().position(|c| c == theme)
}

fn tier_for(tiers: &[Tier], slot: usize) -> &Tier {
    &tiers[slot.min(tiers.len() - 1)]
}

// Reagents caller
fn load_reagent_defs() -> HashMap<String, Reagent> {
    let data = std::fs::read_to_string("data/items.ron").expect("failed to read data/items.ron");
    let reagents: Vec<Reagent> = ron::from_str(&data).expect("failed to parse data/items.ron");
    reagents.into_iter().map(|r| (r.id.clone(), r)).collect()
}

fn selected_count(sel: &[String], id: &str) -> usize {
    sel.iter().filter(|s| s.as_str() == id).count()
}

fn add_reagent(sel: &mut Vec<String>, id: &str, owned: u32, log: &mut Vec<String>) -> bool {
    if sel.len() >= 4 {
        log.push("Rite already has 4 reagents.".to_string());
        false
    } else if selected_count(sel, id) as u32 >= owned {
        log.push("You don't have more of that.".to_string());
        false
    } else {
        sel.push(id.to_string());
        true
    }
}

fn remove_reagent(sel: &mut Vec<String>, id: &str) {
    if let Some(i) = sel.iter().position(|s| s.as_str() == id) {
        sel.remove(i);
    }
}
// faction caller
fn load_faction_defs() -> Vec<FactionDef> {
    let data =
        std::fs::read_to_string("data/factions.ron").expect("failed to read data/factions.ron");
    ron::from_str(&data).expect("failed to parse data/factions.ron")
}
// zone caller
fn load_zone_defs() -> Vec<zone::ZoneDef> {
    let data = std::fs::read_to_string("data/zones.ron").expect("failed to read data/zones.ron");
    ron::from_str(&data).expect("failed to parse data/zones.ron")
}

// load npcs
fn load_npcs() -> Vec<Npc> {
    let data = std::fs::read_to_string("data/npcs.ron").expect("failed to read data/npcs.ron");
    ron::from_str(&data).expect("failed to parse data/npcs.ron")
}

// load milestones
fn load_milestones() -> Vec<Milestone> {
    let data =
        std::fs::read_to_string("data/milestones.ron").expect("failed to read data/milestones.ron");
    ron::from_str(&data).expect("failed to parse data/milestones.ron")
}

fn load_tutorial() -> Vec<TutorialStep> {
    let data =
        std::fs::read_to_string("data/tutorial.ron").expect("failed to read data/tutorial.ron");
    ron::from_str(&data).expect("failed to parse data/tutorial.ron")
}
fn load_story_beats() -> Vec<StoryBeat> {
    let data = std::fs::read_to_string("data/story_beats.ron")
        .expect("failed to read data/story_beats.ron");
    ron::from_str(&data).expect("failed to parse data/story_beats.ron")
}

fn load_cellar_records() -> Vec<CellarRecord> {
    let data = std::fs::read_to_string("data/cellar_records.ron")
        .expect("failed to read data/cellar_records.ron");
    ron::from_str(&data).expect("failed to parse data/cellar_records.ron")
}

fn load_discoveries() -> Vec<Discovery> {
    let data = std::fs::read_to_string("data/discoveries.ron")
        .expect("failed to read data/discoveries.ron");
    ron::from_str(&data).expect("failed to parse data/discoveries.ron")
}

fn load_tending() -> Vec<TendTask> {
    let data =
        std::fs::read_to_string("data/tending.ron").expect("failed to read data/tending.ron");
    ron::from_str(&data).expect("failed to parse data/tending.ron")
}

fn load_quests() -> Vec<QuestDef> {
    let data = std::fs::read_to_string("data/quests.ron").expect("failed to read data/quests.ron");
    ron::from_str(&data).expect("failed to parse data/quests.ron")
}

fn fire_tutorial(
    steps: &[TutorialStep],
    fired: &mut std::collections::HashSet<String>,
    trigger: &str,
    log: &mut Vec<String>,
    banner: &mut Option<(String, f32)>,
) {
    for step in steps {
        if step.trigger == trigger && !fired.contains(&step.id) {
            log.push(step.text.clone());
            *banner = Some((step.text.clone(), 6.0));
            fired.insert(step.id.clone());
            return;
        }
    }
}

fn tilemap_path(map: &MapId) -> String {
    match map {
        MapId::Hub => "data/tilemap.hub.ron".to_string(),
        MapId::Village(id) => format!("data/tilemap_{id}.ron"),
    }
}

fn load_or_generate_tilemap(
    path: &str,
    width: usize,
    height: usize,
    tile_size: f32,
    origin: (f32, f32),
    hub_world: (f32, f32),
    targets: &[(f32, f32)],
) -> render::TileMapDef {
    if let Ok(data) = std::fs::read_to_string(path) {
        if let Ok(def) = ron::from_str(&data) {
            return def;
        }
    }
    generate_tilemap(width, height, tile_size, origin, hub_world, targets)
}

fn closest_npc_in_range<'a>(npcs: &'a [Npc], point: (f32, f32), range: f32) -> Option<&'a Npc> {
    npcs.iter()
        .filter(|n| n.distance_to(point) < range)
        .min_by(|a, b| {
            a.distance_to(point)
                .partial_cmp(&b.distance_to(point))
                .unwrap()
        })
}

fn village_zone_contains<'a>(zones: &'a ZoneRegistry, point: (f32, f32)) -> Option<&'a Zone> {
    zones
        .zones
        .values()
        .find(|z| z.faction_id.is_some() && z.contains(point))
}
fn hub_zones<'a>(
    map: &MapId,
    zones: &'a ZoneRegistry,
    cache: &'a HashMap<String, VillageInstance>,
) -> &'a ZoneRegistry {
    match map {
        MapId::Hub => zones,
        MapId::Village(id) => &cache[id].zones,
    }
}

fn hub_zones_mut<'a>(
    map: &MapId,
    zones: &'a mut ZoneRegistry,
    cache: &'a mut HashMap<String, VillageInstance>,
) -> &'a mut ZoneRegistry {
    match map {
        MapId::Hub => zones,
        MapId::Village(id) => &mut cache.get_mut(id).unwrap().zones,
    }
}

fn village_zones<'a>(
    id: &str,
    map: &MapId,
    zones: &'a ZoneRegistry,
    cache: &'a HashMap<String, VillageInstance>,
) -> Option<&'a ZoneRegistry> {
    if matches!(map, MapId:: Village(cur) if cur == id) {
        Some(zones)
    } else {
        cache.get(id).map(|inst| &inst.zones)
    }
}

fn village_zones_mut<'a>(
    id: &str,
    map: &MapId,
    zones: &'a mut ZoneRegistry,
    cache: &'a mut HashMap<String, VillageInstance>,
) -> Option<&'a mut ZoneRegistry> {
    if matches!(map, MapId::Village(cur) if cur == id) {
        Some(zones)
    } else {
        cache.get_mut(id).map(|inst| &mut inst.zones)
    }
}

fn hash(n: i32) -> f32 {
    let x = (n as u32).wrapping_mul(0x9E3779B1) ^ 0x85EBCA6B;
    let x = x.wrapping_mul(0xC2B2AE35);
    ((x ^ (x >> 15)) & 0xFFFF) as f32 / 65535.0
}

fn noise(t: f32) -> f32 {
    let i = t.floor();
    let f = t - i;
    let s = f * f * (3.0 - 2.0 * f);
    let a = hash(i as i32);
    let b = hash(i as i32 + 1);
    a + (b - a) * s
}

fn candle(t: f32) -> f32 {
    let slow = noise(t * 1.3) - 0.5;
    let mid = noise(t * 4.0 + 17.0) - 0.5;
    let fast = noise(t * 11.0 + 91.0) - 0.5;

    let g = noise(t * 0.7 + 200.0);
    let gutter = ((g - 0.8) / 0.2).clamp(0.0, 1.0) * -0.5;

    1.0 + slow * 0.6 + mid * 0.04 + fast * 0.02 + gutter * 0.1
}

fn window_conf() -> Conf {
    Conf {
        window_title: "The Latest Keeper".to_string(),
        window_width: 1600,
        window_height: 1200,
        ..Default::default()
    }
}

async fn load_assets(assets: &mut AssetStore) {
    const LIST: [(&str, &str); 9] = [
        ("grass", "assets/tiles/grass.png"),
        ("path", "assets/tiles/path.png"),
        ("player", "assets/sprites/player.png"),
        ("npc_elsbet", "assets/sprites/npc_elsbet.png"),
        ("npc_wren", "assets/sprites/npc_wren.png"),
        ("npc_halbrecht", "assets/sprites/npc_halbrecht.png"),
        ("npc_tamsin", "assets/sprites/npc_tamsin.png"),
        ("npc_odo", "assets/sprites/npc_odo.png"),
        ("npc_maren", "assets/sprites/npc_maren.png"),
    ];
    for (i, (id, path)) in LIST.iter().enumerate() {
        clear_background(Color::new(0.05, 0.05, 0.07, 1.0));
        let (sw, sh) = (screen_width(), screen_height());
        let title = "THE LATEST KEEPER";
        let d = measure_text(title, None, 48, 1.0);
        draw_text(title, (sw - d.width) / 2.0, sh * 0.4, 48.0, WHITE);

        let (bw, bh) = (400.0, 14.0);
        let (bx, by) = ((sw - bw) / 2.0, sh * 0.5);
        draw_rectangle(bx, by, bw, bh, Color::new(0.15, 0.15, 0.15, 1.0));
        draw_rectangle(bx, by, bw * (i as f32 / LIST.len() as f32), bh, GOLD);
        draw_rectangle_lines(bx, by, bw, bh, 2.0, WHITE);
        draw_text(&format!("Loading {id}..."), bx, by + 40.0, 18.0, GRAY);

        next_frame().await;
        assets.load(id, path).await;
    }
}

async fn main_menu() -> MenuChoice {
    let has_save = std::path::Path::new("save.ron").exists();
    loop {
        clear_background(Color::new(0.05, 0.05, 0.07, 1.0));
        let (sw, sh) = (screen_width(), screen_height());
        let title = "THE LATEST KEEPER";
        let d = measure_text(title, None, 64, 1.0);
        draw_text(title, (sw - d.width) / 2.0, sh * 0.25, 64.0, WHITE);

        let (bw, bh) = (300.0, 52.0);
        let x = (sw - bw) / 2.0;
        let y0 = sh * 0.4;
        if menu_button((x, y0 + 70.0, bw, bh), "New Game", true) {
            return MenuChoice::NewGame;
        }
        if menu_button((x, y0, bw, bh), "Continue", has_save) {
            return MenuChoice::Continue;
        }

        if menu_button((x, y0 + 140.0, bw, bh), "Quit", true) {
            return MenuChoice::Quit;
        }

        next_frame().await;
    }
}
#[macroquad::main(window_conf)]
async fn main() {
    rand::srand(macroquad::miniquad::date::now() as u64);
    let mut assets = AssetStore::new();
    load_assets(&mut assets).await;

    loop {
        let choice = main_menu().await;
        if matches!(choice, MenuChoice::Quit) {
            return;
        }
        next_frame().await;

        let mut clock = GameClock::new();
        let mut cam = GameCamera::new();
        let mut log: Vec<String> = Vec::new();
        let mut active_window = ActiveWindow::None;
        let mut current_map = MapId::Hub;
        let mut edit_mode = false;
        let mut selected_tool: Option<usize> = None;
        let mut edit_tile_id: String = "path".to_string();

        // Game cache
        let mut village_cache: HashMap<String, VillageInstance> = HashMap::new();

        // Player
        let mut player_pos = vec2(0.0, 150.0);
        let mut hub_player_pos = vec2(0.0, 0.0);
        let player_speed = 200.0;
        let mut player_speech: Option<(String, f32)> =
            Some(("Another day among the dead...".to_string(), 8.0));
        let mut player_inventory: HashMap<String, u32> = HashMap::new();
        let mut facing = vec2(0.0, 1.0); // defailt facing: down
                                         // Static defs
        let reagent_defs = load_reagent_defs();
        let mut selected_reagents: Vec<String> = Vec::new();
        let mut reagent_order: Vec<String> = reagent_defs.keys().cloned().collect();
        reagent_order.sort();

        let faction_defs = load_faction_defs();

        // Zones
        let mut zones = ZoneRegistry::new();
        for def in load_zone_defs() {
            zones.register(Zone::from_def(&def));
        }
        // World / tilemap
        let village_centers: Vec<(f32, f32)> = zones
            .zones
            .values()
            .filter(|z| z.faction_id.is_some())
            .map(|z| {
                let (zx, zy) = z.position;
                let (zw, zh) = z.size;
                (zx + zw / 2.0, zy + zh / 2.0)
            })
            .collect();

        let tile_size = 32.0;
        let width = 60;
        let height = 40;
        let origin = (
            -(width as f32 * tile_size) / 2.0,
            -(height as f32 * tile_size) / 2.0,
        );
        let hub_world = (0.0, 0.0);

        let tilemap_def = load_or_generate_tilemap(
            &tilemap_path(&MapId::Hub),
            width,
            height,
            tile_size,
            origin,
            hub_world,
            &village_centers,
        );
        let mut tilemap = render::TileMap::from_def(&tilemap_def);
        let hub_tilemap_def = tilemap_def.clone();

        // Story beats
        let story_beats = load_story_beats();
        let mut fired_beats: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Actions
        let mut active_action: Option<TimedAction> = None;
        let mut burn_timer: Option<f32> = None;
        let quest_defs = load_quests();
        let mut quest_status: HashMap<String, (QuestStatus, u32)> = HashMap::new();

        // Tutorial steps
        let tutorial_steps = load_tutorial();
        let mut fired_tutorial: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut tutorial_banner: Option<(String, f32)> = None;

        // Discovery (village unlocks)
        let discoveries = load_discoveries();
        let tiers = load_tiers();
        let mut discovered_zones: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut all_discovered = false;

        // Pair enforcement
        let mut pair_index: u32 = 0;
        let mut is_cult_life = false;
        let mut chosen_themes: Vec<String> = Vec::new();

        // Sacrifice milestones
        let milestones = load_milestones();
        let mut fire_milestones: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut sacrifice_available = false;
        let mut pending_fragment: Option<String> = None;
        let mut pending_sacrifice_faction: Option<String> = None;
        let mut pending_village: Option<String> = None;
        let mut memory_fragments: Vec<String> = Vec::new();
        let mut keeper_number: u32 = 1;
        let mut tenure_start_day: u32 = clock.day_index;
        let mut last_seen_day: u32 = clock.day_index;
        let mut missed_days_this_tenure: u32 = 0;
        let mut sacrifice_selection: HashMap<String, u32> = HashMap::new();

        // Cellar
        let mut cellar_records = load_cellar_records();

        // Economy
        let mut economy = Economy::new(&faction_defs);

        // Corpse
        let mut corpse: Option<Corpse> = None;
        let mut next_corpse_id = 1;

        // NPCs
        let mut npcs: Vec<Npc> = load_npcs()
            .into_iter()
            .filter(|n| n.map.is_none())
            .collect();
        let mut npc_speech: Option<(String, (f32, f32), f32)> = None;
        let mut npc_dialogue: Option<String> = None;
        let npc_names: HashMap<String, String> =
            load_npcs().into_iter().map(|n| (n.id, n.name)).collect();
        // Daily restock/delivery gating
        let mut last_delivery_day: Option<u32> = None;
        let mut last_restock_day: Option<u32> = None;

        // Tasks
        let tend_tasks = load_tending();
        let mut last_tended: HashMap<String, u32> = HashMap::new();

        // Main Menu
        let mut paused = false;
        let mut request_save = false;
        let mut request_load = matches!(choice, MenuChoice::Continue);
        if matches!(choice, MenuChoice::NewGame) {
            fire_tutorial(
                &tutorial_steps,
                &mut fired_tutorial,
                "game_start",
                &mut log,
                &mut tutorial_banner,
            );
        }

        // Quest progress
        const CARRY_OVER: f32 = 0.2;
        let mut check_progress = false;

        // Taskbar keys
        const TASKBAR_KEYS: [KeyCode; 9] = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
        ];

        let vignette_tex = {
            let n = 64u16;
            let mut img = Image::gen_image_color(n, n, Color::new(0.0, 0.0, 0.02, 0.0));
            for y in 0..n {
                for x in 0..n {
                    let dx = (x as f32 / (n - 1) as f32 - 0.5) * 2.0;
                    let dy = (y as f32 / (n - 1) as f32 - 0.5) * 2.0;
                    let d = (dx * dx + dy * dy).sqrt();
                    let a = ((d - 0.6) / 0.9).clamp(0.0, 1.0).powf(1.5) * 0.7;
                    img.set_pixel(x as u32, y as u32, Color::new(0.0, 0.0, 0.02, a));
                }
            }
            let t = Texture2D::from_image(&img);
            t.set_filter(FilterMode::Linear);
            t
        };

        let light_tex = {
            let n = 128u16;
            let mut img = Image::gen_image_color(n, n, Color::new(0.02, 0.03, 0.09, 1.0));
            for y in 0..n {
                for x in 0..n {
                    let dx = (x as f32 / (n - 1) as f32 - 0.5) * 2.0;
                    let dy = (y as f32 / (n - 1) as f32 - 0.5) * 2.0;
                    let d = (dx * dx + dy * dy).sqrt();
                    let t = ((d - 0.2) / 0.8).clamp(0.0, 1.0);
                    let a = t * t * (3.0 - 2.0 * t);
                    img.set_pixel(x as u32, y as u32, Color::new(0.02, 0.03, 0.09, a));
                }
            }
            let t = Texture2D::from_image(&img);
            t.set_filter(FilterMode::Linear);
            t
        };
        'game: loop {
            draw_texture_ex(
                &vignette_tex,
                0.0,
                0.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(screen_width(), screen_height())),
                    ..Default::default()
                },
            );
            if is_key_pressed(KeyCode::Escape) {
                if matches!(active_window, ActiveWindow::None) {
                    paused = !paused;
                } else if !matches!(
                    active_window,
                    ActiveWindow::Dialogue | ActiveWindow::ThemePicker
                ) {
                    active_window = ActiveWindow::None;
                }
            }
            if paused {
                set_default_camera();
                clear_background(Color::new(0.03, 0.03, 0.05, 1.0));
                let (sw, sh) = (screen_width(), screen_height());
                let d = measure_text("PAUSED", None, 56, 1.0);
                draw_text("PAUSED", (sw - d.width) / 2.0, sh * 0.25, 56.0, WHITE);

                let (bw, bh) = (300.0, 52.0);
                let x = (sw - bw) / 2.0;
                let y0 = sh * 0.35;
                let has_save = std::path::Path::new("save.ron").exists();

                if menu_button((x, y0, bw, bh), "Resume", true) {
                    paused = false;
                }

                if menu_button((x, y0 + 70.0, bw, bh), "Save Game", true) {
                    request_save = true;
                    paused = false;
                }

                if menu_button((x, y0 + 140.0, bw, bh), "Load Game", has_save) {
                    request_load = true;
                    paused = false;
                }

                if menu_button((x, y0 + 210.0, bw, bh), "Quit to Menu", true) {
                    break 'game;
                }

                if menu_button((x, y0 + 280.0, bw, bh), "Quit Game", true) {
                    return;
                }

                if paused {
                    next_frame().await;
                    continue;
                }
            }
            clock.tick(get_frame_time());

            let today = clock.day_index;
            quest_status.retain(|id, v| {
                if let QuestStatus::Done { day } = &v.0 {
                    if let Some(q) = quest_defs.iter().find(|q| &q.id == id) {
                        if q.cooldown_days > 0 {
                            return today < *day + q.cooldown_days;
                        }
                    }
                }
                true
            });
            if corpse
                .as_ref()
                .map_or(false, |c| c.current_quality() <= 0.0)
            {
                let c = corpse.take().unwrap();
                log.push(format!("Corpse #{}  rotted away untended.", c.id));
                missed_days_this_tenure += 1;
            }
            if corpse.is_none() {
                burn_timer = None;
            }
            if let Some(t) = burn_timer.as_mut() {
                *t -= get_frame_time();
                if *t <= 0.0 {
                    if let Some(c) = corpse.as_mut() {
                        c.stoke();
                        log.push("The fire is ready.".to_string());
                    }
                    burn_timer = None;
                }
            }
            if let Some((_, _, ref mut timer)) = npc_speech {
                *timer -= get_frame_time();
                if *timer <= 0.0 {
                    npc_speech = None;
                }
            }

            if let Some((_, ref mut timer)) = player_speech {
                *timer -= get_frame_time();
                if *timer <= 0.0 {
                    player_speech = None;
                }
            }
            if let Some((_, ref mut timer)) = tutorial_banner {
                *timer -= get_frame_time();
                if *timer <= 0.0 {
                    tutorial_banner = None;
                }
            }
            ////////////////////
            // Keyboard keys  //
            ////////////////////

            let ui_open = !matches!(active_window, ActiveWindow::None);

            // Directional Keys
            let mut move_dir = vec2(0.0, 0.0);
            if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
                move_dir.y -= 1.0;
            }
            if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
                move_dir.y += 1.0;
            }
            if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
                move_dir.x -= 1.0;
            }
            if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
                move_dir.x += 1.0;
            }
            if ui_open {
                move_dir = vec2(0.0, 0.0);
            }
            if move_dir.length() > 0.0 {
                facing = move_dir.normalize();
            }
            if move_dir.length() > 0.0 {
                player_pos += move_dir.normalize() * player_speed * get_frame_time();
            }

            let interaction_point = (
                player_pos.x + facing.x * INTERACT_REACH,
                player_pos.y + facing.y * INTERACT_REACH,
            );

            if let Some(mut a) = active_action.take() {
                match a.tick(
                    get_frame_time(),
                    is_key_down(KeyCode::F),
                    (player_pos.x, player_pos.y),
                ) {
                    ActionStatus::InProgress => active_action = Some(a),
                    ActionStatus::Cancelled => {
                        log.push("You stop.".to_string());
                    }
                    ActionStatus::Done => {
                        match a.kind {
                            ActionKind::Autopsy => {
                                if let Some(c) = corpse.as_mut() {
                                    c.autopsy();
                                    log.push(format!("corpse #{} examined.", c.id));
                                }
                                fire_tutorial(
                                    &tutorial_steps,
                                    &mut fired_tutorial,
                                    "first_disposal_choice",
                                    &mut log,
                                    &mut tutorial_banner,
                                );
                            }
                            ActionKind::DigGrave => {
                                if let Some(c) = corpse.as_mut() {
                                    c.dig_grave();
                                    log.push("The grave is ready.".to_string());
                                }
                            }
                        };
                    }
                }
            }
            for zone in zones.zones.values() {
                if zone.faction_id.is_none() {
                    continue;
                }

                let discovered = discoveries
                    .iter()
                    .any(|d| d.zone_id == zone.id && discovered_zones.contains(&d.id));

                if discovered {
                    continue;
                }

                if slot_of(&chosen_themes, &zone.id).is_none() {
                    continue;
                }

                let (zx, zy) = zone.position;
                let (zw, zh) = zone.size;
                let center = (zx + zw / 2.0, zy + zh / 2.0);
                let dist =
                    ((player_pos.x - center.0).powi(2) + (player_pos.y - center.1).powi(2)).sqrt();

                if dist < 150.0 && player_speech.is_none() {
                    player_speech = Some((locked_village_hint(&zone.id).to_string(), 4.0));
                    break;
                }
            }
            if matches!(active_window, ActiveWindow::Dialogue)
                && (is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::Escape))
            {
                active_window = ActiveWindow::None;
                npc_dialogue = None;
            }
            if is_key_pressed(KeyCode::E) && active_action.is_none() && !ui_open {
                let mut handled = false;

                if let Some(npc) = closest_npc_in_range(&npcs, interaction_point, INTERACT_REACH) {
                    let visible = match &npc.zone_id {
                        Some(zid) => discoveries
                            .iter()
                            .any(|d| &d.zone_id == zid && discovered_zones.contains(&d.id)),
                        None => true,
                    };
                    if visible && npc.is_active(&clock) {
                        let rep = economy
                            .faction_state
                            .reputation
                            .get(&npc.faction_id)
                            .copied()
                            .unwrap_or(0.0);
                        let line = npc.line_for_reputation(rep);
                        npc_speech = Some((line.clone(), npc.position, 3.0));
                        log.push(line);
                        if quest_defs.iter().any(|q| q.npc_id == npc.id) {
                            npc_dialogue = Some(npc.id.clone());
                            active_window = ActiveWindow::Dialogue;
                        }
                        handled = true;
                    }
                }

                if !handled && matches!(current_map, MapId::Hub) {
                    if let Some(zone) = village_zone_contains(&zones, interaction_point) {
                        let discovered = discoveries
                            .iter()
                            .any(|d| d.zone_id == zone.id && discovered_zones.contains(&d.id));
                        if discovered {
                            let zone_id = zone.id.clone();
                            let zone_name = zone.name.clone();
                            hub_player_pos = player_pos;
                            current_map = MapId::Village(zone_id.clone());
                            let instance =
                                village_cache.entry(zone_id.clone()).or_insert_with(|| {
                                    generate_village(&zone_id, rand::gen_range(0, u64::MAX) as u64)
                                });
                            tilemap = render::TileMap::from_def(&instance.tilemap);
                            std::mem::swap(&mut zones, &mut instance.zones);
                            std::mem::swap(&mut npcs, &mut instance.npcs);
                            player_pos = vec2(0.0, 150.0);
                            log.push(format!("You step into {}", zone_name));
                            fire_tutorial(
                                &tutorial_steps,
                                &mut fired_tutorial,
                                "first_village_travel",
                                &mut log,
                                &mut tutorial_banner,
                            );
                            handled = true;
                        }
                    }
                }

                if !handled {
                    let mut got_something = false;
                    for (_zone_id, zone) in zones.zones.iter_mut() {
                        if zone.contains(interaction_point) {
                            let item_ids: Vec<String> = zone.storage.keys().cloned().collect();
                            for item_id in item_ids {
                                let taken = zone.withdraw(&item_id, 1);
                                if taken > 0 {
                                    *player_inventory.entry(item_id.to_string()).or_insert(0) +=
                                        taken;
                                    got_something = true;
                                }
                            }
                        }
                    }
                    if got_something {
                        fire_tutorial(
                            &tutorial_steps,
                            &mut fired_tutorial,
                            "first_pickup",
                            &mut log,
                            &mut tutorial_banner,
                        );
                    }
                }
            }

            if is_key_pressed(KeyCode::F) && active_action.is_none() && !ui_open {
                if let Some(c) = corpse.as_ref() {
                    let here = (player_pos.x, player_pos.y);
                    let in_zone =
                        |id: &str| zones.zones.get(id).map_or(false, |z| z.contains(here));
                    let started = match c.state {
                        CorpseState::Delivered if in_zone("morgue") => {
                            Some((ActionKind::Autopsy, AUTOPSY_SECS))
                        }
                        CorpseState::Autopsied if in_zone("burial_site") => {
                            if selected_tool == Some(1) {
                                Some((ActionKind::DigGrave, DIG_SECS))
                            } else {
                                log.push("You need the Shovel [2] to dig.".to_string());
                                None
                            }
                        }
                        CorpseState::Autopsied if in_zone("incinerator") => {
                            if burn_timer.is_none() {
                                burn_timer = Some(STOKE_SECS);
                                log.push("You light the fire. It needs time.".to_string());
                            }
                            None
                        }
                        CorpseState::Delivered => {
                            log.push("Examined the corpse in the Morgue first.".to_string());
                            None
                        }
                        _ => None,
                    };

                    if let Some((kind, secs)) = started {
                        active_action = Some(TimedAction::new(kind, secs, here));
                    }
                }
            }

            if is_key_pressed(KeyCode::I) {
                active_window = match active_window {
                    ActiveWindow::Inventory => ActiveWindow::None,
                    _ => ActiveWindow::Inventory,
                };
            }

            if is_key_pressed(KeyCode::Q) {
                active_window = match active_window {
                    ActiveWindow::QuestLog => ActiveWindow::None,
                    _ => ActiveWindow::QuestLog,
                }
            }
            if is_key_pressed(KeyCode::M) {
                active_window = match active_window {
                    ActiveWindow::GameMap => ActiveWindow::None,
                    _ => ActiveWindow::GameMap,
                };
            }

            if is_key_pressed(KeyCode::T) && !ui_open {
                if let Some(task) = tend_tasks.iter().find(|t| {
                    zones
                        .zones
                        .get(&t.zone_id)
                        .map(|z| z.contains((player_pos.x, player_pos.y)))
                        .unwrap_or(false)
                }) {
                    if last_tended.get(&task.zone_id) == Some(&clock.day_index) {
                        log.push("Already tended today.".to_string());
                    } else {
                        let village_id = VILLAGE_STOCK
                            .iter()
                            .find(|&&(_, g, _)| g == task.zone_id)
                            .map(|&(v, _, _)| v);

                        let in_current_pair = village_id.and_then(|v| slot_of(&chosen_themes, v))
                            == Some(pair_index as usize);

                        if !task.faction_id.is_empty() && in_current_pair {
                            *economy
                                .faction_state
                                .reputation
                                .entry(task.faction_id.clone())
                                .or_insert(0.0) += task.amount;
                        }

                        if let Some(stipend) = task.stipend_amount {
                            economy.stipend += stipend;
                        }
                        log.push(task.flavor_text.clone());
                        last_tended.insert(task.zone_id.clone(), clock.day_index);
                        check_progress = true;

                        if let Some(&(_, grounds_id, item_id)) =
                            VILLAGE_STOCK.iter().find(|&&(_, g, _)| g == task.zone_id)
                        {
                            if let Some(z) = zones.zones.get_mut(grounds_id) {
                                z.deposit(item_id, 1);
                            }
                        }
                    }
                }
            }

            // F-keys
            if is_key_pressed(KeyCode::F2) {
                edit_mode = !edit_mode;
                log.push(if edit_mode {
                        "Map edit mode ON - left click paints, right click erases grass, [ / ] switches tile, F6 saves.".to_string()
                    } else {
                        "Map edit mode OFF.".to_string()
                    });
            }
            if edit_mode {
                if is_key_pressed(KeyCode::LeftBracket) {
                    edit_tile_id = "grass".to_string();
                }
                if is_key_pressed(KeyCode::RightBracket) {
                    edit_tile_id = "path".to_string();
                }

                let (mx, my) = mouse_position();
                let world_mouse = cam.camera.screen_to_world(vec2(mx, my));
                if is_mouse_button_down(MouseButton::Left) {
                    if let Some((row, col)) = tilemap.grid_pos(world_mouse) {
                        tilemap.set_tile(row, col, &edit_tile_id);
                    }
                }
                if is_mouse_button_down(MouseButton::Right) {
                    if let Some((row, col)) = tilemap.grid_pos(world_mouse) {
                        tilemap.set_tile(row, col, "grass")
                    }
                }
                if is_key_pressed(KeyCode::F6) {
                    let path = tilemap_path(&current_map);
                    match std::fs::write(&path, tilemap.to_ron()) {
                        Ok(()) => log.push(format!("Map saved to {path}.")),
                        Err(e) => log.push(format!("Map save failed: {e}")),
                    }
                }
            }
            if is_key_pressed(KeyCode::F5) || std::mem::take(&mut request_save) {
                fn snapshot_stock(z: &ZoneRegistry) -> HashMap<String, HashMap<String, u32>> {
                    z.zones
                        .values()
                        .map(|zone| {
                            let stock = zone
                                .storage
                                .iter()
                                .map(|(id, stack)| (id.clone(), stack.count))
                                .collect();
                            (zone.id.clone(), stock)
                        })
                        .collect()
                }

                let mut zone_stock =
                    snapshot_stock(hub_zones(&current_map, &zones, &village_cache));
                for vid in village_cache.keys() {
                    if let Some(vz) = village_zones(vid, &current_map, &zones, &village_cache) {
                        zone_stock.extend(snapshot_stock(vz));
                    }
                }

                let village_seeds: HashMap<String, u64> = village_cache
                    .iter()
                    .map(|(id, inst)| (id.clone(), inst.seed))
                    .collect();

                let current_map_saved = match &current_map {
                    MapId::Hub => None,
                    MapId::Village(id) => Some(id.clone()),
                };

                let data = SaveData {
                    stipend: economy.stipend,
                    reputation: economy.faction_state.reputation.clone(),
                    day_index: clock.day_index,
                    time_of_day: clock.time_of_day,
                    player_pos: (player_pos.x, player_pos.y),
                    player_inventory: player_inventory.clone(),
                    zone_stock,
                    fired_beats: fired_beats.clone(),
                    fired_milestones: fire_milestones.clone(),
                    memory_fragments: memory_fragments.clone(),
                    keeper_number,
                    tenure_start_day,
                    cellar_records: cellar_records.clone(),
                    discovered_zones: discovered_zones.clone(),
                    last_tended: last_tended.clone(),
                    fired_tutorial: fired_tutorial.clone(),
                    sacrifice_available,
                    pending_fragment: pending_fragment.clone(),
                    pending_sacrifice_faction: pending_sacrifice_faction.clone(),
                    pending_village: pending_village.clone(),
                    next_corpse_id,
                    missed_days_this_tenure: missed_days_this_tenure.clone(),
                    current_map: current_map_saved,
                    village_seeds,
                    hub_player_pos: (hub_player_pos.x, hub_player_pos.y),
                    quest_status: quest_status.clone(),
                    pair_index,
                    is_cult_life,
                    chosen_themes: chosen_themes.clone(),
                    last_delivery_day,
                    last_restock_day,
                    corpse: corpse.clone(),
                    burn_timer,
                };

                match save_game(&data, "save.ron") {
                    Ok(()) => log.push("Game saved.".to_string()),
                    Err(e) => log.push(format!("Save failed: {e}")),
                }
            }

            if is_key_pressed(KeyCode::F9) || std::mem::take(&mut request_load) {
                match load_game("save.ron") {
                    Ok(data) => {
                        // Get back to hub first
                        if let MapId::Village(vid) = current_map.clone() {
                            if let Some(inst) = village_cache.get_mut(&vid) {
                                std::mem::swap(&mut zones, &mut inst.zones);
                                std::mem::swap(&mut npcs, &mut inst.npcs);
                            }
                            current_map = MapId::Hub;
                        }

                        // Plain fields
                        economy.stipend = data.stipend;
                        economy.faction_state.reputation = data.reputation;
                        clock.day_index = data.day_index;
                        clock.time_of_day = data.time_of_day;
                        player_pos = vec2(data.player_pos.0, data.player_pos.1);
                        player_inventory = data.player_inventory;
                        fired_beats = data.fired_beats;
                        fire_milestones = data.fired_milestones;
                        memory_fragments = data.memory_fragments;
                        keeper_number = data.keeper_number;
                        tenure_start_day = data.tenure_start_day;
                        cellar_records = data.cellar_records;
                        discovered_zones = data.discovered_zones;
                        last_tended = data.last_tended;
                        fired_tutorial = data.fired_tutorial;
                        sacrifice_available = data.sacrifice_available;
                        pending_fragment = data.pending_fragment;
                        pending_sacrifice_faction = data.pending_sacrifice_faction;
                        pending_village = data.pending_village;
                        next_corpse_id = data.next_corpse_id;
                        missed_days_this_tenure = data.missed_days_this_tenure;
                        all_discovered = false;
                        player_pos = vec2(data.player_pos.0, data.player_pos.1);
                        hub_player_pos = vec2(data.hub_player_pos.0, data.hub_player_pos.1);
                        quest_status = data.quest_status;
                        active_action = None;
                        active_window = ActiveWindow::None;
                        npc_dialogue = None;
                        selected_reagents.clear();
                        last_delivery_day = data.last_delivery_day;
                        last_restock_day = data.last_restock_day;
                        pair_index = data.pair_index;
                        is_cult_life = data.is_cult_life;
                        chosen_themes = data.chosen_themes;
                        corpse = data.corpse;
                        burn_timer = data.burn_timer;

                        // Rebuild villages from seeds
                        village_cache.clear();
                        for (vid, seed) in &data.village_seeds {
                            village_cache.insert(vid.clone(), generate_village(vid, *seed));
                        }

                        // Restore stock (hub first, then villages)
                        for (zone_id, stock) in data.zone_stock {
                            let target = zones.zones.get_mut(&zone_id).or_else(|| {
                                village_cache
                                    .values_mut()
                                    .find_map(|i| i.zones.zones.get_mut(&zone_id))
                            });
                            if let Some(zone) = target {
                                zone.storage.clear();
                                for (item_id, count) in stock {
                                    zone.deposit(&item_id, count);
                                }
                            }
                        }
                        match &data.current_map {
                            Some(vid) => {
                                if let Some(inst) = village_cache.get_mut(vid) {
                                    tilemap = render::TileMap::from_def(&inst.tilemap);
                                    std::mem::swap(&mut zones, &mut inst.zones);
                                    std::mem::swap(&mut npcs, &mut inst.npcs);
                                    current_map = MapId::Village(vid.clone());
                                }
                            }
                            None => {
                                tilemap = render::TileMap::from_def(&hub_tilemap_def);
                            }
                        }
                        log.push("Game loaded.".to_string());
                    }
                    Err(e) => log.push(format!("Load failed: {e}")),
                }
            }

            if corpse.is_none() && last_delivery_day != Some(clock.day_index) {
                let quality = rand::gen_range(2.0, 5.0);
                let mut new_corpse = Corpse::new(next_corpse_id, quality);

                let mut all_tags: Vec<String> = faction_defs
                    .iter()
                    .flat_map(|f| f.tag_reactions.keys().cloned())
                    .collect();
                all_tags.sort();
                all_tags.dedup();

                if !all_tags.is_empty() {
                    new_corpse.requested_tag =
                        Some(all_tags[rand::gen_range(0, all_tags.len())].clone());
                }

                log.push(format!(
                    "Corpse #{} delivery. Quality {:.2}. Requested tag: {}",
                    next_corpse_id,
                    quality,
                    new_corpse.requested_tag.as_deref().unwrap_or("none"),
                ));
                corpse = Some(new_corpse);
                next_corpse_id += 1;
                last_delivery_day = Some(clock.day_index);
            }
            if last_restock_day != Some(clock.day_index) {
                let hz = hub_zones_mut(&current_map, &mut zones, &mut village_cache);
                hz.get_mut("morgue").deposit("salt", 5);
                hz.get_mut("morgue").deposit("candle", 4);
                hz.get_mut("workyard").deposit("grave_dirt", 5);
                hz.get_mut("workyard").deposit("iron_nail", 4);

                for (village_id, grounds_id, item_id) in VILLAGE_STOCK {
                    let found = discoveries
                        .iter()
                        .any(|d| d.zone_id == village_id && discovered_zones.contains(&d.id));
                    if !found {
                        continue;
                    }
                    village_cache
                        .entry(village_id.to_string())
                        .or_insert_with(|| {
                            generate_village(village_id, rand::gen_range(0, u64::MAX) as u64)
                        });
                    if let Some(vz) =
                        village_zones_mut(village_id, &current_map, &mut zones, &mut village_cache)
                    {
                        if let Some(z) = vz.zones.get_mut(grounds_id) {
                            z.deposit(item_id, 1);
                        }
                    }
                }
                log.push("Supplies restocked.".to_string());
                last_restock_day = Some(clock.day_index);
            }

            if let Some(c) = corpse.as_mut() {
                c.tick_decay(get_frame_time());
            }
            let player_in_cellar = zones
                .zones
                .get("cellar")
                .map(|z| z.contains((player_pos.x, player_pos.y)))
                .unwrap_or(false);
            if let MapId::Village(zone_id) = current_map.clone() {
                if let Some(instance) = village_cache.get_mut(&zone_id) {
                    if point_in_rect((player_pos.x, player_pos.y), instance.exit_zone) {
                        current_map = MapId::Hub;
                        tilemap = render::TileMap::from_def(&hub_tilemap_def);
                        std::mem::swap(&mut zones, &mut instance.zones);
                        std::mem::swap(&mut npcs, &mut instance.npcs);
                        player_pos = hub_player_pos;
                        log.push("You return to the graveyard.".to_string());
                    }
                }
            }
            if is_key_pressed(KeyCode::C) {
                if player_in_cellar {
                    let opening = !matches!(active_window, ActiveWindow::Cellar);
                    active_window = match active_window {
                        ActiveWindow::Cellar => ActiveWindow::None,
                        _ => ActiveWindow::Cellar,
                    };
                    if opening {
                        fire_tutorial(
                            &tutorial_steps,
                            &mut fired_tutorial,
                            "first_cellar_visit",
                            &mut log,
                            &mut tutorial_banner,
                        );
                    }
                } else {
                    log.push("Step into the cellar to read the markers.".to_string());
                }
            }

            let here = (player_pos.x, player_pos.y);
            let at_burial = zones
                .zones
                .get("burial_site")
                .map_or(false, |z| z.contains(here));
            let at_incinerator = zones
                .zones
                .get("incinerator")
                .map_or(false, |z| z.contains(here));
            let can_dispose = match corpse.as_ref().map(|c| c.state) {
                Some(CorpseState::GraveDug) => at_burial,
                Some(CorpseState::Fueled) => at_incinerator,
                _ => false,
            };

            if is_key_pressed(KeyCode::Space) && can_dispose && !ui_open {
                if let Some(c) = corpse.as_mut() {
                    let mut available = Vec::new();
                    for item_id in selected_reagents.iter() {
                        if let Some(count) = player_inventory.get_mut(item_id) {
                            if *count > 0 {
                                *count -= 1;
                                if let Some(reagent) = reagent_defs.get(item_id) {
                                    available.push(reagent.clone());
                                }
                            }
                        }
                    }
                    if available.is_empty() {
                        log.push("No reagents available.".to_string());
                    } else {
                        let requested_tag = c.requested_tag.clone();
                        let quality_at_burial = c.current_quality();

                        let mut rite = resolve_rite(&available);

                        let request_met = requested_tag
                            .as_ref()
                            .map_or(false, |t| rite.tags.contains_key(t));
                        let hidden_tag = if c.state == CorpseState::GraveDug {
                            "holy"
                        } else {
                            "profane"
                        };

                        let base_tags: Vec<String> = rite.tags.keys().cloned().collect();
                        *rite.tags.entry(hidden_tag.to_string()).or_insert(0.0) += DISPOSAL_WEIGHT;

                        c.assign_rite(rite);
                        let (performed_rite, method) = c.dispose();

                        for quest in &quest_defs {
                            if let QuestKind::Tag { tag } = &quest.kind {
                                if base_tags.contains(tag) {
                                    if let Some((QuestStatus::Accepted, day)) =
                                        quest_status.get(&quest.id)
                                    {
                                        quest_status
                                            .insert(quest.id.clone(), (QuestStatus::Ready, *day));
                                    }
                                }
                            }
                        }
                        let mut result = economy.settle_burial(
                            &performed_rite,
                            &faction_defs,
                            10.0,
                            quality_at_burial,
                        );
                        if request_met {
                            if let Some(tag) = &requested_tag {
                                let bonus = 5.0;
                                economy.stipend += bonus;
                                result.payment += bonus;
                                log.push(format!(
                                    "Request fulfilled ({tag}): +{bonus:.1} bonus stipend"
                                ));
                            }
                        }
                        let verb = match method {
                            Disposal::Burial => "Buried",
                            Disposal::Burning => "Burned",
                        };
                        log.push(format!(
                            "{verb} corpse #{}. Paid {:.1} stipend.",
                            c.id, result.payment
                        ));

                        for (faction_id, delta) in &result.reputation_deltas {
                            log.push(format!("    {faction_id}: {delta:+.1} reputation"));
                        }

                        if result.reputation_deltas.values().any(|d| *d > 0.0)
                            && result.reputation_deltas.values().any(|d| *d < 0.0)
                        {
                            fire_tutorial(
                                &tutorial_steps,
                                &mut fired_tutorial,
                                "first_reputation_conflict",
                                &mut log,
                                &mut tutorial_banner,
                            );
                        }
                        check_progress = true;

                        let mut seen: HashMap<String, u32> = HashMap::new();
                        selected_reagents.retain(|id| {
                            let used = seen.entry(id.clone()).or_insert(0);
                            *used += 1;
                            *used <= player_inventory.get(id).copied().unwrap_or(0)
                        });
                        corpse = None;

                        fire_tutorial(
                            &tutorial_steps,
                            &mut fired_tutorial,
                            "first_burial",
                            &mut log,
                            &mut tutorial_banner,
                        )
                    }
                }
            }

            if check_progress {
                check_progress = false;

                for beat in &story_beats {
                    if fired_beats.contains(&beat.id) {
                        continue;
                    }
                    let reputation = &economy.faction_state.reputation;
                    if beat.is_triggered(|fid| reputation.get(fid).copied().unwrap_or(0.0)) {
                        npc_speech = Some((beat.text.clone(), (0.0, -150.0), 5.0));
                        log.push(beat.text.clone());
                        fired_beats.insert(beat.id.clone());
                    }
                }
                let target = target_zone(&chosen_themes, pair_index, is_cult_life);
                for milestone in &milestones {
                    if fire_milestones.contains(&milestone.id) {
                        continue;
                    }
                    if Some(milestone.zone_id.as_str()) != target {
                        continue;
                    }
                    let need = tier_for(&tiers, pair_index as usize).milestone;
                    let village_found = discoveries.iter().any(|d| {
                        d.zone_id == milestone.zone_id && discovered_zones.contains(&d.id)
                    });
                    if !village_found {
                        continue;
                    }
                    let rep = economy
                        .faction_state
                        .reputation
                        .get(&milestone.faction_id)
                        .copied()
                        .unwrap_or(0.0);
                    if rep >= need {
                        log.push(milestone.announce_text.clone());
                        sacrifice_available = true;
                        pending_fragment = Some(milestone.memory_fragment.clone());
                        pending_sacrifice_faction = Some(milestone.faction_id.clone());
                        pending_village = Some(milestone.zone_id.clone());
                        fire_milestones.insert(milestone.id.clone());
                    }
                }
                for discovery in &discoveries {
                    if discovered_zones.contains(&discovery.id) {
                        continue;
                    }
                    let Some(slot) = slot_of(&chosen_themes, &discovery.zone_id) else {
                        continue;
                    };
                    let rep = economy
                        .faction_state
                        .reputation
                        .get(&discovery.faction_id)
                        .copied()
                        .unwrap_or(0.0);
                    if rep >= tier_for(&tiers, slot).discovery {
                        log.push(discovery.announce_text.clone());
                        discovered_zones.insert(discovery.id.clone());
                    }
                }
            }
            clear_background(Color::new(0.05, 0.05, 0.07, 1.0));
            cam.update(player_pos);
            tilemap.draw(&assets);

            let player_sprite = assets.get("player");
            draw_texture_ex(
                player_sprite,
                player_pos.x - 16.0,
                player_pos.y - 16.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(32.0, 32.0)),
                    flip_x: facing.x < 0.0,
                    ..Default::default()
                },
            );

            if let Some((text, _)) = &player_speech {
                let bubble_w = text.len() as f32 * 8.0 + 24.0;
                let bubble_h = 34.0;
                let bx = player_pos.x - bubble_w / 2.0;
                let by = player_pos.y - 55.0;

                draw_rectangle(bx, by, bubble_w, bubble_h, Color::new(1.0, 1.0, 1.0, 0.9));
                draw_rectangle_lines(bx, by, bubble_w, bubble_h, 2.0, BLACK);
                draw_triangle(
                    vec2(player_pos.x - 6.0, by + bubble_h),
                    vec2(player_pos.x + 4.0, by + bubble_h),
                    vec2(player_pos.x, by + bubble_h + 14.0),
                    Color::new(1.0, 1.0, 1.0, 0.9),
                );
                draw_text(text, bx + 12.0, by + bubble_h / 2.0 + 6.0, 16.0, BLACK);
            }

            for zone in zones.zones.values() {
                let is_village = zone.faction_id.is_some();
                let discovered = if is_village {
                    discoveries
                        .iter()
                        .any(|d| d.zone_id == zone.id && discovered_zones.contains(&d.id))
                } else {
                    true
                };

                if is_village && !discovered {
                    let (zx, zy) = zone.position;
                    let (zw, zh) = zone.size;
                    draw_rectangle(zx, zy, zw, zh, Color::new(0.1, 0.1, 0.1, 0.35));
                    draw_rectangle_lines(zx, zy, zw, zh, 20.0, GRAY);
                    draw_text("???", zx + zw / 2.0 - 16.0, zy + zh / 2.0, 20.0, GRAY);
                    continue;
                }

                let (zx, zy) = zone.position;
                let (zw, zh) = zone.size;
                let color = match zone.id.as_str() {
                    "morgue" => Color::new(0.3, 0.1, 0.4, 0.4),
                    "workyard" => Color::new(0.4, 0.3, 0.1, 0.4),
                    "cellar" => Color::new(0.1, 0.3, 0.2, 0.4),
                    "burial_site" => Color::new(0.2, 0.35, 0.2, 0.4),
                    "incinerator" => Color::new(0.5, 0.25, 0.05, 0.5),
                    _ => match zone.faction_id.as_deref() {
                        Some("village") => Color::new(0.1, 0.2, 0.5, 0.4),
                        Some("cult") => Color::new(0.5, 0.1, 0.1, 0.4),
                        _ => Color::new(0.2, 0.2, 0.2, 0.4),
                    },
                };
                draw_rectangle(zx, zy, zw, zh, color);
                draw_rectangle_lines(zx, zy, zw, zh, 2.0, PURPLE);
                draw_text(&zone.name, zx + 8.0, zy + 20.0, 20.0, WHITE);
                if zone.faction_id.is_some() {
                    draw_grave_markers(zone, 4);
                    let sacrificed = milestones
                        .iter()
                        .any(|m| m.zone_id == zone.id && fire_milestones.contains(&m.id));
                    draw_temple(zone, sacrificed);
                }
            }

            if let MapId::Village(zone_id) = current_map.clone() {
                if let Some(instance) = village_cache.get(&zone_id) {
                    let (ex, ey, ew, eh) = instance.exit_zone;
                    draw_rectangle(ex, ey, ew, eh, Color::new(0.2, 0.6, 0.2, 0.3));
                    draw_rectangle_lines(ex, ey, ew, eh, 2.0, GREEN);
                    draw_text("Path home", ex + 8.0, ey + 20.0, 16.0, WHITE);
                }
            }

            for n in &npcs {
                let visible = match &n.zone_id {
                    Some(zid) => discoveries
                        .iter()
                        .any(|d| &d.zone_id == zid && discovered_zones.contains(&d.id)),
                    None => true,
                };

                if visible && n.is_active(&clock) {
                    let tex = assets.get(&n.sprite_id);
                    draw_texture_ex(
                        tex,
                        n.position.0 - 16.0,
                        n.position.1 - 16.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(32.0, 32.0)),
                            ..Default::default()
                        },
                    );
                    draw_text(
                        &n.name,
                        n.position.0 - 30.0,
                        n.position.1 - 20.0,
                        18.0,
                        WHITE,
                    );
                    if let Some(marker) =
                        quest_marker(&n.id, &quest_defs, &quest_status, &player_inventory)
                    {
                        let (glyph, color) = match marker {
                            QuestMarker::Available => ("!", GOLD),
                            QuestMarker::Pending => ("?", GRAY),
                            QuestMarker::Ready => ("?", GREEN),
                        };
                        let bob = if marker == QuestMarker::Ready {
                            (get_time() as f32 * 4.0).sin() * 2.0
                        } else {
                            0.0
                        };
                        draw_text(
                            glyph,
                            n.position.0 - 4.0,
                            n.position.1 - 40.0 + bob,
                            28.0,
                            color,
                        );
                    }
                }
            }

            if let (Some((text, pos, _)), false) =
                (&npc_speech, matches!(active_window, ActiveWindow::Dialogue))
            {
                let max_width = 260.0;
                let lines = wrap_text(text, 16.0, max_width - 20.0);
                let bubble_w = max_width;
                let bubble_h = 20.0 + lines.len() as f32 * 20.0;
                let bx = pos.0 - bubble_w / 2.0;
                let by = pos.1 - 20.0 - bubble_h;

                draw_rectangle(bx, by, bubble_w, bubble_h, Color::new(1.0, 1.0, 1.0, 0.85));
                draw_rectangle_lines(bx, by, bubble_w, bubble_h, 2.0, BLACK);

                let mut ty = by + 20.0;
                for line in &lines {
                    draw_text(line, bx + 10.0, ty, 16.0, BLACK);
                    ty += 20.0;
                }
            }

            if edit_mode {
                draw_text(
                    &format!(
                    "MAP EDIT - tile {}([ grass, ] path, LMB paint, RMB erase, F6 save, F2 exit",
                    edit_tile_id
                ),
                    10.0,
                    screen_height() - 90.0,
                    18.0,
                    YELLOW,
                );
            }

            set_default_camera();
            let daylight = (1.0 - (clock.time_of_day * std::f32::consts::TAU).cos()) / 2.0;
            let gloom = 0.15 + 0.70 * (1.0 - daylight);
            let dark = Color::new(0.02, 0.03, 0.09, gloom);
            let (sw, sh) = (screen_width(), screen_height());

            let t = get_time() as f32;
            let flick = candle(t);
            let r = 260.0 * (1.0 + 0.02 * (t * 7.0).sin() + 0.01 * (t * 13.0).sin());
            let lp = cam.camera.world_to_screen(player_pos);
            let (x0, y0, x1, y1) = (lp.x - r, lp.y - r, lp.x + r, lp.y + r);

            draw_texture_ex(
                &light_tex,
                x0,
                y0,
                Color::new(1.0, 1.0, 1.0, (gloom * (0.92 + 0.08 * flick)).min(1.0)),
                DrawTextureParams {
                    dest_size: Some(vec2(r * 2.0, r * 2.0)),
                    ..Default::default()
                },
            );
            draw_rectangle(0.0, 0.0, sw, y0.max(0.0), dark); // top
            draw_rectangle(0.0, y1, sw, (sh - y1).max(0.0), dark); // bottom
            draw_rectangle(0.0, y0, x0.max(0.0), r * 2.0, dark); // left
            draw_rectangle(x1, y0, (sw - x1).max(0.0), r * 2.0, dark); // right

            if let Some(a) = &active_action {
                draw_progress_bar(a.progress(), a.label());
            }

            if let Some((text, _)) = &tutorial_banner {
                let panel_w = screen_width() * 0.6;
                let lines = wrap_text(text, 20.0, panel_w - 40.0);
                let panel_h = 40.0 + lines.len() as f32 * 26.0;
                let px = (screen_width() - panel_w) / 2.0;
                let py = 50.0;

                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.05, 0.05, 0.1, 0.92));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, GOLD);

                let mut ty = py + 30.0;
                for line in &lines {
                    draw_text(line, px + 20.0, ty, 20.0, WHITE);
                    ty += 26.0;
                }
            }

            ////////////////////
            // User Interface //
            ////////////////////

            // Inventory
            if let ActiveWindow::Inventory = active_window {
                let cols = 4;
                let box_w = 90.0;
                let box_h = 90.0;
                let gap = 10.0;
                let carried_ids: Vec<String> = reagent_order
                    .iter()
                    .filter(|id| player_inventory.get(*id).copied().unwrap_or(0) > 0)
                    .cloned()
                    .collect();
                let rows = (carried_ids.len() as f32 / cols as f32).ceil().max(1.0) as usize;

                let panel_w = cols as f32 * box_w + (cols as f32 + 1.0) * gap;
                let panel_h = rows as f32 * box_h + (rows as f32 + 1.0) * gap + 90.0;
                let px = (screen_width() - panel_w) - 2.0;
                let py = (screen_height() - panel_h) / 2.0;

                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.95));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, WHITE);
                draw_text(
                    "Inventory - select up to 4  for your rite",
                    px + 20.0,
                    py + 30.0,
                    22.0,
                    WHITE,
                );
                let clicked = is_mouse_button_pressed(MouseButton::Left);
                let mouse = mouse_position();

                if carried_ids.is_empty() {
                    draw_text("Nothing carried yet.", px + 20.0, py + 70.0, 16.0, GRAY);
                }

                for (i, id) in carried_ids.iter().enumerate() {
                    let col = i % cols;
                    let row = i / cols;
                    let bx = px + gap + col as f32 * (box_w + gap);
                    let by = py + 55.0 + gap + row as f32 * (box_h + gap);
                    let rect = (bx, by, box_w, box_h);
                    let picked = selected_count(&selected_reagents, id);
                    let active = picked > 0;
                    let count = player_inventory.get(id).copied().unwrap_or(0);
                    let reagent = reagent_defs.get(id);

                    if clicked && point_in_rect(mouse, rect) {
                        if add_reagent(&mut selected_reagents, id, count, &mut log) {
                            fire_tutorial(
                                &tutorial_steps,
                                &mut fired_tutorial,
                                "first_reagent_toggle",
                                &mut log,
                                &mut tutorial_banner,
                            );
                        }
                    }
                    if is_mouse_button_pressed(MouseButton::Right) && point_in_rect(mouse, rect) {
                        remove_reagent(&mut selected_reagents, id);
                    }

                    let bg = if active {
                        Color::new(0.2, 0.5, 0.2, 0.85)
                    } else {
                        Color::new(0.15, 0.15, 0.15, 0.68)
                    };
                    draw_rectangle(bx, by, box_w, box_h, bg);
                    draw_rectangle_lines(
                        bx,
                        by,
                        box_w,
                        box_h,
                        2.0,
                        if active { GREEN } else { WHITE },
                    );
                    if let Some(r) = reagent {
                        let lines = wrap_text(&r.name, 14.0, box_w - 12.0);
                        let mut ty = by + 20.0;
                        for line in &lines {
                            draw_text(line, bx + 6.0, ty, 14.0, WHITE);
                            ty += 16.0;
                        }
                    }
                    draw_text(
                        &format!("x{}    sel{}", count, picked),
                        bx + 6.0,
                        by + box_h - 10.0,
                        14.0,
                        SKYBLUE,
                    );
                }
                draw_text(
                    &format!("Selected: {}/4", selected_reagents.len()),
                    px + 20.0,
                    py + panel_h - 20.0,
                    16.0,
                    GRAY,
                );
                draw_text(
                    "[I] to close",
                    px + panel_w - 100.0,
                    py + panel_h - 20.0,
                    16.0,
                    GRAY,
                );
            }
            // Cellar
            if let ActiveWindow::Cellar = active_window {
                let panel_w = 500.0;
                let panel_h = 360.0;
                let px = (screen_width() - panel_w) / 2.0;
                let py = (screen_height() - panel_h) / 2.0;
                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.95));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, WHITE);
                draw_text("Cellar Records", px + 20.0, py + 30.0, 24.0, WHITE);

                let mut cy = py + 60.0;
                for record in &cellar_records {
                    draw_text(&record.name, px + 20.0, cy, 18.0, WHITE);
                    draw_text(&record.years, px + 20.0, cy + 18.0, 14.0, GRAY);
                    draw_text(&record.epitaph, px + 20.0, cy + 36.0, 14.0, LIGHTGRAY);
                    draw_text(
                        &format!(
                            "Sacrificed to: {} ({})",
                            record.sacrificed_to, record.village
                        ),
                        px + 20.0,
                        cy + 54.0,
                        14.0,
                        SKYBLUE,
                    );
                    cy += 78.0;
                }
                if !memory_fragments.is_empty() {
                    cy += 10.0;
                    draw_text("What you remember:", px + 20.0, cy, 18.0, GOLD);
                    cy += 24.0;
                    for fragment in &memory_fragments {
                        draw_text(fragment, px + 20.0, cy, 14.0, LIGHTGRAY);
                        cy += 20.0;
                    }
                }
                draw_text("[C] to close", px + 20.0, py + panel_h - 20.0, 16.0, GRAY);
            }

            if is_key_pressed(KeyCode::R) {
                if player_in_cellar && sacrifice_available {
                    sacrifice_selection.clear();
                    active_window = match active_window {
                        ActiveWindow::Sacrifice => ActiveWindow::None,
                        _ => ActiveWindow::Sacrifice,
                    };
                } else if player_in_cellar {
                    log.push("Nothing calls for the ritual yet.".to_string());
                }
            }

            // Theme picker
            if chosen_themes.len() as u32 == pair_index
                && pair_index < 4
                && matches!(active_window, ActiveWindow::None)
            {
                active_window = ActiveWindow::ThemePicker;
            }
            if let ActiveWindow::ThemePicker = active_window {
                let remaining: Vec<&'static str> = THEMES
                    .iter()
                    .map(|t| t.0)
                    .filter(|n| !chosen_themes.iter().any(|c| c.as_str() == *n))
                    .collect();

                let panel_w = 420.0;
                let panel_h = 100.0 + remaining.len() as f32 * 50.0;
                let px = (screen_width() - panel_w) / 2.0;
                let py = (screen_height() - panel_h) / 2.0;
                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.97));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, GOLD);
                draw_text(
                    "Where does the next road lead?",
                    px + 20.0,
                    py + 32.0,
                    24.0,
                    WHITE,
                );

                let mut ry = py + 60.0;
                for name in &remaining {
                    let rect = (px + 20.0, ry, panel_w - 40.0, 40.0);
                    let hover = point_in_rect(mouse_position(), rect);
                    draw_rectangle(
                        rect.0,
                        rect.1,
                        rect.2,
                        rect.3,
                        if hover {
                            Color::new(0.2, 0.5, 0.2, 0.85)
                        } else {
                            Color::new(0.15, 0.15, 0.15, 0.68)
                        },
                    );
                    draw_rectangle_lines(rect.0, rect.1, rect.2, rect.3, 2.0, WHITE);
                    draw_text(
                        &name.to_uppercase(),
                        rect.0 + 12.0,
                        rect.1 + 27.0,
                        22.0,
                        WHITE,
                    );

                    if hover && is_mouse_button_pressed(MouseButton::Left) {
                        chosen_themes.push(name.to_string());
                        log.push(format!("You turn towards {}.", name));
                        active_window = ActiveWindow::None;
                    }
                    ry += 50.0;
                }
            }
            // Sacrifice
            if let ActiveWindow::Sacrifice = active_window {
                let panel_w = 460.0;
                let sacrifice_ids: Vec<&String> = reagent_order
                    .iter()
                    .filter(|id| player_inventory.get(*id).copied().unwrap_or(0) > 0)
                    .collect();
                let panel_h = 140.0 + sacrifice_ids.len() as f32 * 44.0;
                let px = (screen_width() - panel_w) / 2.0;
                let py = (screen_height() - panel_h) / 2.0;
                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.12, 0.05, 0.08, 0.97));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, RED);
                draw_text("The Ritual", px + 20.0, py + 30.0, 24.0, WHITE);
                draw_text(
                    "Choose up to 5 items to carry into your next life.",
                    px + 20.0,
                    py + 52.0,
                    16.0,
                    LIGHTGRAY,
                );

                let selected_total: u32 = sacrifice_selection.values().sum();
                let mut ry = py + 80.0;
                for id in sacrifice_ids {
                    let reagent = &reagent_defs[id];
                    let owned = player_inventory.get(&reagent.id).copied().unwrap_or(0);
                    let chosen = sacrifice_selection.get(&reagent.id).copied().unwrap_or(0);
                    let row_rect = (px + 20.0, ry, panel_w - 40.0, 36.0);

                    if is_mouse_button_pressed(MouseButton::Left)
                        && point_in_rect(mouse_position(), row_rect)
                        && chosen < owned
                        && selected_total < 5
                    {
                        *sacrifice_selection.entry(reagent.id.clone()).or_insert(0) += 1;
                    }
                    if is_mouse_button_pressed(MouseButton::Right)
                        && point_in_rect(mouse_position(), row_rect)
                        && chosen > 0
                    {
                        *sacrifice_selection.entry(reagent.id.clone()).or_insert(0) -= 1;
                    }

                    draw_rectangle_lines(row_rect.0, row_rect.1, row_rect.2, row_rect.3, 1.0, GRAY);
                    draw_text(
                        &format!("{} - carrying {} - taking {}", reagent.name, owned, chosen),
                        px + 28.0,
                        ry + 24.0,
                        18.0,
                        WHITE,
                    );
                    ry += 44.0;
                }
                draw_text(
                    &format!(
                        "Selected: {}/5  (left-click a row to add, right-click to remove)",
                        selected_total
                    ),
                    px + 20.0,
                    ry + 10.0,
                    14.0,
                    GRAY,
                );
                draw_text(
                    "[ENTER] confirm the ritual   [R] cancel",
                    px + 20.0,
                    py + panel_h - 20.0,
                    16.0,
                    GRAY,
                );

                if is_key_pressed(KeyCode::Enter) {
                    if let Some(shack) = zones.zones.get_mut("keeper_shack") {
                        for (item_id, count) in &sacrifice_selection {
                            if *count > 0 {
                                shack.deposit(item_id, *count);
                            }
                        }
                    } else {
                        log.push(
                            "No shack to store items - they're lost to the next life.".to_string(),
                        );
                    }

                    cellar_records.push(CellarRecord {
                        name: format!("Keeper #{}", keeper_number),
                        years: format!(
                            "Day {} - Day {}",
                            tenure_start_day + 1,
                            clock.day_index + 1
                        ),
                        epitaph: {
                            let mut text = pending_fragment.clone().unwrap_or_else(|| {
                                "Little is known of this keeper's end.".to_string()
                            });
                            if missed_days_this_tenure > 0 {
                                text.push_str(&format!(
                                    " They did not always return in time - {} day(s) passed without them. They kept going anyway.",
                                    missed_days_this_tenure
                                ));
                            }
                            text
                        },
                        sacrificed_to: pending_sacrifice_faction
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        village: pending_village
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                    });

                    if let Some(fragment) = pending_fragment.take() {
                        memory_fragments.push(fragment);
                    }
                    let carry = tier_for(&tiers, pair_index as usize).milestone * CARRY_OVER;
                    pending_sacrifice_faction = None;
                    pending_village = None;
                    let kept_stipend = economy.stipend;
                    economy = Economy::new(&faction_defs);
                    economy.stipend = kept_stipend;

                    let next_faction = if is_cult_life { "village" } else { "cult" };
                    for (fid, rep) in economy.faction_state.reputation.iter_mut() {
                        *rep = if fid == next_faction { carry } else { 0.0 };
                    }

                    selected_reagents.clear();
                    player_inventory.clear();
                    corpse = None;
                    burn_timer = None;
                    last_tended.clear();
                    active_action = None;
                    keeper_number += 1;
                    if is_cult_life {
                        pair_index += 1;
                    }
                    is_cult_life = !is_cult_life;
                    if pair_index >= 4 {
                        log.push("The cycle is complete.".to_string());
                    }
                    missed_days_this_tenure = 0;
                    tenure_start_day = clock.day_index;
                    sacrifice_available = false;
                    sacrifice_selection.clear();
                    active_window = ActiveWindow::None;
                }
            }

            // Dialogue

            if let ActiveWindow::Dialogue = active_window {
                let npc_info = npc_dialogue
                    .as_ref()
                    .and_then(|id| npcs.iter().find(|n| &n.id == id))
                    .map(|n| (n.name.clone(), n.faction_id.clone(), n.position));
                let current = npc_dialogue.as_ref().and_then(|id| {
                    quest_defs.iter().filter(|q| &q.npc_id == id).find(|q| {
                        !matches!(quest_status.get(&q.id), Some((QuestStatus::Done { .. }, _)))
                    })
                });

                let panel_w = 480.0;
                let panel_h = 280.0;
                let px = (screen_width() - panel_w) / 2.0;
                let py = (screen_height() - panel_h) / 2.0 - 60.0;
                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.95));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, WHITE);
                match (npc_info, current) {
                    (Some((name, faction, npc_pos)), Some(q)) => {
                        draw_text(&name, px + 20.0, py + 30.0, 24.0, WHITE);

                        let st = quest_status.get(&q.id).map(|(s, _)| s);
                        let fresh = st.is_none();
                        let accepted = matches!(st, Some(QuestStatus::Accepted));
                        let ready = matches!(st, Some(QuestStatus::Ready));
                        let have_item = match &q.kind {
                            QuestKind::Fetch { item_id, count } => {
                                player_inventory.get(item_id).copied().unwrap_or(0) >= *count
                            }
                            _ => false,
                        };
                        let can_turn_in = ready || (accepted && have_item);
                        let (body, label) = if fresh {
                            (q.offer_text.as_str(), Some("Accept"))
                        } else if can_turn_in {
                            ("You have what I asked for.", Some("Turn in"))
                        } else {
                            ("Come back when it's done.", None)
                        };

                        let mut ty = py + 62.0;
                        for line in wrap_text(body, 18.0, panel_w - 40.0) {
                            draw_text(&line, px + 20.0, ty, 18.0, LIGHTGRAY);
                            ty += 22.0;
                        }

                        ty += 10.0;
                        draw_text(
                            &quest_objective(q, &player_inventory, &reagent_defs),
                            px + 20.0,
                            ty,
                            16.0,
                            SKYBLUE,
                        );
                        if fresh || can_turn_in {
                            ty += 22.0;
                            draw_text(
                                &quest_reward_text(q, &reagent_defs),
                                px + 20.0,
                                ty,
                                16.0,
                                GOLD,
                            );
                        }

                        if let Some(label) = label {
                            let btn = (px + 20.0, py + panel_h - 60.0, 120.0, 36.0);
                            draw_rectangle(
                                btn.0,
                                btn.1,
                                btn.2,
                                btn.3,
                                Color::new(0.2, 0.5, 0.2, 0.85),
                            );
                            draw_rectangle_lines(btn.0, btn.1, btn.2, btn.3, 2.0, GREEN);
                            draw_text(label, btn.0 + 14.0, btn.1 + 24.0, 20.0, WHITE);

                            let pressed = is_key_pressed(KeyCode::Enter)
                                || (is_mouse_button_pressed(MouseButton::Left)
                                    && point_in_rect(mouse_position(), btn));
                            if pressed {
                                if fresh {
                                    quest_status.insert(
                                        q.id.clone(),
                                        (QuestStatus::Accepted, clock.day_index),
                                    );
                                } else {
                                    if let QuestKind::Fetch { item_id, count } = &q.kind {
                                        if let Some(c) = player_inventory.get_mut(item_id) {
                                            *c = c.saturating_sub(*count);
                                        }
                                    }
                                    economy.stipend += q.reward_stipend;
                                    if q.reward_rep != 0.0 {
                                        *economy
                                            .faction_state
                                            .reputation
                                            .entry(faction.clone())
                                            .or_insert(0.0) += q.reward_rep;
                                    }
                                    if let Some((item_id, count)) = &q.reward_item {
                                        *player_inventory.entry(item_id.clone()).or_insert(0) +=
                                            count;
                                    }

                                    quest_status.insert(
                                        q.id.clone(),
                                        (
                                            QuestStatus::Done {
                                                day: clock.day_index,
                                            },
                                            clock.day_index,
                                        ),
                                    );

                                    check_progress = true;

                                    let line = format!("{}: \"{}\"", name, q.turn_in_text);
                                    npc_speech = Some((line.clone(), npc_pos, 4.0));
                                    log.push(line);

                                    active_window = ActiveWindow::None;
                                    npc_dialogue = None;
                                }
                            }
                        }
                    }
                    (Some((name, _, _)), None) => {
                        draw_text(&name, px + 20.0, py + 30.0, 24.0, WHITE);
                        let msg = match next_quest_in_days(
                            npc_dialogue.as_deref().unwrap_or(""),
                            &quest_defs,
                            &quest_status,
                            clock.day_index,
                        ) {
                            Some(d) => format!("Nothing for now. Come bak in {d} day(s)."),
                            None => "Nothing more to say.".to_string(),
                        };
                        draw_text(&msg, px + 20.0, py + 62.0, 18.0, LIGHTGRAY);
                    }
                    _ => {
                        draw_text(
                            "Nothing more to say.",
                            px + 20.0,
                            py + 62.0,
                            18.0,
                            LIGHTGRAY,
                        );
                    }
                }
            }
            // Game Map
            if let ActiveWindow::GameMap = active_window {
                let panel_w = screen_width() * 0.85;
                let panel_h = screen_height() * 0.85;
                let px = (screen_width() - panel_w) / 2.0;
                let py = (screen_height() - panel_h) / 2.0;
                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.95));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, WHITE);
                draw_text("Game Map (placeholder)", px + 20.0, py + 30.0, 24.0, WHITE);
                draw_text("[M] to close", px + 20.0, py + panel_h - 20.0, 16.0, GRAY);
            }

            if let ActiveWindow::QuestLog = active_window {
                let panel_w = 560.0;
                let panel_h = 440.0;
                let px = (screen_width() - panel_w) / 2.0;
                let py = (screen_height() - panel_h) / 2.0;
                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.95));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, WHITE);
                draw_text("Quest Log", px + 20.0, py + 30.0, 24.0, WHITE);

                let mut cy = py + 64.0;
                let mut shown = 0;
                for q in &quest_defs {
                    let Some((status, _)) = quest_status.get(&q.id) else {
                        continue;
                    };
                    if matches!(status, QuestStatus::Done { .. }) {
                        continue;
                    }
                    let have_item = match &q.kind {
                        QuestKind::Fetch { item_id, count } => {
                            player_inventory.get(item_id).copied().unwrap_or(0) >= *count
                        }
                        _ => false,
                    };
                    let ready = matches!(status, QuestStatus::Ready) || have_item;
                    let (label, color) = if ready {
                        ("Ready to turn in", GREEN)
                    } else {
                        ("In progress", LIGHTGRAY)
                    };
                    let who = npc_names
                        .get(&q.npc_id)
                        .map(|s| s.as_str())
                        .unwrap_or("Unknown");

                    draw_text(who, px + 20.0, cy, 18.0, WHITE);
                    draw_text(label, px + panel_w - 170.0, cy, 16.0, color);
                    draw_text(
                        &quest_objective(q, &player_inventory, &reagent_defs),
                        px + 20.0,
                        cy + 20.0,
                        16.0,
                        SKYBLUE,
                    );
                    cy += 50.0;
                    shown += 1;
                    if cy > py + panel_h - 110.0 {
                        break;
                    }
                }
                if shown == 0 {
                    draw_text(
                        "No active quests. Talk to someone marked with !",
                        px + 20.0,
                        py + 70.0,
                        16.0,
                        GRAY,
                    );
                }
                let mut cooling: Vec<(String, u32)> = quest_defs
                    .iter()
                    .filter_map(|q| match quest_status.get(&q.id) {
                        Some((QuestStatus::Done { day }, _)) if q.cooldown_days > 0 => {
                            let who = npc_names
                                .get(&q.npc_id)
                                .cloned()
                                .unwrap_or_else(|| "Unknown".to_string());
                            Some((who, (day + q.cooldown_days).saturating_sub(clock.day_index)))
                        }
                        _ => None,
                    })
                    .collect();
                cooling.sort_by_key(|(_, d)| *d);
                if !cooling.is_empty() {
                    draw_text(
                        "Available again soon:",
                        px + 20.0,
                        py + panel_h - 88.0,
                        16.0,
                        GOLD,
                    );
                    let mut ry = py + panel_h - 68.0;
                    for (who, d) in cooling.iter().take(3) {
                        draw_text(
                            &format!("{who}: {d} day(s)"),
                            px + 28.0,
                            ry,
                            14.0,
                            LIGHTGRAY,
                        );
                        ry += 18.0;
                    }
                }
                draw_text(
                    "[Q] to close",
                    px + panel_w - 110.0,
                    py + panel_h - 14.0,
                    14.0,
                    GRAY,
                );
            }
            // top status bar
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                34.0,
                Color::new(0.0, 0.0, 0.0, 0.6),
            );
            let hour = (clock.time_of_day * 24.0) as u32;
            let minute = ((clock.time_of_day * 24.0 * 60.0) as u32) % 60;
            draw_text(
                &format!(
                    "Day {} - {}    {:02}:{:02}    stipend: {:.1}     [F5 save / F9 load]",
                    clock.day_index + 1,
                    weekday_label(clock.weekday()),
                    hour,
                    minute,
                    economy.stipend,
                ),
                16.0,
                23.0,
                22.0,
                WHITE,
            );

            // Corpse panel
            draw_rectangle(10.0, 46.0, 300.0, 70.0, Color::new(0.0, 0.0, 0.0, 0.5));
            let mut y = 68.0;
            if let Some(c) = corpse.as_ref() {
                draw_text(
                    &format!("Corpse #{} quality {:.2}", c.id, c.current_quality()),
                    20.0,
                    y,
                    20.0,
                    LIGHTGRAY,
                );
                y += 22.0;
                if let Some(tag) = &c.requested_tag {
                    draw_text(&format!("Requests: {tag}"), 20.0, y, 16.0, GOLD);
                    y += 22.0;
                }
                let hint = match c.state {
                    CorpseState::Delivered => "Hold [F] in Morgue: examine",
                    CorpseState::Autopsied => "Dig [2]+[F] or light fire [F]",
                    CorpseState::GraveDug => "At the grave: [SPACE] bury",
                    CorpseState::Fueled => "At the fire: [SPACE] burn",
                    _ => "",
                };
                draw_text(hint, 20.0, y, 18.0, GRAY);
            } else {
                draw_text("No corpse - awaiting delivery.", 20.0, y, 20.0, LIGHTGRAY);
            }

            if player_in_cellar {
                y += 22.0;
                draw_text("[C] Read the markers", 20.0, y, 18.0, GRAY);
            }

            if player_in_cellar && sacrifice_available {
                y += 22.0;
                draw_text("[R] The ritual calls to you...", 20.0, y, 18.0, RED);
            }
            if let Some(task) = tend_tasks.iter().find(|t| {
                zones
                    .zones
                    .get(&t.zone_id)
                    .map(|z| z.contains((player_pos.x, player_pos.y)))
                    .unwrap_or(false)
            }) {
                y += 20.0;
                if last_tended.get(&task.zone_id) == Some(&clock.day_index) {
                    draw_text("Already tended today.", 20.0, y, 18.0, GRAY);
                } else {
                    draw_text("[T] Tend this place", 20.0, y, 18.0, GOLD);
                }
            }

            for quest in &quest_defs {
                if let QuestKind::Timing { zone_id, by_day } = &quest.kind {
                    let here_now = zones
                        .zones
                        .get(zone_id)
                        .map_or(false, |z| z.contains((player_pos.x, player_pos.y)));
                    if here_now {
                        if let Some((QuestStatus::Accepted, day)) =
                            quest_status.get(quest.id.as_str())
                        {
                            if clock.day_index <= *by_day {
                                quest_status.insert(quest.id.clone(), (QuestStatus::Ready, *day));
                            }
                        }
                    }
                }
            }
            // Morgue panel
            if matches!(current_map, MapId::Hub) {
                draw_rectangle(10.0, 124.0, 300.0, 132.0, Color::new(0.0, 0.0, 0.0, 0.5));
                let salt_count = zones.get_mut("morgue").quantity_of("salt");
                let candle_count = zones.get_mut("morgue").quantity_of("candle");
                let mut y = 146.0;
                draw_text(
                    &format!("Morgue: {} salt, {} candle(s)", salt_count, candle_count),
                    20.0,
                    y,
                    20.0,
                    LIGHTGRAY,
                );

                y += 22.0;
                let carried_salt = player_inventory.get("salt").copied().unwrap_or(0);
                let carried_candle = player_inventory.get("candle").copied().unwrap_or(0);
                draw_text(
                    &format!(
                        "carrying: {} salt | {} candle(s)",
                        carried_salt, carried_candle
                    ),
                    20.0,
                    y,
                    18.0,
                    LIGHTGRAY,
                );

                y += 22.0;
                let carried_dirt = player_inventory.get("grave_dirt").copied().unwrap_or(0);
                let carried_nail = player_inventory.get("iron_nail").copied().unwrap_or(0);
                draw_text(
                    &format!("carrying: {} dirt | {} nail(s)", carried_dirt, carried_nail),
                    20.0,
                    y,
                    18.0,
                    LIGHTGRAY,
                );
            }
            // Reputation panel
            let rep_panel_h = 30.0 + faction_defs.len() as f32 * 22.0;
            draw_rectangle(
                10.0,
                268.0,
                300.0,
                rep_panel_h,
                Color::new(0.0, 0.0, 0.0, 0.5),
            );
            let mut y = 290.0;

            for def in &faction_defs {
                let rep = economy
                    .faction_state
                    .reputation
                    .get(&def.id)
                    .copied()
                    .unwrap_or(0.0);
                draw_text(
                    &format!("{}: {:+.1}", def.name, rep),
                    20.0,
                    y,
                    20.0,
                    SKYBLUE,
                );
                y += 22.0;
            }

            let log_panel_w = 320.0;
            let max_log_lines = 8;

            let mut wrapped: Vec<String> = Vec::new();
            'outer: for line in log.iter().rev() {
                for wrapped_line in wrap_text(line, 16.0, log_panel_w - 20.0) {
                    wrapped.push(wrapped_line);
                    if wrapped.len() >= max_log_lines {
                        break 'outer;
                    }
                }
            }
            let log_h = 20.0 + wrapped.len() as f32 * 20.0;
            let log_y0 = screen_height() - log_h - 10.0;
            draw_rectangle(
                10.0,
                log_y0,
                log_panel_w,
                log_h,
                Color::new(0.0, 0.0, 0.0, 0.5),
            );
            let mut y = log_y0 + 20.0;
            for line in &wrapped {
                draw_text(line, 20.0, y, 16.0, GREEN);
                y += 20.0;
            }

            // Row 1 (bottom): useable slots - placeholders for now, no functionality
            let use_w = 70.0;
            let use_h = 70.0;
            let use_gap = 8.0;
            let use_total_w =
                USEABLES.len() as f32 * use_w + (USEABLES.len() as f32 - 1.0) * use_gap;
            let use_bar_x = (screen_width() - use_total_w) / 2.0;
            let use_bar_y = screen_height() - use_h - 16.0;

            for (i, slot) in USEABLES.iter().enumerate() {
                let sx = use_bar_x + i as f32 * (use_w + use_gap);
                let rect = (sx, use_bar_y, use_w, use_h);

                let key_hit = !ui_open && TASKBAR_KEYS.get(i).map_or(false, |k| is_key_pressed(*k));
                let click_hit = is_mouse_button_pressed(MouseButton::Left)
                    && point_in_rect(mouse_position(), rect);

                if key_hit || click_hit {
                    selected_tool = if selected_tool == Some(i) {
                        None
                    } else {
                        Some(i)
                    };
                }

                let active = selected_tool == Some(i);
                let bg = if active {
                    Color::new(0.2, 0.5, 0.2, 0.85)
                } else {
                    Color::new(0.15, 0.15, 0.15, 0.68)
                };
                draw_rectangle(sx, use_bar_y, use_w, use_h, bg);
                draw_rectangle_lines(
                    sx,
                    use_bar_y,
                    use_w,
                    use_h,
                    2.0,
                    if active { GREEN } else { WHITE },
                );
                draw_text(slot.label, sx + 8.0, use_bar_y + 30.0, 16.0, WHITE);
                draw_text(
                    &format!("[{}]", slot.hint),
                    sx + 8.0,
                    use_bar_y + 50.0,
                    13.0,
                    GRAY,
                );
            }
            // Row 2 (above row 1, smaller): active reagents - clickable
            let carried: Vec<String> = reagent_order
                .iter()
                .filter(|id| player_inventory.get(*id).copied().unwrap_or(0) > 0)
                .cloned()
                .collect();

            let reagent_w = use_w / 2.0;
            let reagent_h = use_h / 2.0;
            let reagent_gap = 6.0;
            let reagent_total_w = carried.len() as f32 * reagent_w
                + (carried.len().saturating_sub(1)) as f32 * reagent_gap;
            let reagent_bar_x = (screen_width() - reagent_total_w) / 2.0;
            let reagent_bar_y = use_bar_y - reagent_h - 10.0;

            let (mouse_x, mouse_y) = mouse_position();
            let clicked = is_mouse_button_pressed(MouseButton::Left);

            let mut hover_name: Option<String> = None;

            for (i, id) in carried.iter().enumerate() {
                let sx = reagent_bar_x + i as f32 * (reagent_w + reagent_gap);
                let rect = (sx, reagent_bar_y, reagent_w, reagent_h);
                let picked = selected_count(&selected_reagents, id);
                let active = picked > 0;
                let owned_now = player_inventory.get(id).copied().unwrap_or(0);

                if clicked && point_in_rect((mouse_x, mouse_y), rect) {
                    if add_reagent(&mut selected_reagents, id, owned_now, &mut log) {
                        fire_tutorial(
                            &tutorial_steps,
                            &mut fired_tutorial,
                            "first_reagent_toggle",
                            &mut log,
                            &mut tutorial_banner,
                        );
                    }
                }
                if is_mouse_button_pressed(MouseButton::Right)
                    && point_in_rect((mouse_x, mouse_y), rect)
                {
                    remove_reagent(&mut selected_reagents, id);
                }

                let bg = if active {
                    Color::new(0.2, 0.5, 0.2, 0.85)
                } else {
                    Color::new(0.15, 0.15, 0.15, 0.68)
                };
                draw_rectangle(sx, reagent_bar_y, reagent_w, reagent_h, bg);
                draw_rectangle_lines(sx, reagent_bar_y, reagent_w, reagent_h, 1.5, WHITE);

                if picked > 0 {
                    draw_text(
                        &picked.to_string(),
                        sx + reagent_w - 12.0,
                        reagent_bar_y + 14.0,
                        18.0,
                        GOLD,
                    );
                }

                if let Some(r) = reagent_defs.get(id) {
                    let abbr: String = r
                        .name
                        .chars()
                        .filter(|c| c.is_uppercase())
                        .take(2)
                        .collect();
                    draw_text(&abbr, sx + 4.0, reagent_bar_y + 16.0, 16.0, WHITE);
                    if point_in_rect((mouse_x, mouse_y), rect) {
                        hover_name = Some(r.name.clone());
                    }
                }
                let owned = player_inventory.get(id).copied().unwrap_or(0);
                draw_text(
                    &format!("x{owned}"),
                    sx + 4.0,
                    reagent_bar_y + reagent_h + 4.0,
                    13.0,
                    SKYBLUE,
                );
            }

            if let Some(name) = hover_name {
                let w = name.len() as f32 * 9.0 + 16.0;
                draw_rectangle(
                    mouse_x + 12.0,
                    mouse_y - 28.0,
                    w,
                    24.0,
                    Color::new(0.0, 0.0, 0.0, 0.85),
                );
                draw_text(&name, mouse_x + 20.0, mouse_y - 10.0, 18.0, WHITE);
            }
            next_frame().await
        }
    }
}
