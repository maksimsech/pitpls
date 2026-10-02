use gpui_kit::{
    component::{input::*, select::*, *},
    *,
};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct Choice {
    title: SharedString,
    value: SharedString,
}

impl Choice {
    pub fn new(value: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
        }
    }
}

impl SelectItem for Choice {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn value(&self) -> &SharedString {
        &self.value
    }
}

pub type ChoiceState = SelectState<Vec<Choice>>;

pub fn select(
    items: Vec<Choice>,
    value: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<ChoiceState> {
    let index = items
        .iter()
        .position(|choice| choice.value.as_ref() == value)
        .unwrap_or(0);
    cx.new(|cx| SelectState::new(items, Some(IndexPath::default().row(index)), window, cx))
}

enum Control {
    Input(Entity<InputState>),
    Select(Entity<ChoiceState>),
}

struct Field {
    name: &'static str,
    label: &'static str,
    control: Control,
    locked: bool,
}

pub struct Form {
    pub title: SharedString,
    fields: Vec<Field>,
}

const CURRENCIES: &[&str] = &[
    "PLN", "USD", "EUR", "GBP", "CHF", "AUD", "CAD", "NZD", "SGD", "HKD", "THB", "HUF", "UAH",
    "JPY", "CZK", "DKK", "ISK", "NOK", "SEK", "RON", "BGN", "TRY", "ILS", "CLP", "PHP", "MXN",
    "ZAR", "BRL", "MYR", "IDR", "INR", "KRW", "CNY", "XDR",
];

impl Form {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            fields: vec![],
        }
    }

    pub fn input(
        &mut self,
        name: &'static str,
        label: &'static str,
        values: &Value,
        locked: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let value = values[name].as_str().unwrap_or_default().to_string();
        self.fields.push(Field {
            name,
            label,
            locked,
            control: Control::Input(cx.new(|cx| InputState::new(window, cx).default_value(value))),
        });
    }

    pub fn currency(
        &mut self,
        name: &'static str,
        label: &'static str,
        values: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.choice(
            name,
            label,
            CURRENCIES.iter().map(|s| Choice::new(*s, *s)).collect(),
            values,
            window,
            cx,
        );
    }

    pub fn choice(
        &mut self,
        name: &'static str,
        label: &'static str,
        choices: Vec<Choice>,
        values: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.fields.push(Field {
            name,
            label,
            locked: false,
            control: Control::Select(select(
                choices,
                values[name].as_str().unwrap_or_default(),
                window,
                cx,
            )),
        });
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if let Some(field) = self.fields.iter().find(|field| !field.locked) {
            match &field.control {
                Control::Input(state) => window.focus(&state.focus_handle(cx), cx),
                Control::Select(state) => window.focus(&state.focus_handle(cx), cx),
            }
        }
    }

    pub fn render(&self, busy: bool, cx: &App) -> Div {
        v_flex()
            .gap_4()
            .max_w(px(700.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_4()
                    .children(self.fields.iter().map(|field| {
                        let control = match &field.control {
                            Control::Input(state) => Input::new(state)
                                .aria_label(field.label)
                                .disabled(busy || field.locked)
                                .into_any_element(),
                            Control::Select(state) => div()
                                .w_full()
                                .h_8()
                                .flex_shrink_0()
                                .child(
                                    Select::new(state)
                                        .accessibility_label(field.label)
                                        .disabled(busy),
                                )
                                .into_any_element(),
                        };
                        v_flex()
                            .w(px(300.))
                            .flex_grow(1.)
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(field.label),
                            )
                            .child(control)
                    })),
            )
    }

    pub fn values(&self, cx: &App) -> Result<Value, String> {
        let mut values = json!({});
        for field in &self.fields {
            let value = match &field.control {
                Control::Input(state) => state.read(cx).value().trim().to_string(),
                Control::Select(state) => state
                    .read(cx)
                    .selected_value()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            };
            if value.is_empty() && field.name != "id" {
                return Err(format!("{} is required", field.label));
            }
            values[field.name] = json!(value);
        }
        Ok(values)
    }
}
