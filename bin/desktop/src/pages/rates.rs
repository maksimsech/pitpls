use crate::{
    components::{ButtonText, Status, dialog, file_picker, header, nbp, notice, spinner},
    format,
    navigation::{Page, PageContext},
    theme::{palette, tabular_digits},
};
use chrono::Datelike;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{button::*, dialog::Dialog, input::InputState, scroll::ScrollableElement, *},
    *,
};
use pitpls_app::use_case::rate::{self, RateCoverage};
use std::ops::Range;

/// The pinned Date column, including the panel's 20px inset.
const DATE_WIDTH: Pixels = px(120.);
const RATE_WIDTH: Pixels = px(104.);
const RATE_GAP: Pixels = px(24.);
const END_PADDING: Pixels = px(20.);
const HEADING_HEIGHT: Pixels = px(32.);
const ROW_HEIGHT: Pixels = px(34.);
const TEXT_SIZE: Pixels = px(13.);

#[derive(PartialEq)]
struct RateRow {
    date: SharedString,
    rates: Vec<SharedString>,
}

#[derive(Default, PartialEq)]
struct RateData {
    coverage: Option<RateCoverage>,
    currencies: Vec<SharedString>,
    rows: Vec<RateRow>,
}

#[derive(Default)]
struct Columns {
    starts: Vec<Pixels>,
    ends: Vec<Pixels>,
    width: Pixels,
}

