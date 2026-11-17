mod actions;
mod auth;
mod console;
mod finish;
pub(crate) mod guard;
mod launch;
pub(crate) mod preflight;
pub(crate) mod review;
mod speed;
mod stages;
mod state;
mod verify;

pub use state::BuildState;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, UiBuilder, Vec2};

use crate::app::{App, Page};
use crate::features::accounts;
use crate::model::{BuildStatus, Project};
use crate::steamcmd::{BuildResult, Stage};
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::layout::Layout;
use crate::ui::theme;
use crate::validate::validate_project;

pub(crate) fn pump(app: &mut App, ctx: &egui::Context) {
    let was_authorizing = app.build_ui.authorizing;
    if let Some(res) = app.build_ui.drain(ctx) {
        let (level, msg) = if was_authorizing {
            match res {
                BuildResult::Success { .. } => (
                    kit::ToastLevel::Success,
                    "Account authorized; Steam Guard is cached",
                ),
                BuildResult::Failure { .. } => (kit::ToastLevel::Error, "Authorization failed"),
                BuildResult::Cancelled => (kit::ToastLevel::Warning, "Authorization cancelled"),
            }
        } else {
            if !matches!(res, BuildResult::Cancelled) {
                app.on_run_finished(ctx, matches!(res, BuildResult::Success { .. }));
            }
            match res {
                BuildResult::Success { .. } => (kit::ToastLevel::Success, "Build finished"),
                BuildResult::Failure { .. } => (kit::ToastLevel::Error, "Build failed"),
                BuildResult::Cancelled => (kit::ToastLevel::Warning, "Build cancelled"),
            }
        };
        app.toasts.push(level, msg);
    }
    if let Some(record) = app.build_ui.finished_record.take() {
        finish::after_finish(app, &record, ctx);
        app.history_ui.save(record);
    }
    if app.build_ui.pending_prompt.is_some() && app.page != crate::app::Page::Build {
        app.nav_request = Some(crate::app::Page::Build);
    }
    verify::drain(app, ctx);
}

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let ctx = ui.ctx().clone();
    let s = layout.scale;
    let px = |v: f32| theme::px(v, s);
    let body = layout.body;

    app.history_ui.ensure_loaded();

    if !app.build_ui.inited {
        app.build_ui.project_sel = app
            .projects_ui
            .selected
            .as_ref()
            .and_then(|id| app.projects.iter().position(|p| &p.id == id))
            .unwrap_or(0);
        if let Some(p) = app.projects.get(app.build_ui.project_sel)
            && app.history_ui.latest_for(&p.id).is_none()
        {
            app.build_ui.preview_only = true;
        }
        app.build_ui.inited = true;
    }

    if app.projects.is_empty() {
        kit::card(ui.painter(), body, s);
        kit::empty_state(
            ui,
            body,
            theme::icon::ROCKET,
            "No projects to build",
            "Create a project first, then run a build here.",
            s,
        );
        return;
    }
    app.build_ui.project_sel = app.build_ui.project_sel.min(app.projects.len() - 1);
    let project = app.projects[app.build_ui.project_sel].clone();

    header(app, ui, layout, &project, &ctx);

    let gap = px(15.0);
    let left_w = px(341.5);
    let stages_h = px(302.0);
    let running = app.build_ui.running && !app.build_ui.authorizing;
    let status = Rect::from_min_size(body.min, Vec2::new(left_w, px(85.0)));
    // Left column order follows what the user can act on. Idle: the run's settings sit
    // under the status and the stage readout drops to the bottom. Running: progress
    // moves up under the status and upload speed takes the settings' place.
    let (stages_rect, lower) = if running {
        let stages_rect = Rect::from_min_size(
            Pos2::new(body.left(), status.bottom() + gap),
            Vec2::new(left_w, stages_h),
        );
        let lower = Rect::from_min_max(
            Pos2::new(body.left(), stages_rect.bottom() + gap),
            Pos2::new(body.left() + left_w, body.bottom()),
        );
        (stages_rect, lower)
    } else {
        let stages_rect = Rect::from_min_max(
            Pos2::new(body.left(), body.bottom() - stages_h),
            Pos2::new(body.left() + left_w, body.bottom()),
        );
        // Settings card hugs its content; spare height goes to the end of the column.
        let lower = Rect::from_min_size(
            Pos2::new(body.left(), status.bottom() + gap),
            Vec2::new(left_w, options_card_h(app, &project, s)),
        );
        let stages_rect = Rect::from_min_max(
            Pos2::new(body.left(), (lower.bottom() + gap).min(stages_rect.top())),
            stages_rect.max,
        );
        (stages_rect, lower)
    };
    let right = Rect::from_min_max(Pos2::new(status.right() + gap, body.top()), body.max);
    let tiles = Rect::from_min_size(right.min, Vec2::new(right.width(), px(69.0)));
    let console_rect = Rect::from_min_max(Pos2::new(right.left(), tiles.bottom() + gap), right.max);

    let speed_rect = Rect::from_min_size(lower.min, Vec2::new(lower.width(), px(239.5)));
    for r in [status, stages_rect, console_rect] {
        kit::card(ui.painter(), r, s);
    }
    status_card(app, ui, status, &project, s, &ctx);
    stages::show(app, ui, stages_rect, s);
    if running {
        kit::card(ui.painter(), speed_rect, s);
        stages::speed_card(app, ui, speed_rect, &project, s);
    } else {
        kit::card(ui.painter(), lower, s);
        options_card(app, ui, lower, &project, s, &ctx);
    }
    stat_tiles(app, ui, tiles, &project, s);
    let cctx = ui.ctx().clone();
    console::show(&mut app.build_ui, ui, console_rect, s, &cctx);

    auth::show(app, ui, s);
    guard::show(app, ui, s, &ctx);
    preflight::show(app, ui, s, &ctx);
    review::show(app, ui, s, &ctx);
    verify::show(app, ui, s, &ctx);
}

