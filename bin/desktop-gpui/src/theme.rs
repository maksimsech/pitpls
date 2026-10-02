use gpui_kit::{component::Theme, *};

pub fn configure_theme(cx: &mut App) {
    Theme::update(cx, |theme| {
        theme.radius = px(10.);
        theme.radius_lg = px(10.);
        theme.font_size = px(14.);
        let dark = theme.is_dark();
        theme.colors.background = rgb(if dark { 0x171717 } else { 0xffffff }).into();
        theme.colors.foreground = rgb(if dark { 0xfafafa } else { 0x171717 }).into();
        theme.colors.border = rgb(if dark { 0x363636 } else { 0xe5e5e5 }).into();
        theme.colors.muted = rgb(if dark { 0x262626 } else { 0xf5f5f5 }).into();
        theme.colors.muted_foreground = rgb(if dark { 0xa3a3a3 } else { 0x737373 }).into();
        theme.colors.primary = theme.colors.foreground;
        theme.colors.primary_foreground = theme.colors.background;
        theme.colors.ring = rgb(0x737373).into();
    });
}
