use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::history;
use crate::model::{BuildRecord, BuildStatus, Project};
use crate::steamcmd::{self, BuildEvent, BuildHandle, BuildResult, PromptKind, Stage};

const MAX_LINES: usize = 5000;

pub const STAGES: [(Stage, &str); 6] = [
    (Stage::Connecting, "Connecting"),
    (Stage::LoggingIn, "Logging in"),
    (Stage::Scanning, "Scanning"),
    (Stage::Uploading, "Uploading"),
    (Stage::Committing, "Committing"),
    (Stage::Finished, "Finished"),
];

pub struct LogLine {
    pub text: String,
    pub error: bool,
    pub time: String,
}

pub struct BuildState {
    pub project_sel: usize,
    pub branch_sel: usize,
    pub description: String,
    pub preview_only: bool,
    pub verify: bool,
    pub set_live: bool,
    pub more_open: bool,
    pub log: VecDeque<LogLine>,
    pub search: String,
    pub stage: Option<Stage>,
    pub progress: f32,
    pub outcome: Option<BuildResult>,
    pub running: bool,
    pub authorizing: bool,
    pub inited: bool,
    pub pending_prompt: Option<PromptKind>,
    pub prompt_input: String,
    pub finished_record: Option<BuildRecord>,
    pub preflight: super::preflight::PreflightState,
    pub review: super::review::ReviewState,
    pub verify_ui: super::verify::VerifyState,
    pub pending_manifest_path: Option<PathBuf>,
    pub speed: super::speed::SpeedTracker,
    pub guard: super::guard::GuardState,
    pub snapshot: Option<Project>,
    pub stage_marks: Vec<(Stage, Instant)>,
    pub(super) interactive: bool,
    pub(super) run_project: Option<Project>,
    pub(super) run_branch: String,
    pub(super) run_desc: String,
    pub(super) run_preview: bool,
    pub(super) run_set_live: bool,
    pub(super) log_path: Option<PathBuf>,
    pub(super) started: Option<Instant>,
    pub(super) finished_elapsed: Option<Duration>,
    pub(super) handle: Option<BuildHandle>,
    pub(super) keepawake: Option<crate::keepawake::KeepAwake>,
    pub(super) tx: Sender<BuildEvent>,
    pub(super) rx: Receiver<BuildEvent>,
}

