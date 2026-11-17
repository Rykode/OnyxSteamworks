use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

use super::editor::row;
use crate::app::App;
use crate::features::checklist;
use crate::model::ChecklistItem;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::theme;

pub(super) fn card(app: &mut App, ui: &mut egui::Ui, card: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::card_title(ui, card, "Release checklist", s);
    let items = &app.projects_ui.base.checklist;
    let done = items.iter().filter(|i| i.done).count();
    let count = kit::card_note(
        ui.painter(),
        card,
        &format!("{done} / {}", items.len()),
        s,
    );
    let editing = app.projects_ui.checklist_edit;
    if kit::link(
        ui,
        Pos2::new(count.left() - px(14.0), count.center().y),
        Align2::RIGHT_CENTER,
        if editing { "Done" } else { "Edit" },
        if editing { theme::ACCENT_HI } else { theme::INK_FAINT },
        s,
    )
    .clicked()
    {
        app.projects_ui.checklist_edit = !editing;
        app.projects_ui
            .base
            .checklist
            .retain(|i| !i.text.trim().is_empty());
    }

    let area = Rect::from_min_max(
        Pos2::new(card.left() + px(21.0), card.top() + px(44.0)),
        Pos2::new(card.right() - px(16.0), card.bottom() - px(10.0)),
    );
    let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(area));
    egui::ScrollArea::vertical()
        .id_salt("checklist_card")
        .auto_shrink([false, false])
        .show(&mut cui, |ui| {
            ui.set_width(area.width() - px(6.0));
            if editing {
                edit_list(app, ui, s);
            } else {
                view_list(app, ui, s);
            }
        });
}

fn view_list(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    if app.projects_ui.base.checklist.is_empty() {
        let r = row(ui, s, 34.0);
        ui.painter().text(
            Pos2::new(r.left(), r.center().y),
            Align2::LEFT_CENTER,
            "No items yet.",
            FontId::new(theme::text_px(13.0, s), theme::family_medium()),
            theme::INK_FAINT,
        );
        let r = row(ui, s, 24.0);
        if kit::link(
            ui,
            Pos2::new(r.left(), r.center().y),
            Align2::LEFT_CENTER,
            "Load the default template",
            theme::ACCENT_HI,
            s,
        )
        .clicked()
        {
            app.projects_ui.base.checklist = checklist::template();
        }
        return;
    }
    ui.add_space(px(1.0));
    for i in 0..app.projects_ui.base.checklist.len() {
        let r = row(ui, s, 33.0);
        let item = &mut app.projects_ui.base.checklist[i];
        let hit = Rect::from_min_max(r.min, Pos2::new(r.right(), r.bottom()));
        let mut done = item.done;
        if kit::checkbox(ui, hit, &mut done, &item.text, s).changed() {
            item.done = done;
        }
        let color = if item.done { theme::INK_MUTED } else { theme::INK_DIM };
        let text_rect = ui.painter().text(
            Pos2::new(r.left() + px(29.0), r.center().y),
            Align2::LEFT_CENTER,
            &item.text,
            FontId::new(theme::text_px(14.0, s), theme::family_medium()),
            color,
        );
        if item.done {
            ui.painter().line_segment(
                [
                    Pos2::new(text_rect.left(), text_rect.center().y + px(1.0)),
                    Pos2::new(text_rect.right(), text_rect.center().y + px(1.0)),
                ],
                Stroke::new(px(1.0), color),
            );
        }
    }
}

fn edit_list(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let mut remove = None;
    for i in 0..app.projects_ui.base.checklist.len() {
        let r = row(ui, s, 40.0);
        let field = Rect::from_min_size(r.min, Vec2::new(r.width() - px(40.0), px(34.0)));
        Input::new(&mut app.projects_ui.base.checklist[i].text)
            .hint("Checklist item")
            .show(ui, field, s);
        let xz = Rect::from_min_size(
            Pos2::new(field.right() + px(6.0), field.top() + px(2.0)),
            Vec2::splat(px(30.0)),
        );
        if kit::icon_button(ui, xz, theme::icon::TRASH, "Remove checklist item", s).clicked() {
            remove = Some(i);
        }
    }
    if let Some(i) = remove
        && i < app.projects_ui.base.checklist.len()
    {
        let item = app.projects_ui.base.checklist.remove(i);
        let owner = app.projects_ui.selected.clone();
        app.offer_undo("Removed checklist item", move |app| {
            if app.projects_ui.selected != owner {
                return Err("Can't undo: a different project is open".to_string());
            }
            let list = &mut app.projects_ui.base.checklist;
            list.insert(i.min(list.len()), item);
            Ok(())
        });
    }
    ui.add_space(px(4.0));
    let r = row(ui, s, 34.0);
    let half = (r.width() - px(8.0)) / 2.0;
    if Button::new("Add item", ButtonKind::Secondary)
        .icon(theme::icon::PLUS)
        .show(ui, Rect::from_min_size(r.min, Vec2::new(half, r.height())), s)
        .clicked()
    {
        app.projects_ui
            .base
            .checklist
            .push(ChecklistItem::default());
    }
    if Button::new("Load template", ButtonKind::Secondary)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(r.left() + half + px(8.0), r.top()), Vec2::new(half, r.height())),
            s,
        )
        .clicked()
    {
        app.projects_ui.base.checklist = checklist::template();
    }
    ui.add_space(px(12.0));
    let tr = row(ui, s, 26.0);
    let tog = Rect::from_min_size(tr.min, Vec2::new(px(40.0), px(22.0)));
    let mut cfg = crate::config::load_settings().unwrap_or_default();
    let mut reset_on = cfg.checklist_reset_on_live;
    if kit::toggle(ui, tog, &mut reset_on, "Reset after each Live build", s).changed() {
        cfg.checklist_reset_on_live = reset_on;
        if let Err(e) = crate::config::save_settings(&cfg) {
            crate::config::report_problem(format!("Could not save settings: {e}"));
        }
    }
    ui.painter().text(
        Pos2::new(tog.right() + px(10.0), tog.center().y),
        Align2::LEFT_CENTER,
        "Reset after each Live build",
        FontId::new(theme::text_px(12.5, s), theme::family_medium()),
        theme::INK_DIM,
    );
}
