//! Opt-in check for a newer Onyx release on GitHub. Runs once on a background
//! thread, never downloads anything, and stays silent if the request fails.

use std::sync::Mutex;

const LATEST_URL: &str = "https://api.github.com/repos/RykoTheDev/OnyxSteamworks/releases/latest";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Update {
    pub version: String,
    pub url: String,
}

static FOUND: Mutex<Option<Update>> = Mutex::new(None);

/// Start the check if the user enabled it. Results arrive through `take_found`.
pub fn spawn_check(ctx: &eframe::egui::Context) {
    if !crate::config::load_settings()
        .map(|s| s.check_updates)
        .unwrap_or(false)
    {
        return;
    }
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        if let Some(update) = fetch_latest().and_then(|u| newer(env!("CARGO_PKG_VERSION"), u)) {
            if let Ok(mut slot) = FOUND.lock() {
                *slot = Some(update);
            }
            ctx.request_repaint();
        }
    });
}

pub fn take_found() -> Option<Update> {
    FOUND.lock().ok().and_then(|mut s| s.take())
}

fn fetch_latest() -> Option<Update> {
    let resp = ureq::get(LATEST_URL)
        .set("User-Agent", "Onyx-update-check")
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .ok()?;
    let json: serde_json::Value = resp.into_json().ok()?;
    if json.get("draft").and_then(|v| v.as_bool()) == Some(true)
        || json.get("prerelease").and_then(|v| v.as_bool()) == Some(true)
    {
        return None;
    }
    Some(Update {
        version: json.get("tag_name")?.as_str()?.to_string(),
        url: json.get("html_url")?.as_str()?.to_string(),
    })
}

fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.trim().trim_start_matches(['v', 'V']);
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    Some((parts.next()??, parts.next().flatten().unwrap_or(0), parts.next().flatten().unwrap_or(0)))
}

/// `latest` if it is a strictly higher version than `current`.
fn newer(current: &str, latest: Update) -> Option<Update> {
    (parse(&latest.version)? > parse(current)?).then_some(latest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up(v: &str) -> Update {
        Update { version: v.to_string(), url: String::new() }
    }

    #[test]
    fn only_strictly_newer_versions_count() {
        assert!(newer("1.1.0", up("v1.2.0")).is_some());
        assert!(newer("1.1.0", up("v1.1.1")).is_some());
        assert!(newer("1.1.0", up("2.0")).is_some());
        assert!(newer("1.1.0", up("v1.1.0")).is_none());
        assert!(newer("1.1.0", up("v1.0.9")).is_none());
        assert!(newer("1.1.0", up("not-a-version")).is_none());
    }
}
