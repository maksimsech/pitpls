use gpui_kit::{
    base::{TooltipOverlay, TooltipRequest},
    component::{ActiveTheme, ElementExt, Placement, tooltip::Tooltip},
    *,
};
use std::{cell::Cell, rc::Rc};

/// App-owned tooltip styling, with the toolkit's delayed display and placement.
pub fn hover_label(
    id: &'static str,
    content: impl IntoElement,
    label: impl Into<SharedString>,
    placement: Placement,
    overlay: &Entity<TooltipOverlay>,
) -> Stateful<Div> {
    let label = label.into();
    let bounds = Rc::new(Cell::new(Bounds::default()));
    let bounds_writer = bounds.clone();
    let hover_overlay = overlay.clone();
    let click_overlay = overlay.clone();

    div()
        .id(id)
        .relative()
        .flex()
        .flex_shrink_0()
        .on_prepaint(move |bounds, _, _| bounds_writer.set(bounds))
        .on_hover(move |hovered, window, cx| {
            hover_overlay.update(cx, |overlay, cx| {
                if *hovered {
                    let label = label.clone();
                    overlay.request_show(
                        TooltipRequest::new(bounds.get(), move |window, cx| {
                            Tooltip::new(label.clone())
                                .m(px(4.))
                                .px(px(12.))
                                .py(px(6.))
                                .rounded(px(12.))
                                .text_size(px(14.))
                                .line_height(px(18.))
                                .font_weight(FontWeight::NORMAL)
                                .bg(if cx.theme().is_dark() {
                                    rgb(0x2b2b2b)
                                } else {
                                    rgb(0xffffff)
                                })
                                .border_color(cx.theme().border)
                                .text_color(cx.theme().foreground)
                                .build(window, cx)
                        })
                        .placement(placement),
                        window,
                        cx,
                    );
                } else {
                    overlay.request_hide(window, cx);
                }
            });
        })
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            click_overlay.update(cx, |overlay, cx| overlay.hide(cx));
        })
        .child(content)
}
