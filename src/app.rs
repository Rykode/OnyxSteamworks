use eframe::egui::{self, Align2, FontId, Pos2, UiBuilder};

use std::path::{Path, PathBuf};

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
    login_status: (String, f64, bool),
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, open_folder: Option<PathBuf>) -> Self {
        let mut app = Self::create(cc);
        if let Some(folder) = open_folder {
            app.open_folder(folder, &cc.egui_ctx);
        }
        app
    }

    /// Shows the project whose content folder holds `folder` on the Build page, or offers
    /// to create a project from the folder when no project does.
    fn open_folder(&mut self, folder: PathBuf, ctx: &egui::Context) {
        if !folder.is_dir() {
            self.toasts.push(
                kit::ToastLevel::Error,
                format!("Folder not found: {}", folder.display()),
            );
            return;
        }
        match project_for_folder(&self.projects, &folder).cloned() {
            Some(project) => {
                self.projects_ui.select(&project);
                self.nav_request = Some(Page::Build);
            }
            None => crate::ui::dropimport::begin(self, folder, ctx),
        }
    }

    fn create(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply_theme(&cc.egui_ctx);
        crate::ui::window::round_corners(cc);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let startup = crate::config::load_settings().unwrap_or_default();
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

        Self {
            page: Page::default(),
            prev_page: None,
            transition: None,
            projects,
            sub_tabs: [0; 5],
            projects_ui: pages::projects::ProjectsState::new(),
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
            theme::INK,
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
                    .layout_no_wrap((*t).to_owned(), font.clone(), theme::INK)
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
            theme::FIELD,
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
            .rect_filled(pill, theme::corner(px(8.0)), theme::SURFACE_2);
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
                theme::INK
            } else if resp.hovered() {
                theme::INK_DIM
            } else {
                theme::INK_MUTED
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

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::ui::pages::build::pump(self, ctx);
        crate::ui::pages::history::pump(self, ctx);
        crate::ui::pages::settings::pump(self);
        for (level, msg) in self.pipeline_ui.drain(ctx) {
            self.toasts.push(level, msg);
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
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let lay = layout::compute(ui.max_rect());
                let s = lay.scale;

                ui.painter()
                    .rect_filled(ui.max_rect(), theme::corner(0.0), theme::SIDEBAR);
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
                if !blocked
                    && !self.palette.open
                    && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter))
                {
                    crate::ui::palette::start_build(self, false, &ctx);
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

/// The project whose content folder is `folder` or contains it. When content folders
/// nest, the deepest one wins.
fn project_for_folder<'a>(projects: &'a [Project], folder: &Path) -> Option<&'a Project> {
    let folder = normalize(folder);
    projects
        .iter()
        .filter(|p| !p.content_root.as_os_str().is_empty())
        .map(|p| (p, normalize(&p.content_root)))
        .filter(|(_, root)| folder.starts_with(root))
        .max_by_key(|(_, root)| root.components().count())
        .map(|(p, _)| p)
}

fn normalize(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(id: &str, root: &Path) -> Project {
        Project {
            id: id.into(),
            content_root: root.to_path_buf(),
            ..Default::default()
        }
    }

    #[test]
    fn folder_matches_deepest_content_root() {
        let base = std::env::temp_dir().join(format!("onyx-open-{}", uuid::Uuid::new_v4()));
        let inner = base.join("game");
        let export = inner.join("windows");
        std::fs::create_dir_all(&export).unwrap();
        let projects = vec![
            project("outer", &base),
            project("inner", &inner),
            project("empty", Path::new("")),
        ];
        assert_eq!(project_for_folder(&projects, &export).map(|p| p.id.as_str()), Some("inner"));
        assert_eq!(project_for_folder(&projects, &inner).map(|p| p.id.as_str()), Some("inner"));
        assert_eq!(project_for_folder(&projects, &std::env::temp_dir()).map(|p| p.id.as_str()), None);
        std::fs::remove_dir_all(&base).unwrap();
    }
}
