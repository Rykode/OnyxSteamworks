use std::io::{BufRead, Write};

fn arg_after(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

fn scenario() -> String {
    let args: Vec<String> = std::env::args().collect();
    if let Some(vdf) = arg_after(&args, "+run_app_build") {
        return std::path::Path::new(&vdf)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "success".to_string());
    }
    arg_after(&args, "+login").unwrap_or_else(|| "success".to_string())
}

fn line(text: &str) {
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{text}");
    let _ = stdout.flush();
}

fn carriage(text: &str) {
    let mut stdout = std::io::stdout();
    let _ = write!(stdout, "{text}\r");
    let _ = stdout.flush();
}

fn raw(text: &str) {
    let mut stdout = std::io::stdout();
    let _ = write!(stdout, "{text}");
    let _ = stdout.flush();
}

fn succeed() {
    line("Building depot 481...");
    for pct in [0, 25, 50, 75, 100] {
        carriage(&format!("Uploading depot 481: {pct}.0%"));
    }
    line("");
    line("Committing build for appID 480...");
    line("Successfully finished appID 480 build (BuildID 9001)");
}

fn main() {
    match scenario().as_str() {
        "login_fail" => {
            line("Connecting anonymously to Steam Public...OK");
            line("FAILED login with result code Invalid Password");
            std::process::exit(5);
        }
        "prompt" => {
            line("Connecting anonymously to Steam Public...OK");
            raw("Please enter Steam Guard code: ");
            let mut input = String::new();
            let _ = std::io::stdin().lock().read_line(&mut input);
            line("");
            line("Two-factor code accepted");
            line("Logging in user 'tester' to Steam Public...OK");
            succeed();
        }
        "progress" => {
            line("Logging in user 'tester' to Steam Public...OK");
            for step in 0..=10 {
                carriage(&format!("Uploading depot 481: {}.0%", step * 10));
            }
            line("");
            line("Successfully finished appID 480 build (BuildID 9002)");
        }
        "cancel" => {
            line("Connecting anonymously to Steam Public...OK");
            line("Uploading depot 481: 0.0%");
            loop {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
        "guard_required" => {
            line("Connecting anonymously to Steam Public...OK");
            raw("Please enter the Steam Guard code from your device: ");
            let mut input = String::new();
            let _ = std::io::stdin().lock().read_line(&mut input);
            line("");
            line("Logged in OK");
            std::process::exit(0);
        }
        "wrong_password" => {
            raw("password: ");
            let mut input = String::new();
            let _ = std::io::stdin().lock().read_line(&mut input);
            line("");
            line("FAILED login with result code Invalid Password");
            std::process::exit(5);
        }
        "stored_password" => {
            raw("password: ");
            let mut input = String::new();
            let _ = std::io::stdin().lock().read_line(&mut input);
            line("");
            let on_argv = std::env::args().any(|a| a.contains("hunter2"));
            if input.trim() == "hunter2" && !on_argv {
                line("Logging in user 'stored_password' to Steam Public...OK");
                std::process::exit(0);
            }
            line("FAILED login with result code Invalid Password");
            std::process::exit(5);
        }
        "net_reset" => {
            line("Connecting anonymously to Steam Public...");
            line("ERROR! Connection reset by peer while contacting the content server");
            std::process::exit(1);
        }
        "net_timeout" => {
            line("Logging in user 'tester' to Steam Public...OK");
            line("ERROR! Upload timed out after 60 seconds");
            std::process::exit(1);
        }
        "commit_fail" => {
            line("Logging in user 'tester' to Steam Public...OK");
            line(
                "Waiting for client config...ERROR! Info request for AppId 7 returned error Timeout.",
            );
            line("OK");
            line("[2026-10-26 19:47:49]: Building depot 481...");
            line("Uploading content...");
            line("[2026-10-26 19:48:30]: ERROR! Failed to commit build for AppID 480 : Failure");
            std::process::exit(5);
        }
        "rate_limit" => {
            line("Connecting anonymously to Steam Public...");
            line("ERROR! Rate limit exceeded, service temporarily unavailable");
            std::process::exit(1);
        }
        _ => {
            line("Connecting anonymously to Steam Public...OK");
            line("Logging in user 'tester' to Steam Public...OK");
            succeed();
        }
    }
}