fn selected_branch(app: &App, project: &Project) -> String {
    project
        .branches
        .get(app.build_ui.branch_sel)
        .map(|b| b.name.clone())
        .unwrap_or_default()
}

fn header(app: &mut App, ui: &mut egui::Ui, layout: &Layout, project: &Project, ctx: &egui::Context) {
    let s = layout.scale;
    let px = |v: f32| theme::px(v, s);
    let h = layout.header;
    let title_w = ui
        .painter()
        .layout_no_wrap(
            "Build".to_owned(),
            FontId::new(theme::text_px(28.0, s), theme::family_semibold()),
            theme::ink(),
        )
        .size()
        .x;
    let name = if project.name.is_empty() { "Untitled" } else { project.name.as_str() };
    let name_font = FontId::new(theme::text_px(13.5, s), theme::family_semibold());
    let name_w = ui
        .painter()
        .layout_no_wrap(name.to_owned(), name_font.clone(), theme::ink())
        .size()
        .x;
    let pill = Rect::from_min_size(
        Pos2::new(h.left() + title_w + px(15.5), h.center().y - px(20.5)),
        Vec2::new(px(46.0) + name_w + px(40.0), px(41.0)),
    );
    let resp = ui
        .interact(pill, ui.make_persistent_id("build_project_pill"), egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    crate::ui::kit::accessible(&resp, pill, egui::Role::ComboBox, &format!("Project: {}", project.name), None, px(theme::RADIUS_FIELD));
    kit::bordered(
        ui.painter(),
        pill,
        if resp.hovered() { theme::field_hi() } else { theme::field() },
        theme::line_soft(),
        px(theme::RADIUS_FIELD),
    );
    let tile = Rect::from_center_size(Pos2::new(pill.left() + px(23.0), pill.center().y), Vec2::splat(px(21.0)));
    kit::monogram(ui.painter(), tile, &kit::initials(name), s);
    ui.painter().text(
        Pos2::new(tile.right() + px(12.0), pill.center().y),
        Align2::LEFT_CENTER,
        name,
        name_font,
        theme::ink(),
    );
    ui.painter().text(
        Pos2::new(pill.right() - px(18.0), pill.center().y),
        Align2::CENTER_CENTER,
        theme::icon::CARET_DOWN.to_string(),
        theme::icon_font(px(12.0), false),
        theme::ink_muted(),
    );
    let mut pick = None;
    egui::Popup::menu(&resp).show(|ui| {
        for (i, p) in app.projects.iter().enumerate() {
            let label = if p.name.is_empty() { "Untitled" } else { p.name.as_str() };
            if ui.selectable_label(i == app.build_ui.project_sel, label).clicked() {
                pick = Some(i);
            }
        }
    });
    if let Some(i) = pick
        && i != app.build_ui.project_sel
        && !app.build_ui.running
    {
        app.build_ui.project_sel = i;
        app.build_ui.branch_sel = 0;
        if let Some(p) = app.projects.get(i)
            && app.history_ui.latest_for(&p.id).is_none()
        {
            app.build_ui.preview_only = true;
        }
    }

    let (label, color) = match project.branches.get(app.build_ui.branch_sel) {
        Some(b) => (
            if b.name.is_empty() { "branch".to_string() } else { b.name.clone() },
            crate::ui::pages::projects::level_color(b.level),
        ),
        None => ("default".to_string(), theme::ink_muted()),
    };
    let badge = kit::badge(
        ui.painter(),
        Pos2::new(pill.right() + px(15.5), h.center().y),
        Align2::LEFT_CENTER,
        &label,
        color,
        true,
        s,
    );
    if !project.branches.is_empty() {
        let bresp = ui
            .interact(badge, ui.make_persistent_id("build_branch_badge"), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        crate::ui::kit::accessible(&bresp, badge, egui::Role::ComboBox, &format!("Branch: {}", selected_branch(app, project)), None, badge.height() / 2.0);
        egui::Popup::menu(&bresp).show(|ui| {
            for (i, b) in project.branches.iter().enumerate() {
                let name = if b.name.is_empty() { "branch" } else { b.name.as_str() };
                if ui.selectable_label(i == app.build_ui.branch_sel, name).clicked() && !app.build_ui.running {
                    app.build_ui.branch_sel = i;
                }
            }
        });
    }

    let running = app.build_ui.running;
    if running {
        let busy = match app.build_ui.stage {
            Some(Stage::Connecting) => "Connecting...",
            Some(Stage::LoggingIn) => "Logging in...",
            Some(Stage::Scanning) => "Scanning...",
            Some(Stage::Committing) => "Committing...",
            _ if app.build_ui.authorizing => "Authorizing...",
            _ => "Uploading...",
        };
        if kit::header_buttons(
            ui,
            h,
            &[
                ("Cancel", theme::icon::X, ButtonKind::Danger, true),
                (busy, theme::icon::ROCKET, ButtonKind::Primary, true),
            ],
            s,
        ) == Some(0)
        {
            app.build_ui.cancel();
        }
    } else {
        let reason = build_reason(project);
        let branch = selected_branch(app, project);
        match kit::header_buttons(
            ui,
            h,
            &[
                ("Preflight", theme::icon::CHECK, ButtonKind::Secondary, true),
                ("Start build", theme::icon::ROCKET, ButtonKind::Primary, reason.is_none()),
            ],
            s,
        ) {
            Some(0) => app.build_ui.preflight.request(project, &branch, ctx),
            Some(1) => {
                if let Err(e) = guard::launch(app, &branch, None, ctx) {
                    app.toasts.push(kit::ToastLevel::Warning, e);
                }
            }
            _ => {}
        }
    }
}

fn build_number(app: &App, project: &Project) -> usize {
    app.history_ui
        .records
        .iter()
        .filter(|r| r.project_id == project.id)
        .count()
        + usize::from(!app.build_ui.running)
}

fn status_card(app: &mut App, ui: &mut egui::Ui, card: Rect, project: &Project, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    let tile = Rect::from_min_size(
        Pos2::new(card.left() + px(20.0), card.center().y - px(24.0)),
        Vec2::splat(px(48.0)),
    );
    let st = &app.build_ui;
    let elapsed = st.elapsed().as_secs();
    let clock = format!("{:02}:{:02}", elapsed / 60, elapsed % 60);
    let branch = selected_branch(app, project);
    let branch = if branch.is_empty() { "default".to_string() } else { branch };
    let number = build_number(app, project);
    // With no result from this session, the last recorded run decides the card.
    // A failed or cancelled run stays visible and opens its log in History.
    let last_unhappy = if st.running || st.outcome.is_some() {
        None
    } else {
        app.history_ui
            .latest_for(&project.id)
            .filter(|r| matches!(r.status, BuildStatus::Failed | BuildStatus::Cancelled))
            .cloned()
    };
    let (glyph, color, title, sub) = if let Some(rec) = &last_unhappy {
        let failed = rec.status == BuildStatus::Failed;
        let on = if rec.branch.is_empty() { "default" } else { rec.branch.as_str() };
        (
            theme::icon::X,
            if failed { theme::danger() } else { theme::warning() },
            if failed { "Last build failed" } else { "Last build cancelled" }.to_string(),
            format!("On {on} {}", crate::ui::pages::history::rel_time(rec.timestamp)),
        )
    } else if st.running && st.authorizing {
        (theme::icon::GEAR, theme::accent_hi(), "Authorizing account".to_string(), format!("{} elapsed", clock))
    } else if st.running {
        let (glyph, title) = match st.stage {
            Some(Stage::Connecting) | None => (theme::icon::ROCKET, "Connecting to Steam"),
            Some(Stage::LoggingIn) => (theme::icon::GEAR, "Logging in"),
            Some(Stage::Scanning) => (theme::icon::MAGNIFYING_GLASS, "Scanning content"),
            Some(Stage::Uploading) => (theme::icon::UPLOAD, "Uploading to Steam"),
            Some(Stage::Committing) | Some(Stage::Finished) => (theme::icon::PACKAGE, "Committing build"),
        };
        (glyph, theme::accent_hi(), title.to_string(), format!("Build {number} · {branch} · {clock} elapsed"))
    } else {
        match &st.outcome {
            Some(BuildResult::Success { .. }) => (
                theme::icon::CHECK,
                theme::success(),
                "Build finished".to_string(),
                format!("Build {number} · {branch} · took {clock}"),
            ),
            Some(BuildResult::Failure { .. }) => (
                theme::icon::X,
                theme::danger(),
                "Build failed".to_string(),
                format!("Build {number} · {branch} · after {clock}"),
            ),
            Some(BuildResult::Cancelled) => (
                theme::icon::X,
                theme::warning(),
                "Build cancelled".to_string(),
                format!("Build {number} · {branch} · after {clock}"),
            ),
            None => {
                let last = app
                    .history_ui
                    .latest_for(&project.id)
                    .map(|r| format!("last build {}", crate::ui::pages::history::rel_time(r.timestamp)))
                    .unwrap_or_else(|| "no builds yet".to_string());
                (
                    theme::icon::ROCKET,
                    theme::accent_hi(),
                    "Ready to build".to_string(),
                    format!("Build {} · {branch} · {last}", number),
                )
            }
        }
    };
    let links_to_log = last_unhappy.is_some();
    let spoken = format!("{title}. {sub}");
    if let Some(rec) = last_unhappy {
        let resp = ui.interact(card, ui.make_persistent_id("build_last_result"), egui::Sense::click());
        crate::ui::kit::accessible(
            &resp,
            card,
            egui::Role::Button,
            &format!("{spoken}. Open its log in History"),
            None,
            px(kit::RADIUS_CARD_PX),
        );
        // Same hover treatment as the project list rows.
        let hover = ui.ctx().animate_bool_with_time(
            ui.make_persistent_id("build_last_result_hover"),
            resp.hovered() || resp.has_focus(),
            0.12,
        );
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if hover > 0.01 {
            ui.painter().rect_filled(
                card,
                theme::corner(px(kit::RADIUS_CARD_PX)),
                theme::with_alpha(theme::surface_2(), (hover * 200.0) as u8),
            );
        }
        if resp.clicked() {
            app.history_ui.open_detail(rec);
            app.nav_request = Some(Page::History);
        }
    }
    let st = &app.build_ui;
    let painter = ui.painter();
    painter.rect_filled(tile, theme::corner(px(12.0)), theme::with_alpha(color, 34));
    painter.text(
        tile.center(),
        Align2::CENTER_CENTER,
        glyph.to_string(),
        theme::icon_font(px(22.0), false),
        color,
    );
    let tx = tile.right() + px(16.5);
    let clip = Rect::from_min_max(Pos2::new(tx, card.top()), Pos2::new(card.right() - px(14.0), card.bottom()));
    kit::fit_text(
        painter,
        Pos2::new(tx, card.center().y - px(10.5)),
        Align2::LEFT_CENTER,
        &title,
        FontId::new(theme::text_px(17.0, s), theme::family_semibold()),
        theme::ink(),
        clip.width(),
    );
    let sub_font = FontId::new(theme::text_px(13.0, s), theme::family_medium());
    let link_w = if links_to_log {
        painter.layout_no_wrap("View log".to_owned(), sub_font.clone(), theme::ink()).size().x + px(8.0)
    } else {
        0.0
    };
    let (sub_rect, _) = kit::fit_text(
        painter,
        Pos2::new(tx, card.center().y + px(12.0)),
        Align2::LEFT_CENTER,
        &sub,
        sub_font.clone(),
        theme::ink_muted(),
        clip.width() - link_w,
    );
    if !links_to_log {
        kit::readable(ui, card, egui::Role::Status, &spoken, true);
    }
    if links_to_log {
        // Link colour matches the other in-card links (e.g. the checklist template link).
        painter.with_clip_rect(clip).text(
            Pos2::new(sub_rect.right() + px(8.0), sub_rect.center().y),
            Align2::LEFT_CENTER,
            "View log",
            sub_font,
            theme::accent_hi(),
        );
    }
    if st.running {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

fn stat_tiles(app: &App, ui: &mut egui::Ui, row: Rect, project: &Project, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let gap = px(11.0);
    let w = (row.width() - gap * 3.0) / 4.0;
    let os: Vec<&str> = project
        .depots
        .iter()
        .filter_map(|d| {
            crate::features::importer::classify_os(&d.name)
                .or_else(|| crate::features::importer::classify_os(&d.local_path.to_string_lossy()))
        })
        .map(|o| match o {
            crate::features::importer::Os::Windows => "Win",
            crate::features::importer::Os::Mac => "Mac",
            crate::features::importer::Os::Linux => "Linux",
        })
        .collect();
    let depots = if os.is_empty() {
        project.depots.len().to_string()
    } else {
        format!("{} · {}", project.depots.len(), os.join(", "))
    };
    let changed = match app.build_ui.review.summary_for(&project.id) {
        Some(d) => (
            format!("+{} · −{} files", d.added + d.modified, d.removed),
            theme::ink(),
        ),
        None => ("Not scanned".to_string(), theme::ink_faint()),
    };
    let preflight = match app.build_ui.preflight.outcome() {
        Some((0, 0)) => ("Passed".to_string(), theme::success()),
        Some((0, w)) => (format!("{w} warning{}", if w == 1 { "" } else { "s" }), theme::warning()),
        Some((e, _)) => (format!("{e} blocking"), theme::danger()),
        None => ("Not run".to_string(), theme::ink_faint()),
    };
    let app_id = if project.app_id.is_empty() { "-".to_string() } else { project.app_id.clone() };
    let tiles = [
        ("App ID", app_id, theme::ink(), true),
        ("Depots", depots, theme::ink(), false),
        ("Changed", changed.0, changed.1, false),
        ("Preflight", preflight.0, preflight.1, false),
    ];
    for (i, (label, value, color, mono)) in tiles.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(row.left() + (w + gap) * i as f32, row.top()),
            Vec2::new(w, row.height()),
        );
        kit::card(ui.painter(), r, s);
        kit::readable(ui, r, egui::Role::Label, &format!("{label}: {value}"), false);
        kit::caps(
            ui.painter(),
            Pos2::new(r.left() + px(15.0), r.top() + px(24.0)),
            Align2::LEFT_CENTER,
            label,
            11.0,
            theme::ink_faint(),
            s,
        );
        let font = if *mono {
            FontId::new(theme::text_px(15.0, s), egui::FontFamily::Monospace)
        } else {
            FontId::new(theme::text_px(16.5, s), theme::family_semibold())
        };
        kit::fit_text(
            ui.painter(),
            Pos2::new(r.left() + px(15.0), r.top() + px(47.0)),
            Align2::LEFT_CENTER,
            value,
            font,
            *color,
            r.width() - px(30.0),
        );
    }
}

fn options_card(app: &mut App, ui: &mut egui::Ui, card: Rect, project: &Project, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    kit::card_title(ui, card, "Run options", s);
    let area = Rect::from_min_max(
        Pos2::new(card.left() + px(20.0), card.top() + px(44.0)),
        Pos2::new(card.right() - px(14.0), card.bottom() - px(10.0)),
    );
    let mut cui = ui.new_child(UiBuilder::new().max_rect(area));
    egui::ScrollArea::vertical()
        .id_salt("build_options")
        .auto_shrink([false, false])
        .show(&mut cui, |ui| {
            let w = area.width() - px(6.0);
            // Reserve only what is drawn below, so the panel scrolls only when it must.
            let cached = accounts::cached_login_for_project(project).is_some();
            let (r, _) = ui.allocate_exact_size(
                Vec2::new(w, options_content_h(app, project, s)),
                egui::Sense::hover(),
            );
            let x = r.left();
            let mut y = r.top();
            Input::new(&mut app.build_ui.description)
                .hint("Build description")
                .show(ui, Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, px(36.0))), s);
            y += px(50.0);
            let tog = Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(40.0), px(22.0)));
            kit::toggle(ui, tog, &mut app.build_ui.preview_only, "Preview only", s);
            ui.painter().text(
                Pos2::new(tog.right() + px(10.0), tog.center().y),
                Align2::LEFT_CENTER,
                "Preview only",
                FontId::new(theme::text_px(13.0, s), theme::family_medium()),
                theme::ink_dim(),
            );
            y += px(34.0);
            set_live_row(app, ui, Pos2::new(x, y), w, project, s);
            y += px(44.0);
            // A cached login is already shown in the sidebar footer; only a missing one
            // blocks the build, so only that case earns space here.
            if !cached {
                let auth = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, px(32.0)));
                let has_user = !project.steam_username.trim().is_empty();
                ui.painter().circle_filled(Pos2::new(x + px(4.0), auth.center().y), px(3.5), theme::warning());
                ui.painter().text(
                    Pos2::new(x + px(14.0), auth.center().y),
                    Align2::LEFT_CENTER,
                    if has_user { "No cached Steam login" } else { "No Steam account set" },
                    FontId::new(theme::text_px(12.5, s), theme::family_medium()),
                    theme::ink_dim(),
                );
                let bw = px(118.0);
                if Button::new("Authorize", ButtonKind::Secondary)
                    .enabled(has_user)
                    .disabled_reason("Pick an account in Settings > Accounts first")
                    .show(ui, Rect::from_min_size(Pos2::new(auth.right() - bw, auth.top()), Vec2::new(bw, auth.height())), s)
                    .clicked()
                    && has_user
                {
                    authorize(app, project, ctx);
                }
                y += px(46.0);
            }
            let more = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, px(34.0)));
            actions::more_actions(app, ui, more, project, s, ctx);
            y += px(46.0);
            stages::verify_offer(app, ui, Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, px(34.0))), s);
        });
}

