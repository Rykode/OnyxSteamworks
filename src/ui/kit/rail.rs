use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};

use crate::app::Page;
use crate::ui::{anim, nav, theme};

pub enum RailAction {
    Navigate(Page),
    Search,
    Avatar,
}

pub struct RailState<'a> {
    pub active: Page,
    pub initials: &'a str,
    pub account: &'a str,
    pub cached: bool,
    pub brand_y: f32,
}

const AVATAR: Color32 = Color32::from_rgb(0x3F, 0x63, 0xC4);

pub fn account_row(rect: Rect, s: f32) -> Rect {
    let px = |v: f32| theme::px(v, s);
    Rect::from_min_max(
        Pos2::new(rect.left(), rect.bottom() - px(50.0)),
        rect.max,
    )
}

fn palette_row(rect: Rect, s: f32) -> Rect {
    let px = |v: f32| theme::px(v, s);
    let acct = account_row(rect, s);
    Rect::from_min_max(
        Pos2::new(rect.left(), acct.top() - px(11.0) - px(41.0)),
        Pos2::new(rect.right(), acct.top() - px(11.0)),
    )
}

fn row_fill(ui: &egui::Ui, row: Rect, active: f32, hover: f32, s: f32) {
    if active < 0.01 && hover < 0.01 {
        return;
    }
    let fill = theme::lerp_color(
        theme::with_alpha(theme::SURFACE, (hover * 160.0) as u8),
        theme::SURFACE,
        active,
    );
    let radius = theme::corner(theme::px(theme::RADIUS_FIELD, s));
    ui.painter().rect_filled(row, radius, fill);
    if active > 0.01 {
        ui.painter().rect_stroke(
            row,
            radius,
            Stroke::new(
                1.0,
                theme::with_alpha(theme::LINE_SOFT, (active * 255.0) as u8),
            ),
            StrokeKind::Inside,
        );
    }
}

