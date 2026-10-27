use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use eframe::egui;

use super::exec::{base_dir, emit, parse_env, split_args};
use super::{Control, PipelineEvent, StepStatus};
use crate::model::{PipelineStep, Project};
use crate::steamcmd::{self, BuildEvent, BuildResult};
use crate::vdf::{self, BuildOptions};

pub(super) fn run_command(
    project: &Project,
    i: usize,
    step: &PipelineStep,
    control: &Control,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    if step.exe.trim().is_empty() {
        emit(tx, ctx, i, "No executable set for this step.", true);
        return StepStatus::Failed;
    }
    run_process(
        step.exe.trim(),
        &split_args(&step.args),
        &base_dir(project, step),
        step,
        i,
        control,
        ctx,
        tx,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_process(
    exe: &str,
    args: &[String],
    dir: &Path,
    step: &PipelineStep,
    i: usize,
    control: &Control,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    cmd.current_dir(dir);
    for (k, v) in parse_env(&step.env) {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    crate::proctree::prepare(&mut cmd);
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            emit(tx, ctx, i, format!("Could not launch {exe}: {e}"), true);
            return StepStatus::Failed;
        }
    };
    let tree = crate::proctree::Tree::attach(&child);
    if let Ok(mut pid) = control.child_pid.lock() {
        *pid = Some(child.id());
    }
    let (ltx, lrx) = channel::<(bool, String)>();
    let readers = [
        child
            .stdout
            .take()
            .map(|r| spawn_lines(r, ltx.clone(), false)),
        child
            .stderr
            .take()
            .map(|r| spawn_lines(r, ltx.clone(), true)),
    ];
    drop(ltx);

    let start = Instant::now();
    let timeout = (step.timeout_secs > 0).then(|| Duration::from_secs(step.timeout_secs as u64));
    let status = loop {
        while let Ok((err, line)) = lrx.try_recv() {
            emit(tx, ctx, i, line, err);
        }
        if control.cancelled.load(Ordering::SeqCst) {
            tree.kill();
            let _ = child.wait();
            break StepStatus::Failed;
        }
        if let Some(t) = timeout
            && start.elapsed() > t
        {
            emit(tx, ctx, i, "Step timed out.", true);
            tree.kill();
            let _ = child.wait();
            break StepStatus::Failed;
        }
        match child.try_wait() {
            Ok(Some(exit)) => {
                while let Ok((err, line)) = lrx.try_recv() {
                    emit(tx, ctx, i, line, err);
                }
                break if exit.success() {
                    StepStatus::Success
                } else {
                    emit(tx, ctx, i, format!("Exited with {exit}."), true);
                    StepStatus::Failed
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(60)),
            Err(_) => break StepStatus::Failed,
        }
    };
    drop(tree);
    let deadline = Instant::now() + Duration::from_secs(5);
    for r in readers.into_iter().flatten() {
        while !r.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        if r.is_finished() {
            let _ = r.join();
        }
    }
    while let Ok((err, line)) = lrx.try_recv() {
        emit(tx, ctx, i, line, err);
    }
    if let Ok(mut pid) = control.child_pid.lock() {
        *pid = None;
    }
    status
}

fn spawn_lines<R: Read + Send + 'static>(
    reader: R,
    tx: Sender<(bool, String)>,
    err: bool,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = BufReader::new(reader);
        let mut raw = Vec::new();
        loop {
            raw.clear();
            match buf.read_until(b'\n', &mut raw) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let line = String::from_utf8_lossy(&raw)
                .trim_end_matches(['\r', '\n'])
                .to_string();
            if tx.send((err, line)).is_err() {
                break;
            }
        }
    })
}

