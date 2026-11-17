mod branches;
mod checklist;
mod depots;
mod editor;
mod io;
mod preview;
mod scan;
mod state;

pub use state::ProjectsState;

use eframe::egui::{self, Align2, FontId, Rect, Sense, Vec2, pos2};

use crate::app::App;
use crate::config;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::{layout::Layout, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let ctx = ui.ctx().clone();
    let s = layout.scale;
    let px = |v: f32| theme::px(v, s);
    app.projects_ui.drain();
    for res in app.projects_ui.io.drain() {
        io::handle(app, res);
    }
    if !app.projects_ui.inited {
        if let Some(p) = app.projects.first().cloned() {
            app.projects_ui.select(&p);
        }
        app.projects_ui.inited = true;
    }

    match kit::header_buttons(
        ui,
        layout.header,
        &[
            ("Import VDF", theme::icon::DOWNLOAD, ButtonKind::Secondary, true),
            ("New project", theme::icon::PLUS, ButtonKind::Primary, true),
        ],
        s,
    ) {
        Some(0) => app.projects_ui.io.spawn_import(&ctx),
        Some(1) => request_switch(app, Switch::New),
        _ => {}
    }

    let body = layout.body;
    if app.projects.is_empty() && app.projects_ui.selected.is_none() && !app.projects_ui.dirty() {
        kit::card(ui.painter(), body, s);
        empty(app, ui, body, s);
        return;
    }

    let gap = px(15.0);
    let list = Rect::from_min_max(body.min, pos2(body.left() + px(311.0), body.bottom()));
    let right = Rect::from_min_max(pos2(list.right() + gap, body.top()), body.max);
    let editor_rect = Rect::from_min_size(right.min, Vec2::new(right.width(), px(285.0)));
    let lower_top = editor_rect.bottom() + gap;
    let depots_w = ((right.width() - gap) * 0.556).round();
    let depots_rect = Rect::from_min_max(
        pos2(right.left(), lower_top),
        pos2(right.left() + depots_w, right.bottom()),
    );
    let checklist_rect = Rect::from_min_max(pos2(depots_rect.right() + gap, lower_top), right.max);
    for r in [list, editor_rect, depots_rect, checklist_rect] {
        kit::card(ui.painter(), r, s);
    }

    left_panel(app, ui, list, s);
    let save = editor::show(app, ui, editor_rect, s, &ctx);
    let ctrl_s = !kit::modal_active(&ctx)
        && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S));
    if save || (ctrl_s && app.projects_ui.valid() && app.projects_ui.dirty()) {
        save_current(app);
    }
    depots::card(app, ui, depots_rect, s, &ctx);
    checklist::card(app, ui, checklist_rect, s);
    depots::modals(app, ui, s, &ctx);
    preview::modal(app, ui, s, &ctx);

    if app.projects_ui.confirm_delete {
        confirm_modal(app, ui, s);
    }
}

pub(crate) fn level_color(level: crate::model::BranchLevel) -> egui::Color32 {
    branches::level_style(level).1
}

pub(super) fn reload(app: &mut App) {
    match config::list_projects() {
        Ok(projects) => app.projects = projects,
        Err(e) => app.toasts.push(
            kit::ToastLevel::Error,
            format!("Could not load projects: {e}"),
        ),
    }
}

#[derive(Clone)]
pub enum Switch {
    New,
    Duplicate,
    Select(Box<crate::model::Project>),
}

fn perform_switch(app: &mut App, switch: Switch) {
    match switch {
        Switch::New => app.projects_ui.start_new(),
        Switch::Duplicate => duplicate_current(app),
        Switch::Select(p) => app.projects_ui.select(&p),
    }
}

fn request_switch(app: &mut App, switch: Switch) {
    if app.projects_ui.unsaved() {
        app.projects_ui.pending_switch = Some(switch);
    } else {
        perform_switch(app, switch);
    }
}

