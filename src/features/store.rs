use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;
use serde::de::DeserializeOwned;

fn path(name: &str) -> Option<PathBuf> {
    crate::config::data_dir().ok().map(|d| d.join(name))
}

pub fn stamp(name: &str) -> Option<SystemTime> {
    path(name)
        .and_then(|p| std::fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
}

pub fn try_load_at<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>, String> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    serde_json::from_slice(&bytes).map_err(|e| {
        let backup = path.with_extension("json.bad");
        if !backup.exists() {
            let _ = std::fs::copy(path, &backup);
        }
        format!(
            "{} is unreadable ({e}); a copy was kept as {}",
            path.display(),
            backup.display()
        )
    })
}

pub fn load<T: DeserializeOwned>(name: &str) -> Vec<T> {
    path(name)
        .and_then(|p| try_load_at(&p).ok())
        .unwrap_or_default()
}

pub fn save_at<T: Serialize>(path: &Path, items: &[T]) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(items).map_err(|e| e.to_string())?;
    crate::config::write_atomic(path, &bytes).map_err(|e| e.to_string())
}

pub fn update_at<T: Serialize + DeserializeOwned>(
    path: &Path,
    change: impl FnOnce(&mut Vec<T>) -> bool,
) -> Result<Vec<T>, String> {
    let _guard = crate::features::filelock::acquire(&path.with_extension("lock"))
        .map_err(|e| format!("could not lock {}: {e}", path.display()))?;
    let mut items = try_load_at(path)?;
    if change(&mut items) {
        save_at(path, &items)?;
    }
    Ok(items)
}

pub fn update<T: Serialize + DeserializeOwned>(
    name: &str,
    change: impl FnOnce(&mut Vec<T>) -> bool,
) -> Result<Vec<T>, String> {
    let path = path(name).ok_or_else(|| "no data directory".to_string())?;
    update_at(&path, change)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("onyx-store-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d.join("items.json")
    }

    #[test]
    fn corrupt_file_is_never_overwritten() {
        let p = tmp();
        std::fs::write(&p, b"{ not json").unwrap();
        let res = update_at::<String>(&p, |v| {
            v.push("new".into());
            true
        });
        assert!(res.is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"{ not json");
        assert!(p.with_extension("json.bad").exists());
    }

    #[test]
    fn update_round_trips() {
        let p = tmp();
        update_at::<String>(&p, |v| {
            v.push("a".into());
            true
        })
        .unwrap();
        let all = update_at::<String>(&p, |v| {
            v.push("b".into());
            true
        })
        .unwrap();
        assert_eq!(all, vec!["a".to_string(), "b".to_string()]);
    }
}
