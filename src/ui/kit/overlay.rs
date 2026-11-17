use eframe::egui::{
    self, Align2, Color32, FontId, Key, Order, Pos2, Rect, Sense, Stroke, StrokeKind, UiBuilder,
    Vec2,
};

use crate::ui::{anim, theme};

pub(crate) fn should_dismiss(
    dismissible: bool,
    clicked: bool,
    panel: Rect,
    pointer: Option<Pos2>,
) -> bool {
    dismissible && clicked && pointer.is_some_and(|p| !panel.contains(p))
}

fn modal_marker() -> egui::Id {
    egui::Id::unique("kit_modal_open_frame")
}

pub fn modal_active(ctx: &egui::Context) -> bool {
    let now = ctx.cumulative_frame_nr();
    ctx.data(|d| d.get_temp::<u64>(modal_marker()))
        .is_some_and(|f| f + 1 >= now)
}

pub fn modal(
    ui: &mut egui::Ui,
    id: egui::Id,
    open: &mut bool,
    size: Vec2,
    s: f32,
    add: impl FnOnce(&mut egui::Ui) -> bool,
) -> egui::Response {
    modal_with(ui, id, open, size, s, true, add)
}

pub fn modal_with(
    ui: &mut egui::Ui,
    id: egui::Id,
    open: &mut bool,
    size: Vec2,
    s: f32,
    dismissible: bool,
    add: impl FnOnce(&mut egui::Ui) -> bool,
) -> egui::Response {
    let ctx = ui.ctx().clone();
    if *open {
        let now = ctx.cumulative_frame_nr();
        ctx.data_mut(|d| d.insert_temp(modal_marker(), now));
    }
    if *open && dismissible && ctx.input(|i| i.key_pressed(Key::Escape)) {
        *open = false;
    }
    let t = anim::anim_bool(&ctx, id.with("m"), *open, 0.2, anim::ease_out_back);
    if !*open && t <= 0.001 {
        return ui.interact(Rect::ZERO, id.with("closed"), Sense::hover());
    }
    let screen = ctx.content_rect();
    let area = egui::Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(screen.min)
        .show(&ctx, |ui| {
            let scale = 0.94 + 0.06 * t;
            let prect = Rect::from_center_size(screen.center(), size * scale);
            let dim = theme::with_alpha(Color32::BLACK, (170.0 * t.clamp(0.0, 1.0)) as u8);
            ui.painter().rect_filled(screen, theme::corner(0.0), dim);
            let clicked = ui
                .interact(screen, id.with("dim"), Sense::CLICK)
                .clicked();
            let pointer = ui.input(|i| i.pointer.interact_pos());
            if should_dismiss(dismissible, clicked, prect, pointer) {
                *open = false;
            }
            ui.expand_to_include_rect(screen);
            let r = theme::px(theme::RADIUS_PANEL, s);
            theme::shadow(
                ui.painter(),
                prect,
                r,
                40,
                (200.0 * t.clamp(0.0, 1.0)) as u8,
            );
            ui.painter()
                .rect_filled(prect, theme::corner(r), theme::SURFACE_2);
            ui.painter().rect_stroke(
                prect,
                theme::corner(r),
                Stroke::new(1.0, theme::LINE),
                StrokeKind::Inside,
            );
            let mut cu = ui.new_child(UiBuilder::new().max_rect(prect));
            cu.set_opacity(t.clamp(0.0, 1.0));
            if add(&mut cu) {
                *open = false;
            }
        });
    area.response
}

