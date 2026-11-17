use std::collections::HashMap;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Vec2};

use crate::app::App;
use crate::model::{BranchLevel, BuildStatus};
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

pub(crate) fn level_style(level: BranchLevel) -> (&'static str, Color32) {
    match level {
        BranchLevel::Dev => ("Dev", theme::accent_hi()),
        BranchLevel::Beta => ("Beta", theme::warning()),
        BranchLevel::Live => ("Live", theme::danger()),
    }
}

pub(super) fn latest_builds(app: &mut App) -> HashMap<String, usize> {
    app.history_ui.ensure_loaded();
    let Some(id) = app.projects_ui.selected.clone() else {
        return HashMap::new();
    };
    let mut records: Vec<_> = app
        .history_ui
        .records
        .iter()
        .filter(|r| r.project_id == id)
        .collect();
    records.sort_by_key(|r| r.timestamp);
    let mut out = HashMap::new();
    for (n, r) in records.iter().enumerate() {
        if r.status == BuildStatus::Success {
            out.insert(r.branch.to_ascii_lowercase(), n + 1);
        }
    }
    out
}

pub(super) fn row(
    app: &App,
    ui: &mut egui::Ui,
    r: Rect,
    i: usize,
    builds: &HashMap<String, usize>,
    s: f32,
) -> bool {
    let px = |v: f32| theme::px(v, s);
    let b = &app.projects_ui.base.branches[i];
    let resp = ui
        .interact(r, ui.make_persistent_id(("branch_row", i)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    crate::ui::kit::accessible(&resp, r, egui::Role::Button, &format!("Edit branch {}", b.name), None, px(theme::RADIUS_FIELD));
    let fill = if resp.hovered() {
        theme::field_hi()
    } else {
        theme::field()
    };
    ui.painter().rect_filled(r, theme::corner(px(10.0)), fill);
    let (label, color) = level_style(b.level);
    kit::badge(
        ui.painter(),
        Pos2::new(r.left() + px(12.0), r.center().y),
        Align2::LEFT_CENTER,
        label,
        color,
        true,
        s,
    );
    let name = if b.name.is_empty() { "unnamed" } else { b.name.as_str() };
    ui.painter().text(
        Pos2::new(r.left() + px(79.0), r.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::new(theme::text_px(14.0, s), theme::family_medium()),
        theme::ink(),
    );
    let live = app
        .projects_ui
        .base
        .set_live_branch
        .eq_ignore_ascii_case(&b.name);
    let note = if b.level == BranchLevel::Live {
        "confirm before push".to_string()
    } else if let Some(n) = builds.get(&b.name.to_ascii_lowercase()) {
        format!("build {n}")
    } else if live {
        "set live on upload".to_string()
    } else {
        "no builds yet".to_string()
    };
    ui.painter().text(
        Pos2::new(r.right() - px(15.0), r.center().y),
        Align2::RIGHT_CENTER,
        note,
        FontId::new(theme::text_px(12.5, s), egui::FontFamily::Monospace),
        theme::ink_muted(),
    );
    resp.clicked()
}

pub(super) fn modal(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let Some(i) = app.projects_ui.edit_branch else {
        return;
    };
    if i >= app.projects_ui.base.branches.len() {
        app.projects_ui.edit_branch = None;
        return;
    }
    let mut open = true;
    let mut remove = false;
    kit::modal(
        ui,
        ui.make_persistent_id("branch_modal"),
        &mut open,
        Vec2::new(px(440.0), px(330.0)),
        s,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(26.0);
            let w = d.width() - px(52.0);
            ui.painter().text(
                Pos2::new(x, d.top() + px(30.0)),
                Align2::LEFT_CENTER,
                "Edit branch",
                FontId::new(theme::text_px(15.0, s), theme::family_semibold()),
                theme::ink(),
            );
            kit::label(ui.painter(), Pos2::new(x, d.top() + px(58.0)), "Name", s);
            Input::new(&mut app.projects_ui.base.branches[i].name)
                .hint("branch name")
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, d.top() + px(78.0)), Vec2::new(w, px(38.0))),
                    s,
                );
            kit::label(ui.painter(), Pos2::new(x, d.top() + px(128.0)), "Level", s);
            let mut lvl = match app.projects_ui.base.branches[i].level {
                BranchLevel::Dev => 0,
                BranchLevel::Beta => 1,
                BranchLevel::Live => 2,
            };
            kit::dropdown(
                ui,
                Rect::from_min_size(Pos2::new(x, d.top() + px(148.0)), Vec2::new(w, px(38.0))),
                &["Dev", "Beta", "Live"],
                &mut lvl,
                s,
            );
            app.projects_ui.base.branches[i].level =
                [BranchLevel::Dev, BranchLevel::Beta, BranchLevel::Live][lvl];

            let name = app.projects_ui.base.branches[i].name.clone();
            let mut live = !name.is_empty()
                && app.projects_ui.base.set_live_branch.eq_ignore_ascii_case(&name);
            let tog = Rect::from_min_size(Pos2::new(x, d.top() + px(204.0)), Vec2::new(px(44.0), px(24.0)));
            if kit::toggle(ui, tog, &mut live, "Set this branch live after upload", s).changed() {
                app.projects_ui.base.set_live_branch = if live { name.clone() } else { String::new() };
            }
            let (text, color) = if live && name.eq_ignore_ascii_case("default") {
                ("SteamCMD cannot set the default branch live", theme::danger())
            } else {
                ("Set this branch live after upload", theme::ink_dim())
            };
            ui.painter().text(
                Pos2::new(tog.right() + px(12.0), tog.center().y),
                Align2::LEFT_CENTER,
                text,
                FontId::new(theme::text_px(12.5, s), theme::family_medium()),
                color,
            );

            let by = d.bottom() - px(26.0) - px(38.0);
            if Button::new("Remove", ButtonKind::Danger)
                .icon(theme::icon::TRASH)
                .show(ui, Rect::from_min_size(Pos2::new(x, by), Vec2::new(px(120.0), px(38.0))), s)
                .clicked()
            {
                remove = true;
                return true;
            }
            Button::new("Done", ButtonKind::Primary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x + w - px(120.0), by), Vec2::new(px(120.0), px(38.0))),
                    s,
                )
                .clicked()
        },
    );
    if remove {
        let owner = app.projects_ui.selected.clone();
        if let Some(branch) = app.projects_ui.remove_branch(i) {
            let label = if branch.name.trim().is_empty() { "branch".to_string() } else { branch.name.clone() };
            app.offer_undo(format!("Removed {label}"), move |app| {
                if app.projects_ui.selected != owner {
                    return Err("Can't undo: a different project is open".to_string());
                }
                app.projects_ui.insert_branch(i, branch);
                Ok(())
            });
        }
    }
    if !open || remove {
        app.projects_ui.edit_branch = None;
    }
}
