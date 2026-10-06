use gpui_kit::{
    component::{Theme, ThemeMode},
    *,
};
use std::sync::Arc;

/// The design D tokens from docs/gpui-redesign-plan.md. `configure_theme`
/// maps them onto the kit theme; read the ones it has no slot for, such as
/// `faint`, `raised` and `ok`, with [`palette`].
pub struct Palette {
    /// Sidebar and window chrome.
    pub chrome: Hsla,
    /// The main panel.
    pub surface: Hsla,
    /// Cards and the opened row band.
    pub raised: Hsla,
    pub line: Hsla,
    /// Popovers and tags.
    pub strong_line: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    /// Extra decimals and hints.
    pub faint: Hsla,
    /// Selected navigation items and rows.
    pub selected: Hsla,
    pub hover: Hsla,
    pub popover: Hsla,
    pub primary: Hsla,
    pub primary_text: Hsla,
    /// Rate warnings only.
    pub warning: Hsla,
    pub danger: Hsla,
    /// The rate status dot and "Copied".
    pub ok: Hsla,
}

impl Global for Palette {}

impl Palette {
    fn dark() -> Self {
        let white = |alpha| gpui_kit::white().opacity(alpha);
        Self {
            chrome: rgb(0x1b1b1b).into(),
            surface: rgb(0x121212).into(),
            raised: rgb(0x181818).into(),
            line: white(0.075),
            strong_line: white(0.12),
            text: rgb(0xececec).into(),
            muted: rgb(0xa3a3a3).into(),
            faint: rgb(0x808080).into(),
            selected: white(0.08),
            hover: white(0.04),
            popover: rgb(0x222222).into(),
            primary: rgb(0xececec).into(),
            primary_text: rgb(0x141414).into(),
            warning: rgb(0xe9b45c).into(),
            danger: rgb(0xff8a80).into(),
            ok: rgb(0x7fcf9a).into(),
        }
    }

    fn light() -> Self {
        let black = |alpha| gpui_kit::black().opacity(alpha);
        Self {
            chrome: rgb(0xf2f2f1).into(),
            surface: rgb(0xffffff).into(),
            raised: rgb(0xf8f8f7).into(),
            line: black(0.08),
            strong_line: black(0.14),
            text: rgb(0x1a1a1a).into(),
            muted: rgb(0x5f5f5f).into(),
            faint: rgb(0x737373).into(),
            selected: black(0.055),
            hover: black(0.03),
            popover: rgb(0xffffff).into(),
            primary: rgb(0x1a1a1a).into(),
            primary_text: rgb(0xffffff).into(),
            warning: rgb(0x9a5b00).into(),
            danger: rgb(0xc42b1c).into(),
            ok: rgb(0x1f7a45).into(),
        }
    }
}

pub fn palette(cx: &App) -> &Palette {
    cx.global::<Palette>()
}

/// Tabular digits for the regular UI font, so numbers line up in columns.
pub fn tabular_digits() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

