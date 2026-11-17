use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui::{self, Color32, Id};

static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);

/// The in-app "Reduce motion" setting.
pub fn set_reduce_motion(on: bool) {
    REDUCE_MOTION.store(on, Ordering::Relaxed);
}

/// Motion is reduced when the user asked for it in Onyx or turned animations off
/// for the whole system (Windows: Settings > Accessibility > Visual effects).
pub fn reduce_motion() -> bool {
    REDUCE_MOTION.load(Ordering::Relaxed) || os_reduces_motion()
}

/// True when the operating system asks apps not to animate. Read once per run.
pub fn os_reduces_motion() -> bool {
    static OS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OS.get_or_init(read_os_setting)
}

#[cfg(windows)]
fn read_os_setting() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW,
    };
    let mut enabled: i32 = 1;
    // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL into the pointed-to i32.
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            &mut enabled as *mut i32 as *mut core::ffi::c_void,
            0,
        )
    };
    ok != 0 && enabled == 0
}

#[cfg(not(windows))]
fn read_os_setting() -> bool {
    false
}

pub fn dur(seconds: f32) -> f32 {
    if reduce_motion() { 0.0 } else { seconds }
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let f = t - 1.0;
    f * f * f + 1.0
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let f = 2.0 * t - 2.0;
        0.5 * f * f * f + 1.0
    }
}

pub fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 1.701_58;
    const C3: f32 = C1 + 1.0;
    let f = t - 1.0;
    1.0 + C3 * f * f * f + C1 * f * f
}

pub fn anim_bool(
    ctx: &egui::Context,
    id: Id,
    target: bool,
    duration: f32,
    easing: fn(f32) -> f32,
) -> f32 {
    ctx.animate_bool_with_time_and_easing(id, target, dur(duration), easing)
}

pub fn anim_value(ctx: &egui::Context, id: Id, target: f32, duration: f32) -> f32 {
    ctx.animate_value_with_time(id, target, dur(duration))
}
#[derive(Clone, Copy)]
pub struct Transition {
    start: f64,
    duration: f32,
    easing: fn(f32) -> f32,
}

impl Transition {
    pub fn new(ctx: &egui::Context, duration: f32, easing: fn(f32) -> f32) -> Self {
        Self {
            start: ctx.input(|i| i.time),
            duration: dur(duration),
            easing,
        }
    }

    pub fn raw(&self, ctx: &egui::Context) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        let elapsed = (ctx.input(|i| i.time) - self.start) as f32;
        (elapsed / self.duration).clamp(0.0, 1.0)
    }

    pub fn progress(&self, ctx: &egui::Context) -> f32 {
        let raw = self.raw(ctx);
        if raw < 1.0 {
            ctx.request_repaint();
        }
        (self.easing)(raw)
    }

    pub fn is_done(&self, ctx: &egui::Context) -> bool {
        self.raw(ctx) >= 1.0
    }
}

pub fn with_opacity(color: Color32, opacity: f32) -> Color32 {
    color.gamma_multiply(opacity.clamp(0.0, 1.0))
}
