use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    LazyLock, RwLock,
};

use gpui::{Background, Fill, Hsla, Rgba};

static LIGHT: AtomicBool = AtomicBool::new(false);
static COMFORTABLE: AtomicBool = AtomicBool::new(false);
static ACCENT_RGB: AtomicU32 = AtomicU32::new(0xa78bfa);
static UI_SCALE_BITS: AtomicU32 = AtomicU32::new(1_f32.to_bits());
static MONO_FONT: LazyLock<RwLock<String>> = LazyLock::new(|| RwLock::new("JetBrains Mono".into()));

#[derive(Clone, Copy, Debug)]
pub struct DynamicColor(Token);

#[derive(Clone, Copy, Debug)]
enum Token {
    Bg,
    Inset,
    Panel,
    PanelMuted,
    PanelRaised,
    Border,
    BorderStrong,
    BorderDivider,
    GridLine,
    Fg,
    FgSecondary,
    FgTertiary,
    FgMuted,
    FgDisabled,
    Insert,
    InsertSoft,
    UpdateSoft,
    DeleteSoft,
    Accent,
    AccentFg,
    Prod,
    Warn,
    WarnSoft,
}

pub const BG: DynamicColor = DynamicColor(Token::Bg);
pub const INSET: DynamicColor = DynamicColor(Token::Inset);
pub const PANEL: DynamicColor = DynamicColor(Token::Panel);
pub const PANEL_MUTED: DynamicColor = DynamicColor(Token::PanelMuted);
pub const PANEL_RAISED: DynamicColor = DynamicColor(Token::PanelRaised);
pub const BORDER: DynamicColor = DynamicColor(Token::Border);
pub const BORDER_STRONG: DynamicColor = DynamicColor(Token::BorderStrong);
pub const BORDER_DIVIDER: DynamicColor = DynamicColor(Token::BorderDivider);
pub const GRID_LINE: DynamicColor = DynamicColor(Token::GridLine);
pub const FG: DynamicColor = DynamicColor(Token::Fg);
pub const FG_SECONDARY: DynamicColor = DynamicColor(Token::FgSecondary);
pub const FG_TERTIARY: DynamicColor = DynamicColor(Token::FgTertiary);
pub const FG_MUTED: DynamicColor = DynamicColor(Token::FgMuted);
pub const FG_DISABLED: DynamicColor = DynamicColor(Token::FgDisabled);
pub const INSERT: DynamicColor = DynamicColor(Token::Insert);
pub const INSERT_SOFT: DynamicColor = DynamicColor(Token::InsertSoft);
pub const UPDATE_SOFT: DynamicColor = DynamicColor(Token::UpdateSoft);
pub const DELETE_SOFT: DynamicColor = DynamicColor(Token::DeleteSoft);
pub const ACCENT: DynamicColor = DynamicColor(Token::Accent);
pub const ACCENT_FG: DynamicColor = DynamicColor(Token::AccentFg);
pub const PROD: DynamicColor = DynamicColor(Token::Prod);
pub const WARN: DynamicColor = DynamicColor(Token::Warn);
pub const WARN_SOFT: DynamicColor = DynamicColor(Token::WarnSoft);

pub fn set_palette(light: bool, accent: &str) {
    LIGHT.store(light, Ordering::Relaxed);
    ACCENT_RGB.store(
        visible_accent(parse_hex(accent).unwrap_or(0xa78bfa), !light),
        Ordering::Relaxed,
    );
}

pub fn set_density(comfortable: bool) {
    COMFORTABLE.store(comfortable, Ordering::Relaxed);
}

pub fn set_mono_font(font: &str) {
    *MONO_FONT.write().expect("mono font lock poisoned") = font.to_owned();
}

pub fn set_ui_scale(scale: f32) {
    UI_SCALE_BITS.store(
        scale.clamp(10. / 13., 22. / 13.).to_bits(),
        Ordering::Relaxed,
    );
}

pub fn ui_scale() -> f32 {
    f32::from_bits(UI_SCALE_BITS.load(Ordering::Relaxed))
}

pub fn ui_px(value: f32) -> gpui::Pixels {
    gpui::px(value * ui_scale())
}

