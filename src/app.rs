use eframe::egui::{self, Align2, FontId, Pos2, UiBuilder};

use crate::model::Project;
use crate::ui::kit;
use crate::ui::kit::rail::RailAction;
use crate::ui::layout::{self, Layout};
use crate::ui::{anim, nav, pages, theme};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Page {
    #[default]
    Projects,
    Build,
    History,
    Automation,
    Settings,
}

pub struct App {
    pub(crate) page: Page,
    prev_page: Option<Page>,
    transition: Option<anim::Transition>,
    pub projects: Vec<Project>,
    pub(crate) sub_tabs: [usize; 5],
    pub(crate) projects_ui: pages::projects::ProjectsState,
    pub(crate) build_ui: pages::build::BuildState,
    pub(crate) history_ui: pages::history::HistoryState,
    pub(crate) backup_ui: pages::history::BackupState,
    pub(crate) patchnotes_ui: pages::history::PatchNotesState,
    pub(crate) analytics_ui: pages::history::AnalyticsState,
    pub(crate) pipeline_ui: pages::automation::PipelineState,
    pub(crate) runs_ui: pages::automation::RunsState,
    pub(crate) scheduler: pages::automation::SchedulerState,
    pub(crate) notify_ui: crate::features::notify::NotifyState,
    pub(crate) doctor_ui: pages::settings::DoctorState,
    pub(crate) advanced_ui: pages::settings::AdvancedState,
    pub(crate) developer_ui: pages::settings::DeveloperState,
    pub(crate) general_ui: pages::settings::GeneralState,
    pub(crate) accounts_ui: pages::settings::AccountsState,
    pub(crate) notifications_ui: pages::settings::NotificationsState,
    pub(crate) account_menu: bool,
    pub(crate) nav_request: Option<Page>,
    pub(crate) palette: crate::ui::palette::PaletteState,
    pub(crate) drop: crate::ui::dropimport::DropState,
    pub(crate) wizard: crate::ui::wizard::WizardState,
    pub(crate) confirm_quit: bool,
    pub(crate) quitting: bool,
    pub(crate) toasts: kit::Toasts,
    pub(crate) undo: crate::ui::undo::UndoStack,
    pub(crate) tray: Option<crate::ui::tray::Tray>,
    /// Set by the tray's Quit: the next close request really quits.
    pub(crate) force_quit: bool,
    taskbar: crate::ui::taskbar::Taskbar,
    /// A build or run failed while the window was not focused; the taskbar stays red until seen.
    failed_unseen: bool,
    login_status: (String, f64, bool),
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply_theme(&cc.egui_ctx);
        crate::ui::window::round_corners(cc);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let startup = crate::config::load_settings().unwrap_or_default();
        theme::set_mode(&cc.egui_ctx, startup.theme);
        anim::set_reduce_motion(startup.reduce_motion);
        if startup.ui_scale > 0.0 {
            theme::set_ui_scale(startup.ui_scale);
        }
        let retention = startup.log_retention_days;
        std::thread::spawn(move || {
            crate::features::maintenance::clean_old_logs(retention);
            crate::features::maintenance::prune_orphan_manifests();
        });
        let open_wizard = !startup.wizard_done;

        let projects = match crate::config::list_projects() {
            Ok(projects) => projects,
            Err(e) => {
                crate::config::report_problem(format!("Could not load projects: {e}"));
                Vec::new()
            }
        };
        crate::features::updates::spawn_check(&cc.egui_ctx);

        // Reopen where the user left off: page, sub-tabs and project.
        let page = nav::ITEMS
            .get(startup.last_page as usize)
            .map(|item| item.page)
            .unwrap_or_default();
        let mut sub_tabs = [0usize; 5];
        for ((slot, saved), item) in sub_tabs.iter_mut().zip(startup.last_sub_tabs).zip(nav::ITEMS.iter()) {
            *slot = (saved as usize).min(item.sub_tabs.len().saturating_sub(1));
        }
        let mut projects_ui = pages::projects::ProjectsState::new();
        if let Some(p) = startup
            .last_opened_project
            .as_ref()
            .and_then(|id| projects.iter().find(|p| &p.id == id))
        {
            projects_ui.select(p);
            projects_ui.inited = true;
        }

