use eframe::egui::{
    self, Color32, CursorIcon, Pos2, Rect, ResizeDirection, Sense, Stroke, Vec2,
    ViewportCommand,
};

use crate::ui::{anim, theme};

pub const TITLEBAR_H: f32 = 30.0;
const BUTTON_W: f32 = 46.0;
const EDGE: f32 = 6.0;
const CLOSE_RED: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

#[derive(Clone, Copy)]
enum Control {
    Minimize,
    Maximize,
    Close,
}

pub fn round_corners(cc: &eframe::CreationContext<'_>) {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::Graphics::Dwm::{
            DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
        };
        let Ok(handle) = cc.window_handle() else {
            return;
        };
        if let RawWindowHandle::Win32(win) = handle.as_raw() {
            let preference = DWMWCP_ROUND;
            unsafe {
                DwmSetWindowAttribute(
                    win.hwnd.get() as _,
                    DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                    &preference as *const _ as *const core::ffi::c_void,
                    std::mem::size_of_val(&preference) as u32,
                );
            }
        }
    }
    #[cfg(not(windows))]
    let _ = cc;
}

pub fn titlebar(ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

    let drag = ui.interact(
        rect,
        ui.make_persistent_id("titlebar_drag"),
        Sense::click_and_drag(),
    );
    if drag.double_clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if drag.drag_started_by(egui::PointerButton::Primary) {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let w = px(BUTTON_W);
    let mut x = rect.right() - w * 3.0;
    for control in [Control::Minimize, Control::Maximize, Control::Close] {
        let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, rect.height()));
        let id = ui.make_persistent_id(("window_control", control as u8));
        let resp = ui.interact(r, id, Sense::click());
        let hover = anim::anim_bool(
            &ctx,
            id.with("h"),
            resp.hovered(),
            0.1,
            anim::ease_out_cubic,
        );
        let pressed = resp.is_pointer_button_down_on();
        let (fill, ink) = match control {
            Control::Close => (
                theme::with_alpha(
                    CLOSE_RED,
                    (hover * if pressed { 200.0 } else { 255.0 }) as u8,
                ),
                theme::lerp_color(theme::INK_MUTED, Color32::WHITE, hover),
            ),
            _ => (
                theme::with_alpha(theme::FIELD_HI, (hover * 255.0) as u8),
                theme::lerp_color(theme::INK_MUTED, theme::INK, hover),
            ),
        };
        ui.painter().rect_filled(r, theme::corner(0.0), fill);
        draw_glyph(ui.painter(), r.center(), control, maximized, ink, s);
        let tip = match control {
            Control::Minimize => "Minimize",
            Control::Maximize if maximized => "Restore",
            Control::Maximize => "Maximize",
            Control::Close => "Close",
        };
        if resp.on_hover_text(tip).clicked() {
            match control {
                Control::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                Control::Maximize => ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized)),
                Control::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
        }
        x += w;
    }
}

