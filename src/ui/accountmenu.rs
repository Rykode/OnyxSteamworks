use eframe::egui::{self, Rect};

use crate::app::{App, Page};
use crate::ui::kit;
use crate::ui::nav;

pub fn show(app: &mut App, ui: &mut egui::Ui, rail: Rect, s: f32) {
    if !app.account_menu {
        return;
    }
    let accounts = crate::config::load_settings()
        .map(|c| c.accounts)
        .unwrap_or_default();
    let sel = app.build_ui.project_sel;
    let current = app
        .projects
        .get(sel)
        .map(|p| p.default_account.clone())
        .unwrap_or_default();
    let row = kit::rail::account_row(rail, s);
    let manage_label = "Manage accounts \u{2192}";
    let items: Vec<kit::MenuItem> = accounts
        .iter()
        .map(|name| kit::MenuItem::new(name).checked(!current.is_empty() && name.eq_ignore_ascii_case(&current)))
        .chain(std::iter::once(kit::MenuItem::new(manage_label)))
        .collect();
    let id = ui.make_persistent_id("account_menu");
    match kit::menu(ui, id, row, row.width(), Some("Switch account"), &items, s) {
        Some(kit::MenuEvent::Picked(i)) if i < accounts.len() => {
            let name = accounts[i].clone();
            set_account(app, sel, &name);
            app.account_menu = false;
        }
        Some(kit::MenuEvent::Picked(_)) => {
            app.sub_tabs[nav::index(Page::Settings)] = 1;
            app.nav_request = Some(Page::Settings);
            app.account_menu = false;
        }
        Some(kit::MenuEvent::Closed) => app.account_menu = false,
        None => {}
    }
}

fn set_account(app: &mut App, sel: usize, account: &str) {
    if let Some(p) = app.projects.get_mut(sel) {
        p.steam_username = account.to_string();
        p.default_account = account.to_string();
        let project = p.clone();
        let _ = crate::config::save_project(&project);
        app.toasts
            .push(kit::ToastLevel::Success, format!("Switched to {account}"));
    }
}
