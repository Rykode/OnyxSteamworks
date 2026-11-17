use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

use crate::features::importer::classify_os;

use super::editor::row;
use super::scan::{ScanState, human_size};
use super::state::PickField;
use crate::app::App;
use crate::model::Depot;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;
use crate::ui::kit;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Preset {
    Windows,
    Linux,
    Macos,
}

fn preset_depot(preset: Preset) -> Depot {
    let (name, exclusions): (&str, Vec<&str>) = match preset {
        Preset::Windows => ("Windows Content", vec![".pdb", ".log", ".git*"]),
        Preset::Linux => ("Linux Content", vec![".log", ".git*"]),
        Preset::Macos => ("macOS Content", vec![".DS_Store", ".log", ".git*"]),
    };
    Depot {
        name: name.to_string(),
        depot_path: ".".to_string(),
        recursive: true,
        exclusions: exclusions.into_iter().map(String::from).collect(),
        ..Default::default()
    }
}

enum DepotAct {
    Move(usize, bool),
    Remove(usize),
    Scan(usize),
    AddExcl(usize),
    RemoveExcl(usize, usize),
}

pub(super) fn card(app: &mut App, ui: &mut egui::Ui, card: Rect, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    kit::card_title(ui, card, "Depots", s);
    let add = kit::link(
        ui,
        Pos2::new(card.right() - px(20.0), card.top() + px(26.0)),
        Align2::RIGHT_CENTER,
        "+ Add depot",
        theme::ink_faint(),
        s,
    );
    egui::Popup::menu(&add).show(|ui| {
        for (lbl, preset) in [
            ("Windows", Some(Preset::Windows)),
            ("Linux", Some(Preset::Linux)),
            ("macOS", Some(Preset::Macos)),
            ("Empty", None),
        ] {
            if ui.button(lbl).clicked() {
                app.projects_ui
                    .add_depot(preset.map(preset_depot).unwrap_or_default());
                app.projects_ui.edit_depot = Some(app.projects_ui.base.depots.len() - 1);
            }
        }
    });

    let area = Rect::from_min_max(
        Pos2::new(card.left(), card.top() + px(44.0)),
        Pos2::new(card.right(), card.bottom() - px(8.0)),
    );
    let n_depots = app.projects_ui.base.depots.len();
    let n_branches = app.projects_ui.base.branches.len();
    const ROW: f32 = 46.7;
    let table_h = 32.0 + ROW * n_depots.max(1) as f32;
    let branches_h = 50.0 + 52.0 * n_branches.max(1) as f32;
    let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(area));
    egui::ScrollArea::vertical()
        .id_salt("depots_card")
        .auto_shrink([false, false])
        .show(&mut cui, |ui| {
            let (r, _) = ui.allocate_exact_size(
                Vec2::new(area.width(), px(table_h + branches_h)),
                egui::Sense::hover(),
            );
            let top = r.top() - px(44.0);
            let x0 = r.left() + px(21.0);
            let x1 = r.right() - px(21.0);
            let cols = [25.0 / 488.0, 137.0 / 488.0, 243.0 / 488.0, 369.0 / 488.0];
            let cx = |i: usize| r.left() + r.width() * cols[i];
            for (i, title) in ["Depot", "Path", "Target", "Ignore"].iter().enumerate() {
                kit::caps(
                    ui.painter(),
                    Pos2::new(cx(i), top + px(57.0)),
                    Align2::LEFT_CENTER,
                    title,
                    11.5,
                    theme::ink_faint(),
                    s,
                );
            }
            let sep = |y: f32| {
                ui.painter().line_segment(
                    [Pos2::new(x0, y), Pos2::new(x1, y)],
                    Stroke::new(1.0, theme::line_soft()),
                )
            };
            let mut y = top + px(76.0);
            sep(y);
            if n_depots == 0 {
                ui.painter().text(
                    Pos2::new(cx(0), y + px(ROW / 2.0)),
                    Align2::LEFT_CENTER,
                    "No depots yet. Add one above.",
                    FontId::new(theme::text_px(13.0, s), theme::family_medium()),
                    theme::ink_faint(),
                );
                y += px(ROW);
            }
            let root = crate::features::maintenance::resolve(&app.projects_ui.project()).content_root;
            for i in 0..n_depots {
                let row = Rect::from_min_max(Pos2::new(x0 - px(6.0), y), Pos2::new(x1 + px(6.0), y + px(ROW)));
                let resp = ui
                    .interact(row, ui.make_persistent_id(("depot_row", i)), egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                crate::ui::kit::accessible(&resp, row, egui::Role::Button, &format!("Edit depot {}", app.projects_ui.base.depots[i].depot_id), None, px(theme::RADIUS_FIELD));
                if resp.hovered() {
                    ui.painter().rect_filled(
                        row.shrink2(Vec2::new(0.0, px(3.0))),
                        theme::corner(px(8.0)),
                        theme::with_alpha(theme::surface_2(), 200),
                    );
                }
                if resp.clicked() {
                    app.projects_ui.edit_depot = Some(i);
                }
                let d = &app.projects_ui.base.depots[i];
                let cy = row.center().y;
                let mono = |size: f32| FontId::new(theme::text_px(size, s), egui::FontFamily::Monospace);
                let cell = |i: usize| {
                    Rect::from_min_max(
                        Pos2::new(cx(i), row.top()),
                        Pos2::new(if i < 3 { cx(i + 1) - px(10.0) } else { x1 }, row.bottom()),
                    )
                };
                let id = if d.depot_id.is_empty() { "-" } else { d.depot_id.as_str() };
                let path_label = depot_path_label(&root, d);
                let (_, cut_id) = kit::fit_text(
                    ui.painter(),
                    Pos2::new(cx(0), cy),
                    Align2::LEFT_CENTER,
                    id,
                    mono(13.0),
                    theme::ink(),
                    cell(0).width(),
                );
                let (_, cut_path) = kit::fit_text(
                    ui.painter(),
                    Pos2::new(cx(1), cy),
                    Align2::LEFT_CENTER,
                    &path_label,
                    mono(13.0),
                    theme::ink_dim(),
                    cell(1).width(),
                );
                let target = classify_os(&d.name)
                    .or_else(|| classify_os(&d.local_path.to_string_lossy()))
                    .map(|o| o.label())
                    .unwrap_or("Any");
                kit::tag(ui.painter(), Pos2::new(cx(2), cy), Align2::LEFT_CENTER, target, s);
                let ignore = d
                    .exclusions
                    .iter()
                    .filter(|e| !e.trim().is_empty())
                    .map(|e| if e.starts_with('.') { format!("*{e}") } else { e.clone() })
                    .collect::<Vec<_>>()
                    .join(", ");
                let ignore_label = if ignore.is_empty() { "-".to_string() } else { ignore };
                let (_, cut_ignore) = kit::fit_text(
                    ui.painter(),
                    Pos2::new(cx(3), cy),
                    Align2::LEFT_CENTER,
                    &ignore_label,
                    mono(12.0),
                    theme::ink_muted(),
                    cell(3).width(),
                );
                if cut_id || cut_path || cut_ignore {
                    resp.clone().on_hover_text(format!(
                        "Depot {id}\nPath: {path_label}\nIgnore: {ignore_label}"
                    ));
                }
                y += px(ROW);
                if i + 1 < n_depots {
                    sep(y);
                }
            }

            let title_y = y + px(23.5);
            kit::caps(
                ui.painter(),
                Pos2::new(x0, title_y),
                Align2::LEFT_CENTER,
                "Branches",
                12.5,
                theme::ink_muted(),
                s,
            );
            if kit::link(
                ui,
                Pos2::new(x1, title_y),
                Align2::RIGHT_CENTER,
                "+ Add branch",
                theme::ink_faint(),
                s,
            )
            .clicked()
            {
                app.projects_ui.add_branch();
                app.projects_ui.edit_branch = Some(app.projects_ui.base.branches.len() - 1);
            }
            let builds = super::branches::latest_builds(app);
            let mut by = title_y + px(21.5);
            if n_branches == 0 {
                ui.painter().text(
                    Pos2::new(x0, by + px(21.0)),
                    Align2::LEFT_CENTER,
                    "No branches yet.",
                    FontId::new(theme::text_px(13.0, s), theme::family_medium()),
                    theme::ink_faint(),
                );
            }
            for i in 0..n_branches {
                let row = Rect::from_min_max(Pos2::new(x0, by), Pos2::new(x1, by + px(42.0)));
                if super::branches::row(app, ui, row, i, &builds, s) {
                    app.projects_ui.edit_branch = Some(i);
                }
                by += px(52.0);
            }
        });
    let _ = ctx;
}

