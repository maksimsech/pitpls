use crate::theme::tabular_digits;
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
