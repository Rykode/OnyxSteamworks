use eframe::egui::{self, Align, FontId, Layout, Rect, RichText, Sense, UiBuilder};
use egui_extras::{Column, TableBuilder};

use super::{fmt_duration, rel_time, status_color, status_label};
use crate::model::{BuildRecord, Project};
use crate::ui::theme;

pub(super) fn show(
    ui: &mut egui::Ui,
    rect: Rect,
    rows: &[&BuildRecord],
    projects: &[Project],
    s: f32,
) -> Option<usize> {
    let px = |v: f32| theme::px(v, s);
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
    child.set_clip_rect(rect);
    let ui = &mut child;

    if rows.is_empty() {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No builds match these filters.",
            FontId::new(theme::text_px(13.0, s), theme::family_medium()),
            theme::ink_muted(),
        );
        return None;
    }

    let mut clicked = None;
    TableBuilder::new(ui)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::exact(px(92.0)))
        .column(Column::remainder().at_least(px(120.0)))
        .column(Column::exact(px(88.0)))
        .column(Column::exact(px(92.0)))
        .column(Column::exact(px(72.0)))
        .column(Column::remainder().at_least(px(140.0)))
        .header(px(26.0), |mut header| {
            for title in ["Date", "Project", "Branch", "Status", "Time", "Description"] {
                header.col(|ui| {
                    ui.label(
                        RichText::new(title)
                            .font(FontId::new(theme::text_px(11.0, s), theme::family_label()))
                            .color(theme::ink_muted()),
                    );
                });
            }
        })
        .body(|body| {
            body.rows(px(34.0), rows.len(), |mut row| {
                let i = row.index();
                let rec = rows[i];
                {
                    cell(&mut row, &rel_time(rec.timestamp), theme::ink_dim(), s);
                    let name = projects
                        .iter()
                        .find(|p| p.id == rec.project_id)
                        .map(|p| {
                            if p.name.is_empty() {
                                "Untitled"
                            } else {
                                p.name.as_str()
                            }
                        })
                        .unwrap_or("Unknown")
                        .to_string();
                    cell(&mut row, &name, theme::ink(), s);
                    cell(&mut row, &rec.branch, theme::ink_dim(), s);
                    cell(
                        &mut row,
                        status_label(rec.status),
                        status_color(rec.status),
                        s,
                    );
                    cell(&mut row, &fmt_duration(rec.duration_ms), theme::ink_dim(), s);
                    cell(&mut row, &rec.description, theme::ink_dim(), s);
                    let resp = row.response();
                    crate::ui::kit::accessible(
                        &resp,
                        resp.rect,
                        egui::Role::Button,
                        &format!(
                            "{} build on {}, {}, {}",
                            status_label(rec.status),
                            rec.branch,
                            name,
                            rel_time(rec.timestamp)
                        ),
                        None,
                        0.0,
                    );
                    if resp.clicked() {
                        clicked = Some(i);
                    }
                }
            });
        });
    clicked
}

fn cell(row: &mut egui_extras::TableRow, text: &str, color: egui::Color32, s: f32) {
    row.col(|ui| {
        ui.label(
            RichText::new(text)
                .font(FontId::new(theme::text_px(12.5, s), theme::family_medium()))
                .color(color),
        );
    });
}
