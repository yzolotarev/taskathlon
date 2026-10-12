//! Original drawn-in-code look: chunky 3D buttons, a progress bar, the mascot «Сэл»
//! (a spreadsheet cell with a face) and confetti. No external art assets.

use egui::{Align2, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub card: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub green: Color32,
    pub green_dark: Color32,
    pub green_soft: Color32,
    pub red: Color32,
    pub red_dark: Color32,
    pub red_soft: Color32,
    pub blue: Color32,
    pub blue_dark: Color32,
    pub gold: Color32,
    pub orange: Color32,
    pub grey: Color32,
    pub grey_dark: Color32,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: Color32::from_rgb(17, 24, 39),
            card: Color32::from_rgb(31, 41, 55),
            border: Color32::from_rgb(55, 65, 81),
            text: Color32::from_rgb(243, 244, 246),
            muted: Color32::from_rgb(156, 163, 175),
            green: Color32::from_rgb(16, 185, 129),
            green_dark: Color32::from_rgb(5, 150, 105),
            green_soft: Color32::from_rgb(6, 78, 59),
            red: Color32::from_rgb(239, 68, 68),
            red_dark: Color32::from_rgb(185, 28, 28),
            red_soft: Color32::from_rgb(69, 26, 26),
            blue: Color32::from_rgb(59, 130, 246),
            blue_dark: Color32::from_rgb(37, 99, 235),
            gold: Color32::from_rgb(245, 180, 0),
            orange: Color32::from_rgb(249, 115, 22),
            grey: Color32::from_rgb(55, 65, 81),
            grey_dark: Color32::from_rgb(43, 52, 66),
        }
    } else {
        Palette {
            bg: Color32::from_rgb(255, 255, 255),
            card: Color32::from_rgb(248, 250, 252),
            border: Color32::from_rgb(226, 232, 240),
            text: Color32::from_rgb(30, 41, 59),
            muted: Color32::from_rgb(100, 116, 139),
            green: Color32::from_rgb(16, 185, 129),
            green_dark: Color32::from_rgb(4, 120, 87),
            green_soft: Color32::from_rgb(209, 250, 229),
            red: Color32::from_rgb(239, 68, 68),
            red_dark: Color32::from_rgb(220, 38, 38),
            red_soft: Color32::from_rgb(254, 226, 226),
            blue: Color32::from_rgb(59, 130, 246),
            blue_dark: Color32::from_rgb(37, 99, 235),
            gold: Color32::from_rgb(245, 180, 0),
            orange: Color32::from_rgb(249, 115, 22),
            grey: Color32::from_rgb(226, 232, 240),
            grey_dark: Color32::from_rgb(203, 213, 225),
        }
    }
}

pub fn darker(c: Color32) -> Color32 {
    Color32::from_rgb((c.r() as f32 * 0.78) as u8, (c.g() as f32 * 0.78) as u8, (c.b() as f32 * 0.78) as u8)
}

/// A chunky button with a darker "lip" under it; it sinks while pressed.
pub fn big_button(ui: &mut Ui, text: &str, fill: Color32, size: egui::Vec2) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    let lip = 4.0;
    let top = if pressed { rect.translate(vec2(0.0, lip)).with_max_y(rect.max.y) } else { rect.with_max_y(rect.max.y - lip) };
    let p = ui.painter();
    p.rect_filled(rect.with_min_y(rect.min.y + lip), 14.0, darker(fill));
    let face = if resp.hovered() && !pressed { fill.gamma_multiply(1.04) } else { fill };
    p.rect_filled(top, 14.0, face);
    p.text(top.center(), Align2::CENTER_CENTER, text, FontId::proportional(17.0), Color32::WHITE);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// A white outline button (secondary action).
