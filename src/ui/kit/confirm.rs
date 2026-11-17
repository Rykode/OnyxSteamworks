use eframe::egui::{self, FontId, Pos2, Rect, Vec2};

use super::{Button, ButtonKind, label, modal_with};
use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Pending,
    Confirmed,
    Cancelled,
}

pub struct Confirm<'a> {
    pub eyebrow: &'a str,
    pub question: &'a str,
    pub detail: &'a str,
    pub confirm: &'a str,
    pub cancel: &'a str,
    pub danger: bool,
}

impl<'a> Confirm<'a> {
    pub fn danger(eyebrow: &'a str, question: &'a str, detail: &'a str, confirm: &'a str) -> Self {
        Self {
            eyebrow,
            question,
            detail,
            confirm,
            cancel: "Cancel",
            danger: true,
        }
    }
}

pub fn confirm(ui: &mut egui::Ui, id: egui::Id, spec: &Confirm, s: f32) -> Choice {
    let px = |v: f32| theme::px(v, s);
    let mut open = true;
    let mut choice = Choice::Pending;
    let detail_galley = ui.painter().layout(
        spec.detail.to_string(),
        FontId::new(theme::text_px(12.5, s), theme::family_medium()),
        theme::ink_muted(),
        px(412.0),
    );
    let height = px(176.0) + detail_galley.size().y;
    modal_with(
        ui,
        id,
        &mut open,
        Vec2::new(px(468.0), height),
        s,
        true,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(28.0);
            let painter = ui.painter().clone();
            label(&painter, Pos2::new(x, d.top() + px(26.0)), spec.eyebrow, s);
            let q = painter.layout(
                spec.question.to_string(),
                FontId::new(theme::text_px(16.0, s), theme::family_semibold()),
                theme::ink(),
                d.width() - px(56.0),
            );
            let qh = q.size().y;
            painter.galley(Pos2::new(x, d.top() + px(48.0)), q, theme::ink());
            painter.galley(
                Pos2::new(x, d.top() + px(58.0) + qh),
                detail_galley,
                theme::ink_muted(),
            );
            let by = d.bottom() - px(26.0) - px(42.0);
            let bw = px(170.0);
            if Button::new(spec.cancel, ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, by), Vec2::new(bw, px(42.0))),
                    s,
                )
                .clicked()
            {
                choice = Choice::Cancelled;
            }
            let kind = if spec.danger {
                ButtonKind::Danger
            } else {
                ButtonKind::Primary
            };
            if Button::new(spec.confirm, kind)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - px(28.0) - bw, by),
                        Vec2::new(bw, px(42.0)),
                    ),
                    s,
                )
                .clicked()
            {
                choice = Choice::Confirmed;
            }
            choice != Choice::Pending
        },
    );
    if choice == Choice::Pending && !open {
        Choice::Cancelled
    } else {
        choice
    }
}
