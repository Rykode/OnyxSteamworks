//! The app's one popup menu: placement, hover and keyboard focus, screen-reader
//! names, checkmarks, disabled rows, and closing on Esc or a click elsewhere.

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Vec2};

use crate::ui::theme;

pub struct MenuItem<'a> {
    pub label: &'a str,
    pub icon: Option<char>,
    pub checked: bool,
    pub enabled: bool,
}

impl<'a> MenuItem<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            icon: None,
            checked: false,
            enabled: true,
        }
    }

    pub fn icon(mut self, glyph: char) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

pub enum MenuEvent {
    Picked(usize),
    Closed,
}

/// Draws an open menu for the control at `anchor`. It opens below the anchor, or
/// above it when there is no room. Returns the picked row, `Closed` on Esc or a
/// click outside both the menu and the anchor, or `None` while it stays open.
pub fn menu(
    ui: &egui::Ui,
    id: egui::Id,
    anchor: Rect,
    width: f32,
    title: Option<&str>,
    items: &[MenuItem],
    s: f32,
) -> Option<MenuEvent> {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let row_h = px(32.0);
    let pad = px(6.0);
    let gap = px(6.0);
    let title_h = if title.is_some() { px(26.0) } else { 0.0 };
    let h = pad * 2.0 + title_h + row_h * items.len() as f32;
    let screen = ctx.content_rect();
    let top = if anchor.bottom() + gap + h <= screen.bottom() {
        anchor.bottom() + gap
    } else {
        anchor.top() - gap - h
    };
    let origin = Pos2::new(anchor.left(), top);
    let has_icons = items.iter().any(|i| i.icon.is_some());
    let has_checks = items.iter().any(|i| i.checked);
    let mut event = None;

    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(origin)
        .show(&ctx, |ui| {
            let rect = Rect::from_min_size(origin, Vec2::new(width, h));
            theme::shadow(ui.painter(), rect, px(theme::RADIUS_POPUP), 24, 140);
            super::bordered(ui.painter(), rect, theme::SURFACE_2, theme::LINE, px(theme::RADIUS_POPUP));
            if let Some(title) = title {
                super::label(
                    ui.painter(),
                    Pos2::new(rect.left() + px(14.0), rect.top() + px(10.0)),
                    title,
                    s,
                );
            }
            let mut y = rect.top() + pad + title_h;
            for (i, item) in items.iter().enumerate() {
                let r = Rect::from_min_size(
                    Pos2::new(rect.left() + pad, y),
                    Vec2::new(width - pad * 2.0, row_h),
                );
                let row = ui.interact(r, id.with(("row", i)), Sense::click());
                if item.enabled {
                    super::accessible(
                        &row,
                        r,
                        egui::Role::MenuItem,
                        item.label,
                        has_checks.then_some(item.checked),
                        px(theme::RADIUS_FIELD),
                    );
                    if row.hovered() || row.has_focus() {
                        ui.painter().rect_filled(r, theme::corner(px(theme::RADIUS_FIELD)), theme::FIELD);
                    }
                } else {
                    row.widget_info(|| egui::WidgetInfo::labeled(egui::Role::MenuItem, false, item.label));
                }
                let mut text_x = r.left() + px(10.0);
                if has_icons {
                    if let Some(glyph) = item.icon {
                        ui.painter().text(
                            Pos2::new(text_x, r.center().y),
                            Align2::LEFT_CENTER,
                            glyph.to_string(),
                            theme::icon_font(px(14.0), false),
                            if item.enabled { theme::INK_MUTED } else { theme::INK_FAINT },
                        );
                    }
                    text_x += px(22.0);
                }
                ui.painter().text(
                    Pos2::new(text_x, r.center().y),
                    Align2::LEFT_CENTER,
                    item.label,
                    FontId::new(theme::text_px(12.5, s), theme::family_medium()),
                    if item.enabled { theme::INK } else { theme::INK_FAINT },
                );
                if item.checked {
                    ui.painter().text(
                        Pos2::new(r.right() - px(10.0), r.center().y),
                        Align2::RIGHT_CENTER,
                        theme::icon::CHECK.to_string(),
                        theme::icon_font(px(13.0), false),
                        theme::SUCCESS,
                    );
                }
                if item.enabled && row.clicked() {
                    event = Some(MenuEvent::Picked(i));
                }
                y += row_h;
            }
        });

    if event.is_some() {
        return event;
    }
    let on_anchor = ctx
        .input(|i| i.pointer.interact_pos())
        .is_some_and(|p| anchor.contains(p));
    if (area.response.clicked_elsewhere() && !on_anchor) || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        return Some(MenuEvent::Closed);
    }
    None
}
