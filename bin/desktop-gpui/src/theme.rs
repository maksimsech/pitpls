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
        theme.sheet.margin_top = if cfg!(target_os = "macos") {
            crate::TOOLBAR_HEIGHT
        } else {
            px(0.)
        };
        // sRGB equivalents of the light/dark tokens in bin/desktop/src/index.css.
        // Window chrome and navigation share Tauri's sidebar surface.
        let dark = theme.is_dark();
        let c = &mut theme.colors;
        c.background = rgb(if dark { 0x0a0a0a } else { 0xffffff }).into();
        c.foreground = rgb(if dark { 0xfafafa } else { 0x0a0a0a }).into();
        c.border = if dark {
            gpui_kit::white().opacity(0.1)
        } else {
            rgb(0xe5e5e5).into()
        };
        c.input = if dark {
            gpui_kit::white().opacity(0.15)
        } else {
            c.border
        };
        c.muted = rgb(if dark { 0x262626 } else { 0xf5f5f5 }).into();
        c.muted_foreground = rgb(if dark { 0xa3a3a3 } else { 0x737373 }).into();
        c.accent = c.muted;
        c.accent_foreground = rgb(if dark { 0xfafafa } else { 0x171717 }).into();
        c.secondary = c.muted;
        c.secondary_foreground = c.accent_foreground;
        c.secondary_hover = c.secondary.opacity(0.8);
        c.secondary_active = c.secondary_hover;
        c.primary = rgb(if dark { 0xe5e5e5 } else { 0x171717 }).into();
        c.primary_foreground = rgb(if dark { 0x171717 } else { 0xfafafa }).into();
        c.primary_hover = c.primary.opacity(0.9);
        c.primary_active = c.primary.opacity(0.8);
        c.ring = rgb(if dark { 0x737373 } else { 0xa3a3a3 }).into();
        c.popover = rgb(if dark { 0x171717 } else { 0xffffff }).into();
        c.popover_foreground = c.foreground;
        c.group_box = c.popover;
        c.group_box_foreground = c.foreground;
        c.button = if dark {
            c.input.opacity(0.3)
        } else {
            c.background
        };
        c.button_foreground = c.foreground;
        c.button_hover = if dark { c.input.opacity(0.5) } else { c.muted };
        c.button_active = c.button_hover;
        c.button_primary = c.primary;
        c.button_primary_foreground = c.primary_foreground;
        c.button_primary_hover = c.primary_hover;
        c.button_primary_active = c.primary_active;
        c.button_secondary = c.secondary;
        c.button_secondary_foreground = c.secondary_foreground;
        c.button_secondary_hover = c.secondary_hover;
        c.button_secondary_active = c.secondary_active;
        c.danger = rgb(if dark { 0xff6467 } else { 0xe7000b }).into();
        c.danger_foreground = c.danger;
        c.danger_hover = c.danger;
        c.danger_active = c.danger;
        c.button_danger = c.danger.opacity(if dark { 0.2 } else { 0.1 });
        c.button_danger_foreground = c.danger;
        c.button_danger_hover = c.danger.opacity(if dark { 0.3 } else { 0.2 });
        c.button_danger_active = c.button_danger_hover;
        c.sidebar = rgb(if dark { 0x171717 } else { 0xfafafa }).into();
        c.sidebar_foreground = c.foreground;
        c.sidebar_accent = c.accent;
        c.sidebar_accent_foreground = c.accent_foreground;
        c.sidebar_border = c.border;
        c.sidebar_primary = rgb(if dark { 0x1447e6 } else { 0x171717 }).into();
        c.sidebar_primary_foreground = rgb(0xfafafa).into();
        c.title_bar = c.sidebar;
        c.title_bar_border = c.sidebar_border;
        c.status_bar = c.sidebar;
        c.status_bar_border = c.border;
        c.window_border = c.border;
        c.caret = c.foreground;
        c.link = c.primary;
        c.link_hover = c.primary;
        c.link_active = c.primary;
        c.skeleton = c.muted;
        c.progress_bar = c.primary;
        c.scrollbar = gpui_kit::transparent_black();
        c.scrollbar_thumb = c.border;
        c.scrollbar_thumb_hover = c.ring;
        c.list = c.background;
        c.list_even = c.background;
        c.list_head = c.muted;
        c.list_hover = c.muted;
        c.list_active = c.muted;
        c.list_active_border = c.border;
        c.table = c.background;
        c.table_even = c.background;
        c.table_head = rgb(if dark { 0x111111 } else { 0xfafafa }).into();
        c.table_head_foreground = c.muted_foreground;
        c.table_hover = c.muted.opacity(0.5);
        c.table_active = c.muted;
        c.table_active_border = c.border;
        c.table_row_border = c.border;
        c.table_foot = c.muted.opacity(0.5);
        c.table_foot_foreground = c.foreground;
        c.chart_1 = rgb(0xd4d4d4).into();
        c.chart_2 = rgb(0x737373).into();
        c.chart_3 = rgb(0x525252).into();
        c.chart_4 = rgb(0x404040).into();
        c.chart_5 = rgb(0x262626).into();
        c.chart_grid = c.border;
    });
    // Keep the native title bar and window border in sync on startup,
    // when restoring preferences, and when toggling the app theme.
    cx.set_window_appearance(Some(if cx.theme().is_dark() {
        WindowAppearance::Dark
    } else {
        WindowAppearance::Light
    }));
}
