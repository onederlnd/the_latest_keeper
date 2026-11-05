// src/ui.rs
#![allow(dead_code)]
use crate::render_helpers::point_in_rect;
use macroquad::prelude::*;

pub type R4 = (f32, f32, f32, f32);

pub mod theme {
    use macroquad::prelude::Color;
    pub const PANEL: Color = Color::new(0.08, 0.08, 0.12, 0.96);
    pub const TEXT: Color = Color::new(0.92, 0.90, 0.85, 1.0);
    pub const TEXT_DIM: Color = Color::new(0.60, 0.60, 0.62, 1.0);
    pub const ACCENT_GOLD: Color = Color::new(0.85, 0.70, 0.30, 1.0);
    pub const ACCENT_RED: Color = Color::new(0.75, 0.22, 0.25, 1.0);
    pub const VILLAGE: Color = Color::new(0.45, 0.65, 0.95, 1.0);
    pub const CULT: Color = Color::new(0.90, 0.35, 0.30, 1.0);
    pub const BTN: Color = Color::new(0.15, 0.15, 0.18, 0.95);
    pub const BTN_HOVER: Color = Color::new(0.22, 0.38, 0.22, 0.95);
    pub const BTN_DOWN: Color = Color::new(0.16, 0.28, 0.16, 1.0);
    pub const BTN_OFF: Color = Color::new(0.10, 0.10, 0.10, 0.60);
}

fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

pub struct Ui {
    font: Option<Font>,
}
impl Ui {
    pub fn new(font_bytes: Option<&[u8]>) -> Self {
        let font = font_bytes
            .and_then(|b| load_ttf_font_from_bytes(b).ok())
            .map(|mut f| {
                f.set_filter(FilterMode::Nearest);
                f
            });
        Ui { font }
    }

    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, color: Color) {
        draw_text_ex(
            s,
            x,
            y,
            TextParams {
                font: self.font.as_ref(),
                font_size: size as u16,
                color,
                ..Default::default()
            },
        );
    }

    pub fn measure(&self, s: &str, size: f32) -> TextDimensions {
        measure_text(s, self.font.as_ref(), size as u16, 1.0)
    }

    pub fn dim_world(&self) {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.55),
        );
    }
    pub fn panel(&self, r: R4, title: Option<&str>, accent: Color) {
        let (x, y, w, h) = r;
        draw_rectangle(x + 4.0, y + 4.0, w, h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_rectangle(x, y, w, h, theme::PANEL);
        draw_rectangle_lines(x, y, w, h, 2.0, accent);
        draw_rectangle_lines(x + 4.0, y + 4.0, w - 8.0, h - 8.0, 1.0, alpha(accent, 0.3));
        if let Some(t) = title {
            self.text(t, x + 20.0, y + 34.0, 26.0, theme::TEXT);
            draw_line(x + 16.0, y + 46.0, x + w, y + 46.0, 1.0, alpha(accent, 0.5));
        }
    }

    pub fn button(&self, r: R4, label: &str, enabled: bool) -> bool {
        let hover = enabled && point_in_rect(mouse_position(), r);
        let down = hover && is_mouse_button_down(MouseButton::Left);
        let bg = if !enabled {
            theme::BTN_OFF
        } else if down {
            theme::BTN_DOWN
        } else if hover {
            theme::BTN_HOVER
        } else {
            theme::BTN
        };
        let edge = if !enabled {
            theme::TEXT_DIM
        } else if hover {
            theme::ACCENT_GOLD
        } else {
            theme::TEXT
        };
        draw_rectangle(r.0, r.1, r.2, r.3, bg);
        draw_rectangle_lines(r.0, r.1, r.2, r.3, 2.0, edge);
        let d = self.measure(label, 24.0);
        self.text(
            label,
            r.0 + (r.2 - d.width) / 2.0,
            r.1 + r.3 / 2.0 + d.height / 2.0,
            24.0,
            if enabled {
                theme::TEXT
            } else {
                theme::TEXT_DIM
            },
        );
        hover && is_mouse_button_pressed(MouseButton::Left)
    }

    pub fn tooltip(&self, s: &str, at: (f32, f32)) {
        let d = self.measure(s, 18.0);
        let (w, h) = (d.width + 16.0, 26.0);
        let x = (at.0 + 12.0).min(screen_width() - w - 4.0);
        let y = (at.1 - 30.0).max(4.0);
        draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.9));
        draw_rectangle_lines(x, y, w, h, 1.0, theme::ACCENT_GOLD);
        self.text(s, x + 8.0, y + 18.0, 18.0, theme::TEXT);
    }
    pub fn wrap(&self, text: &str, size: f32, max_w: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut cur = String::new();
        for word in text.split_whitespace() {
            let cand = if cur.is_empty() {
                word.to_string()
            } else {
                format!("{cur} {word}")
            };
            if self.measure(&cand, size).width > max_w && !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
                cur = word.to_string();
            } else {
                cur = cand;
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
        lines
    }
    pub fn bubble(&self, text: &str, anchor: (f32, f32)) {
        let size = 18.0;
        let ink = Color::new(0.10, 0.10, 0.12, 1.0);
        let fill = Color::new(0.96, 0.94, 0.88, 0.95);
        let lines = self.wrap(text, size, 300.0);
        let w = lines
            .iter()
            .map(|l| self.measure(l, size).width)
            .fold(0.0, f32::max)
            + 24.0;
        let h = lines.len() as f32 * 22.0 + 16.0;
        let x = (anchor.0 - w / 2.0).clamp(4.0, screen_width() - w - 4.0);
        let y = (anchor.1 - 14.0 - h).max(40.0);
        draw_rectangle(x, y, w, h, fill);
        draw_rectangle_lines(x, y, w, h, 2.0, ink);
        draw_triangle(
            vec2(anchor.0 - 7.0, y + h),
            vec2(anchor.0 + 7.0, y + h),
            vec2(anchor.0, y + h + 12.0),
            fill,
        );
        let mut ty = y + 24.0;
        for l in &lines {
            self.text(l, x + 12.0, ty, size, ink);
            ty += 22.0;
        }
    }

    pub fn prompt(&self, text: &str, anchor: (f32, f32)) {
        let d = self.measure(text, 20.0);
        let (w, h) = (d.width + 24.0, 30.0);
        let (x, y) = (anchor.0 - w / 2.0, anchor.1);
        let pulse = 0.85 + 0.15 * (get_time() as f32 * 4.0).sin() as f32;
        draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.75));
        draw_rectangle_lines(x, y, w, h, 1.5, alpha(theme::ACCENT_GOLD, pulse));
        self.text(text, x + 12.0, y + 21.0, 20.0, theme::TEXT);
    }
}