fn depot_path_label(root: &std::path::Path, d: &Depot) -> String {
    if d.local_path.as_os_str().is_empty() {
        return "-".to_string();
    }
    let rel = d
        .local_path
        .strip_prefix(root)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| {
            d.local_path
                .file_name()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| d.local_path.clone())
        });
    let mut text = rel.to_string_lossy().to_string();
    if text.is_empty() {
        text = ".".to_string();
    }
    if !text.ends_with(std::path::MAIN_SEPARATOR) && text != "." {
        text.push(std::path::MAIN_SEPARATOR);
    }
    text
}

pub(super) fn modals(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    if let Some(i) = app.projects_ui.edit_depot {
        if i >= app.projects_ui.base.depots.len() {
            app.projects_ui.edit_depot = None;
            return;
        }
        let mut open = true;
        let mut act = None;
        kit::modal(
            ui,
            ui.make_persistent_id("depot_modal"),
            &mut open,
            Vec2::new(px(640.0), px(520.0)),
            s,
            |ui| {
                let d = ui.max_rect();
                ui.painter().text(
                    Pos2::new(d.left() + px(26.0), d.top() + px(30.0)),
                    Align2::LEFT_CENTER,
                    "Edit depot",
                    FontId::new(theme::text_px(15.0, s), theme::family_semibold()),
                    theme::ink(),
                );
                let body = Rect::from_min_max(
                    Pos2::new(d.left() + px(26.0), d.top() + px(56.0)),
                    Pos2::new(d.right() - px(26.0), d.bottom() - px(76.0)),
                );
                let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(body));
                egui::ScrollArea::vertical()
                    .id_salt("depot_modal_scroll")
                    .auto_shrink([false, false])
                    .show(&mut cui, |ui| {
                        ui.set_width(body.width() - px(12.0));
                        act = depot_card(app, ui, s, ctx, i);
                    });
                let done = Rect::from_min_size(
                    Pos2::new(d.right() - px(26.0) - px(120.0), d.bottom() - px(26.0) - px(38.0)),
                    Vec2::new(px(120.0), px(38.0)),
                );
                Button::new("Done", ButtonKind::Primary).show(ui, done, s).clicked()
            },
        );
        if !open {
            app.projects_ui.edit_depot = None;
        }
        match act {
            Some(DepotAct::Move(i, up)) => {
                app.projects_ui.move_depot(i, up);
                app.projects_ui.edit_depot = Some(if up { i - 1 } else { i + 1 });
            }
            Some(DepotAct::Remove(i)) => {
                let owner = app.projects_ui.selected.clone();
                if let Some(depot) = app.projects_ui.remove_depot(i) {
                    let label = if depot.name.trim().is_empty() {
                        format!("depot {}", depot.depot_id)
                    } else {
                        depot.name.clone()
                    };
                    app.offer_undo(format!("Removed {label}"), move |app| {
                        if app.projects_ui.selected != owner {
                            return Err("Can't undo: a different project is open".to_string());
                        }
                        app.projects_ui.insert_depot(i, depot);
                        Ok(())
                    });
                }
                app.projects_ui.edit_depot = None;
            }
            Some(DepotAct::Scan(i)) => app.projects_ui.scan_depot(ctx, i),
            Some(DepotAct::AddExcl(i)) => {
                if let Some(d) = app.projects_ui.base.depots.get_mut(i) {
                    d.exclusions.push(String::new());
                }
            }
            Some(DepotAct::RemoveExcl(i, j)) => {
                if let Some(d) = app.projects_ui.base.depots.get_mut(i)
                    && j < d.exclusions.len()
                {
                    d.exclusions.remove(j);
                }
            }
            None => {}
        }
    }
    super::branches::modal(app, ui, s);
}

