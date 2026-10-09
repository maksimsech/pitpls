use super::copy::CopyButton;
use crate::{
    format::{DisplayText, EXTRA_DIGITS},
    theme::{palette, tabular_digits},
};
use gpui_kit::{
    component::{
        ActiveTheme, StyledExt, h_flex,
        menu::{ContextMenuExt, PopupMenuItem},
        tooltip::Tooltip,
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};
use std::{ops::Range, sync::Arc};

/// As wide as a tabular digit; fills the unused part of the decimals slot.
const FIGURE_SPACE: char = '\u{2007}';
const MORE: &str = "…";

enum Shade {
    Faint,
    Hidden,
    Muted,
}

fn compose(value: &DisplayText, slot: bool, unit: bool) -> (String, Vec<(Range<usize>, Shade)>) {
    let mut text = value.main.to_string();
    let mut parts = Vec::with_capacity(3);
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
        parts.push((start..text.len(), Shade::Faint));
    }
    if value.more || slot {
        let start = text.len();
        text.push_str(MORE);
        let shade = if value.more {
            Shade::Faint
        } else {
            Shade::Hidden
        };
        parts.push((start..text.len(), shade));
    }
    if unit && let Some(unit) = &value.unit {
        let start = text.len();
        text.push(' ');
        text.push_str(unit);
        parts.push((start..text.len(), Shade::Muted));
    }
    (text, parts)
}

/// With `slot`, the room for every extra decimal and the "…" is kept even when
/// they're absent, so a column's decimal points line up. Only for numbers
/// inside an element with tabular digits.
pub fn text(value: &DisplayText, slot: bool, unit: bool, cx: &App) -> StyledText {
    let p = palette(cx);
    let (text, parts) = compose(value, slot, unit);
    let highlights = parts
        .into_iter()
        .map(|(range, shade)| {
            let style = match shade {
                Shade::Faint => HighlightStyle::color(p.faint),
                // Highlight colours blend over the text colour, so hide the
                // placeholder by fading it out instead.
                Shade::Hidden => HighlightStyle {
                    fade_out: Some(1.),
                    ..Default::default()
                },
                Shade::Muted => HighlightStyle::color(p.muted),
            };
            (range, style)
        })
        .collect::<Vec<_>>();
    StyledText::new(text).with_highlights(highlights)
}

pub fn text_width(
    value: &DisplayText,
    slot: bool,
    unit: bool,
    size: Pixels,
    window: &Window,
    cx: &App,
) -> Pixels {
    let (text, _) = compose(value, slot, unit);
    let run = TextRun {
        len: text.len(),
        font: Font {
            features: tabular_digits(),
            ..font(cx.theme().font_family.clone())
        },
        color: Hsla::default(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(text.into(), size, &[run], None)
        .width
}

/// So the tooltips can tell when a "Copy full value" menu is open.
struct ValueMenu(FocusHandle);

impl Global for ValueMenu {}

fn value_menu_open(window: &Window, cx: &App) -> bool {
    cx.try_global::<ValueMenu>()
        .is_some_and(|menu| menu.0.contains_focused(window, cx))
}

pub fn reveal_full(element: Stateful<Div>, value: &DisplayText) -> AnyElement {
    if !value.more {
        return element.into_any_element();
    }
    let full = value.full.clone();
    let copied = value.copy.clone();
    element
        // A right click would focus the row or month header around the
        // value, and the menu focuses itself while it's drawn. Two focused
        // elements in one frame break the accessibility tree, so nothing has
        // focus when the menu opens. Closing it gives focus back.
        .on_mouse_down(MouseButton::Right, |_, window, cx| {
            window.prevent_default();
            window.blur(cx);
        })
        // The menu opens under the pointer, which still hovers the value, so
        // its tooltip would come back over the menu. While a menu is open the
        // tooltip is empty.
        .tooltip(move |window, cx| {
            if value_menu_open(window, cx) {
                cx.new(|_| EmptyView).into()
            } else {
                Tooltip::new(full.clone()).build(window, cx)
            }
        })
        .context_menu(move |menu, _, cx| {
            cx.set_global(ValueMenu(menu.focus_handle(cx)));
            let copied = copied.clone();
            menu.item(
                PopupMenuItem::new("Copy full value").on_click(move |_, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(copied.to_string()))
                }),
            )
        })
        .into_any_element()
}

fn baseline_drop(size: Pixels, unit: Pixels, cx: &App) -> Pixels {
    let system = cx.text_system();
    let font = system.resolve_font(&font(cx.theme().font_family.clone()));
    // A line centres its text by ascent and descent, so the baseline sits
    // half their difference below the middle. Platforms disagree on the sign
    // of the descent.
    let below_middle = |size| (system.ascent(font, size) - system.descent(font, size).abs()) / 2.;
    below_middle(size) - below_middle(unit)
}

fn form_value(
    id: ElementId,
    value: &DisplayText,
    size: Pixels,
    unit: Pixels,
    regular: bool,
    cx: &App,
) -> Div {
    // Layout knows no text baselines, so `items_baseline` lines up the
    // bottoms instead. Centre both and move the unit down to the number's.
    h_flex()
        .flex_shrink_0()
        .items_center()
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
                    .relative()
                    .top(baseline_drop(size, unit, cx))
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

/// Totals sit side by side without a fixed column, so the copy button shows
/// only a check, which doesn't move the next total.
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
                .child(copy_button(&id, value).check_only()),
        )
}

/// The copy button's fixed column keeps "Copied" from moving the value.
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

pub fn card_row_skeleton(label: &'static str, visible: bool, cx: &App) -> Div {
    card_row_frame(cx)
        .child(div().flex_1().min_w_0().truncate().child(label))
        .child(
            div()
                .h(px(20.))
                .w(px(110.))
                .rounded(px(4.))
                .bg(cx.theme().skeleton)
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
                .bg(cx.theme().skeleton)
                .opacity(if visible { 1. } else { 0. }),
        )
}
