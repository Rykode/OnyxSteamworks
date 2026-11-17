use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Painter, Rect,
    Stroke, StrokeKind, TextStyle,
};

/// Every colour the UI paints with, for one appearance. The UI reads the active set
/// through the functions below (`theme::ink()` etc.), so switching theme is one store.
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub sidebar: Color32,
    pub surface: Color32,
    pub surface_2: Color32,
    pub field: Color32,
    pub field_hi: Color32,
    pub line: Color32,
    /// Resting outline of text boxes and dropdowns: >= 3:1 against SURFACE and SURFACE_2
    /// (WCAG 1.4.11), because the field fill alone is nearly invisible on a card.
    pub field_edge: Color32,
    pub line_soft: Color32,
    pub well: Color32,
    pub ink: Color32,
    pub ink_dim: Color32,
    pub ink_muted: Color32,
    /// Lowest text tier. Must stay >= 4.5:1 on BG, SURFACE, SURFACE_2 and FIELD (WCAG 1.4.3).
    pub ink_faint: Color32,
    pub accent: Color32,
    /// Also used as link / emphasis text, so >= 4.5:1 like the ink tiers.
    pub accent_hi: Color32,
    pub primary_ink: Color32,
    /// Primary button fills carry PRIMARY_INK text, so both keep >= 4.5:1 against it.
    /// ACCENT stays brighter for non-text marks (toggles, progress, focus) that need 3:1.
    pub primary: Color32,
    pub primary_hi: Color32,
    pub focus: Color32,
    pub segment_idle: Color32,
    pub track: Color32,
    /// SUCCESS, WARNING and DANGER are used as text: >= 4.5:1 on BG, SURFACE, SURFACE_2, FIELD.
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    /// Text on DANGER-tinted fills (danger buttons), where DANGER itself can drop below 4.5:1.
    pub danger_ink: Color32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub const DARK: Palette = Palette {
    bg: rgb(0x17, 0x18, 0x1C),
    sidebar: rgb(0x17, 0x18, 0x1C),
    surface: rgb(0x21, 0x22, 0x28),
    surface_2: rgb(0x29, 0x2B, 0x32),
    field: rgb(0x27, 0x28, 0x2E),
    field_hi: rgb(0x30, 0x32, 0x3A),
    line: rgb(0x3A, 0x3D, 0x46),
    field_edge: rgb(0x6F, 0x76, 0x85),
    line_soft: rgb(0x2C, 0x2E, 0x36),
    well: rgb(0x12, 0x14, 0x16),
    ink: rgb(0xF4, 0xF5, 0xF7),
    ink_dim: rgb(0xC9, 0xCF, 0xDA),
    ink_muted: rgb(0x9A, 0xA3, 0xB2),
    ink_faint: rgb(0x8A, 0x93, 0xA3),
    accent: rgb(0x4F, 0x7C, 0xFF),
    accent_hi: rgb(0x7A, 0xA0, 0xFF),
    primary_ink: rgb(0xFF, 0xFF, 0xFF),
    primary: rgb(0x3F, 0x66, 0xE0),
    primary_hi: rgb(0x45, 0x69, 0xE0),
    focus: rgb(0x4F, 0x7C, 0xFF),
    segment_idle: rgb(0x34, 0x36, 0x3E),
    track: rgb(0x2C, 0x2E, 0x36),
    success: rgb(0x2F, 0xBF, 0x71),
    warning: rgb(0xF0, 0xB1, 0x32),
    danger: rgb(0xF5, 0x68, 0x6D),
    danger_ink: rgb(0xFF, 0x82, 0x85),
};