pub fn mono_font() -> String {
    MONO_FONT.read().expect("mono font lock poisoned").clone()
}

pub fn row_height() -> f32 {
    (if COMFORTABLE.load(Ordering::Relaxed) {
        28.
    } else {
        25.
    }) * ui_scale()
}

pub fn tab_height() -> f32 {
    (if COMFORTABLE.load(Ordering::Relaxed) {
        34.
    } else {
        30.
    }) * ui_scale()
}

pub fn accent(alpha: f32) -> Rgba {
    Rgba {
        a: alpha,
        ..ACCENT.rgba()
    }
}

pub fn accent_soft() -> Rgba {
    accent(if LIGHT.load(Ordering::Relaxed) {
        0.10
    } else {
        0.14
    })
}

pub fn hover_bright(color: Rgba) -> Rgba {
    Rgba {
        r: (color.r * 1.07).min(1.),
        g: (color.g * 1.07).min(1.),
        b: (color.b * 1.07).min(1.),
        ..color
    }
}

pub fn overlay() -> Rgba {
    if LIGHT.load(Ordering::Relaxed) {
        rgba(0xf6f6f6, 0.78)
    } else {
        rgba(0x080808, 0.72)
    }
}

pub fn syntax_keyword(alpha: f32) -> Rgba {
    rgba(
        if LIGHT.load(Ordering::Relaxed) {
            0x7c3aed
        } else {
            0xb794f6
        },
        alpha,
    )
}

impl DynamicColor {
    pub fn rgba(self) -> Rgba {
        let light = LIGHT.load(Ordering::Relaxed);
        match (self.0, light) {
            (Token::Bg, false) => rgb(0x0e0e0e),
            (Token::Bg, true) => rgb(0xf6f6f6),
            (Token::Inset, false) => rgb(0x0a0a0a),
            (Token::Inset, true) => rgb(0xfafafa),
            (Token::Panel, false) => rgb(0x161616),
            (Token::Panel, true) => rgb(0xffffff),
            (Token::PanelMuted, false) => rgb(0x1a1a1a),
            (Token::PanelMuted, true) => rgb(0xf0f0f0),
            (Token::PanelRaised, false) => rgb(0x1f1f1f),
            (Token::PanelRaised, true) => rgb(0xe8e8e8),
            (Token::Border, false) => rgba(0xffffff, 0.04),
            (Token::Border, true) => rgba(0x000000, 0.08),
            (Token::BorderStrong, false) => rgba(0xffffff, 0.13),
            (Token::BorderStrong, true) => rgba(0x000000, 0.16),
            (Token::BorderDivider, false) => rgba(0xffffff, 0.05),
            (Token::BorderDivider, true) => rgba(0x000000, 0.10),
            (Token::GridLine, false) => rgba(0xffffff, 0.08),
            (Token::GridLine, true) => rgba(0x000000, 0.12),
            (Token::Fg, false) => rgb(0xeeeeee),
            (Token::Fg, true) => rgb(0x1a1a1a),
            (Token::FgSecondary, false) => rgb(0xcccccc),
            (Token::FgSecondary, true) => rgb(0x444444),
            (Token::FgTertiary, false) => rgb(0xb5b5b5),
            (Token::FgTertiary, true) => rgb(0x6e6e6e),
            (Token::FgMuted, false) => rgb(0x6a6a6a),
            (Token::FgMuted, true) => rgb(0x9a9a9a),
            (Token::FgDisabled, false) => rgb(0x484848),
            (Token::FgDisabled, true) => rgb(0xc4c4c4),
            (Token::Insert, false) => rgb(0x4ade80),
            (Token::Insert, true) => rgb(0x16a34a),
            (Token::InsertSoft, false) => rgba(0x4ade80, 0.09),
            (Token::InsertSoft, true) => rgba(0x16a34a, 0.10),
            (Token::UpdateSoft, _) => rgba(0xfbbf24, 0.09),
            (Token::DeleteSoft, _) => rgba(0xf87171, 0.09),
            (Token::Accent, _) => rgb(ACCENT_RGB.load(Ordering::Relaxed)),
            (Token::AccentFg, false) => rgb(0x0a0b0e),
            (Token::AccentFg, true) => rgb(0xffffff),
            (Token::Prod, false) => rgb(0xf87171),
            (Token::Prod, true) => rgb(0xdc2626),
            (Token::Warn, false) => rgb(0xfbbf24),
            (Token::Warn, true) => rgb(0xa16207),
            (Token::WarnSoft, false) => rgb(0x3a3018),
            (Token::WarnSoft, true) => rgb(0xfff7df),
        }
    }
}

