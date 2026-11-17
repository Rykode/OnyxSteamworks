#![allow(unused_imports)]

pub mod button;
pub mod chip;
pub mod confirm;
pub mod input;
pub mod menu;
pub mod overlay;
pub mod progress;
pub mod rail;
pub mod select;
pub mod toggles;

pub use button::{Button, ButtonKind, icon_button};
pub use chip::{chip, chip_size};
pub use confirm::{Choice, Confirm, confirm};
pub use input::Input;
pub use menu::{MenuEvent, MenuItem, menu};
pub use overlay::{ToastLevel, Toasts, modal, modal_active, modal_with, tooltip};
pub use progress::{line_chart, progress_bar, progress_ring, segment_progress};
pub use rail::{RailAction, RailState, kbd, rail};
pub use select::dropdown;
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

pub fn label(painter: &egui::Painter, pos: Pos2, text: &str, s: f32) {
    painter.text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::new(theme::text_px(12.0, s), theme::family_label()),
        theme::INK_MUTED,
    );
}

pub fn heading(painter: &egui::Painter, pos: Pos2, text: &str, s: f32) {
    painter.text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::new(theme::text_px(14.0, s), theme::family_semibold()),
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
        FontId::new(theme::text_px(15.0, s), theme::family_semibold()),
        theme::INK_DIM,
    );
    painter.text(
        Pos2::new(cx, cy + px(22.0)),
        Align2::CENTER_CENTER,
        body,
        FontId::new(theme::text_px(12.5, s), theme::family_medium()),
        theme::INK_MUTED,
    );
    let area = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(px(320.0), px(80.0)));
    readable(ui, area, egui::Role::Label, &format!("{title}. {body}"), false);
}

pub const RADIUS_CARD_PX: f32 = 17.0;

pub fn card(painter: &egui::Painter, rect: Rect, s: f32) {
    let r = theme::px(RADIUS_CARD_PX, s);
    theme::shadow(painter, rect, r, 18, 60);
    bordered(painter, rect, theme::SURFACE, theme::LINE_SOFT, r);
}

pub fn spaced(text: &str, font: FontId, color: egui::Color32, spacing: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font,
            color,
            extra_letter_spacing: spacing,
            ..Default::default()
        },
    );
    job
}

pub fn caps(painter: &egui::Painter, pos: Pos2, align: Align2, text: &str, size: f32, color: egui::Color32, s: f32) -> Rect {
    let job = spaced(
        &text.to_uppercase(),
        FontId::new(theme::text_px(size, s), theme::family_semibold()),
        color,
        theme::px(size * 0.1, s),
    );
    let galley = painter.layout_job(job);
    let rect = align.anchor_size(pos, galley.size());
    painter.galley(rect.min, galley, color);
    rect
}

pub fn card_title(ui: &egui::Ui, card: Rect, text: &str, s: f32) -> Rect {
    let rect = caps(
        ui.painter(),
        Pos2::new(card.left() + theme::px(20.0, s), card.top() + theme::px(26.0, s)),
        Align2::LEFT_CENTER,
        text,
        12.5,
        theme::INK_MUTED,
        s,
    );
    readable(ui, rect, egui::Role::Heading, text, false);
    rect
}

pub fn card_note(painter: &egui::Painter, card: Rect, text: &str, s: f32) -> Rect {
    painter.text(
        Pos2::new(card.right() - theme::px(20.0, s), card.top() + theme::px(26.0, s)),
        Align2::RIGHT_CENTER,
        text,
        FontId::new(theme::text_px(12.5, s), theme::family_medium()),
        theme::INK_FAINT,
    )
}

pub fn badge(
    painter: &egui::Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    color: egui::Color32,
    dot: bool,
    s: f32,
) -> Rect {
    let px = |v: f32| theme::px(v, s);
    let font = FontId::new(theme::text_px(12.5, s), theme::family_semibold());
    let galley = painter.layout_no_wrap(text.to_owned(), font, color);
    let lead = if dot { px(17.0) } else { px(9.0) };
    let size = Vec2::new(lead + galley.size().x + px(9.0), px(23.0));
    let rect = align.anchor_size(pos, size);
    painter.rect_filled(rect, theme::corner(px(7.0)), theme::with_alpha(color, 30));
    if dot {
        painter.circle_filled(
            Pos2::new(rect.left() + px(9.5), rect.center().y),
            px(2.8),
            color,
        );
    }
    painter.galley(
        Pos2::new(rect.left() + lead, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    rect
}

pub fn tag(painter: &egui::Painter, pos: Pos2, align: Align2, text: &str, s: f32) -> Rect {
    let px = |v: f32| theme::px(v, s);
    let font = FontId::new(theme::text_px(12.0, s), theme::family_semibold());
    let galley = painter.layout_no_wrap(text.to_owned(), font, theme::INK_DIM);
    let size = Vec2::new(galley.size().x + px(18.0), px(23.0));
    let rect = align.anchor_size(pos, size);
    painter.rect_filled(rect, theme::corner(px(8.0)), theme::FIELD_HI);
    painter.galley(
        rect.center() - galley.size() / 2.0,
        galley,
        theme::INK_DIM,
    );
    rect
}

pub fn monogram(painter: &egui::Painter, rect: Rect, text: &str, s: f32) {
    const TILES: [(egui::Color32, egui::Color32); 5] = [
        (egui::Color32::from_rgb(0xE8, 0xCF, 0x4A), egui::Color32::from_rgb(0x2A, 0x24, 0x10)),
        (egui::Color32::from_rgb(0x5C, 0x4F, 0x86), egui::Color32::from_rgb(0xEC, 0xE8, 0xF8)),
        (egui::Color32::from_rgb(0x2F, 0x6B, 0x5B), egui::Color32::from_rgb(0xE6, 0xF5, 0xEF)),
        (egui::Color32::from_rgb(0x3D, 0x47, 0x5E), egui::Color32::from_rgb(0xE4, 0xE8, 0xF1)),
        (egui::Color32::from_rgb(0x8A, 0x4B, 0x3A), egui::Color32::from_rgb(0xFA, 0xEB, 0xE6)),
    ];
    let hash = text.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    let (fill, ink) = TILES[hash as usize % TILES.len()];
    let _ = s;
    painter.rect_filled(rect, theme::corner(rect.height() * 0.23), fill);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::new(rect.height() * 0.36, theme::family_bold()),
        ink,
    );
}

pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().filter(|w| w.chars().next().is_some_and(char::is_alphanumeric)).collect();
    let out: String = match words.len() {
        0 => "?".into(),
        1 => words[0].chars().take(2).collect(),
        _ => [words[0], words[words.len() - 1]]
            .iter()
            .filter_map(|w| w.chars().next())
            .collect(),
    };
    out.to_uppercase()
}

