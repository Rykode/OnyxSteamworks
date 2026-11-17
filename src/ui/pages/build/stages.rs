use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke, Vec2};

use super::state;
use crate::app::App;
use crate::model::Project;
use crate::steamcmd::{BuildResult, Stage};
use crate::ui::kit;
use crate::ui::theme;

const SHOWN: usize = 5;

pub(super) fn show(app: &mut App, ui: &mut egui::Ui, card: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let st = &app.build_ui;
    let filled = st.filled_stages().min(SHOWN);
    kit::card_title(ui.painter(), card, "Stages", s);
    let current = if st.running {
        st.stage
            .and_then(|c| state::STAGES.iter().position(|(s, _)| *s == c))
    } else {
        None
    };
    let failed_at = match st.outcome {
        Some(BuildResult::Failure { .. }) | Some(BuildResult::Cancelled) => st
            .stage
            .and_then(|c| state::STAGES.iter().position(|(s, _)| *s == c)),
        _ => None,
    };
    let shown = current.map(|c| c + 1).unwrap_or(filled);
    kit::card_note(ui.painter(), card, &format!("{} of {SHOWN}", shown.min(SHOWN)), s);

    let x0 = card.left() + px(20.0);
    let x1 = card.right() - px(20.0);
    let painter = ui.painter();
    for (i, (stage, label)) in state::STAGES.iter().take(SHOWN).enumerate() {
        let cy = card.top() + px(65.0 + 48.4 * i as f32);
        if i > 0 {
            let sy = cy - px(24.2);
            painter.line_segment([Pos2::new(x0, sy), Pos2::new(x1, sy)], Stroke::new(1.0, theme::LINE_SOFT));
        }
        let c = Pos2::new(x0 + px(11.7), cy);
        let r = px(11.7);
        let is_current = current == Some(i);
        let is_failed = failed_at == Some(i);
        let done = !is_current && !is_failed && i < filled;
        let (fill, glyph, ink) = if is_failed {
            (theme::with_alpha(theme::DANGER, 40), theme::icon::X, theme::DANGER)
        } else if is_current {
            painter.circle_stroke(c, r + px(2.5), Stroke::new(px(2.0), theme::with_alpha(theme::ACCENT, 70)));
            (theme::ACCENT, stage_glyph(*stage), theme::PRIMARY_INK)
        } else if done {
            (theme::with_alpha(theme::SUCCESS, 34), theme::icon::CHECK, theme::SUCCESS)
        } else {
            (theme::FIELD_HI, theme::icon::PACKAGE, theme::INK_FAINT)
        };
        painter.circle_filled(c, r, fill);
        painter.text(c, Align2::CENTER_CENTER, glyph.to_string(), theme::icon_font(px(11.5), false), ink);
        let text_color = if done || is_current || is_failed { theme::INK } else { theme::INK_FAINT };
        painter.text(
            Pos2::new(x0 + px(37.5), cy),
            Align2::LEFT_CENTER,
            *label,
            FontId::new(px(15.0), theme::family_medium()),
            text_color,
        );
        let time = st
            .stage_time(*stage)
            .filter(|_| done || is_current || is_failed)
            .map(|d| {
                let t = d.as_secs();
                format!("{:02}:{:02}", t / 60, t % 60)
            })
            .unwrap_or_else(|| "--:--".to_string());
        let time_color = if is_current {
            theme::ACCENT_HI
        } else if done || is_failed {
            theme::INK_FAINT
        } else {
            theme::with_alpha(theme::INK_FAINT, 150)
        };
        painter.text(
            Pos2::new(x1, cy),
            Align2::RIGHT_CENTER,
            time,
            FontId::new(px(13.0), egui::FontFamily::Monospace),
            time_color,
        );
    }
}

fn stage_glyph(stage: Stage) -> char {
    match stage {
        Stage::Connecting => theme::icon::ROCKET,
        Stage::LoggingIn => theme::icon::GEAR,
        Stage::Scanning => theme::icon::MAGNIFYING_GLASS,
        Stage::Uploading => theme::icon::UPLOAD,
        Stage::Committing | Stage::Finished => theme::icon::PACKAGE,
    }
}

fn human_bytes(b: f64) -> String {
    if b >= 1e9 {
        format!("{:.2} GB", b / 1e9)
    } else {
        format!("{:.0} MB", b / 1e6)
    }
}

