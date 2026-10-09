use super::copy::CopyButton;
use crate::{
    format::{DisplayText, EXTRA_DIGITS},
    theme::{palette, tabular_digits},
};
use gpui_kit::{
    component::{
        StyledExt, h_flex,
        menu::{ContextMenuExt, PopupMenuItem},
        tooltip::Tooltip,
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};
use std::sync::Arc;

/// As wide as a tabular digit; fills the unused part of the decimals slot.
const FIGURE_SPACE: char = '\u{2007}';
const MORE: &str = "…";

/// The value as one text element: `main` in the inherited colour, the extra
/// decimals and "…" faint, and the unit muted when `unit` is set. With
/// `slot`, the room for every extra decimal and the "…" is kept even when
/// they are absent, so the decimal points of a column line up. Only use
/// `slot` for numbers, inside an element with tabular digits.
pub fn text(value: &DisplayText, slot: bool, unit: bool, cx: &App) -> StyledText {
    let p = palette(cx);
    let faint = HighlightStyle::color(p.faint);
    let mut text = value.main.to_string();
    let mut highlights = Vec::with_capacity(3);
    let start = text.len();
    text.push_str(&value.extra);
    if slot {
        let used = value.extra.chars().count();
        text.extend(std::iter::repeat_n(
            FIGURE_SPACE,
            EXTRA_DIGITS.saturating_sub(used),
        ));
    }
    if start < text.len() {
        highlights.push((start..text.len(), faint));
    }
    if value.more || slot {
        let start = text.len();
        text.push_str(MORE);
        // Highlight colours blend over the text colour, so hide the
        // placeholder by fading it out instead.
        let style = if value.more {
            faint
        } else {
            HighlightStyle {
                fade_out: Some(1.),
                ..Default::default()
            }
        };
        highlights.push((start..text.len(), style));
    }
    if unit && let Some(unit) = &value.unit {
        let start = text.len();
        text.push(' ');
        text.push_str(unit);
        highlights.push((start..text.len(), HighlightStyle::color(p.muted)));
    }
    StyledText::new(text).with_highlights(highlights)
}

/// When the value hides digits, shows all of them in a tooltip and offers
/// "Copy full value" on right click, which copies what a copy button would.
pub fn reveal_full(element: Stateful<Div>, value: &DisplayText) -> AnyElement {
    if !value.more {
        return element.into_any_element();
    }
    let full = value.full.clone();
    let copied = value.copy.clone();
    element
        .tooltip(move |window, cx| Tooltip::new(full.clone()).build(window, cx))
        .context_menu(move |menu, _, _| {
            let copied = copied.clone();
            menu.item(
                PopupMenuItem::new("Copy full value").on_click(move |_, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(copied.to_string()))
                }),
            )
        })
        .into_any_element()
}

/// A form value: the number in `size`, medium weight unless `regular`, then
/// its unit in a smaller muted type.
fn form_value(
    id: ElementId,
    value: &DisplayText,
    size: Pixels,
    unit: Pixels,
    regular: bool,
    cx: &App,
) -> Div {
    h_flex()
        .flex_shrink_0()
        .items_baseline()
        .gap_1()
        .child(reveal_full(
            div()
                .id(id)
                .text_size(size)
                .when(!regular, |value| value.font_medium())
                .font_features(tabular_digits())
                .child(text(value, false, false, cx)),
            value,
        ))
        .when_some(value.unit.clone(), |line, text| {
            line.child(
                div()
                    .text_size(unit)
                    .text_color(palette(cx).muted)
                    .child(text),
            )
        })
}

fn copy_button(id: &ElementId, value: &DisplayText) -> CopyButton {
    CopyButton::new(
        ElementId::NamedChild(Arc::new(id.clone()), "copy".into()),
        value.copy.clone(),
    )
}

/// A total: the label above its value and copy button.
pub fn stat(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    value: &DisplayText,
    cx: &App,
) -> Div {
    let id = id.into();
    v_flex()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(palette(cx).muted)
                .child(label.into()),
        )
        .child(
            h_flex()
                .gap_1()
                .child(form_value(id.clone(), value, px(18.), px(12.), false, cx))
                .child(copy_button(&id, value)),
        )
}

/// A form row in a card: label, value and copy button. The copy button has a
/// fixed column, so "Copied" does not move the value. A `muted` row is for a
/// value derived from the others, such as a difference.
pub fn card_row(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    value: &DisplayText,
    muted: bool,
    cx: &App,
) -> Div {
    let id = id.into();
    let p = palette(cx);
    card_row_frame(cx)
        .when(muted, |row| row.text_color(p.muted))
        .child(div().flex_1().min_w_0().truncate().child(label.into()))
        .child(form_value(id.clone(), value, px(16.), px(13.), muted, cx))
        .child(
            h_flex()
                .w(px(86.))
                .flex_shrink_0()
                .justify_end()
                .child(copy_button(&id, value)),
        )
}

/// A card row's placeholder: its label, and a bar where the value will be.
/// The bar reserves its space at once but stays hidden until `visible`.
pub fn card_row_skeleton(label: &'static str, visible: bool, cx: &App) -> Div {
    card_row_frame(cx)
        .child(div().flex_1().min_w_0().truncate().child(label))
        .child(
            div()
                .h(px(20.))
                .w(px(110.))
                .rounded(px(4.))
                .bg(gpui_kit::component::ActiveTheme::theme(cx).skeleton)
                .opacity(if visible { 1. } else { 0. }),
        )
        .child(div().w(px(86.)).flex_shrink_0())
}

fn card_row_frame(cx: &App) -> Div {
    h_flex()
        .h(px(46.))
        .pl_4()
        .pr_2()
        .gap_2()
        .border_t_1()
        .border_color(palette(cx).line)
}

/// A total's placeholder: its label, and a bar where the value will be.
/// The bar reserves its space at once but stays hidden until `visible`.
pub fn stat_skeleton(label: &'static str, visible: bool, cx: &App) -> Div {
    v_flex()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(palette(cx).muted)
                .child(label),
        )
        .child(
            div()
                .my(px(2.))
                .h(px(22.))
                .w(px(120.))
                .rounded(px(4.))
                .bg(gpui_kit::component::ActiveTheme::theme(cx).skeleton)
                .opacity(if visible { 1. } else { 0. }),
        )
}