pub fn header_buttons(
    ui: &mut egui::Ui,
    header: Rect,
    buttons: &[(&str, char, ButtonKind, bool)],
    s: f32,
) -> Option<usize> {
    let px = |v: f32| theme::px(v, s);
    let font = FontId::new(theme::text_px(13.0, s), theme::family_medium());
    let mut right = header.right();
    let mut clicked = None;
    for (i, (label, glyph, kind, enabled)) in buttons.iter().enumerate().rev() {
        let w = ui
            .painter()
            .layout_no_wrap((*label).to_owned(), font.clone(), theme::INK)
            .size()
            .x
            + px(14.0 + 6.0 + 40.0);
        let r = Rect::from_min_max(
            Pos2::new(right - w, header.center().y - px(18.5)),
            Pos2::new(right, header.center().y + px(18.5)),
        );
        if Button::new(label, *kind)
            .icon(*glyph)
            .enabled(*enabled)
            .show(ui, r, s)
            .clicked()
            && *enabled
        {
            clicked = Some(i);
        }
        right = r.left() - px(14.0);
    }
    clicked
}

/// Screen-reader name and role plus the shared keyboard-focus outline for a
/// hand-drawn click target. Call it right after `interact`/`allocate_*`. The
/// outline sits just outside `rect`, so a fill the caller paints afterwards
/// cannot cover it. Pass `selected` for tabs, rows and other toggled items.
pub fn accessible(
    resp: &egui::Response,
    rect: Rect,
    role: egui::Role,
    label: &str,
    selected: Option<bool>,
    radius: f32,
) {
    let enabled = resp.enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(role, enabled, label));
    if let Some(on) = selected {
        resp.ctx.accesskit_node_builder(resp.id, |node| node.set_selected(on));
    }
    if resp.has_focus() {
        let grow = 2.0;
        // Clip to what is actually visible of the target (scroll areas, panels).
        let clip = resp.interact_rect.expand(grow + 1.0);
        resp.ctx.layer_painter(resp.layer_id).with_clip_rect(clip).rect_stroke(
            rect.expand(grow),
            theme::corner(radius + grow),
            egui::Stroke::new(1.5, theme::with_alpha(theme::FOCUS, 220)),
            egui::StrokeKind::Inside,
        );
    }
}

/// Exposes hand-painted text to screen readers as a read-only node over `rect`.
/// Painted text is otherwise invisible to assistive tech. `live` marks status
/// text whose changes should be announced when they happen.
pub fn readable(ui: &egui::Ui, rect: Rect, role: egui::Role, text: &str, live: bool) {
    let id = ui.make_persistent_id(("readable", rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(role, true, text));
    if live {
        ui.ctx()
            .accesskit_node_builder(id, |node| node.set_live(egui::accesskit::Live::Polite));
    }
}

/// Single-line text limited to `max_w`: it ends in "…" instead of being cut
/// mid-glyph by a clip rect. Returns the drawn rect and whether it was shortened,
/// so callers can offer the full value on hover.
pub fn fit_text(
    painter: &egui::Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    font: FontId,
    color: egui::Color32,
    max_w: f32,
) -> (Rect, bool) {
    let shown = input::head_fit(painter, text, &font, max_w.max(0.0));
    let cut = shown != text;
    (painter.text(pos, align, shown, font, color), cut)
}

pub fn link(ui: &mut egui::Ui, pos: Pos2, align: Align2, text: &str, color: egui::Color32, s: f32) -> egui::Response {
    let font = FontId::new(theme::text_px(12.5, s), theme::family_semibold());
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let rect = align.anchor_size(pos, galley.size());
    let resp = ui
        .interact(rect.expand(theme::px(4.0, s)), ui.make_persistent_id(("link", text, rect.left() as i32, rect.top() as i32)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    accessible(&resp, rect.expand(theme::px(4.0, s)), egui::Role::Link, text, None, theme::px(4.0, s));
    let c = if resp.hovered() { theme::lerp_color(color, theme::INK, 0.4) } else { color };
    ui.painter().galley(rect.min, galley, c);
    resp
}