fn confirm_switch(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let Some(switch) = app.projects_ui.pending_switch.clone() else {
        return;
    };
    let spec = kit::Confirm::danger(
        "Unsaved changes",
        "Discard your unsaved project changes?",
        "The edits to the current project have not been saved and will be lost.",
        "Discard",
    );
    match kit::confirm(
        ui,
        ui.make_persistent_id("project_discard_confirm"),
        &spec,
        s,
    ) {
        kit::Choice::Confirmed => {
            app.projects_ui.pending_switch = None;
            perform_switch(app, switch);
        }
        kit::Choice::Cancelled => app.projects_ui.pending_switch = None,
        kit::Choice::Pending => {}
    }
}

fn save_current(app: &mut App) {
    let project = app.projects_ui.project();
    match config::save_project(&project) {
        Ok(()) => {
            reload(app);
            app.projects_ui.mark_saved(project);
            app.toasts.push(kit::ToastLevel::Success, "Project saved");
        }
        Err(e) => {
            crate::config::report_problem(format!("Save failed: {e}"));
            app.toasts.push(kit::ToastLevel::Error, "Save failed");
        }
    }
}

fn empty(app: &mut App, ui: &mut egui::Ui, content: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let c = content.center();
    ui.painter().text(
        pos2(c.x, c.y - px(28.0)),
        Align2::CENTER_CENTER,
        "No projects yet",
        FontId::new(theme::text_px(16.0, s), theme::family_semibold()),
        theme::INK_DIM,
    );
    ui.painter().text(
        pos2(c.x, c.y + px(2.0)),
        Align2::CENTER_CENTER,
        "Create one to generate VDFs and run builds.",
        FontId::new(theme::text_px(13.0, s), theme::family_medium()),
        theme::INK_MUTED,
    );
    let btn = Rect::from_center_size(pos2(c.x, c.y + px(56.0)), Vec2::new(px(188.0), px(44.0)));
    if Button::new("New project", ButtonKind::Primary)
        .icon(theme::icon::PLUS)
        .show(ui, btn, s)
        .clicked()
    {
        app.projects_ui.start_new();
    }
}

pub(super) enum Action {
    Duplicate,
    Delete,
    Select(usize),
    Export,
    Context(usize, Box<Action>),
}

