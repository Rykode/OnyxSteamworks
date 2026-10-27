#![allow(unused_imports)]

pub mod button;
pub mod chip;
pub mod confirm;
pub mod input;
pub mod overlay;
pub mod progress;
pub mod rail;
pub mod select;
pub mod toggles;

pub use button::{Button, ButtonKind, icon_button};
pub use chip::{chip, chip_size};
pub use confirm::{Choice, Confirm, confirm};
pub use input::Input;
pub use overlay::{ToastLevel, Toasts, modal, modal_active, modal_with, tooltip};
pub use progress::{line_chart, progress_bar, progress_ring, segment_progress};
pub use rail::{RailAction, RailState, kbd, rail};
pub use select::{dropdown, tabs};
pub use toggles::{checkbox, toggle};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::ui::theme;

pub fn surface(painter: &egui::Painter, rect: Rect, fill: egui::Color32, radius_px: f32) {
    painter.rect_filled(rect, theme::corner(radius_px), fill);
}

pub fn bordered(
    painter: &egui::Painter,
    rect: Rect,
    fill: egui::Color32,
    border: egui::Color32,
    radius_px: f32,
) {
    painter.rect_filled(rect, theme::corner(radius_px), fill);
    painter.rect_stroke(
        rect,
        theme::corner(radius_px),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
}

pub fn separator(painter: &egui::Painter, a: Pos2, b: Pos2) {
    painter.line_segment([a, b], Stroke::new(1.0, theme::LINE));
}

pub fn label(painter: &egui::Painter, pos: Pos2, text: &str, s: f32) {
    painter.text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::new(theme::px(12.0, s), theme::family_label()),
        theme::INK_MUTED,
    );
}

pub fn label_optional(painter: &egui::Painter, pos: Pos2, text: &str, suffix: &str, s: f32) {
    let font = FontId::new(theme::px(12.0, s), theme::family_label());
    let w = painter
        .layout_no_wrap(text.to_owned(), font.clone(), theme::INK_MUTED)
        .size()
        .x;
    painter.text(pos, Align2::LEFT_TOP, text, font, theme::INK_MUTED);
    painter.text(
        Pos2::new(pos.x + w + theme::px(8.0, s), pos.y),
        Align2::LEFT_TOP,
        suffix,
        FontId::new(theme::px(11.0, s), theme::family_label()),
        theme::INK_FAINT,
    );
}

pub fn heading(painter: &egui::Painter, pos: Pos2, text: &str, s: f32) {
    painter.text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::new(theme::px(14.0, s), theme::family_semibold()),
        theme::INK,
    );
}

pub fn spinner(
    painter: &egui::Painter,
    center: Pos2,
    r: f32,
    color: egui::Color32,
    ctx: &egui::Context,
) {
    ctx.request_repaint();
    let t = ctx.input(|i| i.time) as f32;
    let start = t * 5.0;
    let sweep = 4.6;
    let n = 24;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| center + Vec2::angled(start + sweep * (i as f32 / n as f32)) * r)
        .collect();
    painter.add(egui::epaint::Shape::line(pts, Stroke::new(2.0, color)));
}

pub fn empty_state(ui: &mut egui::Ui, rect: Rect, glyph: char, title: &str, body: &str, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let painter = ui.painter();
    let cx = rect.center().x;
    let cy = rect.center().y;
    painter.text(
        Pos2::new(cx, cy - px(34.0)),
        Align2::CENTER_CENTER,
        glyph.to_string(),
        theme::icon_font(px(34.0), false),
        theme::INK_MUTED,
    );
    painter.text(
        Pos2::new(cx, cy),
        Align2::CENTER_CENTER,
        title,
        FontId::new(px(15.0), theme::family_semibold()),
        theme::INK_DIM,
    );
    painter.text(
        Pos2::new(cx, cy + px(22.0)),
        Align2::CENTER_CENTER,
        body,
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_MUTED,
    );
}
