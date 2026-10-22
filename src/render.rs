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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileKind {
    Grass,
    Path,
}

impl GameCamera {
    pub fn new() -> Self {
        let zoom_level = 2.5;
        let camera = Camera2D {
            target: vec2(0.0, 0.0),
            zoom: vec2(
                zoom_level / screen_width() * 2.0,
                zoom_level / screen_height() * 2.0,
            ),
            ..Default::default()
        };
        GameCamera { camera, zoom_level }
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

    // Async because macroquad's texture loading is async. Call this
    // during a loading state, not mid-frame during gameplay.
    pub async fn load(&mut self, id: &str, path: &str) {
        let texture = load_texture(path).await.expect("failed to load texture");
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
}

#[derive(Clone, Debug, Deserialize)]
pub struct TileMapDef {
    pub tile_size: f32,
    pub origin: (f32, f32),
    pub legend: HashMap<char, String>,
    pub rows: Vec<String>,
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
                assets.draw(tile_id, vec2(x, y));
            }
        }
    }

    pub fn set_tile(&mut self, row: usize, col: usize, tile_id: &str) {
        if let Some(row_vec) = self.tiles.get_mut(row) {
            if let Some(cell) = row_vec.get_mut(col) {
                *cell = tile_id.to_string();
            }
        }
    }

    pub fn grid_pos(&self, world_pos: Vec2) -> Option<(usize, usize)> {
        let col = ((world_pos.x - self.origin.0) / self.tile_size).floor();
        let row = ((world_pos.y - self.origin.1) / self.tile_size).floor();
        if col < 0.0 || row < 0.0 {
            return None;
        }
        let (col, row) = (col as usize, row as usize);
        if row >= self.tiles.len() || self.tiles.first().map_or(true, |r| col >= r.len()) {
            return None;
        }
        Some((row, col))
    }

    pub fn to_ron(&self) -> String {
        let char_for = |tile_id: &str| -> char {
            match tile_id {
                "grass" => 'G',
                "path" => 'P',
                _ => '?',
            }
        };

        let mut ron = String::new();
        ron.push_str("(\n");
        ron.push_str(&format!("    tile_size: {:.1}, \n", self.tile_size));
        ron.push_str(&format!(
            "     origin: ({1:.1}, {:.1}),\n",
            self.origin.0, self.origin.1
        ));
        ron.push_str("    legend: {\n");
        ron.push_str("        'G': \"grass\",\n");
        ron.push_str("        'P': \"path\",\n");
        ron.push_str("    },\n");
        ron.push_str("    rows: [\n");
        for row in &self.tiles {
            let row_str: String = row.iter().map(|id| char_for(id)).collect();
            ron.push_str(&format!("        \"{}\",\n", row_str));
        }
        ron.push_str("    ],\n");
        ron.push_str(")\n");
        ron
    }
}
