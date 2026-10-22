// src/render_helpers.rs
use crate::zone::Zone;
use macroquad::prelude::*;

pub fn point_in_rect(point: (f32, f32), rect: (f32, f32, f32, f32)) -> bool {
    let (px, py) = point;
    let (rx, ry, rw, rh) = rect;
    px >= rx && px <= rx + rw && py >= ry && py <= ry + rh
}

pub fn draw_grave_markers(zone: &Zone, count: usize) {
    for i in 0..count {
        let (zx, zy) = zone.position;
        let (zw, _) = zone.size;
        let spacing = zw / (count as f32 + 1.0);
        let gx = zx + spacing * (i as f32 + 1.0) - 6.0;
        let gy = zy + 40.0;
        draw_rectangle(gx, gy, 12.0, 20.0, Color::new(0.6, 0.6, 0.6, 0.8));
    }
}

pub fn draw_temple(zone: &Zone, sacrificed: bool) {
    let (zx, zy) = zone.position;
    let (zw, zh) = zone.size;
    let tw = 40.0;
    let th = 40.0;
    let tx = zx + zw / 2.0 - tw / 2.0;
    let ty = zy + zh - th - 12.0;

    let color = if sacrificed {
        Color::new(1.0, 0.85, 0.3, 0.9)
    } else {
        Color::new(0.4, 0.4, 0.45, 0.6)
    };

    draw_rectangle(tx, ty, tw, th, color);
    draw_rectangle_lines(tx, ty, tw, th, 2.0, if sacrificed { GOLD } else { GRAY });

    if sacrificed {
        draw_text("*", tx + tw / 2.0 - 4.0, ty - 6.0, 24.0, GOLD);
    }
}

pub fn wrap_text(text: &str, font_size: f32, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };
        let dims = measure_text(&candidate, None, font_size as u16, 1.0);
        if dims.width > max_width && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub fn draw_progress_bar(progress: f32, label: &str) {
    let w = 300.0;
    let h = 22.0;
    let x = (screen_width() - w) / 2.0;
    let y = screen_height() - 190.0;
    draw_rectangle(
        x - 4.0,
        y - 26.0,
        w + 8.0,
        h + 34.0,
        Color::new(0.0, 0.0, 0.0, 0.6),
    );
    draw_text(label, x, y - 8.0, 20.0, WHITE);
    draw_rectangle(x, y, w, h, Color::new(0.15, 0.15, 0.15, 0.9));
    draw_rectangle(x, y, w * progress.clamp(0.0, 1.0), h, GREEN);
    draw_rectangle_lines(x, y, w, h, 2.0, WHITE)
}
