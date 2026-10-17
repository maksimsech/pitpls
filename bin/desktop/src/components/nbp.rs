use super::{Status, dialog, form, spinner};
use crate::{i18n::plural, services::Error as ServiceError, theme::palette};
use chrono::Datelike;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{
        dialog::Dialog,
        input::{Input, InputState},
    },
    *,
};
use pitpls_app::use_case::rate;
use rust_i18n::t;
use std::sync::Arc;

pub fn open<V: 'static>(
    year: i32,
    render: fn(&V, Dialog, &mut Context<V>) -> Dialog,
    window: &mut Window,
    cx: &mut Context<V>,
) -> Entity<InputState> {
    let input = form::input(year.to_string(), window, cx);
    dialog::open(window, cx, render);
    window.focus(&input.focus_handle(cx), cx);
    input
}

pub async fn import(app: Arc<pitpls_app::App>, year: i32) -> Result<String, ServiceError> {
    let count = rate::import_api(&app, year).await?;
    Ok(plural("nbp.imported", count as u64).into_owned())
}

/// The year in the field, if NBP publishes rates for it.
pub fn year(input: &Entity<InputState>, cx: &App) -> Result<i32, String> {
    let current_year = chrono::Local::now().year();
    form::year(input, cx).and_then(|year| {
        if (2002..=current_year).contains(&year) {
            Ok(year)
        } else {
            Err(t!("nbp.year_range", first = 2002, last = current_year).into_owned())
        }
    })
}

pub fn dialog<V: 'static>(
    this: &V,
    dialog: Dialog,
    input: &Entity<InputState>,
    status: fn(&V) -> &Status,
    submit: fn(&mut V, &mut Window, &mut Context<V>),
    close: fn(&mut V, &mut Context<V>),
    cx: &mut Context<V>,
) -> Dialog {
    let busy = status(this).busy;
    dialog::form(
        this,
        dialog,
        form::field(
            t!("field.year"),
            Input::new(input)
                .aria_label(t!("field.year"))
                .disabled(busy)
                .when(busy, |input| {
                    input.suffix(spinner().color(palette(cx).muted))
                }),
            cx,
        ),
        if busy {
            t!("common.importing")
        } else {
            t!("common.import")
        },
        status,
        submit,
        close,
        cx,
    )
    .title(t!("nbp.import_title"))
}