pub(super) fn run_clean(
    project: &Project,
    i: usize,
    step: &PipelineStep,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    let base = base_dir(project, step);
    if base.as_os_str().is_empty() || !base.is_absolute() || !base.is_dir() {
        emit(
            tx,
            ctx,
            i,
            format!(
                "Clean needs an existing absolute folder to work in (got \"{}\"). Set the content root or the step's working folder.",
                base.display()
            ),
            true,
        );
        return StepStatus::Failed;
    }
    let mut ok = true;
    for target in step
        .targets
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
    {
        if target.contains(['*', '?', '[']) {
            ok &= clean_glob(&base, target, i, ctx, tx);
        } else {
            let path = match safe_target(&base, target) {
                Ok(p) => p,
                Err(e) => {
                    emit(tx, ctx, i, e, true);
                    ok = false;
                    continue;
                }
            };
            match remove_path(&path) {
                Ok(true) => emit(tx, ctx, i, format!("Removed {}", path.display()), false),
                Ok(false) => emit(tx, ctx, i, format!("Not found: {}", path.display()), false),
                Err(e) => {
                    emit(
                        tx,
                        ctx,
                        i,
                        format!("Could not remove {}: {e}", path.display()),
                        true,
                    );
                    ok = false;
                }
            }
        }
    }
    if ok {
        StepStatus::Success
    } else {
        StepStatus::Failed
    }
}

fn normalize(path: &Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub(super) fn safe_target(base: &Path, target: &str) -> Result<std::path::PathBuf, String> {
    let base_norm = normalize(base);
    let joined = normalize(&base.join(target));
    let real_base = base.canonicalize().unwrap_or_else(|_| base_norm.clone());
    let real = joined.canonicalize().unwrap_or_else(|_| joined.clone());
    let inside = joined.starts_with(&base_norm)
        && joined != base_norm
        && (!joined.exists() || (real.starts_with(&real_base) && real != real_base));
    if inside {
        Ok(joined)
    } else {
        Err(format!(
            "Refusing to remove \"{target}\": it is outside {}",
            base.display()
        ))
    }
}

fn remove_path(path: &Path) -> std::io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(true)
}

