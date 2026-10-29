use eframe::egui::{Pos2, Rect, pos2, vec2};

use crate::ui::theme;

pub struct Layout {
    pub scale: f32,
    pub rail: Rect,
    pub titlebar: Rect,
    pub header: Rect,
    pub body: Rect,
    pub content: Rect,
    pub toast_anchor: Pos2,
}

pub fn compute(window: Rect) -> Layout {
    let s = theme::scale(window.size());
    let px = |v: f32| v * s;
    let titlebar = Rect::from_min_size(
        window.min,
        vec2(window.width(), px(crate::ui::window::TITLEBAR_H)),
    );
    let rail = Rect::from_min_max(
        pos2(window.left() + px(15.0), titlebar.bottom()),
        pos2(window.left() + px(229.0), window.bottom() - px(19.0)),
    );
    let main_left = window.left() + px(251.0);
    let header = Rect::from_min_max(
        pos2(main_left, titlebar.bottom()),
        pos2(window.right() - px(23.0), titlebar.bottom() + px(48.0)),
    );
    let body = Rect::from_min_max(
        pos2(main_left, header.bottom() + px(12.0)),
        pos2(header.right(), window.bottom() - px(22.0)),
    );
    let content = Rect::from_min_max(
        pos2(body.left() + px(20.0), body.top() + px(20.0)),
        pos2(body.right() - px(20.0), body.bottom() - px(20.0)),
    );
    let toast_anchor = pos2(body.right() - px(16.0), body.top() + px(12.0));
    Layout {
        scale: s,
        rail,
        titlebar,
        header,
        body,
        content,
        toast_anchor,
    }
}