        Self {
            page: if open_wizard { Page::default() } else { page },
            prev_page: None,
            transition: None,
            projects,
            sub_tabs,
            projects_ui,
            build_ui: pages::build::BuildState::new(),
            history_ui: pages::history::HistoryState::new(),
            backup_ui: pages::history::BackupState::new(),
            patchnotes_ui: pages::history::PatchNotesState::new(),
            analytics_ui: pages::history::AnalyticsState::new(),
            pipeline_ui: pages::automation::PipelineState::new(),
            runs_ui: pages::automation::RunsState::default(),
            scheduler: pages::automation::SchedulerState::new(),
            notify_ui: crate::features::notify::NotifyState::new(),
            doctor_ui: pages::settings::DoctorState::new(),
            advanced_ui: pages::settings::AdvancedState::new(),
            developer_ui: pages::settings::DeveloperState::default(),
            general_ui: pages::settings::GeneralState::default(),
            accounts_ui: pages::settings::AccountsState::default(),
            notifications_ui: pages::settings::NotificationsState::default(),
            account_menu: false,
            nav_request: None,
            palette: crate::ui::palette::PaletteState::new(),
            drop: crate::ui::dropimport::DropState::new(),
            wizard: crate::ui::wizard::WizardState::new(open_wizard),
            confirm_quit: false,
            quitting: false,
            toasts: kit::Toasts::default(),
            undo: Default::default(),
            tray: None,
            force_quit: false,
            taskbar: crate::ui::taskbar::Taskbar::new(cc),
            failed_unseen: false,
            login_status: (String::new(), f64::NEG_INFINITY, false),
        }
    }

    fn goto(&mut self, ctx: &egui::Context, page: Page) {
        if page == self.page {
            return;
        }
        self.prev_page = Some(self.page);
        self.page = page;
        self.transition = Some(anim::Transition::new(ctx, 0.26, anim::ease_in_out_cubic));
    }

    fn render_page(
        &mut self,
        ui: &mut egui::Ui,
        page: Page,
        lay: &Layout,
        clip: egui::Rect,
        opacity: f32,
    ) {
        let mut cu = ui.new_child(
            UiBuilder::new()
                .max_rect(ui.max_rect())
                .id_salt(nav::item(page).label),
        );
        cu.set_opacity(opacity);
        cu.set_clip_rect(clip);
        pages::render(self, &mut cu, page, lay);
    }

    fn header_bar(&mut self, ui: &mut egui::Ui, lay: &Layout) {
        let s = lay.scale;
        let px = |v: f32| theme::px(v, s);
        let h = lay.header;
        let idx = nav::index(self.page);
        let item = nav::item(self.page);
        let title = ui.painter().text(
            Pos2::new(h.left(), h.center().y),
            Align2::LEFT_CENTER,
            item.label,
            FontId::new(theme::text_px(28.0, s), theme::family_semibold()),
            theme::ink(),
        );
        kit::readable(ui, title, egui::Role::Heading, item.label, false);
        if item.sub_tabs.is_empty() {
            return;
        }
        let font = FontId::new(theme::text_px(13.5, s), theme::family_medium());
        let pad = px(14.5);
        let inset = px(4.0);
        let widths: Vec<f32> = item
            .sub_tabs
            .iter()
            .map(|t| {
                ui.painter()
                    .layout_no_wrap((*t).to_owned(), font.clone(), theme::ink())
                    .size()
                    .x
                    + pad * 2.0
            })
            .collect();
        let total: f32 = widths.iter().sum::<f32>() + inset * 2.0;
        let bar = egui::Rect::from_min_size(
            Pos2::new(title.right() + px(24.0), h.center().y - px(18.5)),
            egui::vec2(total, px(37.0)),
        );
        ui.painter().rect_filled(
            bar,
            theme::corner(px(theme::RADIUS_FIELD)),
            theme::field(),
        );
        let id = ui.make_persistent_id(("header_tabs", idx));
        let active = self.sub_tabs[idx].min(widths.len() - 1);
        let target_x: f32 = widths[..active].iter().sum();
        let x = ui
            .ctx()
            .animate_value_with_time(id.with("x"), target_x, 0.18);
        let w = ui
            .ctx()
            .animate_value_with_time(id.with("w"), widths[active], 0.18);
        let pill = egui::Rect::from_min_size(
            Pos2::new(bar.left() + inset + x, bar.top() + inset),
            egui::vec2(w, bar.height() - inset * 2.0),
        );
        theme::shadow(ui.painter(), pill, px(8.0), 6, 40);
        ui.painter()
            .rect_filled(pill, theme::corner(px(8.0)), theme::surface_2());
        let mut left = bar.left() + inset;
        for (i, label) in item.sub_tabs.iter().enumerate() {
            let seg = egui::Rect::from_min_size(
                Pos2::new(left, pill.top()),
                egui::vec2(widths[i], pill.height()),
            );
            let resp = ui.interact(seg, id.with(i), egui::Sense::click());
            kit::accessible(&resp, seg, egui::Role::Tab, label, Some(i == active), px(8.0));
            if resp.clicked() {
                self.sub_tabs[idx] = i;
            }
            let color = if i == active {
                theme::ink()
            } else if resp.hovered() {
                theme::ink_dim()
            } else {
                theme::ink_muted()
            };
            ui.painter()
                .text(seg.center(), Align2::CENTER_CENTER, *label, font.clone(), color);
            left += widths[i];
        }
    }

    fn login_cached(&mut self, ctx: &egui::Context) -> bool {
        let Some(project) = self.projects.get(self.build_ui.project_sel) else {
            return false;
        };
        let now = ctx.input(|i| i.time);
        let key = project.steam_username.clone();
        if self.login_status.0 != key || now - self.login_status.1 > 5.0 {
            let cached = crate::features::accounts::cached_login_for_project(project).is_some();
            self.login_status = (key, now, cached);
        }
        self.login_status.2
    }
}

