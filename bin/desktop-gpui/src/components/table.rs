use crate::{format::DisplayText, theme::tabular_digits};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{component::*, *};

#[derive(PartialEq)]
pub struct Column {
    pub label: SharedString,
    pub width: f32,
    pub numeric: bool,
}

impl Column {
    pub fn text(label: impl Into<SharedString>, width: f32) -> Self {
        Self {
            label: label.into(),
            width,
            numeric: false,
        }
    }

    pub fn number(label: impl Into<SharedString>, width: f32) -> Self {
        Self {
            label: label.into(),
            width,
            numeric: true,
        }
    }
}

fn frame(column: &Column, heading: bool, cx: &App) -> Div {
    div()
        .w(rems(column.width / 14.))
        .flex_shrink_0()
        .px_3()
        .py_2()
        .truncate()
        .when(column.numeric, |cell| cell.text_right())
        .when(!heading && column.numeric, |cell| {
            cell.font_features(tabular_digits())
        })
        .when(heading, |cell| {
            cell.font_medium().text_color(cx.theme().muted_foreground)
        })
}

pub fn cell(value: SharedString, column: &Column, heading: bool, cx: &App) -> Div {
    frame(column, heading, cx).child(value)
}

/// Numbers keep a fixed slot for their extra decimals, so a column's decimal
/// points line up. The cell is plain text: hundreds of selectable cells are too
/// slow, and the full value is in the tooltip and context menu.
pub fn value_cell(
    id: impl Into<ElementId>,
    value: &DisplayText,
    column: &Column,
    cx: &App,
) -> AnyElement {
    let text = super::value::text(value, column.numeric, true, cx);
    super::value::reveal_full(frame(column, false, cx).id(id).child(text), value)
}
