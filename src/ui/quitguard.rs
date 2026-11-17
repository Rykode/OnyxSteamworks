use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2, ViewportCommand};

use crate::app::App;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

fn pending_work(app: &App) -> Vec<&'static str> {
    let mut work = Vec::new();
    if app.build_ui.running {
        work.push("a SteamCMD build");
    }
    if app.build_ui.verify_ui.running() {
        work.push("a build verification");
    }
    if app.pipeline_ui.running || app.scheduler.busy() {
        work.push("a pipeline run");
    }
    if app.backup_ui.running() {
        work.push("a backup");
    }
    if app.projects_ui.unsaved() {
        work.push("unsaved project edits");
    }
    work
}

pub fn show(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let ctx = ui.ctx().clone();

    if app.quitting {
        ctx.send_viewport_cmd(ViewportCommand::Close);
        return;
    }

    let work = pending_work(app);
    if ctx.input(|i| i.viewport().close_requested()) {
        if !work.is_empty() {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            app.confirm_quit = true;
        } else {
            return;
        }
    }

    if !app.confirm_quit {
        return;
    }
    if work.is_empty() {
        app.confirm_quit = false;
        return;
    }
    let summary = match work.as_slice() {
        [one] => format!("Quitting now stops {one}."),
        [rest @ .., last] => format!("Quitting now stops {} and {last}.", rest.join(", ")),
        [] => String::new(),
    };

    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id("confirm_quit");
    let mut open = true;
    let mut quit = false;
    let mut cancel = false;
    kit::modal_with(
        ui,
        id,
        &mut open,
        Vec2::new(px(468.0), px(224.0)),
        s,
        false,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(28.0);
            let painter = ui.painter().clone();
            kit::label(
                &painter,
                Pos2::new(x, d.top() + px(26.0)),
                "Work in progress",
                s,
            );
            painter.text(
                Pos2::new(x, d.top() + px(48.0)),
                Align2::LEFT_TOP,
                "Quit Onyx anyway?",
                FontId::new(theme::text_px(16.0, s), theme::family_semibold()),
                theme::INK,
            );
            painter.text(
                Pos2::new(x, d.top() + px(80.0)),
                Align2::LEFT_TOP,
                summary.as_str(),
                FontId::new(theme::text_px(12.5, s), theme::family_medium()),
                theme::INK_MUTED,
            );
            let by = d.bottom() - px(26.0) - px(42.0);
            if Button::new("Keep working", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, by), Vec2::new(px(170.0), px(42.0))),
                    s,
                )
                .clicked()
            {
                cancel = true;
            }
            if Button::new("Quit anyway", ButtonKind::Danger)
                .icon(theme::icon::WARNING)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - px(28.0) - px(170.0), by),
                        Vec2::new(px(170.0), px(42.0)),
                    ),
                    s,
                )
                .clicked()
            {
                quit = true;
            }
            false
        },
    );

    if quit {
        if app.build_ui.running {
            app.build_ui.cancel();
        }
        app.build_ui.verify_ui.cancel_run();
        app.pipeline_ui.cancel();
        app.scheduler.cancel();
        app.confirm_quit = false;
        app.quitting = true;
    } else if cancel || !open {
        app.confirm_quit = false;
    }
}
