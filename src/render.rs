// src/render.rs

use macroquad::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

// World units, not screen pixels. Keep gameplay code in world space and
// let the camera handle the conversion — this is what lets us zoom/pan
// later without touching draw calls elsewhere.

pub const TILE_SIZE: f32 = 32.0;

pub struct GameCamera {
    pub camera: Camera2D,
    pub zoom_level: f32,
    smooth: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileKind {
    Grass,
    Path,
}

impl GameCamera {
    pub fn new() -> Self {
        let zoom_level = 2.0;
        let camera = Camera2D {
            target: vec2(0.0, 0.0),
            zoom: vec2(
                zoom_level / screen_width() * 2.0,
                zoom_level / screen_height() * 2.0,
            ),
            ..Default::default()
        };
        GameCamera {
            camera,
            zoom_level,
            smooth: vec2(0.0, 0.0),
        }
    }

    // Call once per frame before drawing world-space content. Recomputes
    // zoom against current screen size so resizing the window doesn't
    // distort the view.
    pub fn update(&mut self, target: Vec2) {
        self.camera.target = target;
        self.camera.zoom = vec2(
            self.zoom_level / screen_width() * 2.0,
            self.zoom_level / screen_height() * 2.0,
        );
        set_camera(&self.camera);
    }

    pub fn follow(&mut self, target: Vec2, dt: f32) {
        let delta = target - self.smooth;
        if delta.length() > 100.0 {
            self.smooth = target;
        } else {
            self.smooth += delta * (1.0 - (-10.0 * dt).exp());
        }
        let z = self.zoom_level;
        let snapped = vec2(
            (self.smooth.x * z).round() / z,
            (self.smooth.y * z).round() / z,
        );
        self.update(snapped);
    }
    pub fn set_zoom(&mut self, zoom_level: f32) {
        self.zoom_level = zoom_level.clamp(0.25, 4.0);
    }
}

// Simple id -> texture registry so gameplay code refers to sprites by
// string id (matching items.ron etc.) instead of holding Texture2D
// handles directly.
pub struct AssetStore {
    textures: HashMap<String, Texture2D>,
}

impl AssetStore {
    pub fn new() -> Self {
        AssetStore {
            textures: HashMap::new(),
        }
    }

    pub fn load(&mut self, id: &str, bytes: &[u8]) {
        let texture = Texture2D::from_file_with_format(bytes, None);
        texture.set_filter(FilterMode::Nearest);
        self.textures.insert(id.to_string(), texture);
    }

    pub fn get(&self, id: &str) -> &Texture2D {
        self.textures
            .get(id)
            .unwrap_or_else(|| panic!("missing texture: {id}"))
    }

    pub fn draw(&self, id: &str, world_pos: Vec2) {
        let texture = self.get(id);
        draw_texture(texture, world_pos.x, world_pos.y, WHITE);
    }
    pub fn draw_tinted(&self, id: &str, world_pos: Vec2, color: Color) {
        draw_texture(self.get(id), world_pos.x, world_pos.y, color);
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct TileMapDef {
    pub tile_size: f32,
    pub origin: (f32, f32),
    pub legend: HashMap<char, String>,
    pub rows: Vec<String>,
    #[serde(default)]
    pub solids: Vec<(f32, f32, f32, f32)>,
}

pub struct TileMap {
    pub tiles: Vec<Vec<String>>,
    pub tile_size: f32,
    pub origin: (f32, f32),
}

impl TileMap {
    pub fn from_def(def: &TileMapDef) -> Self {
        let tiles = def
            .rows
            .iter()
            .map(|row| {
                row.chars()
                    .map(|c| {
                        def.legend
                            .get(&c)
                            .cloned()
                            .unwrap_or_else(|| panic!("unmapped tile char: {c:?}"))
                    })
                    .collect()
            })
            .collect();
        TileMap {
            tiles,
            tile_size: def.tile_size,
            origin: def.origin,
        }
    }

    pub fn draw(&self, assets: &AssetStore) {
        for (row_idx, row) in self.tiles.iter().enumerate() {
            for (col_idx, tile_id) in row.iter().enumerate() {
                let x = self.origin.0 + col_idx as f32 * self.tile_size;
                let y = self.origin.1 + row_idx as f32 * self.tile_size;
                let h = (col_idx as u32).wrapping_mul(73856093)
                    ^ (row_idx as u32).wrapping_mul(19349663);
                let shade = 0.65 + 0.12 * ((h % 100) as f32 / 100.0); // shade strength = higher
                let tint = if tile_id == "grass" {
                    Color::new(shade, shade, shade, 1.0)
                } else {
                    WHITE
                };
                assets.draw_tinted(tile_id, vec2(x, y), tint);
            }
        }
        let ts = self.tile_size;
        let edge = Color::new(0.08, 0.06, 0.04, 0.35);
        let t = 4.0;
        let is_path = |r: isize, c: isize| {
            if r < 0 || c < 0 {
                return true;
            }
            self.tiles
                .get(r as usize)
                .and_then(|row| row.get(c as usize))
                .map_or(true, |s| s.as_str() == "path")
        };
        for (r, row) in self.tiles.iter().enumerate() {
            for (c, id) in row.iter().enumerate() {
                if id.as_str() != "path" {
                    continue;
                }
                let x = self.origin.0 + c as f32 * ts;
                let y = self.origin.1 + r as f32 * ts;
                let (ri, ci) = (r as isize, c as isize);
                if !is_path(ri - 1, ci) {
                    draw_rectangle(x, y, ts, t, edge);
                }
                if !is_path(ri + 1, ci) {
                    draw_rectangle(x, y + ts - t, ts, t, edge);
                }
                if !is_path(ri, ci - 1) {
                    draw_rectangle(x, y, t, ts, edge);
                }
                if !is_path(ri, ci + 1) {
                    draw_rectangle(x + ts - t, y, t, ts, edge);
                }
            }
        }
    }
}
