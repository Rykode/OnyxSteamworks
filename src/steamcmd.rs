use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::model::Project;

mod reader;
#[cfg(test)]
mod tests;

use reader::{Parsed, finish, spawn_reader};

pub const INACTIVITY: std::time::Duration = std::time::Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Connecting,
    LoggingIn,
    Scanning,
    Uploading,
    Committing,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Password,
    SteamGuard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildResult {
    Success { app_id: String },
    Failure { message: String },
    Cancelled,
}

pub enum BuildEvent {
    Log(String),
    Stage(Stage),
    Progress(f32),
    Prompt(PromptKind),
    Finished(BuildResult),
}

#[derive(Debug, thiserror::Error)]
pub enum SteamError {
    #[error("failed to launch steamcmd: {0}")]
    Spawn(#[source] std::io::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct RunOptions {
    pub app_build_vdf: PathBuf,
    pub log_path: Option<PathBuf>,
    pub interactive: bool,
}

pub struct BuildHandle {
    tree: crate::proctree::Tree,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    cancelled: Arc<AtomicBool>,
    lock: Option<crate::features::filelock::Held>,
    pub log_path: PathBuf,
}

impl BuildHandle {
    pub fn hold(&mut self, lock: crate::features::filelock::Held) {
        self.lock = Some(lock);
    }

