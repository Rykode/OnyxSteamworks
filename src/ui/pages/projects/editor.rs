use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, UiBuilder, Vec2, pos2};

use super::state::PickField;
use crate::app::App;
use crate::features::template;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

const PITCH: f32 = 77.0;
const FIELD_H: f32 = 39.0;

pub fn show(app: &mut App, ui: &mut egui::Ui, card: Rect, s: f32, ctx: &egui::Context) -> bool {
    let px = |v: f32| theme::px(v, s);
    kit::card_title(ui, card, "Editor", s);
    let save = status(app, ui, card, s);

    let area = Rect::from_min_max(
        pos2(card.left(), card.top() + px(44.0)),
        pos2(card.right(), card.bottom() - px(6.0)),
    );
    let mut changed = false;
    let mut cui = ui.new_child(UiBuilder::new().max_rect(area));
    egui::ScrollArea::vertical()
        .id_salt("editor_scroll")
        .auto_shrink([false, false])
        .show(&mut cui, |ui| {
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(area.width(), px(PITCH * 4.0 + 6.0)),
                Sense::hover(),
            );
            changed |= fields(app, ui, rect, s, ctx);
        });
    if changed {
        app.projects_ui.revalidate();
    }
    save
}

fn status(app: &mut App, ui: &mut egui::Ui, card: Rect, s: f32) -> bool {
    let px = |v: f32| theme::px(v, s);
    let y = card.top() + px(26.0);
    let mut right = card.right() - px(20.0);
    let dirty = app.projects_ui.dirty();
    let mut save = false;
    if let Some(err) = app.projects_ui.first_error().filter(|_| dirty) {
        right = ui
            .painter()
            .text(
                Pos2::new(right, y),
                Align2::RIGHT_CENTER,
                err,
                FontId::new(theme::text_px(12.5, s), theme::family_medium()),
                theme::DANGER,
            )
            .left();
    } else if dirty {
        let btn = Rect::from_min_max(Pos2::new(right - px(64.0), y - px(13.0)), Pos2::new(right, y + px(13.0)));
        save = Button::new("Save", ButtonKind::Primary)
            .show(ui, btn, s)
            .on_hover_text("Ctrl S")
            .clicked();
        right = ui
            .painter()
            .text(
                Pos2::new(btn.left() - px(10.0), y),
                Align2::RIGHT_CENTER,
                "Unsaved changes",
                FontId::new(theme::text_px(12.5, s), theme::family_medium()),
                theme::WARNING,
            )
            .left();
    } else {
        right = kit::card_note(ui.painter(), card, "Saved", s).left();
    }
    if kit::link(
        ui,
        Pos2::new(right - px(16.0), y),
        Align2::RIGHT_CENTER,
        "VDF preview",
        theme::INK_MUTED,
        s,
    )
    .clicked()
    {
        app.projects_ui.vdf_open = true;
    }
    save
}

fn fields(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32, ctx: &egui::Context) -> bool {
    let px = |v: f32| theme::px(v, s);
    let left = rect.left() + px(20.0);
    let width = rect.width() - px(40.0);
    let gap = px(17.0);
    let half = (width - gap) / 2.0;
    let col = |i: usize| left + (half + gap) * i as f32;
    let label_y = |row: usize| rect.top() + px(15.0 + PITCH * row as f32);
    let field = |row: usize, x: f32, w: f32| {
        Rect::from_min_size(
            Pos2::new(x, label_y(row) + px(15.0)),
            Vec2::new(w, px(FIELD_H)),
        )
    };
    let mut changed = false;

    label(ui, Pos2::new(col(0), label_y(0)), "Display name", s);
    changed |= text(app, ui, field(0, col(0), half), "name", false, s);
    label(ui, Pos2::new(col(1), label_y(0)), "Steam app ID", s);
    changed |= text(app, ui, field(0, col(1), half), "app_id", true, s);

    let settings = crate::config::load_settings().unwrap_or_default();
    label(ui, Pos2::new(col(0), label_y(1)), "Content root", s);
    changed |= path(
        app,
        ui,
        field(1, col(0), half),
        PickField::ContentRoot,
        &settings.default_content,
        s,
        ctx,
    );
    label(ui, Pos2::new(col(1), label_y(1)), "Build output", s);
    changed |= path(
        app,
        ui,
        field(1, col(1), half),
        PickField::BuildOutput,
        &settings.default_output,
        s,
        ctx,
    );

    label(ui, Pos2::new(col(0), label_y(2)), "Description template", s);
    let project = app.projects_ui.project();
    let branch = if project.set_live_branch.is_empty() {
        "default".to_string()
    } else {
        project.set_live_branch.clone()
    };
    let resolved = template::expand(&project.desc_template, &template::resolve(&project, &branch));
    if !resolved.trim().is_empty() {
        let clip = Rect::from_min_max(
            Pos2::new(col(0) + px(170.0), label_y(2) - px(10.0)),
            Pos2::new(left + width, label_y(2) + px(10.0)),
        );
        kit::fit_text(
            ui.painter(),
            Pos2::new(left + width, label_y(2)),
            Align2::RIGHT_CENTER,
            &format!("Resolves to  {resolved}"),
            FontId::new(theme::text_px(12.0, s), theme::family_medium()),
            theme::INK_FAINT,
            clip.width(),
        );
    }
    changed |= Input::new(&mut app.projects_ui.base.desc_template)
        .hint("v{version}-{branch} · build {build_number} · {date}")
        .label("Description template")
        .mono(true)
        .show(ui, field(2, left, width), s)
        .changed();

    label(ui, Pos2::new(col(0), label_y(3)), "Version file", s);
    changed |= Input::new(&mut app.projects_ui.base.version_file)
        .hint("version.txt or version.json (optional)")
        .label("Version file")
        .mono(true)
        .show(ui, field(3, col(0), half), s)
        .changed();
    label(ui, Pos2::new(col(1), label_y(3)), "Default account", s);
    default_account(app, ui, field(3, col(1), half), s);
    changed
}

