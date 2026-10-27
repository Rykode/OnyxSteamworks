use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};

pub struct Held {
    _file: File,
    owner_path: Option<PathBuf>,
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Some(p) = &self.owner_path {
            let _ = std::fs::remove_file(p);
        }
    }
}

fn open(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
}

pub fn try_acquire(path: &Path) -> io::Result<Option<Held>> {
    let file = open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(Held {
            _file: file,
            owner_path: None,
        })),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

pub fn acquire(path: &Path) -> io::Result<Held> {
    let file = open(path)?;
    file.lock()?;
    Ok(Held {
        _file: file,
        owner_path: None,
    })
}

fn upload_paths() -> Result<(PathBuf, PathBuf), String> {
    #[cfg(test)]
    let dir = std::env::temp_dir().join(format!("onyx-upload-{}", uuid::Uuid::new_v4()));
    #[cfg(not(test))]
    let dir = crate::config::data_dir().map_err(|e| e.to_string())?;
    Ok((dir.join("upload.lock"), dir.join("upload.owner")))
}

pub fn upload_in(lock: &Path, owner_file: &Path, owner: &str) -> Result<Held, String> {
    match try_acquire(lock) {
        Ok(Some(mut held)) => {
            let _ = std::fs::write(owner_file, owner);
            held.owner_path = Some(owner_file.to_path_buf());
            Ok(held)
        }
        Ok(None) => {
            let who = std::fs::read_to_string(owner_file).unwrap_or_default();
            let who = who.trim();
            if who.is_empty() {
                Err("Another SteamCMD run is already in progress.".to_string())
            } else {
                Err(format!(
                    "Another SteamCMD run is already in progress ({who})."
                ))
            }
        }
        Err(e) => Err(format!("Could not take the upload lock: {e}")),
    }
}

pub fn upload(owner: &str) -> Result<Held, String> {
    let (lock, owner_file) = upload_paths()?;
    upload_in(&lock, &owner_file, owner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_holder_is_refused_until_first_drops() {
        let dir = std::env::temp_dir().join(format!("onyx-lock-{}", uuid::Uuid::new_v4()));
        let lock = dir.join("upload.lock");
        let owner = dir.join("upload.owner");
        let first = upload_in(&lock, &owner, "first build").expect("first");
        let err = upload_in(&lock, &owner, "second").err().expect("refused");
        assert!(err.contains("first build"), "{err}");
        drop(first);
        assert!(upload_in(&lock, &owner, "second").is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }
}
