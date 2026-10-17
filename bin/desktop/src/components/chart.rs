use super::value;
use crate::{
    format::{DisplayText, money},
    theme::{palette, tabular_digits},
};
use gpui_kit::{
    component::{StyledExt, h_flex, tooltip::Tooltip, v_flex},
    prelude::FluentBuilder,
    *,
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use std::rc::Rc;

type TooltipContent<T> = Rc<dyn Fn(&T, &mut Window, &mut App) -> AnyElement>;
type Month<T> = Box<dyn Fn(&T) -> (i32, u32)>;

#[derive(IntoElement)]
pub struct BarChart<T: 'static> {
    id: ElementId,
    data: Vec<T>,
    label: Box<dyn Fn(&T) -> SharedString>,
    value: Box<dyn Fn(&T) -> Decimal>,
    tooltip: TooltipContent<T>,
    month: Option<Month<T>>,
    height: Pixels,
}

impl<T: 'static> BarChart<T> {
    pub fn new(
        id: impl Into<ElementId>,
        data: Vec<T>,
        label: impl Fn(&T) -> SharedString + 'static,
        value: impl Fn(&T) -> Decimal + 'static,
        tooltip: impl Fn(&T, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            data,
            label: Box::new(label),
            value: Box::new(value),
            tooltip: Rc::new(tooltip),
            month: None,
            height: px(196.),
        }
    }

    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    #[expect(dead_code, reason = "The Interests portfolio adopts this in task 9.")]
    pub fn months(mut self, month: impl Fn(&T) -> (i32, u32) + 'static) -> Self {
        self.month = Some(Box::new(month));
        self
    }
}

// The kit fixes its domain to the data extrema; these ticks need round boundaries.
struct Scale {
    low: f64,
    high: f64,
    step: f64,
}

impl Scale {
    fn new(values: &[f64]) -> Self {
        let low = values.iter().copied().fold(0., f64::min);
        let high = values.iter().copied().fold(0., f64::max);
        let target = ((high - low) / 4.).max(1.);
        let magnitude = 10_f64.powf(target.log10().floor());
        let step = [1., 2., 5., 10.]
            .into_iter()
            .map(|factor| factor * magnitude)
            .find(|step| *step >= target)
            .unwrap_or(magnitude * 10.);
        let tick_count = |step: f64| (high / step).ceil() - (low / step).ceil() + 1.;
        let step = if tick_count(step) < 4. && step > 1. {
            (step / 2.).floor().max(1.)
        } else {
            step
        };
        Self {
            low,
            high: ((high / step).ceil() * step).max(((low / step).ceil() + 3.) * step),
            step,
        }
    }

    fn y(&self, value: f64, height: f32) -> f32 {
        ((self.high - value) / (self.high - self.low)) as f32 * height
    }

    fn ticks(&self) -> impl Iterator<Item = f64> + '_ {
        let start = (self.low / self.step).ceil();
        let count = ((self.high / self.step).round() - start) as usize + 1;
        (0..count).map(move |index| (start + index as f64) * self.step)
    }
}

fn tick_label(value: f64) -> String {
    let sign = if value < 0. { "−" } else { "" };
    let value = value.abs();
    if value >= 1000. {
        let thousands = format!("{:.2}", value / 1000.);
        format!(
            "{sign}{}K",
            thousands.trim_end_matches('0').trim_end_matches('.')
        )
    } else {
        format!("{sign}{value:.0}")
    }
}

