use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

use eframe::egui;

use super::{BuildEvent, BuildResult, PromptKind, RunOptions, cancel, start_build, start_login};
use crate::model::Project;

/// The `fake_steamcmd` binary (`cargo build --bin fake_steamcmd`) sits in the target
/// profile folder, e.g. `target/debug`. Unit test binaries live somewhere below it:
/// `target/debug/deps` up to Rust 1.99, `target/debug/build/onyx/<hash>/out` from
/// 1.100. So look in each folder above the test binary until it turns up.
pub(crate) fn fake_bin() -> PathBuf {
    if let Some(p) = option_env!("CARGO_BIN_EXE_fake_steamcmd") {
        return PathBuf::from(p);
    }
    let name = format!("fake_steamcmd{}", std::env::consts::EXE_SUFFIX);
    let exe = std::env::current_exe().expect("current exe");
    exe.ancestors()
        .skip(1)
        .map(|dir| dir.join(&name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("{name} not found above {}; run `cargo build --bin fake_steamcmd` first", exe.display()))
}

fn project() -> Project {
    Project {
        steamcmd_path: fake_bin(),
        steam_username: "tester".to_string(),
        ..Default::default()
    }
}

fn scenario_opts(name: &str) -> RunOptions {
    let dir = std::env::temp_dir().join(format!("onyx-k5-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let vdf = dir.join(format!("{name}.vdf"));
    std::fs::write(&vdf, "\"appbuild\"\n{\n}\n").expect("vdf");
    RunOptions {
        app_build_vdf: vdf,
        log_path: Some(dir.join("build.log")),
        interactive: true,
    }
}

fn login_project(scenario: &str) -> Project {
    Project {
        steamcmd_path: fake_bin(),
        steam_username: scenario.to_string(),
        ..Default::default()
    }
}

fn run_login(scenario: &str) -> Vec<BuildEvent> {
    let ctx = egui::Context::default();
    let (tx, rx) = channel();
    let handle = start_login(&login_project(scenario), &ctx, tx).expect("start_login");
    let mut events = Vec::new();
    loop {
        match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(ev) => {
                let done = matches!(ev, BuildEvent::Finished(_));
                if let BuildEvent::Prompt(_) = &ev {
                    handle.send_input("12345");
                }
                events.push(ev);
                if done {
                    break;
                }
            }
            Err(_) => panic!("timed out waiting for {scenario}"),
        }
    }
    events
}

fn run(name: &str) -> (Vec<BuildEvent>, PathBuf) {
    let ctx = egui::Context::default();
    let (tx, rx) = channel();
    let opts = scenario_opts(name);
    let handle = start_build(&project(), &opts, &ctx, tx).expect("start");
    let mut events = Vec::new();
    loop {
        match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(ev) => {
                let done = matches!(ev, BuildEvent::Finished(_));
                if let BuildEvent::Prompt(_) = &ev {
                    handle.send_input("12345");
                }
                events.push(ev);
                if done {
                    break;
                }
            }
            Err(_) => panic!("timed out waiting for {name}"),
        }
    }
    (events, handle.log_path.clone())
}

fn finished(events: &[BuildEvent]) -> Option<BuildResult> {
    events.iter().find_map(|e| match e {
        BuildEvent::Finished(r) => Some(r.clone()),
        _ => None,
    })
}

#[test]
fn steamcmd_runs_from_its_install_directory() {
    let exe = fake_bin();
    let cmd = super::command_for(&exe);
    assert_eq!(cmd.get_current_dir(), exe.parent());
}

#[test]
fn success_reports_app_id_and_logs() {
    let (events, log) = run("success");
    assert_eq!(
        finished(&events),
        Some(BuildResult::Success {
            app_id: "480".to_string()
        })
    );
    assert!(events.iter().any(|e| matches!(e, BuildEvent::Progress(_))));
    let contents = std::fs::read_to_string(&log).expect("log file");
    assert!(contents.contains("Successfully finished appID 480"));
}

#[test]
fn commit_failure_wins_over_client_config_noise() {
    let (events, _) = run("commit_fail");
    match finished(&events) {
        Some(BuildResult::Failure { message }) => {
            assert!(
                message.contains("Failed to commit build for AppID 480"),
                "{message}"
            );
            assert!(!message.contains("AppId 7"), "{message}");
        }
        other => panic!("expected failure, got {other:?}"),
    }
}

#[test]
fn login_failure_reports_failure() {
    let (events, _) = run("login_fail");
    assert!(matches!(
        finished(&events),
        Some(BuildResult::Failure { .. })
    ));
}

#[test]
fn prompt_then_input_succeeds() {
    let (events, _) = run("prompt");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BuildEvent::Prompt(PromptKind::SteamGuard)))
    );
    assert_eq!(
        finished(&events),
        Some(BuildResult::Success {
            app_id: "480".to_string()
        })
    );
}

#[test]
fn progress_fractions_emitted() {
    let (events, _) = run("progress");
    let count = events
        .iter()
        .filter(|e| matches!(e, BuildEvent::Progress(_)))
        .count();
    assert!(count >= 3, "expected several progress events, got {count}");
}

#[test]
fn cancel_reports_cancelled() {
    let ctx = egui::Context::default();
    let (tx, rx) = channel();
    let opts = scenario_opts("cancel");
    let handle = start_build(&project(), &opts, &ctx, tx).expect("start");
    rx.recv_timeout(Duration::from_secs(10))
        .expect("first event");
    cancel(&handle);
    loop {
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(BuildEvent::Finished(r)) => {
                assert_eq!(r, BuildResult::Cancelled);
                break;
            }
            Ok(_) => continue,
            Err(_) => panic!("timed out waiting for cancel"),
        }
    }
}

#[test]
fn login_guard_required_then_code_succeeds() {
    let events = run_login("guard_required");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BuildEvent::Prompt(PromptKind::SteamGuard)))
    );
    assert_eq!(
        finished(&events),
        Some(BuildResult::Success {
            app_id: String::new()
        })
    );
}

#[test]
fn login_wrong_password_reports_failure() {
    let events = run_login("wrong_password");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, BuildEvent::Prompt(PromptKind::Password)))
    );
    match finished(&events) {
        Some(BuildResult::Failure { message }) => {
            assert!(message.to_ascii_lowercase().contains("invalid password"))
        }
        other => panic!("expected failure, got {other:?}"),
    }
}

#[test]
fn login_fail_reports_failure() {
    let events = run_login("login_fail");
    assert!(matches!(
        finished(&events),
        Some(BuildResult::Failure { .. })
    ));
}

#[test]
fn stored_password_is_sent_on_stdin_not_argv() {
    let ctx = egui::Context::default();
    let (tx, rx) = channel();
    let dir = std::env::temp_dir().join(format!("onyx-pw-{}", uuid::Uuid::new_v4()));
    let mut cmd = super::command_for(&fake_bin());
    cmd.arg("+login").arg("stored_password").arg("+quit");
    let _handle = super::run(
        cmd,
        dir.join("login.log"),
        &ctx,
        tx,
        Some("hunter2".to_string()),
    )
    .expect("run");
    let mut events = Vec::new();
    loop {
        match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(ev) => {
                let done = matches!(ev, BuildEvent::Finished(_));
                events.push(ev);
                if done {
                    break;
                }
            }
            Err(_) => panic!("timed out waiting for stored_password"),
        }
    }
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, BuildEvent::Prompt(PromptKind::Password)))
    );
    assert_eq!(
        finished(&events),
        Some(BuildResult::Success {
            app_id: String::new()
        })
    );
}
