use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, UiBuilder, Vec2};

use crate::app::App;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::{anim, theme};

#[derive(Clone, Copy)]
pub enum Pick {
    SteamCmd,
    Content,
    Output,
    Backup,
}

pub struct GeneralState {
    tx: Sender<(Pick, String)>,
    rx: Receiver<(Pick, String)>,
}

impl Default for GeneralState {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx }
    }
}

fn cfg() -> crate::config::Settings {
    crate::config::load_settings().unwrap_or_default()
}

fn save(c: &crate::config::Settings) {
    if let Err(e) = crate::config::save_settings(c) {
        crate::config::report_problem(format!("Could not save settings: {e}"));
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let mut c = cfg();
    let mut dirty = false;
    while let Ok((which, path)) = app.general_ui.rx.try_recv() {
        match which {
            Pick::SteamCmd => c.steamcmd_path = path,
            Pick::Content => c.default_content = path,
            Pick::Output => c.default_output = path,
            Pick::Backup => c.backup_dir = path,
        }
        dirty = true;
    }
    if c.ui_scale <= 0.0 {
        c.ui_scale = 1.0;
    }
    if c.backup_keep == 0 {
        c.backup_keep = 5;
    }

    let mut cu = ui.new_child(UiBuilder::new().max_rect(rect));
    cu.set_clip_rect(rect);
    egui::ScrollArea::vertical()
        .id_salt("general_scroll")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            ui.set_width(rect.width() - px(16.0));
            section(ui, "Getting started", s);
            let wz = alloc(ui, 44.0, s);
            if Button::new("Run setup wizard", ButtonKind::Secondary)
                .icon(theme::icon::SPARKLE)
                .show(
                    ui,
                    Rect::from_min_size(wz.min, Vec2::new(px(200.0), px(36.0))),
                    s,
                )
                .clicked()
            {
                app.wizard.start();
            }

            section(ui, "SteamCMD", s);
            dirty |= path_row(
                app,
                ui,
                "SteamCMD executable",
                &c.steamcmd_path,
                Pick::SteamCmd,
                s,
            );
            let hint = alloc(ui, 24.0, s);
            ui.painter().text(
                Pos2::new(hint.left(), hint.top() + px(2.0)),
                Align2::LEFT_TOP,
                "Every project builds with this SteamCMD. The setup wizard can download it for you.",
                FontId::new(theme::text_px(11.5, s), theme::family_medium()),
                theme::INK_MUTED,
            );

            section(ui, "Default folders", s);
            dirty |= path_row(
                app,
                ui,
                "Default content folder",
                &c.default_content,
                Pick::Content,
                s,
            );
            dirty |= path_row(
                app,
                ui,
                "Default build output",
                &c.default_output,
                Pick::Output,
                s,
            );
            let hint = alloc(ui, 24.0, s);
            ui.painter().text(
                Pos2::new(hint.left(), hint.top() + px(2.0)),
                Align2::LEFT_TOP,
                "Projects use these folders unless you override them in the project editor.",
                FontId::new(theme::text_px(11.5, s), theme::family_medium()),
                theme::INK_MUTED,
            );

            section(ui, "Display", s);
            let mut scale_pct = (c.ui_scale * 100.0).round() as u32;
            if stepper(ui, "UI scale (%)", &mut scale_pct, 5, 80, 130, s) {
                c.ui_scale = scale_pct as f32 / 100.0;
                theme::set_ui_scale(c.ui_scale);
                dirty = true;
            }
            if toggle_row(ui, "Reduce motion", &mut c.reduce_motion, s) {
                anim::set_reduce_motion(c.reduce_motion);
                dirty = true;
            }
            if anim::os_reduces_motion() && !c.reduce_motion {
                let note = alloc(ui, 24.0, s);
                ui.painter().text(
                    Pos2::new(note.left(), note.top() + px(2.0)),
                    Align2::LEFT_TOP,
                    "Windows has animations turned off, so Onyx reduces motion anyway.",
                    FontId::new(theme::text_px(11.5, s), theme::family_medium()),
                    theme::INK_MUTED,
                );
            }

            section(ui, "Logs & backups", s);
            let mut days = c.log_retention_days;
            if stepper(
                ui,
                "Delete logs older than (days, 0 = keep all)",
                &mut days,
                1,
                0,
                365,
                s,
            ) {
                c.log_retention_days = days;
                dirty = true;
            }
            let clean = alloc(ui, 44.0, s);
            let confirm_id = ui.make_persistent_id("clean_logs_confirm");
            let keep_all = c.log_retention_days == 0;
            let mut clean_btn = Button::new("Clean logs now", ButtonKind::Secondary)
                .icon(theme::icon::TRASH)
                .enabled(!keep_all);
            if keep_all {
                clean_btn = clean_btn.disabled_reason("Retention is set to keep all logs");
            }
            if clean_btn
                .show(
                    ui,
                    Rect::from_min_size(clean.min, Vec2::new(px(170.0), px(34.0))),
                    s,
                )
                .clicked()
                && !keep_all
            {
                ui.data_mut(|d| d.insert_temp(confirm_id, true));
            }
            if ui.data(|d| d.get_temp::<bool>(confirm_id)).unwrap_or(false) {
                let detail = format!(
                    "Log files older than {} day(s) are deleted. History entries that point at them will no longer show a log.",
                    c.log_retention_days
                );
                let spec = kit::Confirm::danger(
                    "Clean logs",
                    "Delete old log files?",
                    &detail,
                    "Delete logs",
                );
                match kit::confirm(ui, confirm_id.with("modal"), &spec, s) {
                    kit::Choice::Confirmed => {
                        ui.data_mut(|d| d.remove::<bool>(confirm_id));
                        let n = crate::features::maintenance::clean_old_logs(c.log_retention_days);
                        app.toasts.push(
                            kit::ToastLevel::Success,
                            format!("Removed {n} old log file(s)"),
                        );
                    }
                    kit::Choice::Cancelled => ui.data_mut(|d| d.remove::<bool>(confirm_id)),
                    kit::Choice::Pending => {}
                }
            }
            dirty |= path_row(app, ui, "Backup folder", &c.backup_dir, Pick::Backup, s);
            let mut keep = c.backup_keep;
            if stepper(ui, "Keep last N backups", &mut keep, 1, 1, 50, s) {
                c.backup_keep = keep;
                dirty = true;
            }
            ui.add_space(px(8.0));
        });

    if dirty {
        save(&c);
        if let Ok(projects) = crate::config::list_projects() {
            app.projects = projects;
        }
    }
    let _ = ctx;
}