pub fn ghost_button(ui: &mut Ui, text: &str, pal: &Palette, size: egui::Vec2) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    let lip = 3.0;
    let top = if pressed { rect.translate(vec2(0.0, lip)).with_max_y(rect.max.y) } else { rect.with_max_y(rect.max.y - lip) };
    let p = ui.painter();
    p.rect_filled(rect.with_min_y(rect.min.y + lip), 14.0, pal.border);
    p.rect_filled(top, 14.0, pal.bg);
    p.rect_stroke(top, 14.0, Stroke::new(2.0, pal.border), StrokeKind::Inside);
    p.text(top.center(), Align2::CENTER_CENTER, text, FontId::proportional(15.0), pal.muted);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

pub fn progress(ui: &mut Ui, frac: f32, fill: Color32, track: Color32, size: egui::Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    let r = size.y / 2.0;
    p.rect_filled(rect, r, track);
    let w = (rect.width() * frac.clamp(0.0, 1.0)).max(if frac > 0.0 { size.y } else { 0.0 });
    if w > 0.0 {
        let bar = Rect::from_min_size(rect.min, vec2(w, size.y));
        p.rect_filled(bar, r, fill);
        // Highlight stripe, like light on a glossy bar.
        let shine = Rect::from_min_size(rect.min + vec2(r * 0.6, size.y * 0.22), vec2((w - r * 1.2).max(0.0), size.y * 0.2));
        p.rect_filled(shine, size.y * 0.1, Color32::from_white_alpha(70));
    }
}

pub fn chip(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(12.0), fg);
    let size = galley.size() + vec2(16.0, 8.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, 8.0, bg);
    ui.painter().galley(rect.min + vec2(8.0, 4.0), galley, fg);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Happy,
    Cheer,
    Sad,
    Think,
}

/// «Сэл» — a spreadsheet cell with eyes. Bounces when cheering.
pub fn mascot(ui: &mut Ui, size: f32, mood: Mood, pal: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let t = ui.ctx().input(|i| i.time) as f32;
    let bounce = match mood {
        Mood::Cheer => (t * 8.0).sin().abs() * size * 0.08,
        Mood::Happy => (t * 2.0).sin() * size * 0.015,
        _ => 0.0,
    };
    if mood == Mood::Cheer || mood == Mood::Happy {
        ui.ctx().request_repaint();
    }
    let body = Rect::from_center_size(rect.center() - vec2(0.0, bounce), vec2(size * 0.82, size * 0.74));
    let p = ui.painter();
    let fill = if mood == Mood::Sad { pal.blue } else { pal.green };
    p.rect_filled(body.translate(vec2(0.0, size * 0.05)), size * 0.16, darker(fill));
    p.rect_filled(body, size * 0.16, fill);
    // Grid lines on the body: it is a cell.
    let line = Stroke::new((size * 0.02).max(1.0), Color32::from_white_alpha(60));
    p.line_segment(
        [pos2(body.min.x + body.width() * 0.5, body.min.y + 4.0), pos2(body.min.x + body.width() * 0.5, body.min.y + body.height() * 0.22)],
        line,
    );
    // Fill handle: the little square in the corner of a selected cell.
    let h = size * 0.09;
    p.rect_filled(Rect::from_center_size(body.right_bottom() - vec2(h * 0.2, h * 0.2), vec2(h, h)), 2.0, pal.gold);
    let eye_y = body.min.y + body.height() * 0.42;
    let ex = body.width() * 0.2;
    for dx in [-ex, ex] {
        let c = pos2(body.center().x + dx, eye_y);
        p.circle_filled(c, size * 0.11, Color32::WHITE);
        let look = match mood {
            Mood::Think => vec2(size * 0.03, -size * 0.03),
            _ => vec2(0.0, size * 0.02),
        };
        p.circle_filled(c + look, size * 0.055, Color32::from_rgb(40, 40, 40));
        p.circle_filled(c + look + vec2(-size * 0.02, -size * 0.02), size * 0.018, Color32::WHITE);
    }
    let mouth_c = pos2(body.center().x, body.min.y + body.height() * 0.7);
    let w = body.width() * 0.22;
    let pts: Vec<Pos2> = (0..=12)
        .map(|i| {
            let x = -1.0 + i as f32 / 6.0;
            let curve = match mood {
                Mood::Sad => -0.35,
                Mood::Think => 0.0,
                _ => 0.55,
            };
            pos2(mouth_c.x + x * w, mouth_c.y + (1.0 - x * x) * curve * w * 0.6)
        })
        .collect();
    p.add(egui::Shape::line(pts, Stroke::new((size * 0.04).max(2.0), Color32::from_rgb(40, 40, 40))));
}