/// Light appearance. Contrast was checked against the same rules as DARK
/// (text tiers >= 4.5:1 on bg/surface/surface_2/field, field_edge >= 3:1).
pub const LIGHT: Palette = Palette {
    bg: rgb(0xEE, 0xF0, 0xF4),
    sidebar: rgb(0xEE, 0xF0, 0xF4),
    surface: rgb(0xFF, 0xFF, 0xFF),
    surface_2: rgb(0xF6, 0xF7, 0xF9),
    field: rgb(0xF1, 0xF3, 0xF6),
    field_hi: rgb(0xE6, 0xE9, 0xEF),
    line: rgb(0xD5, 0xD9, 0xE1),
    field_edge: rgb(0x7A, 0x83, 0x93),
    line_soft: rgb(0xE3, 0xE6, 0xEC),
    well: rgb(0xF4, 0xF5, 0xF8),
    ink: rgb(0x16, 0x18, 0x1D),
    ink_dim: rgb(0x34, 0x3A, 0x46),
    ink_muted: rgb(0x53, 0x5B, 0x69),
    ink_faint: rgb(0x5F, 0x67, 0x74),
    accent: rgb(0x3F, 0x66, 0xE0),
    accent_hi: rgb(0x2F, 0x55, 0xCC),
    primary_ink: rgb(0xFF, 0xFF, 0xFF),
    primary: rgb(0x3F, 0x66, 0xE0),
    primary_hi: rgb(0x34, 0x59, 0xCF),
    focus: rgb(0x3F, 0x66, 0xE0),
    segment_idle: rgb(0xD9, 0xDD, 0xE4),
    track: rgb(0xE3, 0xE6, 0xEC),
    success: rgb(0x16, 0x6E, 0x3D),
    warning: rgb(0x8F, 0x5B, 0x00),
    danger: rgb(0xC4, 0x2E, 0x34),
    danger_ink: rgb(0xB3, 0x26, 0x2B),
};

static LIGHT_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static MODE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// The palette currently in use.
pub fn palette() -> &'static Palette {
    if is_light() { &LIGHT } else { &DARK }
}

pub fn is_light() -> bool {
    LIGHT_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn bg() -> Color32 {
    palette().bg
}

pub fn sidebar() -> Color32 {
    palette().sidebar
}

pub fn surface() -> Color32 {
    palette().surface
}

pub fn surface_2() -> Color32 {
    palette().surface_2
}

pub fn field() -> Color32 {
    palette().field
}

pub fn field_hi() -> Color32 {
    palette().field_hi
}

pub fn line() -> Color32 {
    palette().line
}

pub fn field_edge() -> Color32 {
    palette().field_edge
}

pub fn line_soft() -> Color32 {
    palette().line_soft
}

pub fn well() -> Color32 {
    palette().well
}

pub fn ink() -> Color32 {
    palette().ink
}

pub fn ink_dim() -> Color32 {
    palette().ink_dim
}

pub fn ink_muted() -> Color32 {
    palette().ink_muted
}

pub fn ink_faint() -> Color32 {
    palette().ink_faint
}

pub fn accent() -> Color32 {
    palette().accent
}

pub fn accent_hi() -> Color32 {
    palette().accent_hi
}

pub fn primary_ink() -> Color32 {
    palette().primary_ink
}

pub fn primary() -> Color32 {
    palette().primary
}

pub fn primary_hi() -> Color32 {
    palette().primary_hi
}

pub fn focus() -> Color32 {
    palette().focus
}

pub fn segment_idle() -> Color32 {
    palette().segment_idle
}

pub fn track() -> Color32 {
    palette().track
}

pub fn success() -> Color32 {
    palette().success
}

pub fn warning() -> Color32 {
    palette().warning
}

pub fn danger() -> Color32 {
    palette().danger
}

pub fn danger_ink() -> Color32 {
    palette().danger_ink
}

pub const RADIUS_APP: f32 = 12.0;
pub const RADIUS_PANEL: f32 = 18.0;
pub const RADIUS_PANEL_MIN: f32 = 14.0;
pub const RADIUS_CARD: f32 = 14.0;
pub const RADIUS_FIELD: f32 = 10.0;
pub const RADIUS_POPUP: f32 = 14.0;

pub const REF_W: f32 = 1320.0;
pub const REF_H: f32 = 840.0;
const SCALE_MIN: f32 = 0.82;
const SCALE_MAX: f32 = 1.35;

static UI_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1000);

pub fn set_ui_scale(factor: f32) {
    let factor = factor.clamp(0.8, 1.3);
    UI_SCALE.store(
        (factor * 1000.0) as u32,
        std::sync::atomic::Ordering::Relaxed,
    );
}

pub fn ui_scale() -> f32 {
    UI_SCALE.load(std::sync::atomic::Ordering::Relaxed) as f32 / 1000.0
}

pub fn scale(available: egui::Vec2) -> f32 {
    ((available.x / REF_W).min(available.y / REF_H) * ui_scale()).clamp(SCALE_MIN, SCALE_MAX)
}

pub fn px(value: f32, scale: f32) -> f32 {
    value * scale
}

