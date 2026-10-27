use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::model::Project;

use crate::config::Settings;

pub fn effective_steamcmd(project: &Project) -> PathBuf {
    resolve(project).steamcmd_path
}

#[cfg(not(test))]
pub fn resolve(project: &Project) -> Project {
    resolve_with(project, &crate::config::load_settings().unwrap_or_default())
}

#[cfg(test)]
pub fn resolve(project: &Project) -> Project {
    project.clone()
}

pub fn resolve_with(project: &Project, settings: &Settings) -> Project {
    let mut p = project.clone();
    if p.content_root.as_os_str().is_empty() {
        p.content_root = PathBuf::from(settings.default_content.trim());
    }
    if p.build_output_dir.as_os_str().is_empty() {
        p.build_output_dir = PathBuf::from(settings.default_output.trim());
    }
    let steamcmd = settings.steamcmd_path.trim();
    if !steamcmd.is_empty() {
        p.steamcmd_path = PathBuf::from(steamcmd);
    }
    p
}

pub fn clean_old_logs(days: u32) -> usize {
    if days == 0 {
        return 0;
    }
    let Ok(dir) = crate::config::log_dir() else {
        return 0;
    };
    let cutoff = Duration::from_secs(days as u64 * 86_400);
    let now = SystemTime::now();
    let mut removed = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let aged = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| now.duration_since(t).ok())
                .is_some_and(|age| age > cutoff);
            if aged && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

pub fn prune_orphan_manifests() -> usize {
    let Ok(dir) = crate::config::data_dir().map(|d| d.join("manifests")) else {
        return 0;
    };
    let Ok(conn) = crate::history::connect() else {
        return 0;
    };
    let Ok(referenced) = crate::history::manifest_paths(&conn) else {
        return 0;
    };
    let referenced: std::collections::HashSet<String> = referenced
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect();
    let min_age = Duration::from_secs(86_400);
    let now = SystemTime::now();
    let mut removed = 0;
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".mf.gz") || name.starts_with("pipeline-") || referenced.contains(&name)
        {
            continue;
        }
        let old = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age > min_age);
        if old && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::resolve_with;
    use crate::config::Settings;
    use crate::model::Project;

    fn settings() -> Settings {
        Settings {
            default_content: "D:/defaults/content".into(),
            default_output: "D:/defaults/output".into(),
            steamcmd_path: "D:/tools/steamcmd.exe".into(),
            ..Default::default()
        }
    }

    #[test]
    fn empty_project_paths_fall_back_to_settings() {
        let p = resolve_with(&Project::default(), &settings());
        assert_eq!(p.content_root, PathBuf::from("D:/defaults/content"));
        assert_eq!(p.build_output_dir, PathBuf::from("D:/defaults/output"));
        assert_eq!(p.steamcmd_path, PathBuf::from("D:/tools/steamcmd.exe"));
    }

    #[test]
    fn project_overrides_win_over_settings_defaults() {
        let project = Project {
            content_root: "E:/game/content".into(),
            build_output_dir: "E:/game/output".into(),
            ..Default::default()
        };
        let p = resolve_with(&project, &settings());
        assert_eq!(p.content_root, PathBuf::from("E:/game/content"));
        assert_eq!(p.build_output_dir, PathBuf::from("E:/game/output"));
    }

    #[test]
    fn settings_steamcmd_replaces_a_legacy_project_path() {
        let project = Project {
            steamcmd_path: "C:/old/steamcmd.exe".into(),
            ..Default::default()
        };
        let p = resolve_with(&project, &settings());
        assert_eq!(p.steamcmd_path, PathBuf::from("D:/tools/steamcmd.exe"));
        let kept = resolve_with(&project, &Settings::default());
        assert_eq!(kept.steamcmd_path, PathBuf::from("C:/old/steamcmd.exe"));
    }
}