fn section(ui: &mut egui::Ui, title: &str, s: f32) {
    let px = |v: f32| theme::px(v, s);
    ui.add_space(px(12.0));
    let r = alloc(ui, 30.0, s);
    kit::heading(
        &ui.painter().clone(),
        Pos2::new(r.left(), r.top() + px(4.0)),
        title,
        s,
    );
}

fn alloc(ui: &mut egui::Ui, h: f32, s: f32) -> Rect {
    let (r, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), theme::px(h, s)),
        Sense::hover(),
    );
    r
}

fn path_row(
    app: &mut App,
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    pick: Pick,
    s: f32,
) -> bool {
    let px = |v: f32| theme::px(v, s);
    let r = alloc(ui, 60.0, s);
    ui.painter().text(
        Pos2::new(r.left(), r.top()),
        Align2::LEFT_TOP,
        label,
        FontId::new(theme::text_px(11.5, s), theme::family_label()),
        theme::INK_MUTED,
    );
    let mut shown = value.to_string();
    let field = Rect::from_min_size(
        Pos2::new(r.left(), r.top() + px(20.0)),
        Vec2::new(r.width() - px(114.0), px(36.0)),
    );
    Input::new(&mut shown)
        .hint(if matches!(pick, Pick::SteamCmd) {
            "No SteamCMD set"
        } else {
            "No folder set"
        })
        .readonly(true)
        .show(ui, field, s);
    let browse = Rect::from_min_size(
        Pos2::new(field.right() + px(10.0), field.top()),
        Vec2::new(px(104.0), px(36.0)),
    );
    if Button::new("Browse", ButtonKind::Secondary)
        .icon(theme::icon::FOLDER)
        .show(ui, browse, s)
        .clicked()
    {
        let tx = app.general_ui.tx.clone();
        let ctx = ui.ctx().clone();
        std::thread::spawn(move || {
            let dialog = rfd::FileDialog::new();
            let picked = if matches!(pick, Pick::SteamCmd) {
                dialog.pick_file()
            } else {
                dialog.pick_folder()
            };
            if let Some(dir) = picked {
                let _ = tx.send((pick, dir.to_string_lossy().to_string()));
                ctx.request_repaint();
            }
        });
    }
    false
}

fn toggle_row(ui: &mut egui::Ui, label: &str, value: &mut bool, s: f32) -> bool {
    let px = |v: f32| theme::px(v, s);
    let r = alloc(ui, 34.0, s);
    let tog = Rect::from_min_size(Pos2::new(r.left(), r.top()), Vec2::new(px(44.0), px(24.0)));
    let mut on = *value;
    let changed = kit::toggle(ui, tog, &mut on, label, s).changed();
    if changed {
        *value = on;
    }
    ui.painter().text(
        Pos2::new(tog.right() + px(12.0), tog.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::new(theme::text_px(12.5, s), theme::family_medium()),
        theme::INK_DIM,
    );
    changed
}

fn stepper(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    step: u32,
    min: u32,
    max: u32,
    s: f32,
) -> bool {
    let px = |v: f32| theme::px(v, s);
    let r = alloc(ui, 42.0, s);
    ui.painter().text(
        Pos2::new(r.left(), r.top() + px(16.0)),
        Align2::LEFT_CENTER,
        label,
        FontId::new(theme::text_px(12.5, s), theme::family_medium()),
        theme::INK_DIM,
    );
    let minus = Rect::from_min_size(
        Pos2::new(r.left() + px(320.0), r.top()),
        Vec2::splat(px(30.0)),
    );
    let plus = Rect::from_min_size(
        Pos2::new(minus.right() + px(44.0), r.top()),
        Vec2::splat(px(30.0)),
    );
    ui.painter().text(
        Pos2::new((minus.right() + plus.left()) / 2.0, r.top() + px(15.0)),
        Align2::CENTER_CENTER,
        value.to_string(),
        FontId::new(theme::text_px(13.5, s), theme::family_semibold()),
        theme::INK,
    );
    let mut changed = false;
    if Button::new("-", ButtonKind::Secondary)
        .show(ui, minus, s)
        .clicked()
        && *value > min
    {
        *value = value.saturating_sub(step).max(min);
        changed = true;
    }
    if Button::new("+", ButtonKind::Secondary)
        .show(ui, plus, s)
        .clicked()
        && *value < max
    {
        *value = (*value + step).min(max);
        changed = true;
    }
    changed
}