/// Smallest rendered text size, in logical px. Layout keeps shrinking with small
/// windows (down to SCALE_MIN), but text stops here so it stays readable.
const MIN_TEXT_PX: f32 = 11.0;

/// Text size for a design size at the current scale, floored at MIN_TEXT_PX.
pub fn text_px(size: f32, scale: f32) -> f32 {
    (size * scale).max(MIN_TEXT_PX)
}

pub fn corner(radius_px: f32) -> CornerRadius {
    CornerRadius::same(radius_px.round().clamp(0.0, 255.0) as u8)
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn focus_ring(painter: &Painter, rect: Rect, radius_px: f32, color: Color32) {
    painter.rect_stroke(
        rect,
        corner(radius_px),
        Stroke::new(1.5, color),
        StrokeKind::Inside,
    );
}

pub fn shadow(painter: &Painter, rect: Rect, radius_px: f32, blur: u8, alpha: u8) {
    let shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur,
        spread: 0,
        color: Color32::from_black_alpha(alpha),
    };
    painter.add(shadow.as_shape(rect, corner(radius_px)));
}

const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
const INTER_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Inter-Medium.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-Bold.ttf");
const INTER_BLACK_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Inter-BlackItalic.ttf");
const JETBRAINS_MONO: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
const PHOSPHOR: &[u8] = include_bytes!("../../assets/fonts/Phosphor-Regular.ttf");
const PHOSPHOR_FILL: &[u8] = include_bytes!("../../assets/fonts/Phosphor-Fill.ttf");

pub fn family_medium() -> FontFamily {
    FontFamily::Name("Medium".into())
}

pub fn family_semibold() -> FontFamily {
    FontFamily::Name("SemiBold".into())
}

pub fn family_bold() -> FontFamily {
    FontFamily::Name("Bold".into())
}

pub fn family_brand() -> FontFamily {
    FontFamily::Name("Brand".into())
}

pub fn family_display() -> FontFamily {
    FontFamily::Name("Display".into())
}

pub fn family_label() -> FontFamily {
    FontFamily::Name("Label".into())
}

pub fn family_icon() -> FontFamily {
    FontFamily::Name("Phosphor".into())
}

pub fn family_icon_fill() -> FontFamily {
    FontFamily::Name("PhosphorFill".into())
}

pub fn icon_font(size: f32, filled: bool) -> FontId {
    FontId::new(
        size,
        if filled {
            family_icon_fill()
        } else {
            family_icon()
        },
    )
}

pub mod icon {
    pub const STACK: char = '\u{e466}';
    pub const HISTORY: char = '\u{e1a0}';
    pub const CLOCK: char = '\u{e19a}';
    pub const FLOW: char = '\u{e6ec}';
    pub const GEAR: char = '\u{e270}';
    pub const MAGNIFYING_GLASS: char = '\u{e30c}';
    pub const SPARKLE: char = '\u{e6a2}';
    pub const DOWNLOAD: char = '\u{e20c}';
    pub const UPLOAD: char = '\u{e4c0}';
    pub const PLUS: char = '\u{e3d4}';
    pub const COPY: char = '\u{e1ca}';
    pub const TRASH: char = '\u{e4a6}';
    pub const CARET_DOWN: char = '\u{e136}';
    pub const CHECK: char = '\u{e182}';
    pub const FOLDER: char = '\u{e24a}';
    pub const PLAY: char = '\u{e3d0}';
    pub const PACKAGE: char = '\u{e390}';
    pub const ROCKET: char = '\u{e3fe}';
    pub const WARNING: char = '\u{e4e0}';
    pub const X: char = '\u{e4f6}';
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let add = |name: &str, bytes: &'static [u8], defs: &mut FontDefinitions| {
        defs.font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add("Inter", INTER_REGULAR, &mut fonts);
    add("Inter-Medium", INTER_MEDIUM, &mut fonts);
    add("Inter-SemiBold", INTER_SEMIBOLD, &mut fonts);
    add("Inter-Bold", INTER_BOLD, &mut fonts);
    add("Inter-BlackItalic", INTER_BLACK_ITALIC, &mut fonts);
    add("JetBrainsMono", JETBRAINS_MONO, &mut fonts);
    add("Phosphor", PHOSPHOR, &mut fonts);
    add("Phosphor-Fill", PHOSPHOR_FILL, &mut fonts);

    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.insert(0, "Inter".to_owned());
    prop.push("Phosphor".to_owned());

