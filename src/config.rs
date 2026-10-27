use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

#[cfg(not(test))]
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::model::Project;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[cfg_attr(test, allow(dead_code))]
    #[error("could not determine the config directory")]
    NoConfigDir,
    #[error("project not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct ProjectFile {
    schema_version: u32,
    project: Project,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub last_opened_project: Option<String>,
    pub remember_password: bool,
    pub skip_review: bool,
    pub backup_dir: String,
    pub backup_keep: u32,
    pub backup_auto_prune: bool,
    pub keep_verify_temp: bool,
    pub advisor_large_mb: u32,
    pub analytics_growth_pct: u32,
    pub queue_max_retries: u32,
    pub accounts: Vec<String>,
    pub checklist_reset_on_live: bool,
    pub notify_toast: bool,
    pub notify_success: bool,
    pub notify_failure: bool,
    pub discord_enabled: bool,
    pub default_content: String,
    pub default_output: String,
    pub ui_scale: f32,
    pub reduce_motion: bool,
    pub log_retention_days: u32,
    #[serde(alias = "steamcmd_override")]
    pub steamcmd_path: String,
    pub wizard_done: bool,
}

fn project_dirs() -> Result<(PathBuf, PathBuf)> {
    #[cfg(test)]
    {
        let base = std::env::temp_dir().join(format!("onyx-test-{}", std::process::id()));
        Ok((base.join("config"), base.join("data")))
    }
    #[cfg(not(test))]
    {
        static DIRS: std::sync::OnceLock<Option<(PathBuf, PathBuf)>> = std::sync::OnceLock::new();
        DIRS.get_or_init(|| {
            let new = ProjectDirs::from("dev", "onyx", "Onyx")?;
            let new = (new.config_dir().to_path_buf(), new.data_dir().to_path_buf());
            match ProjectDirs::from("dev", "steampipe", "SteamPipe GUI") {
                Some(old) => {
                    let old = (old.config_dir().to_path_buf(), old.data_dir().to_path_buf());
                    Some(migrate_dirs(old, new))
                }
                None => Some(new),
            }
        })
        .clone()
        .ok_or(ConfigError::NoConfigDir)
    }
}

fn move_dir(from: &Path, to: &Path) -> bool {
    if to.exists() {
        return true;
    }
    if !from.exists() {
        return true;
    }
    if let Some(parent) = to.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return false;
    }
    fs::rename(from, to).is_ok()
}

fn path_variants(old: &Path, new: &Path) -> Vec<(String, String)> {
    let o = old.to_string_lossy().to_string();
    let n = new.to_string_lossy().to_string();
    let mut out = vec![
        (o.replace('\\', "\\\\"), n.replace('\\', "\\\\")),
        (o.replace('\\', "/"), n.replace('\\', "/")),
    ];
    if o.contains('\\') {
        out.push((o, n));
    }
    out.dedup();
    out
}

fn rewrite_json_paths(dir: &Path, recursive: bool, pairs: &[(String, String)]) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                rewrite_json_paths(&path, true, pairs);
            }
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let mut updated = text.clone();
        for (from, to) in pairs {
            updated = updated.replace(from.as_str(), to);
        }
        if updated != text {
            let _ = write_atomic(&path, updated.as_bytes());
        }
    }
}

fn migrate_dirs(old: (PathBuf, PathBuf), new: (PathBuf, PathBuf)) -> (PathBuf, PathBuf) {
    let config_moves = old.0.exists() && !new.0.exists();
    let data_moves = old.1.exists() && !new.1.exists();
    let config_ok = move_dir(&old.0, &new.0);
    let data_ok = config_ok && move_dir(&old.1, &new.1);
    if config_ok && data_ok {
        if config_moves || data_moves {
            let mut pairs = path_variants(&old.1, &new.1);
            pairs.extend(path_variants(&old.0, &new.0));
            rewrite_json_paths(&new.0, true, &pairs);
            rewrite_json_paths(&new.1, false, &pairs);
            let raw: Vec<(String, String)> = pairs
                .iter()
                .filter(|(f, _)| !f.contains("\\\\"))
                .cloned()
                .collect();
            crate::history::rewrite_paths(&new.1.join("history.db"), &raw);
        }
        return new;
    }
    if config_ok && old.0 != new.0 && new.0.exists() && !old.0.exists() {
        let _ = fs::rename(&new.0, &old.0);
    }
    old
}