/// Height of everything `options_card` draws below its title, in layout order:
/// description, Preview, Set live, the missing-login row, More actions, Verify.
fn options_content_h(app: &App, project: &Project, s: f32) -> f32 {
    let login_row = if accounts::cached_login_for_project(project).is_some() { 0.0 } else { 46.0 };
    let verify = if stages::verify_ready(app) { 12.0 + 34.0 } else { 0.0 };
    theme::px(50.0 + 34.0 + 44.0 + login_row + 34.0 + verify, s)
}

/// Card height for `options_card`: its 44px title band, the content, and bottom padding.
fn options_card_h(app: &App, project: &Project, s: f32) -> f32 {
    theme::px(44.0 + 20.0, s) + options_content_h(app, project, s)
}

/// The switch that decides whether this upload goes live on the chosen branch.
/// SteamCMD cannot set the default branch live, so that case is explained instead.
fn set_live_row(app: &mut App, ui: &mut egui::Ui, at: Pos2, w: f32, project: &Project, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let branch = selected_branch(app, project);
    let font = FontId::new(theme::text_px(13.0, s), theme::family_medium());
    let row = Rect::from_min_size(at, Vec2::new(w, px(22.0)));
    if branch.trim().is_empty() || branch.eq_ignore_ascii_case("default") {
        ui.painter().text(
            Pos2::new(at.x, row.center().y),
            Align2::LEFT_CENTER,
            "Uploads only. Set the default branch live in Steamworks.",
            FontId::new(theme::text_px(12.5, s), theme::family_medium()),
            theme::ink_muted(),
        );
        return;
    }
    let live_level = guard::is_live(project, &branch);
    let label = if live_level {
        format!("Set live on {branch} (Live branch)")
    } else {
        format!("Set live on {branch}")
    };
    let tog = Rect::from_min_size(at, Vec2::new(px(40.0), px(22.0)));
    kit::toggle(ui, tog, &mut app.build_ui.set_live, &label, s);
    let on = app.build_ui.set_live;
    let tx = tog.right() + px(10.0);
    let clip = Rect::from_min_max(Pos2::new(tx, row.top()), Pos2::new(row.right(), row.bottom()));
    let painter = ui.painter().with_clip_rect(clip);
    let suffix = "· upload only";
    let suffix_w = if on {
        0.0
    } else {
        painter.layout_no_wrap(suffix.to_owned(), font.clone(), theme::ink()).size().x + px(8.0)
    };
    let shown = kit::input::head_fit(&painter, &label, &font, (clip.width() - suffix_w).max(px(40.0)));
    let drawn = painter.text(
        Pos2::new(tx, row.center().y),
        Align2::LEFT_CENTER,
        &shown,
        font.clone(),
        if live_level && on { theme::warning() } else { theme::ink_dim() },
    );
    if !on {
        painter.text(
            Pos2::new(drawn.right() + px(8.0), row.center().y),
            Align2::LEFT_CENTER,
            suffix,
            font,
            theme::ink_muted(),
        );
    }
}