    let mono = fonts.families.entry(FontFamily::Monospace).or_default();
    mono.insert(0, "JetBrainsMono".to_owned());
    mono.push("Phosphor".to_owned());

    let named = [
        (family_medium(), vec!["Inter-Medium", "Inter", "Phosphor"]),
        (
            family_semibold(),
            vec!["Inter-SemiBold", "Inter", "Phosphor"],
        ),
        (family_bold(), vec!["Inter-Bold", "Inter", "Phosphor"]),
        (family_brand(), vec!["Inter-BlackItalic", "Inter"]),
        (family_display(), vec!["Inter-SemiBold", "Inter"]),
        (family_label(), vec!["Inter-Medium", "Inter", "Phosphor"]),
        (family_icon(), vec!["Phosphor"]),
        (family_icon_fill(), vec!["Phosphor-Fill"]),
    ];
    for (family, stack) in named {
        fonts
            .families
            .insert(family, stack.into_iter().map(str::to_owned).collect());
    }
    ctx.set_fonts(fonts);
}

fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    [
        (TextStyle::Heading, FontId::new(16.0, family_semibold())),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        ),
        (TextStyle::Button, FontId::new(13.0, family_medium())),
    ]
    .into()
}

fn build_visuals() -> egui::Visuals {
    let light = is_light();
    let mut v = if light { egui::Visuals::light() } else { egui::Visuals::dark() };
    v.dark_mode = !light;
    v.override_text_color = Some(ink());

    v.panel_fill = bg();
    v.window_fill = surface();
    v.window_stroke = Stroke::new(1.0, line());
    v.extreme_bg_color = field();
    v.faint_bg_color = surface_2();
    v.code_bg_color = field();
    v.hyperlink_color = ink();

    v.selection.bg_fill = with_alpha(accent(), 90);
    v.selection.stroke = Stroke::new(1.0, accent_hi());
    v.text_cursor.stroke = Stroke::new(1.5, accent_hi());
    v.popup_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if light { 60 } else { 140 }),
    };
    v.window_shadow = v.popup_shadow;
    v.window_corner_radius = corner(RADIUS_POPUP);
    v.menu_corner_radius = corner(RADIUS_POPUP);

    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.bg_stroke = Stroke::NONE;
        w.fg_stroke = Stroke::new(1.0, ink());
        w.corner_radius = corner(RADIUS_FIELD);
        w.expansion = 0.0;
    }
    v.widgets.noninteractive.bg_fill = surface();
    v.widgets.noninteractive.weak_bg_fill = surface();
    v.widgets.inactive.bg_fill = field();
    v.widgets.inactive.weak_bg_fill = field();
    v.widgets.hovered.bg_fill = field_hi();
    v.widgets.hovered.weak_bg_fill = field_hi();
    v.widgets.active.bg_fill = field_hi();
    v.widgets.active.weak_bg_fill = field_hi();
    v
}

pub fn apply_theme(ctx: &egui::Context) {
    install_fonts(ctx);
    let styles = text_styles();
    let visuals = build_visuals();
    ctx.all_styles_mut(|style| {
        style.text_styles = styles.clone();
        style.visuals = visuals.clone();
    });
}

fn mode_code(mode: crate::config::ThemeMode) -> u8 {
    match mode {
        crate::config::ThemeMode::Dark => 0,
        crate::config::ThemeMode::Light => 1,
        crate::config::ThemeMode::System => 2,
    }
}

/// Choose dark, light or follow-the-system, and apply it now.
pub fn set_mode(ctx: &egui::Context, mode: crate::config::ThemeMode) {
    MODE.store(mode_code(mode), std::sync::atomic::Ordering::Relaxed);
    sync(ctx);
}

/// Re-resolve "System" against the OS setting (cheap; called every frame) and
/// re-apply the egui visuals when the effective appearance changed.
pub fn sync(ctx: &egui::Context) {
    let light = match MODE.load(std::sync::atomic::Ordering::Relaxed) {
        1 => true,
        2 => ctx.system_theme() == Some(egui::Theme::Light),
        _ => false,
    };
    if LIGHT_ACTIVE.swap(light, std::sync::atomic::Ordering::Relaxed) != light {
        let visuals = build_visuals();
        ctx.all_styles_mut(|style| style.visuals = visuals.clone());
        ctx.request_repaint();
    }
}