/// Speech bubble to the right of the mascot.
pub fn bubble(ui: &mut Ui, text: &str, pal: &Palette, width: f32) {
    egui::Frame::NONE.fill(pal.bg).stroke(Stroke::new(2.0, pal.border)).corner_radius(14.0).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
        ui.set_width(width);
        ui.add(egui::Label::new(egui::RichText::new(text).size(15.0).color(pal.text)).wrap());
    });
}

pub struct Particle {
    pub pos: Pos2,
    pub vel: egui::Vec2,
    pub color: Color32,
    pub spin: f32,
    pub angle: f32,
}

pub fn spawn_confetti(into: &mut Vec<Particle>, area: Rect, seed: u64) {
    let colors = [
        Color32::from_rgb(16, 185, 129),
        Color32::from_rgb(59, 130, 246),
        Color32::from_rgb(245, 180, 0),
        Color32::from_rgb(239, 68, 68),
        Color32::from_rgb(168, 85, 247),
        Color32::from_rgb(249, 115, 22),
    ];
    let mut x = seed | 1;
    let mut rnd = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % 10_000) as f32 / 10_000.0
    };
    for i in 0..140 {
        let pos = pos2(area.min.x + rnd() * area.width(), area.min.y - rnd() * 60.0);
        let vel = vec2((rnd() - 0.5) * 260.0, 80.0 + rnd() * 260.0);
        let color = colors.get(i % colors.len()).copied().unwrap_or(Color32::WHITE);
        into.push(Particle { pos, vel, color, spin: (rnd() - 0.5) * 10.0, angle: rnd() * 6.0 });
    }
}

pub fn draw_confetti(ctx: &egui::Context, parts: &mut Vec<Particle>) {
    if parts.is_empty() {
        return;
    }
    let dt = ctx.input(|i| i.stable_dt).min(0.05);
    let bottom = ctx.content_rect().max.y + 40.0;
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("learn-confetti")));
    for q in parts.iter_mut() {
        q.vel.y += 420.0 * dt;
        q.vel.x *= 0.995;
        q.pos += q.vel * dt;
        q.angle += q.spin * dt;
        let (s, c) = q.angle.sin_cos();
        let half = vec2(6.0 * c, 3.0 * s.abs() + 1.0);
        painter.rect_filled(Rect::from_center_size(q.pos, vec2(half.x.abs() * 2.0 + 2.0, half.y * 2.0)), 1.0, q.color);
    }
    parts.retain(|q| q.pos.y < bottom);
    ctx.request_repaint();
}

/// Ring around a path node: one quarter per staircase level.
pub fn level_ring(ui: &Ui, center: Pos2, radius: f32, level: u8, color: Color32, track: Color32) {
    let p = ui.painter();
    p.circle_stroke(center, radius, Stroke::new(6.0, track));
    let frac = f32::from(level.min(4)) / 4.0;
    if frac <= 0.0 {
        return;
    }
    let steps = (48.0 * frac).ceil() as usize;
    let pts: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * frac * i as f32 / steps as f32;
            center + vec2(a.cos(), a.sin()) * radius
        })
        .collect();
    p.add(egui::Shape::line(pts, Stroke::new(6.0, color)));
}