pub fn configure_theme(cx: &mut App) {
    let p = if Theme::global(cx).is_dark() {
        Palette::dark()
    } else {
        Palette::light()
    };
    Theme::update(cx, |theme| {
        theme.radius = px(10.);
        theme.radius_lg = px(12.);
        theme.font_size = px(14.);
        // On macOS the toolbar is the title bar; keep sheets below it.
        theme.sheet.margin_top = if cfg!(target_os = "macos") {
            crate::TOOLBAR_HEIGHT
        } else {
            px(0.)
        };
        let dark = theme.is_dark();
        let c = &mut theme.colors;
        c.background = p.surface;
        c.foreground = p.text;
        c.border = p.line;
        c.input = p.strong_line;
        c.muted = p.selected;
        c.muted_foreground = p.muted;
        c.accent = p.selected;
        c.accent_foreground = p.text;
        c.secondary = p.selected;
        c.secondary_foreground = p.text;
        c.secondary_hover = p.hover;
        c.secondary_active = p.selected;
        c.primary = p.primary;
        c.primary_foreground = p.primary_text;
        c.primary_hover = c.primary.opacity(0.9);
        c.primary_active = c.primary.opacity(0.8);
        c.ring = p.faint;
        c.popover = p.popover;
        c.popover_foreground = p.text;
        c.group_box = p.raised;
        c.group_box_foreground = p.text;
        c.button = if dark {
            gpui_kit::white().opacity(0.06)
        } else {
            p.surface
        };
        c.button_foreground = p.text;
        c.button_hover = if dark {
            gpui_kit::white().opacity(0.1)
        } else {
            gpui_kit::black().opacity(0.05)
        };
        c.button_active = c.button_hover;
        c.button_primary = c.primary;
        c.button_primary_foreground = c.primary_foreground;
        c.button_primary_hover = c.primary_hover;
        c.button_primary_active = c.primary_active;
        c.button_secondary = c.secondary;
        c.button_secondary_foreground = c.secondary_foreground;
        c.button_secondary_hover = c.secondary_hover;
        c.button_secondary_active = c.secondary_active;
        c.danger = p.danger;
        c.danger_foreground = c.danger;
        c.danger_hover = c.danger;
        c.danger_active = c.danger;
        c.button_danger = c.danger.opacity(if dark { 0.2 } else { 0.1 });
        c.button_danger_foreground = c.danger;
        c.button_danger_hover = c.danger.opacity(if dark { 0.3 } else { 0.2 });
        c.button_danger_active = c.button_danger_hover;
        c.warning = p.warning;
        c.warning_foreground = p.warning;
        c.warning_hover = p.warning;
        c.warning_active = p.warning;
        c.success = p.ok;
        c.success_foreground = p.ok;
        c.success_hover = p.ok;
        c.success_active = p.ok;
        c.sidebar = p.chrome;
        c.sidebar_foreground = p.text;
        c.sidebar_accent = p.selected;
        c.sidebar_accent_foreground = p.text;
        c.sidebar_border = p.line;
        c.sidebar_primary = p.primary;
        c.sidebar_primary_foreground = p.primary_text;
        c.title_bar = p.chrome;
        c.title_bar_border = p.line;
        c.status_bar = p.chrome;
        c.status_bar_border = p.line;
        c.window_border = p.line;
        c.caret = p.text;
        c.link = p.text;
        c.link_hover = p.text;
        c.link_active = p.text;
        c.skeleton = p.selected;
        c.progress_bar = p.primary;
        c.scrollbar = gpui_kit::transparent_black();
        c.scrollbar_thumb = p.strong_line;
        c.scrollbar_thumb_hover = p.faint;
        c.list = p.surface;
        c.list_even = p.surface;
        c.list_head = p.raised;
        c.list_hover = p.hover;
        c.list_active = p.selected;
        c.list_active_border = p.line;
        c.table = p.surface;
        c.table_even = p.surface;
        c.table_head = p.raised;
        c.table_head_foreground = p.muted;
        c.table_hover = p.hover;
        c.table_active = p.selected;
        c.table_active_border = p.line;
        c.table_row_border = p.line;
        c.table_foot = p.raised;
        c.table_foot_foreground = p.text;
        c.chart_1 = rgb(0xd4d4d4).into();
        c.chart_2 = rgb(0x737373).into();
        c.chart_3 = rgb(0x525252).into();
        c.chart_4 = rgb(0x404040).into();
        c.chart_5 = rgb(0x262626).into();
        c.chart_grid = p.line;
    });
    cx.set_global(p);
}

pub fn apply_theme(dark: Option<bool>, window: &mut Window, cx: &mut App) {
    cx.set_window_appearance(dark.map(|dark| {
        if dark {
            WindowAppearance::Dark
        } else {
            WindowAppearance::Light
        }
    }));
    match dark {
        Some(dark) => Theme::change(
            if dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            },
            Some(window),
            cx,
        ),
        None => Theme::sync_system_appearance(Some(window), cx),
    }
    configure_theme(cx);
}