fn config_dir() -> Result<PathBuf> {
    let dir = project_dirs()?.0;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn projects_dir() -> Result<PathBuf> {
    let dir = config_dir()?.join("projects");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn data_dir() -> Result<PathBuf> {
    let dir = project_dirs()?.1;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn log_dir() -> Result<PathBuf> {
    let dir = project_dirs()?.1.join("logs");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn build_dir() -> Result<PathBuf> {
    let dir = project_dirs()?.1.join("builds");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

static PROBLEMS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn report_problem(message: impl Into<String>) {
    let message = message.into();
    if let Ok(mut p) = PROBLEMS.lock()
        && !p.contains(&message)
    {
        p.push(message);
    }
}

pub fn take_problems() -> Vec<String> {
    PROBLEMS
        .lock()
        .map(|mut p| std::mem::take(&mut *p))
        .unwrap_or_default()
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", uuid::Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn project_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

fn save_project_in(dir: &Path, project: &Project) -> Result<()> {
    let file = ProjectFile {
        schema_version: SCHEMA_VERSION,
        project: project.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&file)?;
    Ok(write_atomic(&project_path(dir, &project.id), &bytes)?)
}

fn load_project_in(dir: &Path, id: &str) -> Result<Project> {
    let path = project_path(dir, id);
    if !path.exists() {
        return Err(ConfigError::NotFound(id.to_string()));
    }
    let bytes = fs::read(&path)?;
    let mut file: ProjectFile = serde_json::from_slice(&bytes)?;
    file.project.migrate();
    Ok(file.project)
}

fn list_projects_in(dir: &Path) -> Result<Vec<Project>> {
    let mut projects = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match fs::read(&path)
            .map_err(ConfigError::from)
            .and_then(|b| serde_json::from_slice::<ProjectFile>(&b).map_err(ConfigError::from))
        {
            Ok(mut file) => {
                file.project.migrate();
                projects.push(file.project);
            }
            Err(e) => report_problem(format!(
                "Skipped unreadable project file {}: {e}",
                path.display()
            )),
        }
    }
    projects.sort_by_key(|p| p.name.to_lowercase());
    Ok(projects)
}

pub fn list_projects() -> Result<Vec<Project>> {
    list_projects_in(&projects_dir()?)
}

pub fn load_project(id: &str) -> Result<Project> {
    load_project_in(&projects_dir()?, id)
}

pub fn save_project(project: &Project) -> Result<()> {
    save_project_in(&projects_dir()?, project)
}

pub fn bump_build_counter(id: &str) -> Result<Project> {
    let dir = projects_dir()?;
    let _guard = crate::features::filelock::acquire(&dir.join(format!("{id}.lock")))?;
    let mut project = load_project_in(&dir, id)?;
    project.build_counter = project.build_counter.saturating_add(1);
    save_project_in(&dir, &project)?;
    Ok(project)
}

pub fn delete_project(id: &str) -> Result<()> {
    let path = project_path(&projects_dir()?, id);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub fn duplicate_project(id: &str) -> Result<Project> {
    let mut copy = load_project(id)?;
    copy.id = uuid::Uuid::new_v4().to_string();
    copy.name = format!("{} (copy)", copy.name);
    save_project(&copy)?;
    Ok(copy)
}

fn settings_path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

type Stamp = (PathBuf, SystemTime, u64);

static SETTINGS_CACHE: Mutex<Option<(Stamp, Settings)>> = Mutex::new(None);

fn file_stamp(path: &Path) -> Option<Stamp> {
    let meta = fs::metadata(path).ok()?;
    Some((path.to_path_buf(), meta.modified().ok()?, meta.len()))
}

fn load_settings_at(path: &Path) -> Result<Settings> {
    let Some(stamp) = file_stamp(path) else {
        return Ok(Settings::default());
    };
    if let Ok(cache) = SETTINGS_CACHE.lock()
        && let Some((cached, settings)) = cache.as_ref()
        && *cached == stamp
    {
        return Ok(settings.clone());
    }
    let bytes = fs::read(path)?;
    match serde_json::from_slice::<Settings>(&bytes) {
        Ok(settings) => {
            if let Ok(mut cache) = SETTINGS_CACHE.lock() {
                *cache = Some((stamp, settings.clone()));
            }
            Ok(settings)
        }
        Err(e) => {
            let backup = path.with_extension("json.bad");
            let _ = fs::copy(path, &backup);
            report_problem(format!(
                "Settings could not be read ({e}). A copy was kept as {}.",
                backup.display()
            ));
            Err(e.into())
        }
    }
}

pub fn load_settings() -> Result<Settings> {
    load_settings_at(&settings_path(&config_dir()?))
}

fn save_settings_at(path: &Path, settings: &Settings) -> Result<()> {
    let mut to_write = settings.clone();
    to_write.schema_version = SCHEMA_VERSION;
    let bytes = serde_json::to_vec_pretty(&to_write)?;
    let _guard = crate::features::filelock::acquire(&path.with_extension("lock"))?;
    write_atomic(path, &bytes)?;
    if let Ok(mut cache) = SETTINGS_CACHE.lock() {
        *cache = file_stamp(path).map(|s| (s, to_write));
    }
    Ok(())
}

pub fn save_settings(settings: &Settings) -> Result<()> {
    save_settings_at(&settings_path(&config_dir()?), settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("steampipe-test-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = temp_dir("roundtrip");
        let mut project = Project {
            id: "p1".to_string(),
            name: "Demo".to_string(),
            app_id: "480".to_string(),
            build_counter: 7,
            ..Default::default()
        };
        project.depots.push(crate::model::Depot {
            depot_id: "481".to_string(),
            name: "base".to_string(),
            recursive: true,
            ..Default::default()
        });

        save_project_in(&dir, &project).expect("save");
        let loaded = load_project_in(&dir, "p1").expect("load");

        assert_eq!(loaded.name, "Demo");
        assert_eq!(loaded.app_id, "480");
        assert_eq!(loaded.build_counter, 7);
        assert_eq!(loaded.depots.len(), 1);
        assert_eq!(loaded.depots[0].depot_id, "481");

        let listed = list_projects_in(&dir).expect("list");
        assert_eq!(listed.len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn loads_file_missing_newer_fields() {
        let dir = temp_dir("legacy");
        let legacy = r#"{ "project": { "id": "old", "name": "Legacy", "app_id": "10" } }"#;
        fs::write(project_path(&dir, "old"), legacy).expect("write legacy");

        let loaded = load_project_in(&dir, "old").expect("load legacy");
        assert_eq!(loaded.name, "Legacy");
        assert_eq!(loaded.app_id, "10");
        assert_eq!(loaded.build_counter, 0);
        assert!(loaded.depots.is_empty());
        assert!(loaded.ignored_findings.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ignores_unknown_fields() {
        let dir = temp_dir("unknown");
        let data = r#"{ "schema_version": 1, "project": { "id": "u", "name": "U",
            "surprise_field": 123, "another": ["x"] } }"#;
        fs::write(project_path(&dir, "u"), data).expect("write");
        let loaded = load_project_in(&dir, "u").expect("load");
        assert_eq!(loaded.name, "U");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_settings_are_kept_and_reported() {
        let dir = temp_dir("settings-bad");
        let path = settings_path(&dir);
        fs::write(&path, b"{ broken").expect("write");
        assert!(load_settings_at(&path).is_err());
        assert!(path.with_extension("json.bad").exists());
        assert!(
            take_problems()
                .iter()
                .any(|p| p.contains("Settings could not be read"))
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn settings_save_then_load_uses_new_values() {
        let dir = temp_dir("settings-ok");
        let path = settings_path(&dir);
        let s = Settings {
            backup_keep: 3,
            ..Default::default()
        };
        save_settings_at(&path, &s).expect("save");
        assert_eq!(load_settings_at(&path).expect("load").backup_keep, 3);
        let s = Settings {
            backup_keep: 42,
            ..Default::default()
        };
        save_settings_at(&path, &s).expect("save");
        assert_eq!(load_settings_at(&path).expect("load").backup_keep, 42);
        let leftovers = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn old_folders_move_to_the_new_location_once() {
        let root = temp_dir("migrate");
        let old = (root.join("old/config"), root.join("old/data"));
        let new = (root.join("new/config"), root.join("new/data"));
        fs::create_dir_all(&old.0).unwrap();
        fs::create_dir_all(&old.1).unwrap();
        let exe = old.1.join("steamcmd").join("steamcmd.exe");
        let json = serde_json::json!({ "steamcmd_path": exe.to_string_lossy() }).to_string();
        fs::write(old.0.join("settings.json"), json).unwrap();
        fs::write(old.1.join("history.db"), b"db").unwrap();
        assert_eq!(migrate_dirs(old.clone(), new.clone()), new);
        let moved = fs::read_to_string(new.0.join("settings.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&moved).unwrap();
        assert_eq!(
            parsed["steamcmd_path"].as_str().unwrap(),
            new.1.join("steamcmd").join("steamcmd.exe").to_string_lossy()
        );
        assert!(new.1.join("history.db").exists());
        assert!(!old.0.exists());
        assert_eq!(migrate_dirs(old.clone(), new.clone()), new);
        fs::remove_dir_all(&root).ok();
    }
}