impl<T: 'static> RenderOnce for BarChart<T> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = *palette(cx);
        let values = self
            .data
            .iter()
            .map(|datum| (self.value)(datum))
            .collect::<Vec<_>>();
        let geometry = values
            .iter()
            .map(|value| value.to_f64().unwrap_or_default())
            .collect::<Vec<_>>();
        let scale = Scale::new(&geometry);
        let height = (self.height.as_f32() - 36.).max(1.);
        let zero = scale.y(0., height);
        let mut plot = div().relative().w_full().h(px(height + 14.));
        for tick in scale.ticks() {
            let y = scale.y(tick, height) + 10.;
            plot = plot.child(
                div()
                    .absolute()
                    .top(px(y))
                    .w_full()
                    .h(px(1.))
                    .bg(p.line)
                    .child(
                        div()
                            .absolute()
                            .right_full()
                            .mr(px(8.))
                            .top(px(-7.))
                            .text_size(px(11.))
                            .text_color(p.faint)
                            .child(tick_label(tick)),
                    ),
            );
        }
        plot = plot.child(
            div()
                .absolute()
                .top(px(zero + 10.))
                .w_full()
                .h(px(1.))
                .bg(p.strong_line),
        );
        let mut bands = h_flex().absolute().size_full().items_start();
        let mut labels = h_flex().h(px(22.));
        for (index, ((datum, decimal), number)) in
            self.data.into_iter().zip(values).zip(geometry).enumerate()
        {
            let label = (self.label)(&datum);
            let band_label = match &self.month {
                Some(month) => {
                    let (year, month) = month(&datum);
                    if index == 0 || month == 1 {
                        year.to_string().into()
                    } else {
                        SharedString::default()
                    }
                }
                None => label.clone(),
            };
            labels = labels.child(
                div().relative().flex_1().min_w_0().h_full().child(
                    div()
                        .absolute()
                        .left_0()
                        .w_full()
                        .text_center()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(band_label),
                ),
            );
            let negative = decimal < Decimal::ZERO;
            let length = (scale.y(number, height) - zero).abs().max(3.);
            let top = if negative { zero } else { zero - length } + 10.;
            let datum = Rc::new(datum);
            let tooltip = self.tooltip.clone();
            bands = bands.child(
                div()
                    .id(index)
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .rounded(px(8.))
                    .role(Role::Image)
                    .aria_label(format!("{label}: {}", money(decimal).full))
                    .hover(move |style| style.bg(p.hover))
                    .tooltip(move |window, cx| {
                        let datum = datum.clone();
                        let tooltip = tooltip.clone();
                        Tooltip::element(move |window, cx| tooltip(&datum, window, cx))
                            .p_0()
                            .border_color(p.strong_line)
                            .build(window, cx)
                    })
                    .child(
                        h_flex()
                            .absolute()
                            .top(px(top))
                            .w_full()
                            .justify_center()
                            .child(
                                div()
                                    .w(px(24.))
                                    .max_w_full()
                                    .h(px(length))
                                    .bg(if negative { p.negative } else { p.series })
                                    .when_else(
                                        negative,
                                        |bar| bar.rounded_b(px(4.)),
                                        |bar| bar.rounded_t(px(4.)),
                                    ),
                            ),
                    ),
            );
        }
        v_flex()
            .id(self.id)
            .flex_shrink_0()
            .h(self.height)
            .w_full()
            .pl(px(52.))
            .pr(px(16.))
            .font_features(tabular_digits())
            .child(plot.child(bands))
            .child(labels)
    }
}

pub fn chart_tooltip(
    title: impl Into<SharedString>,
    rows: Vec<(SharedString, DisplayText)>,
    total: Option<(SharedString, DisplayText)>,
    cx: &App,
) -> Div {
    let p = palette(cx);
    let row = |label, amount: &DisplayText| {
        h_flex()
            .h(px(20.))
            .gap(px(12.))
            .child(div().flex_1().text_color(p.muted).child(label))
            .child(
                div()
                    .text_right()
                    .child(value::text(amount, false, false, cx)),
            )
    };
    v_flex()
        .w(px(236.))
        .px(px(12.))
        .py(px(10.))
        .text_size(px(12.))
        .text_color(p.text)
        .font_features(tabular_digits())
        .child(div().mb(px(4.)).font_semibold().child(title.into()))
        .children(rows.into_iter().map(|(label, amount)| row(label, &amount)))
        .when_some(total, |view, (label, amount)| {
            view.child(
                row(label, &amount)
                    .h(px(26.))
                    .mt(px(4.))
                    .pt(px(4.))
                    .border_t_1()
                    .border_color(p.line)
                    .font_semibold()
                    .child(div().text_color(p.muted).child("PLN")),
            )
        })
}