fn depot_card(
    app: &mut App,
    ui: &mut egui::Ui,
    s: f32,
    ctx: &egui::Context,
    i: usize,
) -> Option<DepotAct> {
    let px = |v: f32| theme::px(v, s);
    let n_depots = app.projects_ui.base.depots.len();
    let n_excl = app.projects_ui.base.depots[i].exclusions.len();
    let id_err = app.projects_ui.error_for(&format!("depots[{i}].depot_id"));

    const PAD: f32 = 16.0;
    const ROW_H: f32 = 40.0;
    const ROW_GAP: f32 = 10.0;
    const OPT_H: f32 = 36.0;
    const LABEL_H: f32 = 18.0;
    const EXCL_H: f32 = 28.0;
    const EXCL_STEP: f32 = 34.0;
    const ADD_H: f32 = 30.0;
    let err_h = if id_err.is_some() { 18.0 } else { 0.0 };
    let card_h = PAD
        + ROW_H
        + err_h
        + ROW_GAP
        + ROW_H
        + ROW_GAP
        + OPT_H
        + ROW_GAP
        + 4.0
        + LABEL_H
        + n_excl as f32 * EXCL_STEP
        + ADD_H
        + PAD;
    let card = row(ui, s, card_h);
    kit::bordered(
        &ui.painter().clone(),
        card,
        theme::surface_2(),
        theme::line(),
        px(theme::RADIUS_CARD),
    );
    let pad = px(PAD);
    let x = card.left() + pad;
    let w = card.width() - pad * 2.0;
    let mut y = card.top() + pad;
    let mut act = None;

    let id_w = px(110.0);
    let btn_w = px(46.0);
    let gap = px(6.0);
    let mut tid = Input::new(&mut app.projects_ui.base.depots[i].depot_id).hint("depot id");
    if let Some(e) = &id_err {
        tid = tid.error(true).helper(e);
    }
    tid.show(
        ui,
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(id_w, px(ROW_H))),
        s,
    );
    let name_w = w - id_w - gap - 3.0 * (btn_w + gap);
    Input::new(&mut app.projects_ui.base.depots[i].name)
        .hint("name")
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x + id_w + gap, y), Vec2::new(name_w, px(ROW_H))),
            s,
        );
    let mut bx = x + w - 3.0 * btn_w - 2.0 * gap;
    for (lbl, kind, enabled, a) in [
        ("UP", ButtonKind::Secondary, i > 0, DepotAct::Move(i, true)),
        (
            "DN",
            ButtonKind::Secondary,
            i + 1 < n_depots,
            DepotAct::Move(i, false),
        ),
        ("X", ButtonKind::Danger, true, DepotAct::Remove(i)),
    ] {
        if Button::new(lbl, kind)
            .enabled(enabled)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(bx, y), Vec2::new(btn_w, px(ROW_H))),
                s,
            )
            .clicked()
        {
            act = Some(a);
        }
        bx += btn_w + gap;
    }
    y += px(ROW_H + err_h + ROW_GAP);

    let scan_w = px(88.0);
    let browse_w = px(110.0);
    let path_w = w - browse_w - scan_w - 2.0 * gap;
    let pr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(path_w, px(ROW_H)));
    kit::bordered(
        &ui.painter().clone(),
        pr,
        theme::field(),
        theme::line(),
        px(theme::RADIUS_FIELD),
    );
    let lp = app.projects_ui.base.depots[i]
        .local_path
        .to_string_lossy()
        .to_string();
    let (txt, col) = if lp.is_empty() {
        ("local path".to_string(), theme::ink_faint())
    } else {
        (lp.clone(), theme::ink_dim())
    };
    let text_clip = pr.shrink2(Vec2::new(px(14.0), 0.0));
    let galley =
        ui.painter()
            .layout_no_wrap(txt, FontId::new(theme::text_px(12.0, s), theme::family_medium()), col);
    let overflow = galley.size().x > text_clip.width();
    let tx = if overflow {
        text_clip.right() - galley.size().x
    } else {
        text_clip.left()
    };
    ui.painter().with_clip_rect(text_clip).galley(
        Pos2::new(tx, pr.center().y - galley.size().y / 2.0),
        galley,
        col,
    );
    if overflow {
        ui.interact(
            pr,
            ui.make_persistent_id(("depot_path_tip", i)),
            egui::Sense::hover(),
        )
        .on_hover_text(lp);
    }
    if Button::new("Browse", ButtonKind::Secondary)
        .icon(theme::icon::FOLDER)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(pr.right() + gap, y),
                Vec2::new(browse_w, px(ROW_H)),
            ),
            s,
        )
        .clicked()
    {
        app.projects_ui.spawn_pick(ctx, PickField::DepotLocal, i);
    }
    let running = matches!(app.projects_ui.scans.get(i), Some(ScanState::Running));
    if Button::new("Scan", ButtonKind::Secondary)
        .loading(running)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x + w - scan_w, y), Vec2::new(scan_w, px(ROW_H))),
            s,
        )
        .clicked()
        && !running
    {
        act = Some(DepotAct::Scan(i));
    }
    y += px(ROW_H + ROW_GAP);

    Input::new(&mut app.projects_ui.base.depots[i].depot_path)
        .hint(".")
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(120.0), px(OPT_H))),
            s,
        );
    kit::checkbox(
        ui,
        Rect::from_min_size(Pos2::new(x + px(140.0), y), Vec2::new(px(160.0), px(OPT_H))),
        &mut app.projects_ui.base.depots[i].recursive,
        "Recursive",
        s,
    );
    if let Some(ScanState::Done { files, bytes }) = app.projects_ui.scans.get(i).copied() {
        ui.painter().text(
            Pos2::new(x + w, y + px(OPT_H) / 2.0),
            Align2::RIGHT_CENTER,
            format!("{files} files  {}", human_size(bytes)),
            FontId::new(theme::text_px(12.0, s), theme::family_medium()),
            theme::ink_muted(),
        );
    }
    y += px(OPT_H + ROW_GAP + 4.0);

    ui.painter().text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        "Exclusions",
        FontId::new(theme::text_px(11.0, s), theme::family_label()),
        theme::ink_muted(),
    );
    y += px(LABEL_H);
    for j in 0..n_excl {
        Input::new(&mut app.projects_ui.base.depots[i].exclusions[j])
            .hint("*.pdb")
            .show(
                ui,
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(220.0), px(EXCL_H))),
                s,
            );
        if Button::new("X", ButtonKind::Danger)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(x + px(228.0), y), Vec2::new(px(38.0), px(EXCL_H))),
                s,
            )
            .clicked()
        {
            act = Some(DepotAct::RemoveExcl(i, j));
        }
        y += px(EXCL_STEP);
    }
    if Button::new("+ Pattern", ButtonKind::Secondary)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(128.0), px(ADD_H))),
            s,
        )
        .clicked()
    {
        act = Some(DepotAct::AddExcl(i));
    }
    act
}
