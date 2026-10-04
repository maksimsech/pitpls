use gpui_kit::{
    component::{ActiveTheme, Theme},
    *,
};

pub fn configure_theme(cx: &mut App) {
    Theme::update(cx, |theme| {
        theme.radius = px(10.);
        theme.radius_lg = px(10.);
        theme.font_size = px(14.);
        // On macOS the toolbar shares the native title bar's content area.
        // Keep sheets below its window controls and app actions.
        theme.sheet.margin_top = px(if cfg!(target_os = "macos") { 64. } else { 0. });
        // Neutral desktop surfaces: the sidebar and header sit one shade above
        // the content canvas, with quiet borders in either appearance.
        let dark = theme.is_dark();
        let c = &mut theme.colors;
        c.background = rgb(if dark { 0x141414 } else { 0xffffff }).into();
        c.foreground = rgb(if dark { 0xfafafa } else { 0x0a0a0a }).into();
        c.border = if dark {
            rgba(0xffffff1a).into()
        } else {
            rgb(0xe5e5e5).into()
        };
        c.input = if dark {
            rgba(0xffffff26).into()
        } else {
            c.border
        };
        c.muted = rgb(if dark { 0x262626 } else { 0xf5f5f5 }).into();
        c.muted_foreground = rgb(if dark { 0xa3a3a3 } else { 0x737373 }).into();
        c.accent = c.muted;
        c.accent_foreground = c.foreground;
        c.secondary = c.muted;
        c.secondary_foreground = c.foreground;
        c.secondary_hover = rgb(if dark { 0x303030 } else { 0xe5e5e5 }).into();
        c.secondary_active = c.secondary_hover;
        c.primary = rgb(if dark { 0xe5e5e5 } else { 0x171717 }).into();
        c.primary_foreground = rgb(if dark { 0x171717 } else { 0xfafafa }).into();
        c.primary_hover = c.primary.opacity(0.9);
        c.primary_active = c.primary.opacity(0.8);
        c.ring = rgb(if dark { 0x737373 } else { 0xa3a3a3 }).into();
        c.popover = rgb(if dark { 0x171717 } else { 0xffffff }).into();
        c.popover_foreground = c.foreground;
        c.group_box = c.background;
        c.group_box_foreground = c.foreground;
        c.button = if dark {
            rgba(0xffffff0d).into()
        } else {
            c.background
        };
        c.button_foreground = c.foreground;
        c.button_hover = c.muted;
        c.button_active = c.muted;
        c.button_primary = c.primary;
        c.button_primary_foreground = c.primary_foreground;
        c.button_primary_hover = c.primary_hover;
        c.button_primary_active = c.primary_active;
        c.button_secondary = c.secondary;
        c.button_secondary_foreground = c.foreground;
        c.button_secondary_hover = c.muted;
        c.button_secondary_active = c.muted;
        c.danger = rgb(if dark { 0xff6467 } else { 0xe7000b }).into();
        c.danger_foreground = c.danger;
        c.danger_hover = c.danger;
        c.danger_active = c.danger;
        c.button_danger = c.danger.opacity(if dark { 0.2 } else { 0.1 });
        c.button_danger_foreground = c.danger;
        c.button_danger_hover = c.danger.opacity(0.3);
        c.button_danger_active = c.danger.opacity(0.4);
        c.sidebar = rgb(if dark { 0x202020 } else { 0xf5f5f5 }).into();
        c.sidebar_foreground = c.foreground;
        c.sidebar_accent = rgb(if dark { 0x303030 } else { 0xe5e5e5 }).into();
        c.sidebar_accent_foreground = c.foreground;
        c.sidebar_border = c.border;
        c.list = c.background;
        c.list_head = c.muted;
        c.list_hover = c.muted;
        c.list_active = c.muted;
        c.list_active_border = c.border;
        c.table = c.background;
        c.table_head = c.muted;
        c.table_head_foreground = c.foreground;
        c.table_hover = c.muted;
        c.table_active = c.muted;
        c.table_row_border = c.border;
    });
    // Keep the native title bar and window border in sync on startup,
    // when restoring preferences, and when toggling the app theme.
    cx.set_window_appearance(Some(if cx.theme().is_dark() {
        WindowAppearance::Dark
    } else {
        WindowAppearance::Light
    }));
}
