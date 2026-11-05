// src/village_layout.rs

use crate::render;
use crate::zone::{DoorSide, Zone, ZoneRegistry};
use macroquad::rand::RandGenerator;
use std::collections::{HashMap, HashSet};

pub const COLS: i32 = 50;
pub const ROWS: i32 = 60;
pub const TILE: f32 = 32.0;
pub const ORIGIN: (f32, f32) = (-800.0, -960.0);

const GREEN: (f32, f32) = (25.5, 32.5);
const GREEN_R: f32 = 4.6;

type Rect = (f32, f32, f32, f32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VillagePropKind {
    Tree,
    Bush,
    Rock,
    Barrel,
    Crate,
    Well,
    Lamp,
    Bench,
}
#[derive(Clone, Copy, Debug)]
pub struct VillageProp {
    pub kind: VillagePropKind,
    pub pos: (f32, f32),
}

pub struct Pose {
    pub pos: (f32, f32),
    pub side: DoorSide,
    pub off: f32,
}

pub struct Layout {
    road: HashSet<(i32, i32)>,
    lanes: Vec<Vec<(i32, i32)>>,
    stubs: Vec<Rect>,
}

fn left(c: i32) -> f32 {
    ORIGIN.0 + c as f32 * TILE
}

fn top(r: i32) -> f32 {
    ORIGIN.1 + r as f32 * TILE
}

fn tile_of(x: f32, y: f32) -> (i32, i32) {
    (
        ((x - ORIGIN.0) / TILE).floor() as i32,
        ((y - ORIGIN.1) / TILE).floor() as i32,
    )
}

fn in_bounds(c: i32, r: i32) -> bool {
    c >= 2 && c < COLS - 2 && r >= 2 && r < ROWS - 2
}

fn green_dist2(c: i32, r: i32) -> f32 {
    let dx = c as f32 + 0.5 - GREEN.0;
    let dy = r as f32 + 0.5 - GREEN.1;
    dx * dx + dy * dy
}

fn near(a: Rect, b: Rect, pad: f32) -> bool {
    a.0 < b.0 + b.2 + pad && a.0 + a.2 + pad > b.0 && a.1 < b.1 + b.3 + pad && a.1 + a.3 + pad > b.1
}

pub fn door_offset(len: f32) -> f32 {
    (len / 2.0 / TILE).floor() * TILE + TILE / 2.0
}

fn rand_door_offset(rng: &RandGenerator, len: f32) -> f32 {
    let kmax = ((len - 32.0) / 32.0).floor().max(0.0) as i32;
    rng.gen_range(0, kmax + 1) as f32 * TILE + TILE / 2.0
}

pub fn door_anchor(z: &Zone) -> Option<(f32, f32)> {
    let (side, off) = z.door.as_ref()?;
    let off = *off; // The door's center measured along
    let (x, y) = z.position;
    let (w, h) = z.size;

    Some(match side {
        DoorSide::Left => (x - 16.0, y + off),
        DoorSide::Right => (x + w + 16.0, y + off),
        DoorSide::Top => (x + off, y - 16.0),
        DoorSide::Bottom => (x + off, y + h + 16.0),
    })
}

pub fn build_roads(rng: &RandGenerator) -> Layout {
    let mut l = Layout {
        road: HashSet::new(),
        lanes: Vec::new(),
        stubs: Vec::new(),
    };
    for r in 0..ROWS {
        for c in 0..COLS {
            if green_dist2(c, r) <= GREEN_R * GREEN_R {
                l.road.insert((c, r));
            }
        }
    }

    let arts: [((i32, i32), (i32, i32), i32); 4] = [
        ((25, 27), (0, -1), 17),
        ((25, 37), (0, 1), 14),
        ((20, 32), (-1, 0), 13),
        ((30, 32), (1, 0), 13),
    ];
    for (start, dir, base) in arts {
        let len = base - rng.gen_range(0, 4);
        l.carve_lane(rng, start, dir, len, 8);
    }

    for _ in 0..5 {
        let a = rng.gen_range(0, 4) as usize;
        let cells = l.lanes[a].clone();
        if cells.len() < 9 {
            continue;
        }
        let i = rng.gen_range(5, cells.len() as i32 - 2) as usize;
        let (c, r) = cells[i];
        let dir = arts[a].1;
        let perp = (dir.1.abs(), dir.0.abs());
        let s = if rng.gen_range(0, 2) == 0 { 1 } else { -1 };
        let start = (c + perp.0 * s, r + perp.1 * s);
        let len = 4 + rng.gen_range(0, 5);
        l.carve_lane(rng, start, (perp.0 * s, perp.1 * s), len, 9);
    }
    l
}

impl Layout {
    fn carve_lane(
        &mut self,
        rng: &RandGenerator,
        start: (i32, i32),
        dir: (i32, i32),
        len: i32,
        wobble: i32,
    ) {
        let lat = (dir.1.abs(), dir.0.abs());
        let mut cells: Vec<(i32, i32)> = Vec::new();
        let (mut c, mut r) = start;
        let mut drift: i32 = 0;
        if in_bounds(c, r) {
            self.road.insert((c, r));
            cells.push((c, r));
        }
        for _ in 0..len {
            c += dir.0;
            r += dir.1;
            if !in_bounds(c, r) {
                break;
            }
            self.road.insert((c, r));
            cells.push((c, r));
            if rng.gen_range(0, wobble) == 0 {
                let s: i32 = if rng.gen_range(0, 2) == 0 { 1 } else { -1 };
                if (drift + s).abs() <= 2 {
                    drift += s;
                    c += lat.0 * s;
                    r += lat.1 * s;
                    if !in_bounds(c, r) {
                        break;
                    }
                    self.road.insert((c, r));
                    cells.push((c, r));
                }
            }
        }
        self.lanes.push(cells);
    }

    pub fn try_place(
        &mut self,
        zones: &ZoneRegistry,
        rng: &RandGenerator,
        w: f32,
        h: f32,
    ) -> Option<Pose> {
        let mut slots: Vec<(i32, i32, bool)> = Vec::new();
        for lane in &self.lanes {
            for (i, &(c, r)) in lane.iter().enumerate() {
                if green_dist2(c, r) < 6.2 * 6.2 {
                    continue;
                }
                let a = lane[i.saturating_sub(2)];
                let b = lane[(i + 2).min(lane.len() - 1)];
                slots.push((c, r, (b.0 - a.0).abs() > (b.1 - a.1).abs()));
            }
        }
        if slots.is_empty() {
            return None;
        }

        for _ in 0..400 {
            let mut best = slots[rng.gen_range(0, slots.len() as i32) as usize];
            for _ in 0..2 {
                let s = slots[rng.gen_range(0, slots.len() as i32) as usize];
                if green_dist2(s.0, s.1) < green_dist2(best.0, best.1) {
                    best = s;
                }
            }
            let (c, r, horizontal) = best;
            let sf = (1 + rng.gen_range(0, 3)) as f32;
            let before = rng.gen_range(0, 2) == 0;
            let (cx, cy) = (left(c) + TILE / 2.0, top(r) + TILE / 2.0);

            let (x, y, side, off, stub): (f32, f32, DoorSide, f32, Rect) = if horizontal {
                let off = rand_door_offset(rng, w);
                let x = cx - off;
                if before {
                    let edge = top(r) - sf * TILE;
                    (
                        x,
                        edge - h,
                        DoorSide::Bottom,
                        off,
                        (cx - 16.0, edge, TILE, sf * TILE),
                    )
                } else {
                    (
                        x,
                        top(r) + (sf + 1.0) * TILE,
                        DoorSide::Top,
                        off,
                        (cx - 16.0, top(r) + TILE, TILE, sf * TILE),
                    )
                }
            } else {
                let off = rand_door_offset(rng, h);
                let y = cy - off;
                if before {
                    let edge = left(c) - sf * TILE;
                    (
                        edge - w,
                        y,
                        DoorSide::Right,
                        off,
                        (edge, cy - 16.0, sf * TILE, TILE),
                    )
                } else {
                    (
                        left(c) + (sf + 1.0) * TILE,
                        y,
                        DoorSide::Left,
                        off,
                        (left(c) + TILE, cy - 16.0, sf * TILE, TILE),
                    )
                }
            };
            let rect: Rect = (x, y, w, h);
            if x < ORIGIN.0 + 48.0
                || x + w > -ORIGIN.0 - 48.0
                || y < ORIGIN.1 + 48.0
                || y + h > -ORIGIN.1 - 48.0
            {
                continue;
            }
            // keep off every road/green/stub tile
            let (c0, r0) = tile_of(x - 12.0, y - 12.0);
            let (c1, r1) = tile_of(x + w + 12.0, y + h + 12.0);
            let mut blocked = false;
            'scan: for cc in c0..=c1 {
                for rr in r0..=r1 {
                    if self.road.contains(&(cc, rr)) {
                        blocked = true;
                        break 'scan;
                    }
                }
            }
            if blocked {
                continue;
            }
            if zones.zones.values().any(|z| {
                z.overlaps(x, y, w, h, 40.0) || z.overlaps(stub.0, stub.1, stub.2, stub.3, 8.0)
            }) {
                continue;
            }
            if self.stubs.iter().any(|&s| near(rect, s, 16.0)) {
                continue;
            }

            // commit: the stub becomes road
            let (sc, sr) = tile_of(stub.0 + 1.0, stub.1 + 1.0);
            let (ec, er) = tile_of(stub.0 + stub.2 - 1.0, stub.1 + stub.3 - 1.0);
            for cc in sc..=ec {
                for rr in sr..=er {
                    self.road.insert((cc, rr));
                }
            }
            self.stubs.push(stub);
            return Some(Pose {
                pos: (x, y),
                side,
                off,
            });
        }
        None
    }

    pub fn tilemap(&self) -> render::TileMapDef {
        let rows: Vec<String> = (0..ROWS)
            .map(|r| {
                (0..COLS)
                    .map(|c| {
                        if self.road.contains(&(c, r)) {
                            'P'
                        } else {
                            'G'
                        }
                    })
                    .collect()
            })
            .collect();
        let mut legend = HashMap::new();
        legend.insert('G', "grass".to_string());
        legend.insert('P', "path".to_string());
        render::TileMapDef {
            tile_size: TILE,
            origin: ORIGIN,
            legend,
            rows,
            solids: vec![],
        }
    }

    fn clear(
        &self,
        props: &[VillageProp],
        rects: &[Rect],
        x: f32,
        y: f32,
        pad: f32,
        min_d: f32,
    ) -> bool {
        if x < ORIGIN.0 + 24.0
            || x > -ORIGIN.0 - 24.0
            || y < ORIGIN.1 + 24.0
            || y > -ORIGIN.1 - 24.0
        {
            return false;
        }
        for dx in [-20.0, 0.0, 20.0] {
            for dy in [-20.0, 0.0, 20.0] {
                if self.road.contains(&tile_of(x + dx, y + dy)) {
                    return false;
                }
            }
        }

        if x.abs() < 50.0 && y.abs() < 50.0 {
            return false; // portal
        }

        if x.abs() < 30.0 && y > 120.0 && y < 180.0 {
            return false; // spawn
        }
        let probe: Rect = (x - 1.0, y - 1.0, 2.0, 2.0);
        if rects.iter().any(|&z| near(probe, z, pad)) {
            return false;
        }
        props.iter().all(|p| {
            let (dx, dy) = (x - p.pos.0, y - p.pos.1);
            dx * dx + dy * dy >= min_d * min_d
        })
    }

    pub fn scatter_props(&self, zones: &ZoneRegistry, rng: &RandGenerator) -> Vec<VillageProp> {
        use VillagePropKind::*;
        let mut props: Vec<VillageProp> = Vec::new();
        let rects: Vec<Rect> = zones
            .zones
            .values()
            .map(|z| (z.position.0, z.position.1, z.size.0, z.size.1))
            .collect();

        // green furniture
        let (wx, wy) = (left(25) + TILE / 2.0, top(32) + TILE / 2.0);
        props.push(VillageProp {
            kind: Well,
            pos: (wx, wy),
        });
        for (sx, sy) in [(-1.0f32, -1.0f32), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
            props.push(VillageProp {
                kind: Lamp,
                pos: (wx + sx * 82.0, wy + sy * 82.0),
            });
        }
        props.push(VillageProp {
            kind: Bench,
            pos: (wx - 96.0, wy),
        });
        props.push(VillageProp {
            kind: Bench,
            pos: (wx + 96.0, wy),
        });

        // cluster beside each door
        for z in zones.zones.values() {
            let Some((ax, ay)) = door_anchor(z) else {
                continue;
            };
            let is_store = z.id.contains("_store_");
            let n = if is_store {
                3
            } else if rng.gen_range(0.0f32, 1.0) < 0.7 {
                1
            } else {
                0
            };
            let horizontal_side = matches!(
                z.door.as_ref().map(|d| &d.0),
                Some(DoorSide::Top) | Some(DoorSide::Bottom)
            );
            for _ in 0..n {
                let lat = if rng.gen_range(0, 2) == 0 {
                    -28.0
                } else {
                    28.0
                };
                let kind = if is_store {
                    if rng.gen_range(0, 2) == 0 {
                        Barrel
                    } else {
                        Crate
                    }
                } else {
                    match rng.gen_range(0, 3) {
                        0 => Barrel,
                        1 => Crate,
                        _ => Bush,
                    }
                };
                let (mut x, mut y) = if horizontal_side {
                    (ax + lat, ay)
                } else {
                    (ax, ay + lat)
                };
                x += rng.gen_range(-6.0f32, 6.0);
                y += rng.gen_range(-6.0f32, 6.0);
                if self.clear(&props, &rects, x, y, 4.0, 14.0) {
                    props.push(VillageProp { kind, pos: (x, y) });
                }
            }
        }

        // lamps along the main road
        let cells: Vec<(i32, i32)> = self.lanes.iter().take(4).flatten().copied().collect();
        let mut lamps = 0;
        for _ in 0..40 {
            if lamps >= 8 || cells.is_empty() {
                break;
            }
            let (c, r) = cells[rng.gen_range(0, cells.len() as i32) as usize];
            for (dc, dr) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                if !self.road.contains(&(c + dc, r + dr)) {
                    let (x, y) = (left(c + dc) + TILE / 2.0, top(r + dr) + TILE / 2.0);
                    if self.clear(&props, &rects, x, y, 12.0, 120.0) {
                        props.push(VillageProp {
                            kind: Lamp,
                            pos: (x, y),
                        });
                        lamps += 1;
                    }
                    break;
                }
            }
        }
        //trees: denser towards the edges, sometimes in clumps
        let mut trees = 0;
        for _ in 0..900 {
            if trees >= 80 {
                break;
            }
            let x = rng.gen_range(ORIGIN.0 + 30.0, -ORIGIN.0 - 30.0);
            let y = rng.gen_range(ORIGIN.1 + 30.0, -ORIGIN.1 - 30.0);
            let d = (x * x + (y - 80.0) * (y - 80.0)).sqrt();
            if rng.gen_range(0.0f32, 1.0) > (d / 650.0).clamp(0.12, 1.0) {
                continue;
            }
            if !self.clear(&props, &rects, x, y, 28.0, 40.0) {
                continue;
            }
            props.push(VillageProp {
                kind: Tree,
                pos: (x, y),
            });
            trees += 1;
            if rng.gen_range(0.0f32, 1.0) < 0.4 {
                for _ in 0..rng.gen_range(1, 3) {
                    let a = rng.gen_range(0.0f32, std::f32::consts::TAU);
                    let dd = rng.gen_range(48.0f32, 80.0);
                    let (nx, ny) = (x + dd * a.cos(), y + dd * a.sin());
                    if self.clear(&props, &rects, nx, ny, 28.0, 40.0) {
                        props.push(VillageProp {
                            kind: Tree,
                            pos: (nx, ny),
                        });
                        trees += 1;
                    }
                }
            }
        }
        for (kind, want, min_d) in [(Bush, 30, 24.0f32), (Rock, 16, 30.0)] {
            let mut have = 0;
            for _ in 0..want * 20 {
                if have >= want {
                    break;
                }
                let x = rng.gen_range(ORIGIN.0 + 30.0, -ORIGIN.0 - 30.0);
                let y = rng.gen_range(ORIGIN.1 + 30.0, -ORIGIN.1 - 30.0);
                if self.clear(&props, &rects, x, y, 20.0, min_d) {
                    props.push(VillageProp { kind, pos: (x, y) });
                    have += 1;
                }
            }
        }
        props
    }
}
