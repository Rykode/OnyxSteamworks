use std::path::{Path, PathBuf};

use crate::model::Project;

pub fn config_vdf(steamcmd_path: &Path) -> Option<std::path::PathBuf> {
    steamcmd_path
        .parent()
        .map(|d| d.join("config").join("config.vdf"))
}

pub fn resolve_steamcmd(settings_path: &str, projects: &[Project]) -> Option<PathBuf> {
    let path = settings_path.trim();
    if !path.is_empty() {
        return Some(PathBuf::from(path));
    }
    projects
        .iter()
        .map(|p| p.steamcmd_path.clone())
        .find(|p| !p.as_os_str().is_empty())
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// SteamCMD keeps each account's login token in config.vdf under
/// `ConnectCache`, keyed by the CRC32 of the lowercased account name plus "1".
fn token_key(account: &str) -> String {
    format!(
        "\"{:08x}1\"",
        crc32(account.trim().to_ascii_lowercase().as_bytes())
    )
}

fn is_token_line(line: &str, key: &str) -> bool {
    line.trim_start().starts_with(key)
}

pub fn is_authorized(steamcmd_path: &Path, account: &str) -> bool {
    if account.trim().is_empty() {
        return false;
    }
    let Some(path) = config_vdf(steamcmd_path) else {
        return false;
    };
    let key = token_key(account);
    match std::fs::read_to_string(&path) {
        Ok(text) => text.lines().any(|l| is_token_line(l, &key)),
        Err(_) => false,
    }
}

/// Drops the account's cached login token so the next SteamCMD login asks
/// for the password and Steam Guard instead of retrying a revoked token.
pub fn forget_cached_token(steamcmd_path: &Path, account: &str) -> std::io::Result<()> {
    if account.trim().is_empty() {
        return Ok(());
    }
    let Some(path) = config_vdf(steamcmd_path) else {
        return Ok(());
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let key = token_key(account);
    if !text.lines().any(|l| is_token_line(l, &key)) {
        return Ok(());
    }
    let kept: String = text
        .split_inclusive('\n')
        .filter(|l| !is_token_line(l, &key))
        .collect();
    std::fs::write(&path, kept)
}

pub fn cached_login(steamcmd_path: &Path, account: &str) -> Option<()> {
    is_authorized(steamcmd_path, account).then_some(())
}

pub fn cached_login_for_project(project: &Project) -> Option<()> {
    let exe = crate::features::maintenance::effective_steamcmd(project);
    cached_login(&exe, project.steam_username.trim())
}

pub fn add(accounts: &mut Vec<String>, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || accounts.iter().any(|a| a.eq_ignore_ascii_case(name)) {
        return false;
    }
    accounts.push(name.to_string());
    true
}

pub fn remove(accounts: &mut Vec<String>, name: &str) {
    accounts.retain(|a| !a.eq_ignore_ascii_case(name));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_dedupes_case_insensitively() {
        let mut accounts = Vec::new();
        assert!(add(&mut accounts, "Builder"));
        assert!(!add(&mut accounts, "builder"));
        assert!(!add(&mut accounts, "  "));
        assert_eq!(accounts, vec!["Builder"]);
        remove(&mut accounts, "BUILDER");
        assert!(accounts.is_empty());
    }

    #[test]
    fn resolve_prefers_override_then_project() {
        assert_eq!(
            resolve_steamcmd("D:/scmd/steamcmd.exe", &[]),
            Some(PathBuf::from("D:/scmd/steamcmd.exe"))
        );
        let projects = vec![
            Project::default(),
            Project {
                steamcmd_path: PathBuf::from("C:/a/steamcmd.exe"),
                ..Default::default()
            },
        ];
        assert_eq!(
            resolve_steamcmd("", &projects),
            Some(PathBuf::from("C:/a/steamcmd.exe"))
        );
        assert_eq!(resolve_steamcmd("  ", &[Project::default()]), None);
    }

    #[test]
    fn token_key_matches_steamcmd() {
        assert_eq!(token_key("itsRyko11"), "\"6954a36e1\"");
    }

    #[test]
    fn authorized_tracks_connect_cache_token() {
        let dir = std::env::temp_dir().join(format!("onyx_acct_{}", uuid::Uuid::new_v4()));
        let cfg = dir.join("config");
        std::fs::create_dir_all(&cfg).unwrap();
        let vdf = format!(
            "\"ConnectCache\"\n{{\n\t\t{}\t\t\"tok\"\n}}\n\"Accounts\"\n{{\n\t\"builder\"\n\t{{\n\t}}\n}}\n",
            token_key("builder")
        );
        std::fs::write(cfg.join("config.vdf"), &vdf).unwrap();
        let steamcmd = dir.join("steamcmd.exe");
        assert!(is_authorized(&steamcmd, "Builder"));
        assert!(!is_authorized(&steamcmd, "someone_else"));

        forget_cached_token(&steamcmd, "Builder").unwrap();
        assert!(!is_authorized(&steamcmd, "Builder"));
        let after = std::fs::read_to_string(cfg.join("config.vdf")).unwrap();
        assert!(after.contains("\"ConnectCache\"") && after.contains("\"builder\""));
        std::fs::remove_dir_all(&dir).ok();
    }
}
