use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::Project;

pub struct Vars {
    pub version: String,
    pub date: String,
    pub time: String,
    pub branch: String,
    pub git_commit: String,
    pub git_tag: String,
    pub build_number: u32,
}

/// (content root + git args, fetched at, output). Short-lived so a long-running app
/// picks up new commits and tags without shelling out to git on every frame.
type GitCacheEntry = (String, std::time::Instant, String);
static GIT_CACHE: Mutex<Vec<GitCacheEntry>> = Mutex::new(Vec::new());
const GIT_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(20);

pub fn expand(template: &str, vars: &Vars) -> String {
    let build = vars.build_number.to_string();
    template
        .replace("{version}", &vars.version)
        .replace("{date}", &vars.date)
        .replace("{time}", &vars.time)
        .replace("{branch}", &vars.branch)
        .replace("{git_commit}", &vars.git_commit)
        .replace("{git_tag}", &vars.git_tag)
        .replace("{build_number}", &build)
}

/// Variables for a build that is about to run: git is asked fresh every time.
pub fn resolve(project: &Project, branch: &str) -> Vars {
    resolve_with(project, branch, Git::Fresh)
}

/// Variables for an on-screen preview, called every frame: git values come from a
/// cache that refreshes on a background thread, so painting never waits on git.
pub fn resolve_preview(project: &Project, branch: &str) -> Vars {
    resolve_with(project, branch, Git::Cached)
}

#[derive(Clone, Copy)]
enum Git {
    Fresh,
    Cached,
}

fn resolve_with(project: &Project, branch: &str, git: Git) -> Vars {
    let project = &crate::features::maintenance::resolve(project);
    let (date, time) = now_date_time();
    let root = &project.content_root;
    Vars {
        version: read_version(project),
        date,
        time,
        branch: branch.to_string(),
        git_commit: git_value(root, &["rev-parse", "--short", "HEAD"], git),
        git_tag: git_value(root, &["describe", "--tags", "--abbrev=0"], git),
        build_number: project.build_counter.saturating_add(1),
    }
}

fn read_version(project: &Project) -> String {
    if project.version_file.trim().is_empty() {
        return String::new();
    }
    let path = project.content_root.join(project.version_file.trim());
    let Ok(text) = std::fs::read_to_string(&path) else {
        return String::new();
    };
    let trimmed = text.trim();
    if trimmed.starts_with('{')
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed)
    {
        for key in ["version", "Version", "app_version"] {
            if let Some(v) = value.get(key).and_then(|v| v.as_str()) {
                return v.to_string();
            }
        }
    }
    trimmed.lines().next().unwrap_or("").trim().to_string()
}

fn git_value(content_root: &Path, args: &'static [&'static str], mode: Git) -> String {
    let root = content_root.to_string_lossy().to_string();
    if root.is_empty() {
        return String::new();
    }
    if let Git::Fresh = mode {
        let value = run_git(content_root, args);
        store(format!("{root}|{}", args.join(" ")), value.clone());
        return value;
    }
    let key = format!("{root}|{}", args.join(" "));
    let cached = GIT_CACHE
        .lock()
        .ok()
        .and_then(|c| c.iter().find(|(k, _, _)| k == &key).map(|(_, at, v)| (*at, v.clone())));
    match cached {
        Some((at, value)) => {
            if at.elapsed() >= GIT_CACHE_TTL {
                // Mark it fresh now so only one refresh runs, then update off-thread.
                store(key.clone(), value.clone());
                let dir = content_root.to_path_buf();
                std::thread::spawn(move || store(key, run_git(&dir, args)));
            }
            value
        }
        // First sight of this folder: one blocking call, as before.
        None => {
            let value = run_git(content_root, args);
            store(key, value.clone());
            value
        }
    }
}

fn store(key: String, value: String) {
    if let Ok(mut cache) = GIT_CACHE.lock() {
        cache.retain(|(k, _, _)| k != &key);
        cache.push((key, std::time::Instant::now(), value));
    }
}

/// Number of uncommitted changes in the content folder's git work tree, or `None`
/// when it is not a git repository (or git is not installed).
pub fn dirty_count(content_root: &Path) -> Option<usize> {
    if content_root.as_os_str().is_empty() {
        return None;
    }
    let inside = run_git(content_root, &["rev-parse", "--is-inside-work-tree"]);
    if inside != "true" {
        return None;
    }
    let status = run_git(content_root, &["status", "--porcelain", "--", "."]);
    Some(status.lines().filter(|l| !l.trim().is_empty()).count())
}

fn run_git(content_root: &Path, args: &[&str]) -> String {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(content_root).args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => String::new(),
    }
}

fn now_date_time() -> (String, String) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (hh, mm) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32);
    (format!("{y:04}-{m:02}-{d:02}"), format!("{hh:02}{mm:02}"))
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vars {
        Vars {
            version: "1.2.3".to_string(),
            date: "2026-10-21".to_string(),
            time: "0930".to_string(),
            branch: "beta".to_string(),
            git_commit: "abc1234".to_string(),
            git_tag: "v1.2.0".to_string(),
            build_number: 42,
        }
    }

    #[test]
    fn expands_all_variables() {
        let out = expand(
            "v{version} {branch} {date}.{time} #{build_number} {git_commit}",
            &vars(),
        );
        assert_eq!(out, "v1.2.3 beta 2026-10-21.0930 #42 abc1234");
    }

    #[test]
    fn expands_git_tag() {
        assert_eq!(expand("{git_tag}-{git_commit}", &vars()), "v1.2.0-abc1234");
    }

    #[test]
    fn leaves_unknown_tokens_untouched() {
        assert_eq!(expand("{unknown} {version}", &vars()), "{unknown} 1.2.3");
    }

    #[test]
    fn civil_date_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(18_993), (2022, 1, 1));
        assert_eq!(civil_from_days(19_358), (2023, 1, 1));
    }
}