fn left_panel(app: &mut App, ui: &mut egui::Ui, card: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let search = Rect::from_min_size(
        pos2(card.left() + px(17.0), card.top() + px(17.0)),
        Vec2::new(card.width() - px(34.0), px(33.0)),
    );
    Input::new(&mut app.projects_ui.search)
        .hint("Search projects")
        .icon(theme::icon::MAGNIFYING_GLASS)
        .show(ui, search, s);

    let query = app.projects_ui.search.trim().to_lowercase();
    let mut action = None;
    let list = Rect::from_min_max(
        pos2(card.left() + px(13.0), search.bottom() + px(10.0)),
        pos2(card.right() - px(13.0), card.bottom() - px(12.0)),
    );
    let selected = app.projects_ui.selected.clone();
    let matches: Vec<usize> = app
        .projects
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            query.is_empty() || p.name.to_lowercase().contains(&query) || p.app_id.contains(&query)
        })
        .map(|(i, _)| i)
        .collect();
    if matches.is_empty() {
        ui.painter().text(
            pos2(list.center().x, list.top() + px(30.0)),
            Align2::CENTER_CENTER,
            "No matching projects",
            FontId::new(theme::text_px(13.0, s), theme::family_medium()),
            theme::INK_FAINT,
        );
    }
    for i in &matches {
        let p = &app.projects[*i];
        if !app.projects_ui.engines.contains_key(&p.id) {
            let engine = guess_engine(p);
            app.projects_ui.engines.insert(p.id.clone(), engine);
        }
    }
    let row_h = px(67.0);
    let pitch = px(68.5);
    let mut lui = ui.new_child(egui::UiBuilder::new().max_rect(list));
    egui::ScrollArea::vertical()
        .id_salt("project_list")
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(&mut lui, |ui| {
            let (area, _) = ui.allocate_exact_size(
                Vec2::new(list.width(), pitch * matches.len() as f32),
                Sense::hover(),
            );
            for (k, i) in matches.iter().copied().enumerate() {
                let p = &app.projects[i];
                let r = Rect::from_min_size(
                    pos2(area.left(), area.top() + pitch * k as f32),
                    Vec2::new(area.width(), row_h),
                );
                let is_sel = selected.as_deref() == Some(p.id.as_str());
                let resp = ui.interact(r, ui.make_persistent_id(("proj_row", i)), Sense::click());
                crate::ui::kit::accessible(&resp, r, egui::Role::Button, if p.name.is_empty() { "Untitled project" } else { p.name.as_str() }, Some(is_sel), px(theme::RADIUS_CARD));
                let hover = ui.ctx().animate_bool_with_time(
                    ui.make_persistent_id(("proj_hover", i)),
                    resp.hovered(),
                    0.12,
                );
                if is_sel {
                    kit::bordered(ui.painter(), r, theme::SURFACE_2, theme::LINE, px(12.0));
                } else if hover > 0.01 {
                    ui.painter().rect_filled(
                        r,
                        theme::corner(px(12.0)),
                        theme::with_alpha(theme::SURFACE_2, (hover * 200.0) as u8),
                    );
                }
                let name = if p.name.is_empty() {
                    "Untitled"
                } else {
                    p.name.as_str()
                };
                let tile = Rect::from_center_size(
                    pos2(r.left() + px(33.5), r.center().y),
                    Vec2::splat(px(43.5)),
                );
                kit::monogram(ui.painter(), tile, &kit::initials(name), s);
                let engine = app.projects_ui.engines.get(&p.id).copied().flatten();
                let tag_left = match engine {
                    Some(label) => {
                        kit::tag(
                            ui.painter(),
                            pos2(r.right() - px(12.0), r.center().y),
                            Align2::RIGHT_CENTER,
                            label,
                            s,
                        )
                        .left()
                    }
                    None => r.right() - px(4.0),
                };
                let text_x = r.left() + px(67.5);
                let clip = Rect::from_min_max(pos2(text_x, r.top()), pos2(tag_left - px(8.0), r.bottom()));
                kit::fit_text(
                    ui.painter(),
                    pos2(text_x, r.center().y - px(9.5)),
                    Align2::LEFT_CENTER,
                    &name,
                    FontId::new(theme::text_px(15.0, s), theme::family_semibold()),
                    theme::INK,
                    clip.width(),
                );
                let sub = if p.app_id.is_empty() {
                    "No app ID".to_string()
                } else {
                    p.app_id.clone()
                };
                kit::fit_text(
                    ui.painter(),
                    pos2(text_x, r.center().y + px(10.5)),
                    Align2::LEFT_CENTER,
                    &sub,
                    FontId::new(theme::text_px(12.0, s), egui::FontFamily::Monospace),
                    theme::INK_MUTED,
                    clip.width(),
                );
                if resp.clicked() {
                    action = Some(Action::Select(i));
                }
                let app_id = p.app_id.clone();
                resp.context_menu(|ui| {
                    if ui.button("Duplicate").clicked() {
                        action = Some(Action::Context(i, Box::new(Action::Duplicate)));
                    }
                    if ui.button("Export JSON").clicked() {
                        action = Some(Action::Context(i, Box::new(Action::Export)));
                    }
                    if !app_id.is_empty() {
                        ui.separator();
                        if ui.button("Open in Steamworks").clicked() {
                            let url = crate::features::quick::steamworks_builds_url(&app_id);
                            std::thread::spawn(move || {
                                let _ = open::that(url);
                            });
                        }
                        if ui.button("Launch via Steam").clicked() {
                            let url = crate::features::quick::steam_run_url(&app_id);
                            std::thread::spawn(move || {
                                let _ = open::that(url);
                            });
                        }
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() {
                        action = Some(Action::Context(i, Box::new(Action::Delete)));
                    }
                });
            }
        });

    let action = match action {
        Some(Action::Context(i, inner)) => {
            let p = app.projects[i].clone();
            if app.projects_ui.selected.as_deref() == Some(p.id.as_str()) {
                Some(*inner)
            } else if app.projects_ui.unsaved() {
                request_switch(app, Switch::Select(Box::new(p)));
                None
            } else {
                app.projects_ui.select(&p);
                Some(*inner)
            }
        }
        other => other,
    };
    match action {
        Some(Action::Duplicate) => request_switch(app, Switch::Duplicate),
        Some(Action::Delete) => app.projects_ui.confirm_delete = true,
        Some(Action::Select(i)) => {
            let p = app.projects[i].clone();
            if app.projects_ui.selected.as_deref() != Some(p.id.as_str()) {
                request_switch(app, Switch::Select(Box::new(p)));
            }
        }
        Some(Action::Export) => {
            let project = app.projects_ui.project();
            app.projects_ui.io.spawn_export(project, &ui.ctx().clone());
        }
        _ => {}
    }
    confirm_switch(app, ui, s);
}