/// A big word that pops in, holds and fades out (`k` runs 0 → 1).
pub fn draw_pop(ctx: &egui::Context, text: &str, color: Color32, k: f32) {
    let k = k.clamp(0.0, 1.0);
    let scale = if k < 0.15 {
        0.6 + k / 0.15 * 0.6
    } else if k < 0.25 {
        1.2 - (k - 0.15) * 2.0
    } else {
        1.0
    };
    let alpha = if k > 0.7 { 1.0 - (k - 0.7) / 0.3 } else { 1.0 };
    let area = ctx.content_rect();
    let center = pos2(area.center().x, area.min.y + area.height() * 0.32 - k * 30.0);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("learn-pop")));
    let font = FontId::proportional(54.0 * scale);
    let shadow = Color32::from_black_alpha((90.0 * alpha) as u8);
    painter.text(center + vec2(0.0, 4.0), Align2::CENTER_CENTER, text, font.clone(), shadow);
    painter.text(center, Align2::CENTER_CENTER, text, font, color.gamma_multiply(alpha));
}

/// Toasts stacked at the top right; each slides in and fades after a few seconds.
pub fn draw_toasts(ctx: &egui::Context, toasts: &[crate::Toast], now: f64, pal: &Palette) {
    let area = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("learn-toasts")));
    for (i, t) in toasts.iter().enumerate() {
        let age = (now - t.born) as f32;
        let slide = (1.0 - (age / 0.25).min(1.0)) * 80.0;
        let alpha = if age > 4.2 { (1.0 - (age - 4.2) / 0.8).max(0.0) } else { 1.0 };
        let galley = painter.layout_no_wrap(t.text.clone(), FontId::proportional(15.0), pal.text.gamma_multiply(alpha));
        let w = (galley.size().x + 72.0).clamp(240.0, area.width() - 40.0);
        let rect = Rect::from_min_size(pos2(area.max.x - w - 20.0 + slide, area.min.y + 70.0 + i as f32 * 62.0), vec2(w, 52.0));
        painter.rect_filled(rect.translate(vec2(0.0, 3.0)), 14.0, Color32::from_black_alpha((40.0 * alpha) as u8));
        painter.rect_filled(rect, 14.0, pal.card.gamma_multiply(alpha));
        painter.rect_stroke(rect, 14.0, Stroke::new(2.0, pal.gold.gamma_multiply(alpha)), egui::StrokeKind::Inside);
        painter.text(
            rect.left_center() + vec2(26.0, 0.0),
            Align2::CENTER_CENTER,
            &t.icon,
            FontId::proportional(24.0),
            pal.gold.gamma_multiply(alpha),
        );
        painter.galley(rect.left_center() + vec2(50.0, -galley.size().y / 2.0), galley, pal.text);
    }
}

/// Simple vertical bars with labels under them; `goal` draws a dashed line.
pub fn bars(ui: &mut Ui, values: &[(String, f32)], goal: Option<f32>, color: Color32, pal: &Palette, height: f32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height + 22.0), Sense::hover());
    let painter = ui.painter();
    let max = values.iter().map(|v| v.1).fold(goal.unwrap_or(1.0), f32::max).max(1.0);
    let n = values.len().max(1) as f32;
    let slot = width / n;
    for (i, (label, v)) in values.iter().enumerate() {
        let h = v / max * height;
        let x = rect.min.x + slot * i as f32;
        let bar = Rect::from_min_max(pos2(x + slot * 0.18, rect.min.y + height - h), pos2(x + slot * 0.82, rect.min.y + height));
        painter.rect_filled(bar, 4.0, if goal.is_some_and(|g| *v >= g) { color } else { color.gamma_multiply(0.55) });
        if *v > 0.0 {
            painter.text(pos2(bar.center().x, bar.min.y - 8.0), Align2::CENTER_CENTER, format!("{v:.0}"), FontId::proportional(11.0), pal.muted);
        }
        painter.text(pos2(bar.center().x, rect.min.y + height + 11.0), Align2::CENTER_CENTER, label, FontId::proportional(11.0), pal.muted);
    }
    if let Some(g) = goal {
        let y = rect.min.y + height - g / max * height;
        let mut x = rect.min.x;
        while x < rect.max.x {
            painter.line_segment([pos2(x, y), pos2((x + 6.0).min(rect.max.x), y)], Stroke::new(1.5, pal.gold));
            x += 12.0;
        }
    }
}