fn draw_glyph(
    painter: &egui::Painter,
    c: Pos2,
    control: Control,
    maximized: bool,
    ink: Color32,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let stroke = Stroke::new(1.0, ink);
    let half = px(5.0);
    let c = Pos2::new(c.x.round() + 0.5, c.y.round() + 0.5);
    match control {
        Control::Minimize => {
            painter.line_segment(
                [Pos2::new(c.x - half, c.y), Pos2::new(c.x + half, c.y)],
                stroke,
            );
        }
        Control::Maximize if maximized => {
            let back = Rect::from_min_max(
                Pos2::new(c.x - half + px(2.0), c.y - half),
                Pos2::new(c.x + half, c.y + half - px(2.0)),
            );
            let front = Rect::from_min_max(
                Pos2::new(c.x - half, c.y - half + px(2.0)),
                Pos2::new(c.x + half - px(2.0), c.y + half),
            );
            painter.line_segment([back.left_top(), back.right_top()], stroke);
            painter.line_segment([back.right_top(), back.right_bottom()], stroke);
            painter.rect_stroke(
                front,
                theme::corner(px(1.0)),
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        Control::Maximize => {
            let r = Rect::from_center_size(c, Vec2::splat(half * 2.0));
            painter.rect_stroke(r, theme::corner(px(1.5)), stroke, egui::StrokeKind::Middle);
        }
        Control::Close => {
            painter.line_segment(
                [
                    Pos2::new(c.x - half, c.y - half),
                    Pos2::new(c.x + half, c.y + half),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + half, c.y - half),
                    Pos2::new(c.x - half, c.y + half),
                ],
                stroke,
            );
        }
    }
}

pub fn resize_edges(ui: &mut egui::Ui, window: Rect, s: f32) {
    let ctx = ui.ctx().clone();
    if ctx.input(|i| {
        i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
    }) {
        return;
    }
    let e = theme::px(EDGE, s).max(4.0);
    let c = e * 2.0;
    let (l, r, t, b) = (window.left(), window.right(), window.top(), window.bottom());
    let zones = [
        (
            Rect::from_min_max(Pos2::new(l, t), Pos2::new(l + c, t + c)),
            ResizeDirection::NorthWest,
            CursorIcon::ResizeNorthWest,
        ),
        (
            Rect::from_min_max(Pos2::new(r - c, t), Pos2::new(r, t + c)),
            ResizeDirection::NorthEast,
            CursorIcon::ResizeNorthEast,
        ),
        (
            Rect::from_min_max(Pos2::new(l, b - c), Pos2::new(l + c, b)),
            ResizeDirection::SouthWest,
            CursorIcon::ResizeSouthWest,
        ),
        (
            Rect::from_min_max(Pos2::new(r - c, b - c), Pos2::new(r, b)),
            ResizeDirection::SouthEast,
            CursorIcon::ResizeSouthEast,
        ),
        (
            Rect::from_min_max(Pos2::new(l + c, t), Pos2::new(r - c, t + e)),
            ResizeDirection::North,
            CursorIcon::ResizeVertical,
        ),
        (
            Rect::from_min_max(Pos2::new(l + c, b - e), Pos2::new(r - c, b)),
            ResizeDirection::South,
            CursorIcon::ResizeVertical,
        ),
        (
            Rect::from_min_max(Pos2::new(l, t + c), Pos2::new(l + e, b - c)),
            ResizeDirection::West,
            CursorIcon::ResizeHorizontal,
        ),
        (
            Rect::from_min_max(Pos2::new(r - e, t + c), Pos2::new(r, b - c)),
            ResizeDirection::East,
            CursorIcon::ResizeHorizontal,
        ),
    ];
    for (i, (zone, dir, cursor)) in zones.into_iter().enumerate() {
        let resp = ui.interact(
            zone,
            ui.make_persistent_id(("resize_edge", i)),
            Sense::drag(),
        );
        if resp.hovered() || resp.dragged() {
            ctx.set_cursor_icon(cursor);
        }
        if resp.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
        }
    }
}

pub fn app_icon() -> egui::IconData {
    const N: u32 = 64;
    let bg = [0x16u8, 0x17, 0x19];
    let top = [0x8A, 0x94, 0xF0];
    let bottom = [0x5E, 0x6A, 0xD2];
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    let n = N as f32;
    let radius = 14.0;
    let inside_rounded = |x: f32, y: f32| {
        let cx = x.clamp(radius, n - radius);
        let cy = y.clamp(radius, n - radius);
        (x - cx).powi(2) + (y - cy).powi(2) <= radius * radius
    };
    for py in 0..N {
        for px in 0..N {
            let mut cover_bg = 0.0;
            let mut cover_gem = 0.0;
            let mut cover_top = 0.0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = px as f32 + (sx as f32 + 0.5) / 4.0;
                    let y = py as f32 + (sy as f32 + 0.5) / 4.0;
                    if !inside_rounded(x, y) {
                        continue;
                    }
                    cover_bg += 1.0;
                    let dx = (x - n / 2.0).abs() / 20.0;
                    let dy = (y - n / 2.0).abs() / 24.0;
                    if dx + dy <= 1.0 {
                        cover_gem += 1.0;
                        if y < n / 2.0 - 2.0 {
                            cover_top += 1.0;
                        }
                    }
                }
            }
            let alpha = cover_bg / 16.0;
            let gem = if cover_bg > 0.0 {
                cover_gem / cover_bg
            } else {
                0.0
            };
            let top_share = if cover_gem > 0.0 {
                cover_top / cover_gem
            } else {
                0.0
            };
            for c in 0..3 {
                let gem_c = top[c] as f32 * top_share + bottom[c] as f32 * (1.0 - top_share);
                let v = bg[c] as f32 * (1.0 - gem) + gem_c * gem;
                rgba.push(v.round() as u8);
            }
            rgba.push((alpha * 255.0).round() as u8);
        }
    }
    egui::IconData {
        rgba,
        width: N,
        height: N,
    }
}
