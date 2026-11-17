use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Vec2};

use crate::app::App;
use crate::features::quick;
use crate::model::Project;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

pub(super) fn open_target(target: String) {
    std::thread::spawn(move || {
        let _ = open::that(target);
    });
}

#[derive(Clone, Copy)]
enum Tool {
    CopyCli,
    CopySteamCmd,
    ExportCi,
    OpenContent,
    OpenOutput,
    Steamworks,
    Launch,
    Reauthorize,
}

const TOOLS: [(Tool, char, &str); 8] = [
    (Tool::CopyCli, theme::icon::COPY, "Copy Onyx CLI command"),
    (Tool::CopySteamCmd, theme::icon::COPY, "Copy SteamCMD command"),
    (Tool::ExportCi, theme::icon::UPLOAD, "Export CI workflow\u{2026}"),
    (Tool::OpenContent, theme::icon::FOLDER, "Open content folder"),
    (Tool::OpenOutput, theme::icon::FOLDER, "Open build output folder"),
    (Tool::Steamworks, theme::icon::PACKAGE, "Open Steamworks builds page"),
    (Tool::Launch, theme::icon::PLAY, "Launch game in Steam"),
    (Tool::Reauthorize, theme::icon::GEAR, "Re-authorize Steam login"),
];

/// Project tools that are not part of this run's settings, behind one labeled menu.
pub(super) fn more_actions(
    app: &mut App,
    ui: &mut egui::Ui,
    button: Rect,
    project: &Project,
    s: f32,
    ctx: &egui::Context,
) {
    let px = |v: f32| theme::px(v, s);
    let resp = Button::new("More actions", ButtonKind::Secondary)
        .icon(theme::icon::CARET_DOWN)
        .show(ui, button, s);
    if resp.clicked() {
        app.build_ui.more_open = !app.build_ui.more_open;
    }
    if !app.build_ui.more_open {
        return;
    }

    let row_h = px(32.0);
    let pad = px(6.0);
    let w = button.width().max(px(260.0));
    let h = pad * 2.0 + row_h * TOOLS.len() as f32;
    // The panel sits at the bottom of the window, so open upward when below would clip.
    let screen = ctx.content_rect();
    let top = if button.bottom() + px(6.0) + h <= screen.bottom() {
        button.bottom() + px(6.0)
    } else {
        button.top() - px(6.0) - h
    };
    let anchor = Pos2::new(button.left(), top);
    let has_user = !project.steam_username.trim().is_empty();
    let mut pick = None;
    let area = egui::Area::new(ui.make_persistent_id("build_more_actions"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor)
        .show(ctx, |ui| {
            let rect = Rect::from_min_size(anchor, Vec2::new(w, h));
            theme::shadow(ui.painter(), rect, px(theme::RADIUS_POPUP), 24, 140);
            kit::bordered(ui.painter(), rect, theme::SURFACE_2, theme::LINE, px(theme::RADIUS_POPUP));
            let mut y = rect.top() + pad;
            for (tool, glyph, label) in TOOLS {
                let enabled = !matches!(tool, Tool::Reauthorize) || has_user;
                let r = Rect::from_min_size(Pos2::new(rect.left() + pad, y), Vec2::new(w - pad * 2.0, row_h));
                let row = ui.interact(r, ui.make_persistent_id(("more_action", label)), Sense::click());
                row.widget_info(|| egui::WidgetInfo::labeled(egui::Role::MenuItem, enabled, label));
                if enabled && (row.hovered() || row.has_focus()) {
                    ui.painter().rect_filled(r, theme::corner(px(theme::RADIUS_FIELD)), theme::FIELD);
                }
                let ink = if enabled { theme::INK } else { theme::INK_FAINT };
                ui.painter().text(
                    Pos2::new(r.left() + px(10.0), r.center().y),
                    Align2::LEFT_CENTER,
                    glyph.to_string(),
                    theme::icon_font(px(14.0), false),
                    if enabled { theme::INK_MUTED } else { theme::INK_FAINT },
                );
                ui.painter().text(
                    Pos2::new(r.left() + px(32.0), r.center().y),
                    Align2::LEFT_CENTER,
                    label,
                    FontId::new(px(12.5), theme::family_medium()),
                    ink,
                );
                if enabled && row.clicked() {
                    pick = Some(tool);
                }
                y += row_h;
            }
        });
    let on_button = ctx
        .input(|i| i.pointer.interact_pos())
        .is_some_and(|p| button.contains(p));
    if (area.response.clicked_elsewhere() && !on_button) || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.build_ui.more_open = false;
    }
    if let Some(tool) = pick {
        app.build_ui.more_open = false;
        run(app, tool, project, ctx);
    }
}

fn run(app: &mut App, tool: Tool, project: &Project, ctx: &egui::Context) {
    let folder = |p: std::path::PathBuf| p.to_string_lossy().to_string();
    match tool {
        Tool::CopyCli => {
            let branch = project
                .branches
                .get(app.build_ui.branch_sel)
                .map(|b| b.name.clone())
                .unwrap_or_default();
            ctx.copy_text(crate::cli::build_command_string(
                project,
                &branch,
                &app.build_ui.description,
                app.build_ui.preview_only,
                false,
                !app.build_ui.set_live,
            ));
            app.toasts.push(kit::ToastLevel::Success, "Onyx CLI command copied");
        }
        Tool::CopySteamCmd => {
            ctx.copy_text(quick::steamcmd_command(project));
            app.toasts.push(kit::ToastLevel::Success, "SteamCMD command copied");
        }
        Tool::ExportCi => {
            let yaml = crate::cli::ci_workflow_yaml(project);
            std::thread::spawn(move || {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("steam-deploy.yml")
                    .save_file()
                {
                    let _ = std::fs::write(path, yaml);
                }
            });
            app.toasts.push(kit::ToastLevel::Success, "Exporting CI workflow");
        }
        Tool::OpenContent => {
            open_target(folder(crate::features::maintenance::resolve(project).content_root))
        }
        Tool::OpenOutput => {
            open_target(folder(crate::features::maintenance::resolve(project).build_output_dir))
        }
        Tool::Steamworks => open_target(quick::steamworks_builds_url(&project.app_id)),
        Tool::Launch => open_target(quick::steam_run_url(&project.app_id)),
        Tool::Reauthorize => super::authorize(app, project, ctx),
    }
}