    pub fn send_input(&self, text: &str) {
        if let Ok(mut guard) = self.stdin.lock()
            && let Some(stdin) = guard.as_mut()
        {
            let _ = writeln!(stdin, "{text}");
            let _ = stdin.flush();
        }
    }
}

#[derive(Clone)]
pub(crate) struct Autofill {
    password: Arc<Mutex<Option<String>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
}

impl Autofill {
    pub(crate) fn answer_password(&self) -> bool {
        let Some(password) = self.password.lock().ok().and_then(|mut p| p.take()) else {
            return false;
        };
        let Ok(mut guard) = self.stdin.lock() else {
            return false;
        };
        let Some(stdin) = guard.as_mut() else {
            return false;
        };
        writeln!(stdin, "{password}").is_ok() && stdin.flush().is_ok()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoginMode {
    PreferCached,
    Refresh,
}

pub(crate) fn command_for(exe: &Path) -> Command {
    let mut cmd = Command::new(exe);
    if let Some(dir) = exe.parent().filter(|p| !p.as_os_str().is_empty()) {
        cmd.current_dir(dir);
    }
    cmd
}

fn stored_password(exe: &Path, project: &Project, mode: LoginMode) -> Option<String> {
    let login = project.steam_username.trim();
    if login.is_empty() {
        return None;
    }
    if mode == LoginMode::PreferCached
        && crate::features::accounts::cached_login(exe, login).is_some()
    {
        return None;
    }
    crate::secrets::get_password(&crate::secrets::login_key(login))
        .ok()
        .flatten()
        .filter(|p| !p.is_empty())
}

fn append_login(cmd: &mut Command, project: &Project) {
    let login = project.steam_username.trim();
    if !login.is_empty() {
        cmd.arg("+login").arg(login);
    }
}

fn log_path_for(explicit: Option<&PathBuf>, prefix: &str) -> Result<PathBuf, SteamError> {
    match explicit {
        Some(p) => Ok(p.clone()),
        None => {
            let dir = crate::config::log_dir()
                .map_err(|e| SteamError::Io(std::io::Error::other(e.to_string())))?;
            Ok(dir.join(format!("{prefix}-{}.log", uuid::Uuid::new_v4())))
        }
    }
}

pub fn start_build(
    project: &Project,
    options: &RunOptions,
    ctx: &egui::Context,
    tx: Sender<BuildEvent>,
) -> Result<BuildHandle, SteamError> {
    let log_path = log_path_for(options.log_path.as_ref(), "build")?;
    let exe = crate::features::maintenance::effective_steamcmd(project);
    let password = stored_password(&exe, project, LoginMode::PreferCached);
    let mut cmd = command_for(&exe);
    cmd.arg("+@ShutdownOnFailedCommand").arg("1");
    if !options.interactive && password.is_none() {
        cmd.arg("+@NoPromptForPassword").arg("1");
    }
    append_login(&mut cmd, project);
    cmd.arg("+run_app_build")
        .arg(&options.app_build_vdf)
        .arg("+quit");
    run(cmd, log_path, ctx, tx, password)
}

pub fn start_login(
    project: &Project,
    ctx: &egui::Context,
    tx: Sender<BuildEvent>,
) -> Result<BuildHandle, SteamError> {
    let log_path = log_path_for(None, "login")?;
    let exe = crate::features::maintenance::effective_steamcmd(project);
    // A revoked token makes SteamCMD fail with Access Denied without ever
    // prompting, so Authorize always starts from a fresh password login.
    crate::features::accounts::forget_cached_token(&exe, &project.steam_username)?;
    let password = stored_password(&exe, project, LoginMode::Refresh);
    let mut cmd = command_for(&exe);
    cmd.arg("+@ShutdownOnFailedCommand").arg("1");
    append_login(&mut cmd, project);
    cmd.arg("+quit");
    run(cmd, log_path, ctx, tx, password)
}

pub struct VerifyOptions {
    pub app_id: String,
    pub install_dir: PathBuf,
    pub branch: String,
    pub beta_password: String,
    pub log_path: Option<PathBuf>,
}

pub fn start_verify(
    project: &Project,
    options: &VerifyOptions,
    ctx: &egui::Context,
    tx: Sender<BuildEvent>,
) -> Result<BuildHandle, SteamError> {
    let log_path = log_path_for(options.log_path.as_ref(), "verify")?;
    let exe = crate::features::maintenance::effective_steamcmd(project);
    let password = stored_password(&exe, project, LoginMode::PreferCached);
    let mut cmd = command_for(&exe);
    cmd.arg("+@ShutdownOnFailedCommand").arg("1");
    if password.is_none() {
        cmd.arg("+@NoPromptForPassword").arg("1");
    }
    cmd.arg("+force_install_dir").arg(&options.install_dir);
    append_login(&mut cmd, project);
    cmd.arg("+app_update").arg(&options.app_id);
    let branch = options.branch.trim();
    if !branch.is_empty() && branch != "default" {
        cmd.arg("-beta").arg(branch);
        if !options.beta_password.trim().is_empty() {
            cmd.arg("-betapassword").arg(options.beta_password.trim());
        }
    }
    cmd.arg("validate").arg("+quit");
    run(cmd, log_path, ctx, tx, password)
}

fn run(
    mut cmd: Command,
    log_path: PathBuf,
    ctx: &egui::Context,
    tx: Sender<BuildEvent>,
    password: Option<String>,
) -> Result<BuildHandle, SteamError> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let log = Arc::new(Mutex::new(std::fs::File::create(&log_path)?));

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    crate::proctree::prepare(&mut cmd);
    let mut child = cmd.spawn().map_err(SteamError::Spawn)?;
    let tree = crate::proctree::Tree::attach(&child);
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = Arc::new(Mutex::new(child.stdin.take()));

    let cancelled = Arc::new(AtomicBool::new(false));
    let parsed = Arc::new(Mutex::new(Parsed::default()));
    let autofill = Autofill {
        password: Arc::new(Mutex::new(password)),
        stdin: stdin.clone(),
    };

    let mut readers = Vec::new();
    if let Some(out) = stdout {
        readers.push(spawn_reader(
            out,
            tx.clone(),
            ctx.clone(),
            log.clone(),
            parsed.clone(),
            autofill.clone(),
            true,
        ));
    }
    if let Some(err) = stderr {
        readers.push(spawn_reader(
            err,
            tx.clone(),
            ctx.clone(),
            log.clone(),
            parsed.clone(),
            autofill.clone(),
            false,
        ));
    }

    let waiter_cancelled = cancelled.clone();
    let waiter_ctx = ctx.clone();
    std::thread::spawn(move || {
        for r in readers {
            let _ = r.join();
        }
        let code = child.wait().ok().and_then(|s| s.code());
        let result = if waiter_cancelled.load(Ordering::SeqCst) {
            BuildResult::Cancelled
        } else {
            finish(&parsed, code)
        };
        let _ = tx.send(BuildEvent::Finished(result));
        waiter_ctx.request_repaint();
    });

    Ok(BuildHandle {
        tree,
        stdin,
        cancelled,
        lock: None,
        log_path,
    })
}

pub fn cancel(handle: &BuildHandle) {
    handle.cancelled.store(true, Ordering::SeqCst);
    handle.tree.kill();
}
