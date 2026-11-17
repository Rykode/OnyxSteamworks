#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod config;
mod features;
mod history;
mod keepawake;
mod kv;
mod model;
mod proctree;
mod secrets;
mod steamcmd;
mod steamcmd_patterns;
mod ui;
mod validate;
mod vdf;

use eframe::egui;

fn main() -> eframe::Result {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    // `--open <folder>` is the one argument that still opens the window; it is how other
    // apps such as GodotHub hand a freshly exported build over. Anything else is the CLI.
    let open_folder = match args.as_slice() {
        [] => None,
        [flag, folder] if flag == "--open" => Some(std::path::PathBuf::from(folder)),
        _ => std::process::exit(cli::run()),
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Onyx")
            .with_inner_size([1320.0, 840.0])
            .with_min_inner_size([1080.0, 700.0])
            .with_icon(std::sync::Arc::new(ui::window::app_icon()))
            .with_decorations(false),
        ..Default::default()
    };

    eframe::run_native(
        "Onyx",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, open_folder)))),
    )
}
