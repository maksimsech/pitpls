use crate::theme::palette;
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Disableable, Icon, InteractiveElementExt, Placement, StyledExt,
        button::{Button, ButtonVariants},
    },
    prelude::FluentBuilder,
    *,
};

pub const HEIGHT: Pixels = px(52.);

/// Moves the window when dragged and zooms it on double click, the way the
/// kit's `TitleBar` does: on macOS the app owns titlebar dragging, so empty
/// chrome has to start the move itself. Children that handle the mouse go
/// inside [`no_drag`]. Full screen windows can't move, so nothing is added.
pub fn drag_region(id: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> Stateful<Div> {
    let id = id.into();
    let region = div().id(id.clone());
    if !cfg!(target_os = "macos") || window.is_fullscreen() {
        return region;
    }
    let pressed = window.use_keyed_state(id, cx, |_, _| false);
    let set = |pressed: &Entity<bool>, value| {
        let pressed = pressed.clone();
        move |cx: &mut App| pressed.update(cx, |pressed, _| *pressed = value)
    };
    let (press, release, cancel, zoom) = (
        set(&pressed, true),
        set(&pressed, false),
        set(&pressed, false),
        set(&pressed, false),
    );
    region
        .on_mouse_down(MouseButton::Left, move |_, _, cx| press(cx))
        .on_mouse_up(MouseButton::Left, move |_, _, cx| release(cx))
        .on_mouse_down_out(move |_, _, cx| cancel(cx))
        .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
            if !*pressed.read(cx) {
                return;
            }
            pressed.update(cx, |pressed, _| *pressed = false);
            // The release can miss this region, for example after a double
            // click zooms the window, so only a held button starts a move.
            // Moving on a plain mouse move leaves AppKit swallowing clicks.
            if event.pressed_button == Some(MouseButton::Left) {
                window.start_window_move();
            }
        })
        .on_double_click(move |_, window, cx| {
            zoom(cx);
            window.titlebar_double_click();
        })
}

/// Keeps presses on controls from moving the window or zooming it.
pub fn no_drag() -> Div {
    div().on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

/// The 52px row at the top of a page: title, a faint context label, then the
/// children (usually [`actions`]) on the right. It moves the window.
pub fn page(
    title: &'static str,
    context: Option<SharedString>,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let faint = palette(cx).faint;
    drag_region("page-header", window, cx)
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(10.))
        .h(HEIGHT)
        .pl(px(20.))
        .pr(px(12.))
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(15.))
                .font_semibold()
                .child(title),
        )
        .when_some(context, |row, context| {
            row.child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.))
                    .text_color(faint)
                    .child(context),
            )
        })
        .child(div().flex_1())
}

/// The page's buttons, on the right of the header.
pub fn actions() -> Div {
    no_drag().flex().flex_shrink_0().items_center().gap(px(8.))
}

/// A 28px header button with an optional 14px icon and a 13px label.
pub fn button(
    id: impl Into<ElementId>,
    icon: impl Into<Option<IconName>>,
    label: impl Into<SharedString>,
) -> Button {
    let label = label.into();
    Button::new(id)
        .h(px(28.))
        .px(px(11.))
        .rounded(px(8.))
        .accessibility_label(label.clone())
        .when_some(icon.into(), |button, icon| {
            button.child(Icon::new(icon).size(px(14.)))
        })
        .child(div().text_size(px(13.)).font_medium().child(label))
}

/// A 28px ghost button with a 16px icon and a tooltip.
pub fn icon_button(id: impl Into<ElementId>, icon: IconName, label: &'static str) -> Button {
    // A child icon, since `icon()` sizes it from the 14px rem.
    Button::new(id)
        .ghost()
        .size(px(28.))
        .p_0()
        .rounded(px(8.))
        .child(Icon::new(icon).size(px(16.)))
        .accessibility_label(label)
        .tooltip(label)
        .tooltip_placement(Placement::Bottom)
}

/// The label for a tax year, `None` meaning every year.
pub fn year_label(year: Option<i32>) -> SharedString {
    year.map_or_else(|| "All years".into(), |year| year.to_string().into())
}

/// The page's refresh button.
pub fn refresh(disabled: bool, cx: &App) -> Button {
    icon_button("page-refresh", IconName::RefreshCw, "Refresh")
        .text_color(cx.theme().muted_foreground)
        .disabled(disabled)
}
