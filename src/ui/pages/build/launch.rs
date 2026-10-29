use std::sync::mpsc::channel;
use std::time::Instant;

use eframe::egui;

use super::state::BuildState;
use crate::model::Project;
use crate::steamcmd::{self, BuildHandle};
use crate::vdf::{self, BuildOptions};

impl BuildState {
    pub fn start(
        &mut self,
        project: &Project,
        branch: &str,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        let lock = crate::features::filelock::upload(&format!("Onyx build of {}", project.name))?;
        let work = crate::config::build_dir().map_err(|e| e.to_string())?;
        let options = BuildOptions {
            preview: self.preview_only,
            local: None,
            branch: branch.to_string(),
            desc: self.description.clone(),
        };
        let paths = vdf::write_vdfs(project, &options, &work).map_err(|e| e.to_string())?;
        let run_desc = vdf::resolved_desc(project, &options);
        let app_build = paths
            .iter()
            .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("appbuild.vdf"))
            .cloned()
            .ok_or_else(|| "no appbuild.vdf was generated".to_string())?;

        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        let opts = steamcmd::RunOptions {
            app_build_vdf: app_build,
            log_path: None,
            interactive: true,
        };
        let mut handle = steamcmd::start_build(project, &opts, ctx, self.tx.clone())
            .map_err(|e| e.to_string())?;
        handle.hold(lock);

        self.begin(handle);
        self.authorizing = false;
        self.run_project = Some(project.clone());
        self.run_branch = branch.to_string();
        self.run_desc = run_desc;
        self.run_preview = self.preview_only;
        if crate::features::accounts::cached_login_for_project(project).is_some() {
            self.push_log("Using cached Steam login.", false);
        }
        let note = if self.preview_only {
            "Starting preview build..."
        } else {
            "Starting build..."
        };
        self.push_log(note, false);
        Ok(())
    }

    pub fn authorize(&mut self, project: &Project, ctx: &egui::Context) -> Result<(), String> {
        let lock = crate::features::filelock::upload("Onyx account authorization")?;
        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        let mut handle =
            steamcmd::start_login(project, ctx, self.tx.clone()).map_err(|e| e.to_string())?;
        handle.hold(lock);
        self.begin(handle);
        self.authorizing = true;
        self.push_log("Authorizing account. Answer any prompt below...", false);
        Ok(())
    }

    fn begin(&mut self, handle: BuildHandle) {
        self.log_path = Some(handle.log_path.clone());
        self.handle = Some(handle);
        self.finished_record = None;
        self.log.clear();
        self.outcome = None;
        self.stage = None;
        self.progress = 0.0;
        self.pending_prompt = None;
        self.prompt_input.clear();
        self.started = Some(Instant::now());
        self.stage_marks.clear();
        self.finished_elapsed = None;
        self.running = true;
        self.speed.reset();
        self.keepawake = Some(crate::keepawake::KeepAwake::new());
    }
}
