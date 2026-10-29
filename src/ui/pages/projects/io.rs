use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use crate::app::App;
use crate::config;
use crate::model::Project;
use crate::ui::kit;

pub enum IoResult {
    Imported(Box<Project>),
    Exported,
    Error(String),
}

pub struct ProjectIo {
    tx: Sender<IoResult>,
    rx: Receiver<IoResult>,
}

impl Default for ProjectIo {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx }
    }
}

impl ProjectIo {
    pub fn drain(&self) -> Vec<IoResult> {
        let mut out = Vec::new();
        while let Ok(res) = self.rx.try_recv() {
            out.push(res);
        }
        out
    }

    pub fn spawn_import(&self, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let picked = rfd::FileDialog::new()
                .add_filter("Steam VDF", &["vdf"])
                .add_filter("Project JSON", &["json"])
                .pick_file();
            if let Some(path) = picked {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let result = if ext == "json" {
                    std::fs::read_to_string(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|t| {
                            crate::features::profile::import_json(&t).map_err(|e| e.to_string())
                        })
                } else {
                    crate::features::import_vdf::import_appbuild(&path).map_err(|e| e.to_string())
                };
                let _ = tx.send(match result {
                    Ok(project) => IoResult::Imported(Box::new(project)),
                    Err(e) => IoResult::Error(e),
                });
                ctx.request_repaint();
            }
        });
    }

    pub fn spawn_export(&self, project: Project, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let name = if project.name.is_empty() {
                "project".to_string()
            } else {
                project.name.clone()
            };
            let picked = rfd::FileDialog::new()
                .add_filter("Project JSON", &["json"])
                .set_file_name(format!("{name}.json"))
                .save_file();
            if let Some(path) = picked {
                let result = crate::features::profile::export_json(&project)
                    .map_err(|e| e.to_string())
                    .and_then(|json| std::fs::write(&path, json).map_err(|e| e.to_string()));
                let _ = tx.send(match result {
                    Ok(()) => IoResult::Exported,
                    Err(e) => IoResult::Error(e),
                });
                ctx.request_repaint();
            }
        });
    }
}

pub(super) fn handle(app: &mut App, res: IoResult) {
    match res {
        IoResult::Imported(project) => {
            let project = *project;
            match config::save_project(&project) {
                Ok(()) => {
                    super::reload(app);
                    app.projects_ui.select(&project);
                    let name = if project.name.is_empty() {
                        "project".to_string()
                    } else {
                        project.name.clone()
                    };
                    app.toasts
                        .push(kit::ToastLevel::Success, format!("Imported {name}"));
                }
                Err(e) => app
                    .toasts
                    .push(kit::ToastLevel::Error, format!("Import failed: {e}")),
            }
        }
        IoResult::Exported => app
            .toasts
            .push(kit::ToastLevel::Success, "Project exported"),
        IoResult::Error(e) => app.toasts.push(kit::ToastLevel::Error, e),
    }
}
