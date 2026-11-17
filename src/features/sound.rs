//! A short system sound when a build or pipeline run ends (opt-in in Settings).

use eframe::egui;

/// Play the finish sound if the user turned it on. `success` picks the system
/// "OK" sound, otherwise the "error" sound.
pub fn on_finish(ctx: &egui::Context, success: bool) {
    let settings = crate::config::load_settings().unwrap_or_default();
    if !settings.sound_on_finish {
        return;
    }
    let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
    if settings.sound_only_unfocused && focused {
        return;
    }
    play(success);
}

fn play(success: bool) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONHAND, MB_OK};
        // SAFETY: MessageBeep takes a plain flag and has no pointer arguments.
        unsafe {
            MessageBeep(if success { MB_OK } else { MB_ICONHAND });
        }
    }
    #[cfg(not(windows))]
    let _ = success;
}
