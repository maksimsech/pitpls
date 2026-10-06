use super::{Status, dialog, form};
use chrono::Datelike;
use gpui_kit::{
    component::{dialog::Dialog, input::InputState},
    *,
};

/// The year field of the "Import from NBP" dialog, holding `year`.
pub fn year_input(year: i32, window: &mut Window, cx: &mut App) -> Entity<InputState> {
    form::input(year.to_string(), window, cx)
}

/// The year in the field, if NBP publishes rates for it.
pub fn year(input: &Entity<InputState>, cx: &App) -> Result<i32, String> {
    let current_year = chrono::Local::now().year();
    form::year(input, cx).and_then(|year| {
        if (2002..=current_year).contains(&year) {
            Ok(year)
        } else {
            Err(format!("Enter a year between 2002 and {current_year}"))
        }
    })
}

/// The "Import from NBP" dialog: one year field. `submit` reads it with
/// [`year`] and imports; the view closes the dialog once that succeeds.
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
        if busy { "Importing…" } else { "Import" },
        status,
        submit,
        close,
        cx,
    )
    .title("Import from NBP")
    .child(form::input_field("Year", input, busy, cx))
}