fn authorize(app: &mut App, project: &Project, ctx: &egui::Context) {
    if let Err(e) = app.build_ui.authorize(project, ctx) {
        app.build_ui
            .push_log(format!("Could not authorize: {e}"), true);
        app.toasts
            .push(kit::ToastLevel::Error, "Could not authorize");
    } else {
        app.toasts
            .push(kit::ToastLevel::Success, "Authorizing account");
    }
}

/// Run a recorded build again with the settings it used: the stored project
/// snapshot, branch, description, and its preview / set-live choice. Records from
/// before those choices were stored rebuild as a preview, never as a live push.
/// The Live-branch confirmation and preflight still apply.
pub(crate) fn rebuild(app: &mut App, rec: &crate::model::BuildRecord, ctx: &egui::Context) {
    let Ok(project) = serde_json::from_str::<Project>(&rec.settings_json) else {
        app.toasts.push(kit::ToastLevel::Error, "Stored settings are unreadable");
        return;
    };
    let Some(idx) = app.projects.iter().position(|p| p.id == project.id) else {
        app.toasts.push(kit::ToastLevel::Error, "That project no longer exists");
        return;
    };
    if app.build_ui.running {
        app.toasts.push(kit::ToastLevel::Warning, "A build is already running");
        return;
    }
    app.build_ui.project_sel = idx;
    app.build_ui.branch_sel = project
        .branches
        .iter()
        .position(|b| b.name == rec.branch)
        .unwrap_or(0);
    app.build_ui.description = rec.description.clone();
    app.build_ui.preview_only = rec.preview.unwrap_or(true);
    app.build_ui.set_live = rec.set_live.unwrap_or(false);
    let note = match rec.preview {
        None => "Rebuilding as a preview (this record predates saved run options)",
        Some(true) => "Rebuilding as a preview with saved settings",
        Some(false) => "Rebuilding with saved settings",
    };
    match guard::launch(app, &rec.branch, Some(project), ctx) {
        Ok(()) => app.toasts.push(kit::ToastLevel::Success, note),
        Err(e) => app.toasts.push(kit::ToastLevel::Error, e),
    }
}

pub(crate) fn build_reason(project: &Project) -> Option<String> {
    if let Some(e) = validate_project(project).first() {
        return Some(format!("{}: {}", e.field, e.message));
    }
    if project.depots.is_empty() {
        return Some("add at least one depot".to_string());
    }
    None
}
