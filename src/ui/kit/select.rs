use eframe::egui::{self, Align2, FontId, Key, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::ui::{anim, theme};

const MAX_VISIBLE_ROWS: usize = 8;

pub fn dropdown(
    ui: &mut egui::Ui,
    rect: Rect,
    options: &[&str],
    selected: &mut usize,
    s: f32,
) -> egui::Response {
    let px = |v: f32| theme::px(v, s);
    let radius = px(theme::RADIUS_FIELD);
    let id = ui.next_auto_id().with("dropdown");
    ui.skip_ahead_auto_ids(1);
    let resp = ui.interact(rect, id, Sense::click());
    let current = options.get(*selected).copied().unwrap_or("");
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, current));
    let ctx = ui.ctx().clone();
    let mut open = ctx.data(|d| d.get_temp::<bool>(id.with("open")).unwrap_or(false));
    let mut cursor = ctx
        .data(|d| d.get_temp::<usize>(id.with("cursor")))
        .unwrap_or(*selected);
    let enter_while_open = open && resp.has_focus() && ctx.input(|i| i.key_pressed(Key::Enter));
    if enter_while_open {
        if cursor < options.len() {
            *selected = cursor;
        }
        open = false;
    } else if resp.clicked() {
        open = !open;
        cursor = *selected;
    }
    if resp.has_focus() && !options.is_empty() {
        let (down, up, esc) = ctx.input(|i| {
            (
                i.key_pressed(Key::ArrowDown),
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::Escape),
            )
        });
        if open {
            if down {
                cursor = (cursor + 1).min(options.len() - 1);
            }
            if up {
                cursor = cursor.saturating_sub(1);
            }
            if esc {
                open = false;
            }
        } else if down || up {
            open = true;
            cursor = *selected;
        }
    }
    let hover = anim::anim_bool(
        &ctx,
        id.with("h"),
        resp.hovered(),
        0.12,
        anim::ease_out_cubic,
    );

    let painter = ui.painter();
    painter.rect_filled(
        rect,
        theme::corner(radius),
        theme::lerp_color(theme::FIELD, theme::FIELD_HI, hover),
    );
    painter.rect_stroke(
        rect,
        theme::corner(radius),
        Stroke::new(
            1.0,
            if open {
                theme::with_alpha(theme::ACCENT, 200)
            } else {
                theme::LINE
            },
        ),
        StrokeKind::Inside,
    );
    if resp.has_focus() {
        theme::focus_ring(
            painter,
            rect.expand(2.0),
            radius + 2.0,
            theme::with_alpha(theme::FOCUS, 180),
        );
    }
    painter.text(
        Pos2::new(rect.left() + px(14.0), rect.center().y),
        Align2::LEFT_CENTER,
        current,
        FontId::new(px(13.0), theme::family_medium()),
        theme::INK,
    );
    painter.text(
        Pos2::new(rect.right() - px(16.0), rect.center().y),
        Align2::CENTER_CENTER,
        theme::icon::CARET_DOWN.to_string(),
        theme::icon_font(px(12.0), false),
        theme::INK_MUTED,
    );

    let t = anim::anim_bool(&ctx, id.with("anim"), open, 0.13, anim::ease_out_cubic);
    if t > 0.001 {
        let row_h = px(32.0);
        let visible = options.len().clamp(1, MAX_VISIBLE_ROWS);
        let ph = row_h * visible as f32 + px(10.0);
        let pos = Pos2::new(rect.left(), rect.bottom() + px(6.0) - (1.0 - t) * px(6.0));
        let area = egui::Area::new(id.with("popup"))
            .order(Order::Foreground)
            .fixed_pos(pos)
            .show(&ctx, |ui| {
                ui.set_opacity(t);
                let prect = Rect::from_min_size(pos, Vec2::new(rect.width(), ph));
                theme::shadow(ui.painter(), prect, px(theme::RADIUS_POPUP), 24, 150);
                ui.painter().rect_filled(
                    prect,
                    theme::corner(px(theme::RADIUS_POPUP)),
                    theme::SURFACE_2,
                );
                ui.painter().rect_stroke(
                    prect,
                    theme::corner(px(theme::RADIUS_POPUP)),
                    Stroke::new(1.0, theme::LINE),
                    StrokeKind::Inside,
                );
                let inner = prect.shrink(px(5.0));
                let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                cu.spacing_mut().item_spacing = Vec2::ZERO;
                egui::ScrollArea::vertical()
                    .id_salt(id.with("scroll"))
                    .max_height(inner.height())
                    .auto_shrink([false, true])
                    .show(&mut cu, |ui| {
                        for (i, opt) in options.iter().enumerate() {
                            let (rr, r) = ui.allocate_exact_size(
                                Vec2::new(inner.width(), row_h),
                                Sense::click(),
                            );
                            let hot = r.hovered() || i == cursor;
                            let hov = anim::anim_bool(
                                ui.ctx(),
                                id.with(("rh", i)),
                                hot,
                                0.1,
                                anim::ease_out_cubic,
                            );
                            if i == cursor && open {
                                r.scroll_to_me(None);
                            }
                            ui.painter().rect_filled(
                                rr,
                                theme::corner(px(theme::RADIUS_FIELD) - px(1.0)),
                                theme::lerp_color(theme::SURFACE_2, theme::FIELD_HI, hov),
                            );
                            let on = i == *selected;
                            if on {
                                ui.painter().text(
                                    Pos2::new(rr.right() - px(10.0), rr.center().y),
                                    Align2::RIGHT_CENTER,
                                    theme::icon::CHECK.to_string(),
                                    theme::icon_font(px(12.0), false),
                                    theme::INK_DIM,
                                );
                            }
                            ui.painter().text(
                                Pos2::new(rr.left() + px(10.0), rr.center().y),
                                Align2::LEFT_CENTER,
                                *opt,
                                FontId::new(px(13.0), theme::family_medium()),
                                if on { theme::INK } else { theme::INK_DIM },
                            );
                            if r.clicked() {
                                *selected = i;
                                open = false;
                            }
                        }
                    });
            });
        if area.response.clicked_elsewhere() && !resp.clicked() {
            open = false;
        }
    }
    ctx.data_mut(|d| {
        d.insert_temp(id.with("open"), open);
        d.insert_temp(id.with("cursor"), cursor);
    });
    resp
}