impl Columns {
    /// With tabular digits the longest rate is the widest, so only it is
    /// measured.
    fn new(data: &RateData, window: &Window, cx: &App) -> Self {
        let font = Font {
            features: tabular_digits(),
            ..font(cx.theme().font_family.clone())
        };
        let mut columns = Self::default();
        let mut x = DATE_WIDTH;
        for (index, currency) in data.currencies.iter().enumerate() {
            let longest = data
                .rows
                .iter()
                .map(|row| &row.rates[index])
                .max_by_key(|rate| rate.len())
                .unwrap_or(currency);
            let run = TextRun {
                len: longest.len(),
                font: font.clone(),
                color: Hsla::default(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let text = window
                .text_system()
                .shape_line(longest.clone(), TEXT_SIZE, &[run], None)
                .width;
            columns.starts.push(x);
            x += RATE_WIDTH.max(text + RATE_GAP);
            columns.ends.push(x);
        }
        columns.width = x + END_PADDING;
        columns
    }

    fn visible(&self, scroll: Pixels, viewport: Pixels) -> Range<usize> {
        let first = self.ends.partition_point(|end| *end <= scroll + DATE_WIDTH);
        let end = self
            .starts
            .partition_point(|start| *start < scroll + viewport);
        first..end.max(first)
    }
}

enum Change {
    Csv(String),
    Nbp(i32),
    Reset,
}

#[derive(Clone, Copy, PartialEq)]
enum Running {
    Csv,
    Nbp,
    Reset,
}

pub struct RatesPage {
    context: PageContext,
    status: Status,
    data: RateData,
    columns: Columns,
    nbp_year: Option<Entity<InputState>>,
    confirm_reset: bool,
    running: Option<Running>,
    rate_scroll: UniformListScrollHandle,
    horizontal_scroll: ScrollHandle,
}

impl RatesPage {
    pub fn new(context: PageContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            context,
            status: Status::default(),
            data: RateData::default(),
            columns: Columns::default(),
            nbp_year: None,
            confirm_reset: false,
            running: None,
            rate_scroll: UniformListScrollHandle::new(),
            horizontal_scroll: ScrollHandle::new(),
        };
        page.refresh(window, cx);
        page
    }

    fn locked(&self) -> bool {
        self.status.busy || self.nbp_year.is_some() || self.confirm_reset
    }

    fn disabled(&self) -> bool {
        self.locked() || self.status.loading
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(self.locked(), cx);
        cx.notify();
    }

    fn close_nbp(&mut self, cx: &mut Context<Self>) {
        self.nbp_year = None;
        self.status.error = None;
        self.notify(cx);
    }

    fn change(&mut self, change: Change, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading {
            return;
        }
        self.confirm_reset = false;
        self.running = Some(match change {
            Change::Csv(_) => Running::Csv,
            Change::Nbp(_) => Running::Nbp,
            Change::Reset => Running::Reset,
        });
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                match change {
                    Change::Csv(file) => Ok(format!(
                        "Imported {} rates.",
                        rate::import_csv(&app, file).await?
                    )),
                    Change::Nbp(year) => nbp::import(app, year).await,
                    Change::Reset => {
                        Ok(format!("Removed {} rates.", rate::reset_rates(&app).await?))
                    }
                }
            },
            |this, result, window, cx| {
                this.running = None;
                if this.status.saved(result) {
                    if this.nbp_year.is_some() {
                        window.close_dialog(cx);
                    }
                    this.close_nbp(cx);
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn pick_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading {
            return;
        }
        self.running = Some(Running::Csv);
        self.status.begin_save();
        self.status.task = Some(file_picker::pick(
            "csv",
            window,
            cx,
            |this, result, window, cx| {
                this.running = None;
                this.status.busy = false;
                match result {
                    Ok(Some(file)) => this.change(Change::Csv(file), window, cx),
                    Ok(None) => {}
                    Err(error) => this.status.error = Some(error.into()),
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn open_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let year = chrono::Local::now().year();
        self.nbp_year = Some(nbp::open(year, Self::nbp_dialog, window, cx));
        self.status.error = None;
        self.status.message = None;
        self.notify(cx);
    }

    fn import_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(year) = &self.nbp_year else {
            return;
        };
        match nbp::year(year, cx) {
            Ok(year) => self.change(Change::Nbp(year), window, cx),
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
            }
        }
    }

    fn nbp_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(year) = &self.nbp_year else {
            return dialog;
        };
        nbp::dialog(
            self,
            dialog,
            year,
            |this| &this.status,
            Self::import_nbp,
            Self::close_nbp,
            cx,
        )
    }

    fn open_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_reset = true;
        dialog::confirm(
            "Reset rates",
            "Remove all imported exchange rates? \
             Calculations will be unavailable until rates are reimported.",
            "Reset",
            |this, window, cx| this.change(Change::Reset, window, cx),
            |this, cx| {
                this.confirm_reset = false;
                this.notify(cx);
            },
            window,
            cx,
        );
        self.notify(cx);
    }

    fn header(&self, window: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let disabled = self.disabled();
        let reset_disabled = disabled || !self.status.ready || self.data.rows.is_empty();
        let running = |operation| self.running == Some(operation);
        let range = match self.data.coverage {
            Some(RateCoverage { first, last }) => format!(
                "NBP table A · {} – {}",
                format::date(first).main,
                format::date(last).main
            ),
            None => "NBP table A".into(),
        };
        header::page(Page::Rates.title(), Some(range.into()), window, cx).child(
            header::actions()
                .child(self.status.refreshing(cx))
                .child(
                    header::task_button(
                        "rates-csv",
                        IconName::Upload,
                        "Upload CSV",
                        running(Running::Csv),
                    )
                    .disabled(disabled)
                    .on_click(cx.listener(|this, _, window, cx| this.pick_file(window, cx))),
                )
                .child(
                    header::task_button(
                        "rates-nbp",
                        IconName::Download,
                        "Import from NBP",
                        running(Running::Nbp),
                    )
                    .primary()
                    .disabled(disabled)
                    .on_click(cx.listener(|this, _, window, cx| this.open_nbp(window, cx))),
                )
                .child(
                    // `header::button`'s frame with the colour on the label,
                    // since the kit's hover colour replaces the button's own.
                    // Disabled, it keeps the kit's disabled look.
                    Button::new("rates-reset")
                        .ghost()
                        .h(px(28.))
                        .px(px(11.))
                        .rounded(px(8.))
                        .accessibility_label("Reset")
                        .disabled(reset_disabled)
                        .when(running(Running::Reset), |button| button.child(spinner()))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_medium()
                                .when(!reset_disabled, |label| {
                                    label.text_color(palette(cx).danger)
                                })
                                .child("Reset"),
                        )
                        .on_click(cx.listener(|this, _, window, cx| this.open_reset(window, cx))),
                ),
        )
    }

    /// The NBP dialog shows its own errors, so nothing shows here while it's
    /// open.
    fn notices(&self, cx: &mut Context<Self>) -> Div {
        let disabled = self.disabled();
        let form_open = self.nbp_year.is_some();
        let status = self.status.is_visible() && !form_open;
        let retry = !form_open
            && (self.status.error.is_some() || (!self.status.ready && !self.status.loading));
        v_flex()
            .flex_shrink_0()
            .gap_3()
            .px(px(20.))
            .when(status || retry, |view| view.pb_3())
            .when(status, |view| view.child(self.status.render()))
            .when(retry, |view| {
                view.child(
                    h_flex().child(
                        Button::new("retry-rates")
                            .text_label("Retry")
                            .outline()
                            .disabled(disabled)
                            .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                    ),
                )
            })
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled() {
            return;
        }
        self.status.begin_load(window, cx, |this| &mut this.status);
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            |app| async move {
                let coverage = rate::rate_coverage(&app).await?;
                let data = rate::list_rates(&app).await?;
                let currencies = data
                    .currencies
                    .iter()
                    .map(|currency| currency.to_string().into())
                    .collect();
                let rows = data
                    .rows
                    .into_iter()
                    .rev()
                    .map(|row| RateRow {
                        date: format::date(row.date).main,
                        rates: data
                            .currencies
                            .iter()
                            .map(|currency| {
                                row.rates
                                    .iter()
                                    .find(|rate| rate.currency == *currency)
                                    .map(|rate| rate.rate.to_string().into())
                                    .unwrap_or_else(|| "—".into())
                            })
                            .collect(),
                    })
                    .collect();
                Ok(RateData {
                    coverage,
                    currencies,
                    rows,
                })
            },
            |this, result, window, cx| {
                if let Some(data) = this.status.loaded(result)
                    && data != this.data
                {
                    this.columns = Columns::new(&data, window, cx);
                    this.data = data;
                    this.rate_scroll = UniformListScrollHandle::new();
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for RatesPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.status.ready && self.data.rows.is_empty() {
            notice::notice(
                Page::Rates.icon(),
                false,
                "No exchange rates yet".into(),
                "Import a year from NBP, or upload a CSV.",
                cx,
            )
            .into_any_element()
        } else if self.status.ready {
            self.table(cx)
        } else if self.status.loading {
            self.skeleton(cx).into_any_element()
        } else {
            div().into_any_element()
        };
        v_flex()
            .size_full()
            .min_h_0()
            .child(self.header(window, cx))
            .child(self.notices(cx))
            .child(body)
    }
}

impl RatesPage {
    /// Rows draw only the columns in view, and the Date column stays at the
    /// left edge.
    fn table(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = *palette(cx);
        let mut list = uniform_list(
            "rate-days",
            self.data.rows.len(),
            cx.processor(|this, range: Range<usize>, _, cx| {
                // The list lays out its rows inside the sideways scroll, which
                // has already taken this frame's offset and size.
                let scroll = -this.horizontal_scroll.offset().x;
                let viewport = this.horizontal_scroll.bounds().size.width;
                let columns = this.columns.visible(scroll, viewport);
                range
                    .map(|index| this.row(index, columns.clone(), scroll, cx))
                    .collect::<Vec<_>>()
            }),
        )
        .flex_1()
        .min_h_0()
        .track_scroll(&self.rate_scroll);
        // Keep horizontal wheel events available to the enclosing viewport.
        list.style().restrict_scroll_to_axis = Some(true);
        let table = v_flex()
            .w(self.columns.width)
            .min_w_full()
            .h_full()
            .flex_shrink_0()
            .child(
                h_flex()
                    .flex_shrink_0()
                    .h(HEADING_HEIGHT)
                    .pl(DATE_WIDTH)
                    .border_t_1()
                    .border_b_1()
                    .border_color(p.line)
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .children(
                        self.data
                            .currencies
                            .iter()
                            .zip(self.columns.starts.iter().zip(&self.columns.ends))
                            .map(|(currency, (start, end))| {
                                div()
                                    .w(*end - *start)
                                    .flex_shrink_0()
                                    .text_right()
                                    .child(currency.clone())
                            }),
                    ),
            )
            .child(list);
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .child(
                div()
                    .id("rates-horizontal")
                    .size_full()
                    .overflow_x_scroll()
                    .lock_scroll_axis()
                    .track_scroll(&self.horizontal_scroll)
                    .child(table),
            )
            // The Date heading sits outside the sideways scroll.
            .child(
                pinned(p.surface)
                    .top_0()
                    .left_0()
                    .h(HEADING_HEIGHT)
                    .border_t_1()
                    .border_b_1()
                    .border_color(p.line)
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child("Date"),
            )
            .child(
                div()
                    .absolute()
                    .top(HEADING_HEIGHT)
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .vertical_scrollbar(&self.rate_scroll),
            )
            .child(
                div().absolute().inset_0().child(
                    scroll::Scrollbar::horizontal(&self.horizontal_scroll)
                        .mode(scroll::ScrollbarMode::Always)
                        .viewport_from_layout(),
                ),
            )
            .into_any_element()
    }

    /// The date moves right by `scroll`, so it stays at the left edge over the
    /// rates passing under.
    fn row(&self, index: usize, columns: Range<usize>, scroll: Pixels, cx: &App) -> Div {
        let p = *palette(cx);
        let row = &self.data.rows[index];
        h_flex()
            .group("rate-day")
            .relative()
            .w_full()
            .h(ROW_HEIGHT)
            .border_b_1()
            .border_color(p.line)
            .text_size(TEXT_SIZE)
            .font_features(tabular_digits())
            .hover(|style| style.bg(p.raised))
            .when_some(self.columns.starts.get(columns.start), |view, start| {
                view.pl(*start)
            })
            .children(columns.map(|column| {
                div()
                    .w(self.columns.ends[column] - self.columns.starts[column])
                    .flex_shrink_0()
                    .text_right()
                    .whitespace_nowrap()
                    .child(row.rates[column].clone())
            }))
            .child(
                pinned(p.surface)
                    .top_0()
                    .bottom_0()
                    .left(scroll)
                    .group_hover("rate-day", |style| style.bg(p.raised))
                    .child(row.date.clone()),
            )
    }

    fn skeleton(&self, cx: &App) -> Div {
        let p = *palette(cx);
        let visible = self.status.loading_visible;
        let bar = |width: f32| {
            div()
                .h(px(12.))
                .w(px(width))
                .rounded(px(4.))
                .bg(cx.theme().skeleton)
                .opacity(if visible { 1. } else { 0. })
        };
        v_flex()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(
                h_flex()
                    .h(HEADING_HEIGHT)
                    .pl(px(20.))
                    .border_t_1()
                    .border_b_1()
                    .border_color(p.line)
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child("Date"),
            )
            .children((0..5).map(|_| {
                h_flex()
                    .h(ROW_HEIGHT)
                    .pl(px(20.))
                    .border_b_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .w(DATE_WIDTH - px(20.))
                            .flex_shrink_0()
                            .child(bar(70.)),
                    )
                    .children((0..8).map(|_| {
                        div()
                            .w(RATE_WIDTH)
                            .flex_shrink_0()
                            .flex()
                            .justify_end()
                            .child(bar(48.))
                    }))
            }))
    }
}

/// On the surface colour, so the rates scrolling under it stay hidden.
fn pinned(surface: Hsla) -> Div {
    div()
        .absolute()
        .w(DATE_WIDTH)
        .pl(px(20.))
        .flex()
        .items_center()
        .bg(surface)
}
