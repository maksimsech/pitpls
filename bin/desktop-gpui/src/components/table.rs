use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{component::*, *};

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

pub fn cell(value: SharedString, column: &Column, heading: bool, cx: &App) -> Div {
    div()
        .w(rems(column.width / 14.))
        .flex_shrink_0()
        .px_3()
        .py_2()
        .overflow_hidden()
        .when(column.numeric, |cell| cell.text_right())
        .when(!heading && column.numeric, |cell| {
            cell.font_family(cx.theme().mono_font_family.clone())
        })
        .when(heading, |cell| {
            cell.font_medium().text_color(cx.theme().muted_foreground)
        })
        .child(value)
}