pub(super) fn speed_card(app: &App, ui: &mut egui::Ui, card: Rect, project: &Project, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::card_title(ui.painter(), card, "Upload speed", s);
    let st = &app.build_ui;
    let bytes = st
        .review
        .summary_for(&project.id)
        .map(|d| d.estimated_upload() as f64)
        .filter(|b| *b > 0.0);
    let rate = st.speed.samples().last().copied().unwrap_or(0.0) as f64;
    let x0 = card.left() + px(20.0);
    let x1 = card.right() - px(20.0);
    let painter = ui.painter();
    let (value, unit) = match bytes {
        Some(b) if rate > 0.0 => (format!("{:.1}", rate * b / 1e6), "MB/s"),
        _ => (format!("{:.0}", st.progress * 100.0), "% done"),
    };
    let big = painter.text(
        Pos2::new(x0, card.top() + px(67.5)),
        Align2::LEFT_CENTER,
        value,
        FontId::new(px(37.0), theme::family_bold()),
        theme::INK,
    );
    painter.text(
        Pos2::new(big.right() + px(5.0), big.bottom() - px(7.0)),
        Align2::LEFT_BOTTOM,
        unit,
        FontId::new(px(14.0), theme::family_medium()),
        theme::INK_MUTED,
    );
    let eta = st
        .speed
        .eta_secs()
        .map(|t| format!("ETA {:02}:{:02}", t / 60, t % 60))
        .unwrap_or_else(|| "ETA --:--".to_string());
    painter.text(
        Pos2::new(x1, card.top() + px(75.5)),
        Align2::RIGHT_CENTER,
        eta,
        FontId::new(px(13.0), egui::FontFamily::Monospace),
        theme::INK_DIM,
    );

    let chart = Rect::from_min_max(
        Pos2::new(x0, card.top() + px(106.0)),
        Pos2::new(x1, card.top() + px(165.0)),
    );
    area_chart(painter, chart, st.speed.samples(), s);

    let bar = Rect::from_min_size(Pos2::new(x0, card.top() + px(186.0)), Vec2::new(x1 - x0, px(6.0)));
    let id = ui.make_persistent_id("upload_bar");
    let t = crate::ui::anim::anim_value(ui.ctx(), id, st.progress.clamp(0.0, 1.0), 0.2);
    let painter = ui.painter();
    painter.rect_filled(bar, theme::corner(px(3.0)), theme::TRACK);
    if t > 0.001 {
        painter.rect_filled(
            Rect::from_min_size(bar.min, Vec2::new(bar.width() * t, bar.height())),
            theme::corner(px(3.0)),
            theme::ACCENT,
        );
    }
    let left = match bytes {
        Some(b) => format!("{} / {}", human_bytes(b * st.progress as f64), human_bytes(b)),
        None => format!("{} elapsed", {
            let e = st.elapsed().as_secs();
            format!("{:02}:{:02}", e / 60, e % 60)
        }),
    };
    let mono = FontId::new(px(12.5), egui::FontFamily::Monospace);
    painter.text(Pos2::new(x0, card.top() + px(213.0)), Align2::LEFT_CENTER, left, mono.clone(), theme::INK_DIM);
    painter.text(
        Pos2::new(x1, card.top() + px(213.0)),
        Align2::RIGHT_CENTER,
        format!("{:.0}%", st.progress * 100.0),
        mono,
        theme::INK_DIM,
    );
}

fn area_chart(painter: &egui::Painter, rect: Rect, samples: &[f32], s: f32) {
    if samples.len() < 2 {
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(theme::px(1.0, s), theme::LINE_SOFT),
        );
        return;
    }
    let max = samples.iter().copied().fold(f32::MIN, f32::max).max(1e-6) * 1.1;
    let n = samples.len();
    let pts: Vec<Pos2> = samples
        .iter()
        .enumerate()
        .map(|(i, v)| {
            Pos2::new(
                rect.left() + rect.width() * (i as f32 / (n - 1) as f32),
                rect.bottom() - rect.height() * (v / max).clamp(0.0, 1.0),
            )
        })
        .collect();
    let mut mesh = egui::Mesh::default();
    let top = theme::with_alpha(theme::ACCENT, 90);
    let bottom = theme::with_alpha(theme::ACCENT, 0);
    for w in pts.windows(2) {
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(w[0], top);
        mesh.colored_vertex(w[1], top);
        mesh.colored_vertex(Pos2::new(w[1].x, rect.bottom()), bottom);
        mesh.colored_vertex(Pos2::new(w[0].x, rect.bottom()), bottom);
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(egui::Shape::mesh(mesh));
    painter.add(egui::Shape::line(pts, Stroke::new(theme::px(2.0, s), theme::ACCENT_HI)));
}

pub(super) fn verify_ready(app: &App) -> bool {
    app.build_ui.verify
        && !app.build_ui.verify_ui.running()
        && matches!(&app.build_ui.outcome, Some(BuildResult::Success { app_id }) if !app_id.is_empty())
}

pub(super) fn verify_offer(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    if !verify_ready(app) {
        return;
    }
    if kit::Button::new("Verify build", kit::ButtonKind::Secondary)
        .icon(theme::icon::CHECK)
        .show(ui, rect, s)
        .clicked()
    {
        let ctx = ui.ctx().clone();
        let project = app.projects.get(app.build_ui.project_sel).cloned();
        let record = project
            .as_ref()
            .and_then(|p| app.history_ui.latest_for(&p.id).cloned());
        if let (Some(project), Some(record)) = (project, record) {
            super::verify::begin(app, &project, record, &ctx);
        }
    }
}