pub fn brand(ui: &egui::Ui, left: f32, cy: f32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let painter = ui.painter();
    let c = Pos2::new(left + px(25.0), cy);
    let rx = px(12.5);
    let ry = px(15.0);
    let top = c - Vec2::new(0.0, ry);
    let right = c + Vec2::new(rx, 0.0);
    let bottom = c + Vec2::new(0.0, ry);
    let left_pt = c - Vec2::new(rx, 0.0);
    painter.add(egui::Shape::convex_polygon(
        vec![top, right, bottom, left_pt],
        Color32::from_rgb(0x1E, 0x20, 0x24),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![top, right, c, left_pt],
        Color32::from_rgb(0x34, 0x36, 0x3D),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![top, c, left_pt],
        Color32::from_rgb(0x42, 0x44, 0x4B),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::closed_line(
        vec![top, right, bottom, left_pt],
        Stroke::new(1.0, Color32::from_rgb(0x4A, 0x4C, 0x53)),
    ));
    painter.text(
        Pos2::new(left + px(50.0), cy),
        Align2::LEFT_CENTER,
        "Onyx",
        FontId::new(theme::text_px(27.0, s), theme::family_brand()),
        theme::INK,
    );
}

pub fn rail(ui: &mut egui::Ui, rect: Rect, st: RailState<'_>, s: f32) -> Option<RailAction> {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let mut action = None;

    brand(ui, rect.left(), st.brand_y, s);

    let row_h = px(42.0);
    let mut y = st.brand_y + px(50.0);
    for item in nav::ITEMS.iter() {
        let row = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), row_h));
        let id = ui.make_persistent_id(("rail_slot", item.page));
        let resp = ui.interact(row, id, Sense::click());
        let is_active = item.page == st.active;
        super::accessible(&resp, row, egui::Role::Tab, item.label, Some(is_active), px(theme::RADIUS_FIELD));
        if resp.clicked() {
            action = Some(RailAction::Navigate(item.page));
        }
        let t = anim::anim_bool(
            &ctx,
            id.with("active"),
            is_active,
            0.16,
            anim::ease_out_cubic,
        );
        let hover = anim::anim_bool(
            &ctx,
            id.with("hover"),
            resp.hovered(),
            0.1,
            anim::ease_out_cubic,
        );
        row_fill(ui, row, t, hover, s);
        ui.painter().text(
            Pos2::new(row.left() + px(21.5), row.center().y),
            Align2::CENTER_CENTER,
            item.glyph.to_string(),
            theme::icon_font(px(18.0), false),
            theme::lerp_color(
                theme::lerp_color(theme::INK_MUTED, theme::INK_DIM, hover),
                theme::ACCENT_HI,
                t,
            ),
        );
        ui.painter().text(
            Pos2::new(row.left() + px(44.0), row.center().y),
            Align2::LEFT_CENTER,
            item.label,
            FontId::new(theme::text_px(15.5, s), theme::family_medium()),
            theme::lerp_color(
                theme::lerp_color(theme::INK_DIM, theme::INK, hover),
                theme::INK,
                t,
            ),
        );
        ui.painter().text(
            Pos2::new(row.right() - px(12.0), row.center().y),
            Align2::RIGHT_CENTER,
            shortcut_label(item.shortcut),
            FontId::new(theme::text_px(10.5, s), FontFamily::Monospace),
            theme::INK_FAINT,
        );
        y += px(46.0);
    }

    let search = palette_row(rect, s);
    let sid = ui.make_persistent_id("search_pill");
    let sresp = ui.interact(search, sid, Sense::click());
    super::accessible(&sresp, search, egui::Role::Button, "Command palette (Ctrl+K)", None, px(theme::RADIUS_FIELD));
    let shover = anim::anim_bool(&ctx, sid, sresp.hovered(), 0.12, anim::ease_out_cubic);
    kit_box(
        ui,
        search,
        theme::lerp_color(theme::FIELD, theme::FIELD_HI, shover),
        s,
    );
    ui.painter().text(
        Pos2::new(search.left() + px(18.5), search.center().y),
        Align2::CENTER_CENTER,
        theme::icon::MAGNIFYING_GLASS.to_string(),
        theme::icon_font(px(13.0), false),
        theme::INK_FAINT,
    );
    ui.painter().text(
        Pos2::new(search.left() + px(35.0), search.center().y),
        Align2::LEFT_CENTER,
        "Command palette",
        FontId::new(theme::text_px(13.0, s), theme::family_medium()),
        theme::lerp_color(theme::INK_FAINT, theme::INK_MUTED, shover),
    );
    kbd(
        ui,
        Pos2::new(search.right() - px(10.0), search.center().y),
        "Ctrl K",
        s,
    );
    if sresp.clicked() {
        action = Some(RailAction::Search);
    }

    let acct = account_row(rect, s);
    let aid = ui.make_persistent_id("rail_avatar");
    let aresp = ui.interact(acct, aid, Sense::click());
    super::accessible(&aresp, acct, egui::Role::Button, "Switch account", None, px(theme::RADIUS_FIELD));
    let ahover = anim::anim_bool(&ctx, aid, aresp.hovered(), 0.12, anim::ease_out_cubic);
    ui.painter().rect_filled(
        acct,
        theme::corner(px(theme::RADIUS_FIELD)),
        theme::lerp_color(theme::SURFACE, theme::SURFACE_2, ahover),
    );
    let avatar_r = px(15.5);
    let avatar_c = Pos2::new(acct.left() + px(27.0), acct.center().y);
    ui.painter().circle_filled(avatar_c, avatar_r, AVATAR);
    ui.painter().text(
        avatar_c,
        Align2::CENTER_CENTER,
        st.initials,
        FontId::new(theme::text_px(12.5, s), theme::family_bold()),
        theme::INK,
    );
    let text_x = acct.left() + px(53.0);
    let signed_in = !st.account.is_empty();
    ui.painter().text(
        Pos2::new(text_x, acct.center().y - px(7.5)),
        Align2::LEFT_CENTER,
        if signed_in { st.account } else { "No account" },
        FontId::new(theme::text_px(13.5, s), theme::family_bold()),
        if signed_in {
            theme::INK
        } else {
            theme::INK_MUTED
        },
    );
    let (status, status_color) = match (signed_in, st.cached) {
        (true, true) => ("Cached login", theme::SUCCESS),
        (true, false) => ("Login required", theme::WARNING),
        (false, _) => ("Add one in Settings", theme::INK_FAINT),
    };
    let dot = Pos2::new(text_x + px(3.0), acct.center().y + px(8.0));
    ui.painter().circle_filled(dot, px(3.0), status_color);
    ui.painter().text(
        Pos2::new(text_x + px(11.0), dot.y),
        Align2::LEFT_CENTER,
        status,
        FontId::new(theme::text_px(11.5, s), theme::family_medium()),
        status_color,
    );
    if aresp.clicked() {
        action = Some(RailAction::Avatar);
    }

    action
}

fn kit_box(ui: &egui::Ui, r: Rect, fill: Color32, s: f32) {
    let radius = theme::corner(theme::px(theme::RADIUS_FIELD, s));
    ui.painter().rect_filled(r, radius, fill);
    ui.painter().rect_stroke(
        r,
        radius,
        Stroke::new(1.0, theme::LINE_SOFT),
        StrokeKind::Inside,
    );
}

fn shortcut_label(key: egui::Key) -> String {
    format!("Ctrl {}", key.symbol_or_name())
}

pub fn kbd(ui: &egui::Ui, right: Pos2, text: &str, s: f32) -> Rect {
    let px = |v: f32| theme::px(v, s);
    let font = FontId::new(theme::text_px(10.5, s), FontFamily::Monospace);
    let w = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), theme::INK_FAINT)
        .size()
        .x;
    let r = Rect::from_min_max(
        Pos2::new(right.x - w - px(12.0), right.y - px(9.0)),
        Pos2::new(right.x, right.y + px(9.0)),
    );
    ui.painter().rect_stroke(
        r,
        theme::corner(px(4.0)),
        Stroke::new(1.0, theme::LINE),
        StrokeKind::Inside,
    );
    ui.painter().text(
        r.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        theme::INK_FAINT,
    );
    r
}