impl App {
    /// Hide the window to the notification area ("close to tray").
    pub(crate) fn hide_to_tray(&mut self, ctx: &egui::Context) {
        if self.tray.is_none() {
            self.tray = crate::ui::tray::Tray::spawn(ctx);
        }
        if self.tray.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else {
            // No tray support here: minimise instead so the app is never lost.
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    fn show_from_tray(&mut self, ctx: &egui::Context) {
        self.tray = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn pump_tray(&mut self, ctx: &egui::Context) {
        let events = self.tray.as_ref().map(|t| t.poll()).unwrap_or_default();
        for ev in events {
            match ev {
                crate::ui::tray::TrayEvent::Show => self.show_from_tray(ctx),
                crate::ui::tray::TrayEvent::Quit => {
                    // Show the window first so the quit guard can ask about running work.
                    self.show_from_tray(ctx);
                    self.force_quit = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    fn update_taskbar(&mut self, ctx: &egui::Context) {
        use crate::ui::taskbar::Progress;
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        if focused {
            self.failed_unseen = false;
        }
        let b = &self.build_ui;
        let progress = if b.running && !b.authorizing {
            match b.stage {
                Some(crate::steamcmd::Stage::Uploading) if b.progress > 0.0 => {
                    Progress::Normal(b.progress)
                }
                _ => Progress::Indeterminate,
            }
        } else if self.pipeline_ui.running || self.scheduler.busy() || b.verify_ui.running() {
            Progress::Indeterminate
        } else if self.failed_unseen {
            Progress::Error
        } else {
            Progress::None
        };
        self.taskbar.set(progress);
    }

    /// Called when a build or pipeline run ends: sound, and a red taskbar if unseen.
    pub(crate) fn on_run_finished(&mut self, ctx: &egui::Context, success: bool) {
        crate::features::sound::on_finish(ctx, success);
        if !success && !ctx.input(|i| i.viewport().focused.unwrap_or(true)) {
            self.failed_unseen = true;
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::sync(ctx);
        crate::ui::pages::build::pump(self, ctx);
        crate::ui::pages::history::pump(self, ctx);
        crate::ui::pages::settings::pump(self);
        let pipeline_was_running = self.pipeline_ui.running;
        for (level, msg) in self.pipeline_ui.drain(ctx) {
            self.toasts.push(level, msg);
        }
        if pipeline_was_running && !self.pipeline_ui.running {
            use crate::features::pipeline::PipelineOutcome;
            match self.pipeline_ui.outcome {
                Some(PipelineOutcome::Completed) => self.on_run_finished(ctx, true),
                Some(PipelineOutcome::Failed) => self.on_run_finished(ctx, false),
                // A cancel is the user's own doing: no error sound or red taskbar.
                _ => {}
            }
        }
        let pipeline_busy =
            self.pipeline_ui.running || self.build_ui.running || self.build_ui.verify_ui.running();
        for (level, msg) in self.scheduler.tick(&self.projects, pipeline_busy, ctx) {
            self.toasts.push(level, msg);
        }
        for msg in self.notify_ui.drain() {
            self.toasts.push(kit::ToastLevel::Error, msg);
        }
        for msg in crate::config::take_problems() {
            self.toasts.push(kit::ToastLevel::Error, msg);
        }
        if let Some(update) = crate::features::updates::take_found() {
            let url = update.url.clone();
            self.offer_action(
                kit::ToastLevel::Success,
                format!("Onyx {} is available", update.version),
                "Open",
                false,
                true,
                move |_| {
                    let _ = open::that(url);
                    Ok(())
                },
            );
        }
        self.pump_undo();
        self.pump_tray(ctx);
        crate::ui::quitguard::intercept_close(self, ctx);
        self.update_taskbar(ctx);
    }

    fn on_exit(&mut self) {
        let Ok(mut settings) = crate::config::load_settings() else {
            return;
        };
        settings.last_page = nav::index(self.page) as u8;
        for (slot, tab) in settings.last_sub_tabs.iter_mut().zip(self.sub_tabs) {
            *slot = tab.min(u8::MAX as usize) as u8;
        }
        settings.last_opened_project = self
            .projects_ui
            .selected
            .clone()
            .or_else(|| self.projects.get(self.build_ui.project_sel).map(|p| p.id.clone()));
        let _ = crate::config::save_settings(&settings);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let lay = layout::compute(ui.max_rect());
                let s = lay.scale;

                ui.painter()
                    .rect_filled(ui.max_rect(), theme::corner(0.0), theme::sidebar());
                crate::ui::window::titlebar(ui, lay.titlebar, s);

                let account = self
                    .projects
                    .get(self.build_ui.project_sel)
                    .map(|p| {
                        if p.default_account.is_empty() {
                            p.steam_username.clone()
                        } else {
                            p.default_account.clone()
                        }
                    })
                    .unwrap_or_default();
                let initials: String = account
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(2)
                    .collect::<String>()
                    .to_uppercase();
                let cached = self.login_cached(&ctx);
                let rail_state = kit::rail::RailState {
                    active: self.page,
                    initials: &initials,
                    account: &account,
                    cached,
                    brand_y: lay.header.center().y,
                };
                match kit::rail(ui, lay.rail, rail_state, s) {
                    Some(RailAction::Navigate(page)) => self.goto(&ctx, page),
                    Some(RailAction::Search) => self.palette.toggle(),
                    Some(RailAction::Avatar) => self.account_menu = !self.account_menu,
                    None => {}
                }
                self.header_bar(ui, &lay);

                let blocked = kit::modal_active(&ctx)
                    || ctx.egui_wants_keyboard_input()
                    || self.wizard.open
                    || self.account_menu;
                if !blocked && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
                    self.palette.toggle();
                }
                // Ctrl+Enter builds, Ctrl+Shift+Enter runs a preview. Both go through the
                // normal launch path, so the Live confirmation and preflight still apply.
                if !blocked
                    && !self.palette.open
                    && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter))
                {
                    // Never turn a preview into a real upload: keep the toggle if it is on.
                    let preview = ctx.input(|i| i.modifiers.shift) || self.build_ui.preview_only;
                    crate::ui::palette::start_build(self, preview, &ctx);
                }

                let mut nav_to = None;
                if !blocked && !self.palette.open {
                    for item in nav::ITEMS.iter() {
                        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(item.shortcut)) {
                            nav_to = Some(item.page);
                        }
                    }
                }
                if let Some(page) = nav_to {
                    self.goto(&ctx, page);
                }

                let clip = lay.header.union(lay.body).expand(theme::px(24.0, s));
                if let Some(tr) = self.transition {
                    let t = tr.progress(&ctx);
                    if let Some(prev) = self.prev_page {
                        self.render_page(ui, prev, &lay, clip, 1.0 - t);
                    }
                    self.render_page(ui, self.page, &lay, clip, t);
                    if tr.is_done(&ctx) {
                        self.transition = None;
                        self.prev_page = None;
                    }
                } else {
                    self.render_page(ui, self.page, &lay, clip, 1.0);
                }

                crate::ui::dropimport::show(self, ui, lay.body, s);
                crate::ui::palette::show(self, ui, s);
                crate::ui::accountmenu::show(self, ui, lay.rail, s);
                crate::ui::wizard::show(self, ui, s);
                crate::ui::quitguard::show(self, ui, s);

                if let Some(page) = self.nav_request.take() {
                    self.goto(&ctx, page);
                }

                self.toasts.show(ui, lay.toast_anchor, s);
                crate::ui::window::resize_edges(ui, ui.max_rect(), s);
            });
    }
}
