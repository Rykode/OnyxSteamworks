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
    let inset = px(8.0);
    let rail_w = px(216.0);
    let rail = Rect::from_min_max(
        pos2(window.left() + inset, titlebar.bottom()),
        pos2(window.left() + inset + rail_w, window.bottom() - inset),
    );
    let frame = Rect::from_min_max(
        pos2(rail.right() + inset, titlebar.bottom()),
        pos2(window.right() - inset, window.bottom() - inset),
    );
    let header = Rect::from_min_size(frame.min, vec2(frame.width(), px(60.0)));
    let body = Rect::from_min_max(pos2(frame.left(), header.bottom() + inset), frame.max);
    let content = Rect::from_min_max(
        pos2(body.left() + px(28.0), body.top() + px(22.0)),
        pos2(body.right() - px(28.0), body.bottom() - px(24.0)),
    );
    let toast_anchor = pos2(frame.right() - px(16.0), header.bottom() + px(12.0));
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

impl Layout {}