impl BuildState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            project_sel: 0,
            branch_sel: 0,
            description: String::new(),
            preview_only: false,
            verify: false,
            set_live: true,
            more_open: false,
            log: VecDeque::new(),
            search: String::new(),
            stage: None,
            progress: 0.0,
            outcome: None,
            running: false,
            authorizing: false,
            inited: false,
            pending_prompt: None,
            prompt_input: String::new(),
            finished_record: None,
            preflight: super::preflight::PreflightState::new(),
            review: super::review::ReviewState::new(),
            verify_ui: super::verify::VerifyState::new(),
            pending_manifest_path: None,
            speed: super::speed::SpeedTracker::new(),
            guard: super::guard::GuardState::new(),
            interactive: true,
            run_project: None,
            run_branch: String::new(),
            run_desc: String::new(),
            run_preview: false,
            run_set_live: false,
            snapshot: None,
            stage_marks: Vec::new(),
            log_path: None,
            started: None,
            finished_elapsed: None,
            handle: None,
            keepawake: None,
            tx,
            rx,
        }
    }

    pub fn push_log(&mut self, text: impl Into<String>, error: bool) {
        self.log.push_back(LogLine {
            text: text.into(),
            error,
            time: clock(),
        });
        while self.log.len() > MAX_LINES {
            self.log.pop_front();
        }
    }

    pub fn log_text(&self) -> String {
        self.log
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Only the lines flagged as errors, with their timestamps, for pasting into a report.
    pub fn error_text(&self) -> String {
        self.log
            .iter()
            .filter(|l| l.error)
            .map(|l| format!("{} {}", l.time, l.text))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn elapsed(&self) -> Duration {
        if self.running {
            self.started.map(|t| t.elapsed()).unwrap_or_default()
        } else {
            self.finished_elapsed.unwrap_or_default()
        }
    }

    pub fn filled_stages(&self) -> usize {
        match self.outcome {
            Some(BuildResult::Success { .. }) => STAGES.len(),
            _ => self
                .stage
                .and_then(|st| STAGES.iter().position(|(s, _)| *s == st))
                .map(|i| i + 1)
                .unwrap_or(0),
        }
    }

    pub fn stage_time(&self, stage: Stage) -> Option<Duration> {
        let i = self.stage_marks.iter().position(|(s, _)| *s == stage)?;
        let start = self.stage_marks[i].1;
        let end = match self.stage_marks.get(i + 1) {
            Some((_, t)) => *t,
            None if self.running => Instant::now(),
            None => self
                .started
                .zip(self.finished_elapsed)
                .map(|(s, d)| s + d)
                .unwrap_or(start),
        };
        Some(end.saturating_duration_since(start))
    }

    pub fn drain(&mut self, ctx: &egui::Context) -> Option<BuildResult> {
        let mut finished = None;
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                BuildEvent::Log(s) => {
                    if crate::steamcmd_patterns::mobile_confirmation(&s) {
                        self.push_log(
                            "Steam is waiting for you to approve this login in the Steam Mobile app.",
                            false,
                        );
                    }
                    let lower = s.to_ascii_lowercase();
                    let error = lower.contains("error") || lower.contains("failed");
                    self.push_log(s, error);
                }
                BuildEvent::Stage(st) => {
                    self.stage = Some(st);
                    if self.stage_marks.last().map(|(s, _)| *s) != Some(st) {
                        self.stage_marks.push((st, Instant::now()));
                    }
                }
                BuildEvent::Progress(f) => {
                    self.progress = f.clamp(0.0, 1.0);
                    self.speed.record(self.progress, self.started);
                }
                BuildEvent::Prompt(kind) => {
                    let msg = match kind {
                        PromptKind::Password => "SteamCMD is asking for a password.",
                        PromptKind::SteamGuard => "SteamCMD is asking for a Steam Guard code.",
                    };
                    self.push_log(msg, false);
                    if self.interactive {
                        self.pending_prompt = Some(kind);
                    } else {
                        self.push_log(
                            "Non-interactive run needs a login. Run Authorize first.",
                            true,
                        );
                        self.cancel();
                    }
                }
                BuildEvent::Finished(res) => {
                    let was_auth = self.authorizing;
                    self.running = false;
                    self.authorizing = false;
                    self.pending_prompt = None;
                    self.prompt_input.clear();
                    self.finished_elapsed = self.started.map(|t| t.elapsed());
                    self.handle = None;
                    self.keepawake = None;
                    if !was_auth && let Some(project) = self.run_project.clone() {
                        let status = match &res {
                            BuildResult::Success { .. } => BuildStatus::Success,
                            BuildResult::Failure { .. } => BuildStatus::Failed,
                            BuildResult::Cancelled => BuildStatus::Cancelled,
                        };
                        let duration_ms = self
                            .finished_elapsed
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);
                        let log = self.log_path.clone().unwrap_or_default();
                        let mut record = history::make_record(
                            &project,
                            &self.run_branch,
                            &self.run_desc,
                            status,
                            duration_ms,
                            log.as_path(),
                        );
                        record.git_commit = crate::features::patchnotes::head_commit(&project);
                        record.preview = Some(self.run_preview);
                        record.set_live = Some(self.run_set_live);
                        if status == BuildStatus::Success {
                            record.manifest_path = self.pending_manifest_path.take();
                        } else {
                            self.pending_manifest_path = None;
                        }
                        self.finished_record = Some(record);
                    }
                    match &res {
                        BuildResult::Success { app_id } if app_id.is_empty() => {
                            self.push_log("Signed in. SteamCMD cached the login.", false)
                        }
                        BuildResult::Success { app_id } => {
                            self.push_log(format!("Build finished for app {app_id}."), false)
                        }
                        BuildResult::Failure { message } => {
                            let prefix = if was_auth {
                                "Authorization failed"
                            } else {
                                "Build failed"
                            };
                            self.push_log(format!("{prefix}: {message}"), true)
                        }
                        BuildResult::Cancelled => self.push_log("Cancelled.", false),
                    }
                    self.outcome = Some(res.clone());
                    finished = Some(res);
                }
            }
        }
        if self.running {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        finished
    }

    pub fn respond(&mut self) {
        let Some(kind) = self.pending_prompt else {
            return;
        };
        let value = std::mem::take(&mut self.prompt_input);
        if let Some(handle) = &self.handle {
            handle.send_input(&value);
        }
        let note = match kind {
            PromptKind::Password => "Password sent.",
            PromptKind::SteamGuard => "Steam Guard code sent.",
        };
        self.push_log(note, false);
        self.pending_prompt = None;
    }

    pub fn cancel(&mut self) {
        self.pending_prompt = None;
        self.prompt_input.clear();
        if let Some(handle) = &self.handle {
            steamcmd::cancel(handle);
            self.push_log("Cancelling...", false);
        }
    }
}

fn clock() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let local = now + crate::features::schedule::local_offset_ms(now);
    let secs = local.div_euclid(1000).rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}