fn guess_engine(p: &crate::model::Project) -> Option<&'static str> {
    use crate::features::importer::{Engine, detect};
    if !p.repo_path.as_os_str().is_empty() {
        let engine = detect(&p.repo_path).engine;
        if engine != Engine::Unknown {
            return Some(engine.label());
        }
    }
    let root = if p.content_root.as_os_str().is_empty() {
        let settings = config::load_settings().unwrap_or_default();
        std::path::PathBuf::from(settings.default_content)
    } else {
        p.content_root.clone()
    };
    if root.as_os_str().is_empty() {
        return None;
    }
    for entry in walkdir::WalkDir::new(&root)
        .max_depth(3)
        .into_iter()
        .take(600)
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.ends_with(".pck") || name == "project.godot" {
            return Some(Engine::Godot.label());
        }
        if name == "unityplayer.dll" || (name.ends_with("_data") && entry.file_type().is_dir()) {
            return Some(Engine::Unity.label());
        }
        if name.ends_with(".uproject") || name.ends_with("-shipping.exe") {
            return Some(Engine::Unreal.label());
        }
    }
    None
}

fn duplicate_current(app: &mut App) {
    if let Some(id) = app.projects_ui.selected.clone() {
        match config::duplicate_project(&id) {
            Ok(dup) => {
                reload(app);
                app.projects_ui.select(&dup);
            }
            Err(e) => app
                .toasts
                .push(kit::ToastLevel::Error, format!("Duplicate failed: {e}")),
        }
    }
}

fn confirm_modal(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let name = if app.projects_ui.form.name.is_empty() {
        "this project".to_string()
    } else {
        app.projects_ui.form.name.clone()
    };
    let id = ui.make_persistent_id("delete_modal");
    let mut do_delete = false;
    kit::modal(
        ui,
        id,
        &mut app.projects_ui.confirm_delete,
        Vec2::new(px(390.0), px(188.0)),
        s,
        |ui| {
            let d = ui.max_rect();
            let mut close = false;
            ui.painter().text(
                pos2(d.left() + px(26.0), d.top() + px(28.0)),
                Align2::LEFT_TOP,
                "Delete project",
                FontId::new(theme::text_px(13.0, s), theme::family_label()),
                theme::INK,
            );
            ui.painter().text(
                pos2(d.left() + px(26.0), d.top() + px(64.0)),
                Align2::LEFT_TOP,
                format!("Delete {name}? This cannot be undone."),
                FontId::new(theme::text_px(13.0, s), theme::family_medium()),
                theme::INK_DIM,
            );
            let bw = px(150.0);
            let by = d.bottom() - px(26.0) - px(40.0);
            if Button::new("Cancel", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(pos2(d.left() + px(26.0), by), Vec2::new(bw, px(40.0))),
                    s,
                )
                .clicked()
            {
                close = true;
            }
            if Button::new("Delete", ButtonKind::Danger)
                .icon(theme::icon::TRASH)
                .show(
                    ui,
                    Rect::from_min_size(
                        pos2(d.right() - px(26.0) - bw, by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked()
            {
                do_delete = true;
                close = true;
            }
            close
        },
    );
    if do_delete {
        delete_current(app);
    }
}

fn delete_current(app: &mut App) {
    if let Some(id) = app.projects_ui.selected.clone() {
        if let Err(e) = config::delete_project(&id) {
            crate::config::report_problem(format!("Delete failed: {e}"));
        }
        reload(app);
        app.projects_ui.confirm_delete = false;
        if let Some(p) = app.projects.first().cloned() {
            app.projects_ui.select(&p);
        } else {
            app.projects_ui.clear();
        }
        app.toasts.push(kit::ToastLevel::Success, "Project deleted");
    }
}
