use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Vec2};

use super::state::BuildState;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

fn jump_offset(row_height: f32, row_spacing: f32, rows: usize) -> f32 {
    rows.max(1) as f32 * (row_height + row_spacing)
}

pub fn show(state: &mut BuildState, ui: &mut egui::Ui, card: Rect, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    let title = kit::caps(
        ui.painter(),
        Pos2::new(card.left() + px(21.0), card.top() + px(36.0)),
        Align2::LEFT_CENTER,
        "Console",
        12.5,
        theme::ink_muted(),
        s,
    );
    let copy_w = px(90.0);
    let copy = Rect::from_min_size(
        Pos2::new(card.right() - px(21.0) - copy_w, card.top() + px(18.0)),
        Vec2::new(copy_w, px(36.0)),
    );
    let errors = state.log.iter().filter(|l| l.error).count();
    let errors_w = if errors > 0 { px(128.0) } else { 0.0 };
    let copy_errors = Rect::from_min_size(
        Pos2::new(copy.left() - px(8.0) - errors_w, copy.top()),
        Vec2::new(errors_w, copy.height()),
    );
    let search_right = if errors > 0 { copy_errors.left() } else { copy.left() };
    let search_rect = Rect::from_min_max(
        Pos2::new(title.right().max(card.left() + px(100.0) - px(12.0)) + px(12.0), card.top() + px(20.5)),
        Pos2::new(search_right - px(12.0), card.top() + px(51.5)),
    );
    let search = Input::new(&mut state.search)
        .hint("Filter log (Ctrl+F)")
        .icon(theme::icon::MAGNIFYING_GLASS)
        .show(ui, search_rect, s);
    if !crate::ui::kit::modal_active(ctx)
        && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F))
    {
        search.request_focus();
    }
    if search.has_focus() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.search.clear();
        search.surrender_focus();
    }
    if errors > 0
        && Button::new("Copy errors", ButtonKind::Secondary)
            .icon(theme::icon::WARNING)
            .show(ui, copy_errors, s)
            .clicked()
    {
        ctx.copy_text(state.error_text());
    }
    if Button::new("Copy", ButtonKind::Secondary)
        .icon(theme::icon::COPY)
        .show(ui, copy, s)
        .clicked()
    {
        ctx.copy_text(state.log_text());
    }

    let well = Rect::from_min_max(
        Pos2::new(card.left() + px(21.0), card.top() + px(67.0)),
        Pos2::new(card.right() - px(21.0), card.bottom() - px(12.5)),
    );
    ui.painter()
        .rect_filled(well, theme::corner(px(14.0)), theme::well());
    let log_rect = Rect::from_min_max(
        Pos2::new(well.left() + px(16.0), well.top() + px(14.0)),
        Pos2::new(well.right() - px(8.0), well.bottom() - px(10.0)),
    );
    let query = state.search.as_bytes();
    let rows: Vec<(&str, &str, bool)> = state
        .log
        .iter()
        .filter(|l| {
            query.is_empty()
                || l.text
                    .as_bytes()
                    .windows(query.len())
                    .any(|w| w.eq_ignore_ascii_case(query))
        })
        .map(|l| (l.time.as_str(), l.text.as_str(), l.error))
        .collect();

    let id = ui.make_persistent_id("build_console");
    let jump = ctx.data_mut(|d| d.get_temp::<bool>(id.with("jump")).unwrap_or(false));
    let row_h = px(23.6);
    let font = FontId::new(theme::text_px(13.0, s), egui::FontFamily::Monospace);
    let text_x = px(74.0);

    let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(log_rect));
    cui.set_clip_rect(log_rect);
    cui.spacing_mut().item_spacing.y = 0.0;
    let mut sa = egui::ScrollArea::vertical()
        .id_salt("build_console_scroll")
        .auto_shrink([false, false])
        .stick_to_bottom(true);
    if jump {
        let content_height = jump_offset(row_h, 0.0, rows.len());
        sa = sa.vertical_scroll_offset(content_height);
        ctx.data_mut(|d| d.insert_temp(id.with("jump"), false));
    }
    let mut last_visible = 0usize;
    sa.show_rows(&mut cui, row_h, rows.len().max(1), |ui, range| {
        last_visible = range.end;
        if rows.is_empty() {
            let (r, resp) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, "No output yet."));
            ui.painter().text(
                r.left_center(),
                Align2::LEFT_CENTER,
                "No output yet.",
                font.clone(),
                theme::ink_faint(),
            );
            return;
        }
        for i in range {
            let (r, resp) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
            if let Some((time, text, error)) = rows.get(i) {
                let spoken = if *error { format!("{time} error: {text}") } else { format!("{time} {text}") };
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &spoken));
                ui.painter().text(
                    r.left_center(),
                    Align2::LEFT_CENTER,
                    *time,
                    font.clone(),
                    theme::ink_faint(),
                );
                line(ui.painter(), Pos2::new(r.left() + text_x, r.center().y), text, *error, &font);
            }
        }
    });

    let scrolled_up = !rows.is_empty() && last_visible < rows.len();
    if scrolled_up {
        let pill = Rect::from_min_size(
            Pos2::new(log_rect.right() - px(132.0), log_rect.bottom() - px(42.0)),
            Vec2::new(px(124.0), px(32.0)),
        );
        if Button::new("Jump to latest", ButtonKind::Secondary)
            .show(ui, pill, s)
            .clicked()
        {
            ctx.data_mut(|d| d.insert_temp(id.with("jump"), true));
        }
    }
}

fn line(painter: &egui::Painter, pos: Pos2, text: &str, error: bool, font: &FontId) {
    let lower = text.to_ascii_lowercase();
    let color = if error {
        theme::danger()
    } else if lower.starts_with("note") || lower.contains("warning") {
        theme::warning()
    } else if lower.contains(" done") || lower.contains("success") || lower.contains("finished") {
        theme::success()
    } else if lower.contains("upload") || lower.trim_end().ends_with('%') {
        theme::accent_hi()
    } else {
        theme::ink()
    };
    let trimmed = text.trim_end();
    if color == theme::ink() && trimmed.ends_with("OK") {
        let head = &trimmed[..trimmed.len() - 2];
        let r = painter.text(pos, Align2::LEFT_CENTER, head, font.clone(), theme::ink());
        painter.text(
            Pos2::new(r.right(), pos.y),
            Align2::LEFT_CENTER,
            "OK",
            font.clone(),
            theme::success(),
        );
    } else {
        painter.text(pos, Align2::LEFT_CENTER, text, font.clone(), color);
    }
}

#[cfg(test)]
mod tests {
    use super::jump_offset;

    #[test]
    fn jump_offset_is_finite() {
        assert!(jump_offset(16.0, 4.0, 0).is_finite());
        assert!(jump_offset(16.0, 4.0, 10_000).is_finite());
        assert_eq!(jump_offset(16.0, 4.0, 3), 60.0);
    }
}
