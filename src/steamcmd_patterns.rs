use crate::steamcmd::{PromptKind, Stage};

pub fn detect_login(line: &str) -> Option<bool> {
    let l = line.to_ascii_lowercase();
    if (l.contains("failed") && l.contains("login"))
        || (l.contains("logging in user") && l.contains("error"))
        || l.contains("logon denied")
        || l.contains("code mismatch")
        || l.contains("invalid login auth code")
        || l.contains("invalid password")
        || l.contains("login failure")
        || (l.contains("access denied") && l.contains("logging in"))
        || l.contains("rate limit exceeded")
    {
        return Some(false);
    }
    if l.contains("logged in ok") || (l.contains("logging in user") && l.contains("ok")) {
        return Some(true);
    }
    None
}

pub fn login_failure_reason(line: &str) -> String {
    let l = line.to_ascii_lowercase();
    if l.contains("invalid password") {
        "Invalid password.".to_string()
    } else if l.contains("rate limit") {
        "Rate limited by Steam. Wait a few minutes and try again.".to_string()
    } else if l.contains("code mismatch") || l.contains("invalid login auth code") {
        "The Steam Guard code was wrong or expired. Authorize again with a fresh code.".to_string()
    } else if l.contains("logon denied") {
        "Steam denied the login (Account Logon Denied). Steam Guard needs a fresh code: run Authorize on the Build page.".to_string()
    } else if l.contains("access denied") {
        "Steam denied the login (Access Denied). The cached login is invalid or expired, or the account lacks publishing access. Run Authorize on the Build page to sign in again.".to_string()
    } else if l.contains("expired") || (l.contains("cached") && l.contains("invalid")) {
        "The cached login expired. Run Authorize again.".to_string()
    } else if l.contains("two-factor") || l.contains("steam guard") {
        "Steam Guard code was required or incorrect.".to_string()
    } else {
        "Login failed.".to_string()
    }
}

pub fn mobile_confirmation(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    l.contains("steam mobile app") && l.contains("confirm")
}

pub fn detect_prompt(partial: &str) -> Option<PromptKind> {
    if !partial.trim_end().ends_with(':') {
        return None;
    }
    let l = partial.to_ascii_lowercase();
    if l.contains("steam guard") || l.contains("two-factor") {
        return Some(PromptKind::SteamGuard);
    }
    if l.contains("password:") {
        return Some(PromptKind::Password);
    }
    None
}

pub fn detect_stage(line: &str) -> Option<Stage> {
    let l = line.to_ascii_lowercase();
    if l.contains("connecting") {
        return Some(Stage::Connecting);
    }
    if l.contains("logging in user") {
        return Some(Stage::LoggingIn);
    }
    if l.contains("building depot") || l.contains("scanning") {
        return Some(Stage::Scanning);
    }
    if l.contains("uploading") {
        return Some(Stage::Uploading);
    }
    if l.contains("committing") {
        return Some(Stage::Committing);
    }
    None
}

pub fn detect_progress(line: &str) -> Option<f32> {
    let idx = line.find('%')?;
    let prefix = &line[..idx];
    let start = prefix
        .rfind(|c: char| !(c.is_ascii_digit() || c == '.'))
        .map(|i| i + 1)
        .unwrap_or(0);
    let value: f32 = prefix[start..].parse().ok()?;
    Some((value / 100.0).clamp(0.0, 1.0))
}

pub fn detect_success(line: &str) -> Option<String> {
    let marker = "successfully finished appid";
    let idx = line.to_ascii_lowercase().find(marker)?;
    let rest = line[idx + marker.len()..].trim_start();
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    (!digits.is_empty()).then_some(digits)
}

pub fn is_benign(line: &str) -> bool {
    line.to_ascii_lowercase()
        .contains("info request for appid 7 ")
}

pub fn commit_failure(line: &str) -> Option<String> {
    let l = line.to_ascii_lowercase();
    if !l.contains("failed to commit build") {
        return None;
    }
    let detail = line
        .find("ERROR!")
        .map(|i| line[i + "ERROR!".len()..].trim())
        .unwrap_or(line.trim());
    Some(format!(
        "Steam uploaded the depots but refused to commit the build ({detail}). \
         Usually the set-live branch does not exist in Steamworks, or the account \
         is not allowed to publish builds for this app."
    ))
}

pub fn detect_failure(line: &str) -> Option<String> {
    if is_benign(line) {
        return None;
    }
    let l = line.to_ascii_lowercase();
    if l.contains("error!") || l.starts_with("error ") || l.contains("failed to") {
        Some(line.trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_denied_is_a_login_failure() {
        let line =
            "Logging in user 'itsRyko11' [U:1:1251715880] to Steam Public...ERROR (Access Denied)";
        assert_eq!(detect_login(line), Some(false));
        let reason = login_failure_reason(line);
        assert!(reason.contains("Access Denied"));
    }

    #[test]
    fn plain_access_denied_without_login_context_is_not_a_login_event() {
        assert_eq!(detect_login("depot access denied for something"), None);
    }

    #[test]
    fn newer_login_errors_are_failures() {
        let mismatch = "Logging in user 'bob' to Steam Public...ERROR (Two-factor code mismatch)";
        assert_eq!(detect_login(mismatch), Some(false));
        assert!(login_failure_reason(mismatch).contains("Steam Guard code was wrong"));
        let denied = "Logging in user 'bob' to Steam Public...ERROR (Account Logon Denied)";
        assert_eq!(detect_login(denied), Some(false));
        assert!(login_failure_reason(denied).contains("Logon Denied"));
    }

    #[test]
    fn prompts_need_a_trailing_colon() {
        assert_eq!(
            detect_prompt("Please enter Steam Guard code: "),
            Some(PromptKind::SteamGuard)
        );
        assert_eq!(detect_prompt("password: "), Some(PromptKind::Password));
        assert_eq!(detect_prompt("Steam Guard is enabled for this acc"), None);
        assert_eq!(detect_prompt("Reset password: see the help"), None);
    }

    #[test]
    fn client_config_timeout_is_not_a_build_failure() {
        let line =
            "Waiting for client config...ERROR! Info request for AppId 7 returned error Timeout.";
        assert!(is_benign(line));
        assert_eq!(detect_failure(line), None);
        assert!(
            detect_failure("ERROR! Info request for AppId 70 returned error Timeout.").is_some()
        );
    }

    #[test]
    fn commit_failure_is_explained() {
        let line =
            "[2026-10-26 19:48:30]: ERROR! Failed to commit build for AppID 5267590 : Failure";
        let msg = commit_failure(line).expect("commit failure");
        assert!(msg.contains("Failed to commit build for AppID 5267590 : Failure"));
        assert!(msg.contains("set-live branch"));
        assert_eq!(commit_failure("Building depot 5267591..."), None);
    }

    #[test]
    fn successful_login_still_detected() {
        assert_eq!(
            detect_login("Logging in user 'x' to Steam Public...OK"),
            Some(true)
        );
    }
}