fn clean_glob(
    base: &Path,
    pattern: &str,
    i: usize,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> bool {
    let Ok(glob) = globset::Glob::new(pattern) else {
        emit(tx, ctx, i, format!("Invalid pattern: {pattern}"), true);
        return false;
    };
    let matcher = glob.compile_matcher();
    let Ok(entries) = std::fs::read_dir(base) else {
        return true;
    };
    let mut ok = true;
    for entry in entries.flatten() {
        if matcher.is_match(entry.file_name().to_string_lossy().as_ref()) {
            match remove_path(&entry.path()) {
                Ok(_) => emit(
                    tx,
                    ctx,
                    i,
                    format!("Removed {}", entry.path().display()),
                    false,
                ),
                Err(e) => {
                    emit(tx, ctx, i, format!("Could not remove: {e}"), true);
                    ok = false;
                }
            }
        }
    }
    ok
}

pub(super) fn run_upload(
    project: &Project,
    i: usize,
    control: &Control,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    let owner = format!("Onyx pipeline upload of {}", project.name);
    let mut waited = false;
    let lock = loop {
        match crate::features::filelock::upload(&owner) {
            Ok(l) => break l,
            Err(e) => {
                if !waited {
                    emit(
                        tx,
                        ctx,
                        i,
                        format!("{e} Waiting for it to finish..."),
                        false,
                    );
                    waited = true;
                }
                if control.cancelled.load(Ordering::SeqCst) {
                    return StepStatus::Failed;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    };
    let work = match crate::config::build_dir() {
        Ok(w) => w,
        Err(e) => {
            emit(
                tx,
                ctx,
                i,
                format!("Could not prepare build dir: {e}"),
                true,
            );
            return StepStatus::Failed;
        }
    };
    let build_options = BuildOptions::default();
    let paths = match vdf::write_vdfs(project, &build_options, &work) {
        Ok(p) => p,
        Err(e) => {
            emit(tx, ctx, i, format!("Could not write VDFs: {e}"), true);
            return StepStatus::Failed;
        }
    };
    let Some(app_build) = paths
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("appbuild.vdf"))
        .cloned()
    else {
        emit(tx, ctx, i, "No appbuild.vdf generated.", true);
        return StepStatus::Failed;
    };
    let (btx, brx) = channel();
    let options = steamcmd::RunOptions {
        log_path: None,
        app_build_vdf: app_build,
        interactive: false,
    };
    let mut handle = match steamcmd::start_build(project, &options, ctx, btx) {
        Ok(h) => h,
        Err(e) => {
            emit(tx, ctx, i, format!("Could not start steamcmd: {e}"), true);
            return StepStatus::Failed;
        }
    };
    handle.hold(lock);
    let _awake = crate::keepawake::KeepAwake::new();
    let started = Instant::now();
    let mut last_activity = Instant::now();
    let record = |status: crate::model::BuildStatus| {
        if let Err(e) = crate::history::save_upload(
            project,
            &build_options.branch,
            &vdf::resolved_desc(project, &build_options),
            status,
            started.elapsed().as_millis() as u64,
            &handle.log_path,
            false,
        ) {
            emit(
                tx,
                ctx,
                i,
                format!("Could not record build history: {e}"),
                true,
            );
        }
    };
    loop {
        if control.cancelled.load(Ordering::SeqCst) {
            steamcmd::cancel(&handle);
            return StepStatus::Failed;
        }
        if last_activity.elapsed() > steamcmd::INACTIVITY {
            emit(
                tx,
                ctx,
                i,
                "SteamCMD printed nothing for 30 minutes; stopping it.",
                true,
            );
            steamcmd::cancel(&handle);
            record(crate::model::BuildStatus::Failed);
            return StepStatus::Failed;
        }
        let event = brx.recv_timeout(Duration::from_millis(120));
        if event.is_ok() {
            last_activity = Instant::now();
        }
        match event {
            Ok(BuildEvent::Log(line)) => {
                if crate::steamcmd_patterns::mobile_confirmation(&line) {
                    emit(
                        tx,
                        ctx,
                        i,
                        "Steam is waiting for a login confirmation in the Steam Mobile app.",
                        true,
                    );
                }
                let err = line.to_ascii_lowercase().contains("error") || line.contains("FAILED");
                emit(tx, ctx, i, line, err);
            }
            Ok(BuildEvent::Prompt(kind)) => {
                let what = match kind {
                    steamcmd::PromptKind::Password => "a password",
                    steamcmd::PromptKind::SteamGuard => "a Steam Guard code",
                };
                emit(
                    tx,
                    ctx,
                    i,
                    format!(
                        "SteamCMD is asking for {what}. Authorize this account on the Build page first; background builds cannot answer prompts."
                    ),
                    true,
                );
                steamcmd::cancel(&handle);
                return StepStatus::Failed;
            }
            Ok(BuildEvent::Finished(res)) => {
                record(match res {
                    BuildResult::Success { .. } => crate::model::BuildStatus::Success,
                    BuildResult::Failure { .. } => crate::model::BuildStatus::Failed,
                    BuildResult::Cancelled => crate::model::BuildStatus::Cancelled,
                });
                return match res {
                    BuildResult::Success { .. } => {
                        match super::changes::record_baseline(project) {
                            Ok(()) => emit(
                                tx,
                                ctx,
                                i,
                                "Saved a file snapshot for change detection.",
                                false,
                            ),
                            Err(e) => emit(
                                tx,
                                ctx,
                                i,
                                format!("Could not save a file snapshot: {e}"),
                                true,
                            ),
                        }
                        StepStatus::Success
                    }
                    _ => StepStatus::Failed,
                };
            }
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return StepStatus::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::safe_target;

    #[test]
    fn clean_targets_stay_inside_base() {
        let base = std::env::temp_dir().join(format!("onyx-clean-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(base.join("Saved")).unwrap();
        assert!(safe_target(&base, "Saved").is_ok());
        assert!(safe_target(&base, "Intermediate/obj").is_ok());
        assert!(safe_target(&base, "..").is_err());
        assert!(safe_target(&base, "../sibling").is_err());
        assert!(safe_target(&base, "Saved/../..").is_err());
        assert!(safe_target(&base, ".").is_err());
        let outside = std::env::temp_dir().join("elsewhere");
        assert!(safe_target(&base, &outside.to_string_lossy()).is_err());
        std::fs::remove_dir_all(&base).ok();
    }
}