pub fn tooltip(response: egui::Response, text: &str) -> egui::Response {
    let text = text.to_owned();
    response.on_hover_ui(|ui| {
        ui.colored_label(theme::INK, text);
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Success,
    Warning,
    Error,
}

impl ToastLevel {
    fn color(self) -> Color32 {
        match self {
            ToastLevel::Success => theme::SUCCESS,
            ToastLevel::Warning => theme::WARNING,
            ToastLevel::Error => theme::DANGER,
        }
    }
}

const MAX_TOASTS: usize = 5;

struct Toast {
    level: ToastLevel,
    text: String,
    born: f64,
    /// Optional inline action (label, id). Clicking it reports the id via `take_actions`.
    action: Option<(String, u64)>,
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
    next_action: u64,
    clicked: Vec<u64>,
}

impl Toasts {
    pub fn push(&mut self, level: ToastLevel, text: impl Into<String>) {
        let text = text.into();
        if level == ToastLevel::Error
            && self
                .items
                .iter()
                .any(|t| t.level == ToastLevel::Error && t.text == text)
        {
            return;
        }
        self.items.push(Toast {
            level,
            text,
            born: -1.0,
            action: None,
        });
        self.trim();
    }

    /// A toast with a button (e.g. "Undo"). Returns the id `take_actions` reports on click.
    /// Action toasts live longer than plain ones so the button can be reached.
    pub fn push_action(
        &mut self,
        level: ToastLevel,
        text: impl Into<String>,
        label: impl Into<String>,
    ) -> u64 {
        self.next_action += 1;
        let id = self.next_action;
        self.items.push(Toast {
            level,
            text: text.into(),
            born: -1.0,
            action: Some((label.into(), id)),
        });
        self.trim();
        id
    }

    /// Ids of action buttons clicked since the last call.
    pub fn take_actions(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.clicked)
    }

    /// Ids of action toasts still on screen; anything else has expired.
    pub fn live_actions(&self) -> Vec<u64> {
        self.items
            .iter()
            .filter_map(|t| t.action.as_ref().map(|(_, id)| *id))
            .collect()
    }

    fn trim(&mut self) {
        if self.items.len() > MAX_TOASTS {
            let drop = self.items.len() - MAX_TOASTS;
            self.items.drain(..drop);
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, anchor: Pos2, s: f32) -> egui::Response {
        let px = |v: f32| theme::px(v, s);
        let ttl_of = |t: &Toast| if t.action.is_some() { 8.0f64 } else { 3.5f64 };
        let now = ui.input(|i| i.time);
        for t in &mut self.items {
            if t.born < 0.0 {
                t.born = now;
            }
        }
        self.items
            .retain(|t| t.level == ToastLevel::Error || now - t.born < ttl_of(t));
        let ctx = ui.ctx().clone();
        let animating = self
            .items
            .iter()
            .any(|t| t.level != ToastLevel::Error || now - t.born < 0.3);
        if animating {
            ctx.request_repaint();
        }
        let w = px(288.0);
        let gap = px(10.0);
        let radius = px(theme::RADIUS_POPUP);
        let mut dismiss: Option<usize> = None;
        let mut acted: Option<(usize, u64)> = None;
        let items = &self.items;
        let area = egui::Area::new(ui.make_persistent_id("toasts"))
            .order(Order::Foreground)
            .fixed_pos(Pos2::new(anchor.x - w, anchor.y))
            .show(&ctx, |ui| {
                let mut y = anchor.y;
                for (i, t) in items.iter().enumerate() {
                    let sticky = t.level == ToastLevel::Error;
                    let ttl = ttl_of(t);
                    let action_w = t.action.as_ref().map_or(0.0, |_| px(72.0));
                    let galley = ui.painter().layout(
                        t.text.clone(),
                        FontId::new(theme::text_px(13.0, s), theme::family_medium()),
                        theme::INK,
                        w - px(52.0) - action_w,
                    );
                    let h = (galley.size().y + px(24.0)).max(px(46.0));
                    let age = (now - t.born) as f32;
                    let slide = anim::ease_out_cubic((age / 0.25).min(1.0));
                    let x = anchor.x - w + (1.0 - slide) * px(40.0);
                    let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                    let resp = ui.interact(r, ui.make_persistent_id(("toast", i)), Sense::click());
                    super::accessible(&resp, r, egui::Role::Button, &format!("Dismiss: {}", t.text), None, radius);
                    ui.ctx().accesskit_node_builder(resp.id, |node| {
                        node.set_live(if sticky {
                            egui::accesskit::Live::Assertive
                        } else {
                            egui::accesskit::Live::Polite
                        })
                    });
                    if resp.clicked() {
                        dismiss = Some(i);
                    }
                    if sticky {
                        resp.on_hover_text("Click to dismiss");
                    }
                    theme::shadow(ui.painter(), r, radius, 20, 150);
                    ui.painter()
                        .rect_filled(r, theme::corner(radius), theme::FIELD);
                    ui.painter().rect_stroke(
                        r,
                        theme::corner(radius),
                        Stroke::new(1.0, theme::LINE),
                        StrokeKind::Inside,
                    );
                    ui.painter().circle_filled(
                        Pos2::new(r.left() + px(20.0), r.top() + px(23.0)),
                        px(4.0),
                        t.level.color(),
                    );
                    let text_h = galley.size().y;
                    ui.painter().galley(
                        Pos2::new(r.left() + px(34.0), r.center().y - text_h / 2.0),
                        galley,
                        theme::INK,
                    );
                    if let Some((label, id)) = &t.action {
                        let b = Rect::from_min_size(
                            Pos2::new(r.right() - px(12.0) - px(64.0), r.center().y - px(14.0)),
                            Vec2::new(px(64.0), px(28.0)),
                        );
                        if super::Button::new(label, super::ButtonKind::Secondary)
                            .show(ui, b, s)
                            .clicked()
                        {
                            acted = Some((i, *id));
                        }
                    }
                    if !sticky {
                        let remaining = 1.0 - (age / ttl as f32).clamp(0.0, 1.0);
                        let lw = (w - px(28.0)) * remaining;
                        ui.painter().rect_filled(
                            Rect::from_min_size(
                                Pos2::new(r.left() + px(14.0), r.bottom() - px(6.0)),
                                Vec2::new(lw, px(2.0)),
                            ),
                            theme::corner(px(1.0)),
                            theme::with_alpha(t.level.color(), 160),
                        );
                    }
                    y += h + gap;
                }
            });
        if let Some((i, id)) = acted {
            self.clicked.push(id);
            self.items.remove(i);
        } else if let Some(i) = dismiss {
            self.items.remove(i);
        }
        area.response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Rect {
        Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(400.0, 300.0))
    }

    #[test]
    fn clicks_inside_the_panel_do_not_dismiss() {
        assert!(!should_dismiss(
            true,
            true,
            panel(),
            Some(Pos2::new(300.0, 250.0))
        ));
    }

    #[test]
    fn clicks_outside_dismiss_only_when_allowed() {
        let outside = Some(Pos2::new(50.0, 50.0));
        assert!(should_dismiss(true, true, panel(), outside));
        assert!(!should_dismiss(false, true, panel(), outside));
        assert!(!should_dismiss(true, false, panel(), outside));
        assert!(!should_dismiss(true, true, panel(), None));
    }
}