impl From<DynamicColor> for Hsla {
    fn from(value: DynamicColor) -> Self {
        value.rgba().into()
    }
}

impl From<DynamicColor> for Fill {
    fn from(value: DynamicColor) -> Self {
        value.rgba().into()
    }
}

impl From<DynamicColor> for Background {
    fn from(value: DynamicColor) -> Self {
        value.rgba().into()
    }
}

const fn rgb(hex: u32) -> Rgba {
    rgba(hex, 1.)
}

const fn rgba(hex: u32, a: f32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xff) as f32 / 255.,
        g: ((hex >> 8) & 0xff) as f32 / 255.,
        b: (hex & 0xff) as f32 / 255.,
        a,
    }
}

fn parse_hex(value: &str) -> Option<u32> {
    let value = value.strip_prefix('#').unwrap_or(value);
    (value.len() == 6)
        .then(|| u32::from_str_radix(value, 16).ok())
        .flatten()
}

fn visible_accent(mut value: u32, dark: bool) -> u32 {
    let background = if dark { 0.006 } else { 0.92 };
    let target = if dark { 255. } else { 0. };
    for _ in 0..12 {
        if contrast(luminance(value), background) >= 2.2 {
            break;
        }
        let [r, g, b] = channels(value);
        value = pack(
            r + (target - r) * 0.12,
            g + (target - g) * 0.12,
            b + (target - b) * 0.12,
        );
    }
    value
}

fn channels(value: u32) -> [f32; 3] {
    [
        ((value >> 16) & 0xff) as f32,
        ((value >> 8) & 0xff) as f32,
        (value & 0xff) as f32,
    ]
}

fn pack(r: f32, g: f32, b: f32) -> u32 {
    ((r.round().clamp(0., 255.) as u32) << 16)
        | ((g.round().clamp(0., 255.) as u32) << 8)
        | b.round().clamp(0., 255.) as u32
}

fn luminance(value: u32) -> f32 {
    let [r, g, b] = channels(value).map(|channel| {
        let channel = channel / 255.;
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn contrast(a: f32, b: f32) -> f32 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_matches_classic_dark_and_light_tokens() {
        set_palette(false, "#a78bfa");
        assert_eq!(u32::from(BG.rgba()), 0x0e0e0eff);
        assert_eq!(u32::from(ACCENT.rgba()), 0xa78bfaff);
        assert_eq!(u32::from(FG_TERTIARY.rgba()), 0xb5b5b5ff);
        assert!((BORDER_STRONG.rgba().a - 0.13).abs() < f32::EPSILON);
        assert_eq!(u32::from(accent_soft()), 0xa78bfa23);
        assert_eq!(u32::from(overlay()), 0x080808b7);
        assert_eq!(u32::from(syntax_keyword(0.1)), 0xb794f619);
        assert_eq!(u32::from(hover_bright(rgb(0x808080))), 0x888888ff);
        set_palette(true, "#a78bfa");
        assert_eq!(u32::from(BG.rgba()), 0xf6f6f6ff);
        assert_eq!(u32::from(FG.rgba()), 0x1a1a1aff);
        assert_eq!(u32::from(FG_TERTIARY.rgba()), 0x6e6e6eff);
        assert!((BORDER_STRONG.rgba().a - 0.16).abs() < f32::EPSILON);
        assert_eq!(u32::from(accent_soft()), 0xa78bfa19);
        assert_eq!(u32::from(overlay()), 0xf6f6f6c6);
        assert_eq!(u32::from(syntax_keyword(0.1)), 0x7c3aed19);
    }
}
