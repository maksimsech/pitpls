use crate::{
    format::{DATE_FORMAT, DATE_TIME_FORMAT},
    theme::palette,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc, Weekday};
use gpui_kit::{
    assets::IconName,
    component::{
        date_picker::{DatePicker, DatePickerState},
        input::*,
        select::*,
        time_field::TimePrecision,
        *,
    },
    *,
};
use pitpls_core::{common::Currency, time};
use rust_decimal::Decimal;
use std::str::FromStr;

/// The format the use cases parse.
const INPUT_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Clone)]
pub struct Choice<T> {
    title: SharedString,
    value: T,
}

impl<T> Choice<T> {
    pub fn new(value: T, title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            value,
        }
    }
}

impl<T: Clone + PartialEq> SelectItem for Choice<T> {
    type Value = T;
    fn title(&self) -> SharedString {
        self.title.clone()
    }
    fn value(&self) -> &T {
        &self.value
    }
}

pub type ChoiceState<T> = SelectState<Vec<Choice<T>>>;

pub fn select<T: Clone + PartialEq + 'static>(
    items: Vec<Choice<T>>,
    value: T,
    window: &mut Window,
    cx: &mut App,
) -> Entity<ChoiceState<T>> {
    let index = items
        .iter()
        .position(|choice| choice.value == value)
        .map(|index| IndexPath::default().row(index));
    cx.new(|cx| SelectState::new(items, index, window, cx))
}

pub fn selected<T: Clone + PartialEq + 'static>(
    state: &Entity<ChoiceState<T>>,
    label: &str,
    cx: &App,
) -> Result<T, String> {
    state
        .read(cx)
        .selected_value()
        .cloned()
        .ok_or_else(|| format!("{label} is required"))
}

pub fn input(
    value: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).default_value(value))
}

pub fn text(state: &Entity<InputState>, cx: &App) -> String {
    state.read(cx).value().trim().to_string()
}

pub fn optional(state: &Entity<InputState>, cx: &App) -> Option<String> {
    Some(text(state, cx)).filter(|value| !value.is_empty())
}

pub fn optional_amount(state: &Entity<InputState>, cx: &App) -> Option<String> {
    optional(state, cx).map(|value| decimal_point(&value))
}

pub fn required(state: &Entity<InputState>, label: &str, cx: &App) -> Result<String, String> {
    let value = text(state, cx);
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value)
    }
}

/// A comma decimal, as the copy buttons copy it (`3,06`), is read as a dot;
/// text with both a dot and a comma stays as typed, so the use case rejects it.
pub fn amount(state: &Entity<InputState>, label: &str, cx: &App) -> Result<String, String> {
    required(state, label, cx).map(|value| decimal_point(&value))
}

fn decimal_point(value: &str) -> String {
    if value.contains('.') {
        value.to_owned()
    } else {
        value.replacen(',', ".", 1)
    }
}

pub fn year(state: &Entity<InputState>, cx: &App) -> Result<i32, String> {
    required(state, "Year", cx)?
        .parse()
        .map_err(|_| "Enter a valid whole year".into())
}

pub fn currency(
    value: Currency,
    window: &mut Window,
    cx: &mut App,
) -> Entity<ChoiceState<Currency>> {
    use Currency::*;
    let currencies = [
        PLN, USD, EUR, GBP, CHF, AUD, CAD, NZD, SGD, HKD, THB, HUF, UAH, JPY, CZK, DKK, ISK, NOK,
        SEK, RON, BGN, TRY, ILS, CLP, PHP, MXN, ZAR, BRL, MYR, IDR, INR, KRW, CNY, XDR,
    ];
    select(
        currencies
            .into_iter()
            .map(|value| Choice::new(value, value.as_str()))
            .collect(),
        value,
        window,
        cx,
    )
}

pub fn grid() -> Div {
    div().grid().grid_cols(2).gap_x(px(12.)).gap_y(px(14.))
}

pub fn field(label: impl IntoElement, control: impl IntoElement, cx: &App) -> Div {
    v_flex()
        .min_w_0()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(12.))
                .font_medium()
                .text_color(palette(cx).muted)
                .child(label),
        )
        .child(control)
}

pub fn input_field(
    label: &'static str,
    state: &Entity<InputState>,
    disabled: bool,
    cx: &App,
) -> Div {
    field(
        label,
        Input::new(state).aria_label(label).disabled(disabled),
        cx,
    )
}

pub fn select_field<T: Clone + PartialEq + 'static>(
    label: &'static str,
    state: &Entity<ChoiceState<T>>,
    disabled: bool,
    cx: &App,
) -> Div {
    field(
        label,
        div().w_full().h_8().flex_shrink_0().child(
            Select::new(state)
                .accessibility_label(label)
                .disabled(disabled),
        ),
        cx,
    )
}

pub fn amount_field(
    label: &'static str,
    currency_label: &'static str,
    amount: &Entity<InputState>,
    currency: &Entity<ChoiceState<Currency>>,
    disabled: bool,
    cx: &App,
) -> Div {
    field(
        label,
        amount_control(label, currency_label, amount, currency, disabled, cx),
        cx,
    )
}

