use gpui_kit::{
    component::{input::*, select::*, *},
    *,
};
use pitpls_core::common::Currency;

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

pub fn required(state: &Entity<InputState>, label: &str, cx: &App) -> Result<String, String> {
    let value = text(state, cx);
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value)
    }
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

pub fn field(label: &'static str, control: impl IntoElement, cx: &App) -> Div {
    v_flex()
        .w(px(300.))
        .flex_grow(1.)
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
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