pub fn tabs(
    ui: &mut egui::Ui,
    rect: Rect,
    labels: &[&str],
    active: &mut usize,
    s: f32,
) -> egui::Response {
    let px = |v: f32| theme::px(v, s);
    let n = labels.len().max(1);
    let seg_w = rect.width() / n as f32;
    let id = ui.make_persistent_id(("tabs", rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::hover());

    ui.painter().rect_filled(
        rect,
        theme::corner(px(theme::RADIUS_FIELD) + px(1.0)),
        theme::BG,
    );
    ui.painter().rect_stroke(
        rect,
        theme::corner(px(theme::RADIUS_FIELD) + px(1.0)),
        Stroke::new(1.0, theme::LINE_SOFT),
        StrokeKind::Inside,
    );

    let target = rect.left() + seg_w * (*active as f32);
    let x = ui
        .ctx()
        .animate_value_with_time(id.with("ind"), target, 0.18);
    let ind = Rect::from_min_size(
        Pos2::new(x + px(3.0), rect.top() + px(3.0)),
        Vec2::new(seg_w - px(6.0), rect.height() - px(6.0)),
    );
    ui.painter().rect_filled(
        ind,
        theme::corner(px(theme::RADIUS_FIELD) - px(1.0)),
        theme::FIELD_HI,
    );
    ui.painter().rect_stroke(
        ind,
        theme::corner(px(theme::RADIUS_FIELD) - px(1.0)),
        Stroke::new(1.0, theme::LINE),
        StrokeKind::Inside,
    );

    for (i, lbl) in labels.iter().enumerate() {
        let seg = Rect::from_min_size(
            Pos2::new(rect.left() + seg_w * i as f32, rect.top()),
            Vec2::new(seg_w, rect.height()),
        );
        let r = ui.interact(seg, id.with(i), Sense::click());
        if r.clicked() {
            *active = i;
        }
        let on = i == *active;
        ui.painter().text(
            seg.center(),
            Align2::CENTER_CENTER,
            *lbl,
            FontId::new(px(12.5), theme::family_medium()),
            if on {
                theme::INK
            } else if r.hovered() {
                theme::INK_DIM
            } else {
                theme::INK_MUTED
            },
        );
    }
    resp
}
