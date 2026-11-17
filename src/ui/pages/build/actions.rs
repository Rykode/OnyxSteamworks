use eframe::egui::{self, Rect};

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

    let has_user = !project.steam_username.trim().is_empty();
    let items: Vec<kit::MenuItem> = TOOLS
        .iter()
        .map(|(tool, glyph, label)| {
            kit::MenuItem::new(label)
                .icon(*glyph)
                .enabled(!matches!(tool, Tool::Reauthorize) || has_user)
        })
        .collect();
    let id = ui.make_persistent_id("build_more_actions");
    match kit::menu(ui, id, button, button.width().max(px(260.0)), None, &items, s) {
        Some(kit::MenuEvent::Picked(i)) => {
            app.build_ui.more_open = false;
            run(app, TOOLS[i].0, project, ctx);
        }
        Some(kit::MenuEvent::Closed) => app.build_ui.more_open = false,
        None => {}
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