fn label(ui: &egui::Ui, pos: Pos2, text: &str, s: f32) {
    ui.painter().text(
        pos,
        Align2::LEFT_CENTER,
        text,
        FontId::new(theme::text_px(13.0, s), theme::family_medium()),
        theme::INK_DIM,
    );
}

fn value_mut<'a>(app: &'a mut App, key: &str) -> &'a mut String {
    match key {
        "name" => &mut app.projects_ui.form.name,
        "app_id" => &mut app.projects_ui.form.app_id,
        "content_root" => &mut app.projects_ui.form.content_root,
        "build_output_dir" => &mut app.projects_ui.form.build_output,
        _ => unreachable!("unknown editor field: {key}"),
    }
}

fn field_name(key: &str) -> &'static str {
    match key {
        "name" => "Display name",
        "app_id" => "Steam app ID",
        "content_root" => "Content root",
        "build_output_dir" => "Build output",
        _ => "",
    }
}

fn text(app: &mut App, ui: &mut egui::Ui, rect: Rect, key: &str, mono: bool, s: f32) -> bool {
    let err = app.projects_ui.error_for(key).is_some();
    Input::new(value_mut(app, key))
        .label(field_name(key))
        .mono(mono)
        .error(err)
        .show(ui, rect, s)
        .changed()
}

fn path(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    field: PickField,
    default: &str,
    s: f32,
    ctx: &egui::Context,
) -> bool {
    let px = |v: f32| theme::px(v, s);
    let key = match field {
        PickField::ContentRoot => "content_root",
        _ => "build_output_dir",
    };
    let err = app.projects_ui.error_for(key).is_some();
    let hint = if default.is_empty() {
        "Choose a folder".to_string()
    } else {
        default.to_string()
    };
    let changed = Input::new(value_mut(app, key))
        .hint(&hint)
        .label(field_name(key))
        .mono(true)
        .error(err)
        .right_pad(22.0)
        .show(ui, rect, s)
        .changed();
    let btn = Rect::from_center_size(
        Pos2::new(rect.right() - px(22.0), rect.center().y),
        Vec2::splat(px(28.0)),
    );
    let resp = kit::icon_button(ui, btn, theme::icon::FOLDER, "Choose folder", s);
    if resp
        .on_hover_text("Choose folder. Leave empty to use the Settings default.")
        .clicked()
    {
        app.projects_ui.spawn_pick(ctx, field, 0);
    }
    changed
}

fn default_account(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let accounts = crate::config::load_settings()
        .map(|c| c.accounts)
        .unwrap_or_default();
    if accounts.is_empty() {
        ui.painter().text(
            Pos2::new(rect.left(), rect.center().y),
            Align2::LEFT_CENTER,
            "Add accounts in Settings > Accounts",
            FontId::new(theme::text_px(12.5, s), theme::family_medium()),
            theme::INK_FAINT,
        );
        return;
    }
    let current = app.projects_ui.base.default_account.clone();
    let mut sel = accounts
        .iter()
        .position(|a| a.eq_ignore_ascii_case(&current))
        .unwrap_or(0);
    let refs: Vec<&str> = accounts.iter().map(|x| x.as_str()).collect();
    kit::dropdown(ui, rect, &refs, &mut sel, s);
    if let Some(name) = accounts.get(sel)
        && !name.eq_ignore_ascii_case(&current)
    {
        app.projects_ui.base.default_account = name.clone();
        app.projects_ui.form.username = name.clone();
    }
}

pub(super) fn row(ui: &mut egui::Ui, s: f32, h: f32) -> Rect {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), theme::px(h, s)),
        Sense::hover(),
    );
    rect
}