pub fn amount_control(
    label: &'static str,
    currency_label: &'static str,
    amount: &Entity<InputState>,
    currency: &Entity<ChoiceState<Currency>>,
    disabled: bool,
    cx: &App,
) -> InputGroup {
    let theme = cx.theme();
    InputGroup::new(label)
        .input(Input::new(amount).aria_label(label))
        .addon(
            InputGroupAddon::new(currency_label)
                .align(InputGroupAddonAlignment::InlineEnd)
                .h_full()
                .p_0()
                .border_l_1()
                .border_color(theme.input)
                .bg(theme.button)
                .rounded_r(theme.radius - px(1.))
                .child(
                    div().w(px(86.)).h_full().child(
                        Select::new(currency)
                            .appearance(false)
                            .accessibility_label(currency_label)
                            .disabled(disabled),
                    ),
                ),
        )
        .disabled(disabled)
}

pub fn parsed_amount(
    amount: &Entity<InputState>,
    currency: &Entity<ChoiceState<Currency>>,
    cx: &App,
) -> Option<(Decimal, Currency)> {
    let value = parsed_decimal(amount, cx)?;
    let currency = currency.read(cx).selected_value().copied()?;
    Some((value, currency))
}

pub fn parsed_decimal(state: &Entity<InputState>, cx: &App) -> Option<Decimal> {
    Decimal::from_str(&decimal_point(&text(state, cx))).ok()
}

/// Fields also notify on focus and cursor changes, so `changed` should compare
/// before acting.
pub fn watch<V: 'static, T: 'static>(
    entity: &Entity<T>,
    window: &mut Window,
    cx: &mut Context<V>,
    changed: fn(&mut V, &mut Window, &mut Context<V>),
) -> Subscription {
    cx.observe_in(entity, window, move |view, _, window, cx| {
        changed(view, window, cx)
    })
}

pub fn optional_id(
    value: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<InputState> {
    cx.new(|cx| {
        InputState::new(window, cx)
            .default_value(value)
            .placeholder("Leave blank to auto-generate")
    })
}

pub fn date_picker(
    value: Option<DateTime<Utc>>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<DatePickerState> {
    // Midnight stands for an unknown time, as in date-only sources.
    let value = value.map_or_else(
        || time::local(Utc::now()).date().and_time(NaiveTime::MIN),
        time::local,
    );
    cx.new(|cx| {
        let mut state = DatePickerState::new(window, cx)
            .date_format(DATE_TIME_FORMAT)
            .time_precision(TimePrecision::Second)
            .first_day_of_week(Weekday::Mon);
        state.set_date_time(value, window, cx);
        state
    })
}

pub fn day_picker(
    value: Option<NaiveDate>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<DatePickerState> {
    cx.new(|cx| {
        let mut state = DatePickerState::new(window, cx)
            .date_format(DATE_FORMAT)
            .first_day_of_week(Weekday::Mon);
        if let Some(value) = value {
            state.set_date(value, window, cx);
        }
        state
    })
}

pub fn picked_day(state: &Entity<DatePickerState>, cx: &App) -> Option<NaiveDate> {
    picked_local(state, cx).map(|time| time.date())
}

fn picked_local(state: &Entity<DatePickerState>, cx: &App) -> Option<NaiveDateTime> {
    state.read(cx).date_time().start()
}

pub fn picked_time(state: &Entity<DatePickerState>, cx: &App) -> Option<DateTime<Utc>> {
    picked_local(state, cx).and_then(time::from_local)
}

pub fn selected_time(
    state: &Entity<DatePickerState>,
    label: &str,
    cx: &App,
) -> Result<String, String> {
    picked_local(state, cx)
        .map(|time| time.format(INPUT_TIME_FORMAT).to_string())
        .ok_or_else(|| format!("{label} is required"))
}

pub fn date_field(
    label: &'static str,
    state: &Entity<DatePickerState>,
    disabled: bool,
    cx: &App,
) -> Div {
    field(
        noted_label(label, "Warsaw time", cx),
        DatePicker::new(state).disabled(disabled),
        cx,
    )
}

pub fn noted_label(label: impl Into<SharedString>, note: impl Into<SharedString>, cx: &App) -> Div {
    h_flex()
        .items_baseline()
        .gap(px(6.))
        .child(label.into())
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::NORMAL)
                .text_color(palette(cx).faint)
                .child(note.into()),
        )
}

pub fn hint(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .min_w_0()
        .text_size(px(11.5))
        .text_color(palette(cx).faint)
        .child(text.into())
}

pub fn error(text: impl Into<SharedString>, cx: &App) -> Div {
    h_flex()
        .items_start()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(palette(cx).danger)
        .child(
            Icon::new(IconName::CircleX)
                .size(px(14.))
                .flex_shrink_0()
                .mt(px(1.)),
        )
        .child(div().min_w_0().child(text.into()))
}
