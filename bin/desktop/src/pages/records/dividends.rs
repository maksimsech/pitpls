mod portfolio;

use super::{RecordForm, RecordKind, Submission, conversion, date_label, day, percent};
use crate::{
    components::{
        form::{self, ChoiceState},
        records::{CellStyle, EntryStyle, Preview, RecordColumn, RowDisplay, Step, StepLine},
    },
    format::{DisplayText, amount, date, money, pln},
    navigation::Page,
    services::Error as ServiceError,
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use gpui_kit::{
    component::{date_picker::DatePickerState, input::InputState},
    *,
};
use pitpls_app::use_case::{
    dividend::{self, CreateDividendInput, DividendFields, UpdateDividendInput},
    year::YearInfo,
};
use pitpls_core::{
    common::{Amount, Country, Currency},
    dividend::{CalculatedDividend, Dividend},
    tax::{POLAND_TAX, get_treaty_tax},
    time,
};
use pitpls_importers::OutputType;
use portfolio::DividendPortfolioView;
use rust_decimal::Decimal;
use rust_i18n::t;
use std::{borrow::Cow, sync::Arc};

pub struct Dividends;

impl RecordKind for Dividends {
    type Record = CalculatedDividend;
    type Form = DividendForm;
    type Portfolio = DividendPortfolioView;

    const PAGE: Page = Page::Dividends;
    const EMPTY: &'static str = "dividends.empty";
    const EMPTY_YEAR: &'static str = "dividends.empty_year";
    const OTHER_YEAR: &'static str = "dividends.other_year";
    const TOTAL_LABELS: &'static [&'static str] = &[
        "tax.income_i65",
        "tax.to_pay_short_g47",
        "tax.paid_short_g48",
    ];

    fn id(record: &CalculatedDividend) -> &str {
        &record.id
    }

    fn time(record: &CalculatedDividend) -> DateTime<Utc> {
        record.time
    }

    fn columns() -> Vec<RecordColumn> {
        vec![
            RecordColumn::new("column.date", CellStyle::Time, 104., 44.),
            RecordColumn::new("column.ticker", CellStyle::Strong, 76., 56.),
            RecordColumn::grow("column.provider", CellStyle::Muted),
            RecordColumn::new("column.country", CellStyle::Muted, 64., 46.),
            RecordColumn::new("column.value", CellStyle::Number, 150., 124.),
            RecordColumn::new("column.tax_paid", CellStyle::Number, 124., 104.),
        ]
    }

    fn display(record: &CalculatedDividend) -> RowDisplay {
        let cells = vec![
            day(record.time),
            DisplayText::plain(record.ticker.clone()),
            DisplayText::plain(record.provider.clone()),
            DisplayText::plain(record.country.to_string()),
            amount(record.value),
            amount(record.tax_paid),
        ];
        let steps = vec![
            Step {
                title: "records.step_conversion",
                lines: vec![
                    StepLine::Formula(conversion(record.value, record.nbp_rate)),
                    StepLine::Result(vec![pln(record.calculated_value)]),
                    StepLine::Caption(
                        t!(
                            "records.calculated_value_nbp",
                            date = date(record.nbp_date).main
                        )
                        .into(),
                    ),
                ],
            },
            Step {
                title: "records.step_polish_tax",
                lines: vec![
                    StepLine::Formula(vec![
                        DisplayText::plain(format!("{} ×", percent(POLAND_TAX))),
                        money(record.calculated_value),
                    ]),
                    StepLine::Result(vec![pln(record.calculated_to_pay)]),
                    StepLine::Caption(t!("dividends.calculated_to_pay").into()),
                ],
            },
            Step {
                title: "dividends.step_credit",
                lines: vec![
                    StepLine::Formula(
                        [
                            vec![DisplayText::plain(t!("field.tax_paid"))],
                            conversion(record.tax_paid, record.tax_paid_nbp_rate),
                        ]
                        .concat(),
                    ),
                    StepLine::Entry {
                        label: t!("dividends.calculated_tax_paid").into(),
                        value: vec![pln(record.calculated_tax_paid)],
                        style: EntryStyle::Normal,
                    },
                    StepLine::Entry {
                        label: t!("dividends.max_tax_paid").into(),
                        value: vec![pln(record.max_tax_paid)],
                        style: EntryStyle::Normal,
                    },
                    StepLine::Entry {
                        label: t!("dividends.used_tax_paid").into(),
                        value: vec![pln(record.used_tax_paid)],
                        style: EntryStyle::Strong,
                    },
                    StepLine::Caption(
                        t!(
                            "dividends.treaty_note",
                            rate = percent(get_treaty_tax(&record.country)),
                            country = record.country
                        )
                        .into(),
                    ),
                ],
            },
        ];
        RowDisplay {
            id: record.id.clone().into(),
            time: record.time,
            cells,
            steps,
            search: format!("{}\n{}\n{}", record.ticker, record.provider, record.country)
                .to_lowercase(),
            label: format!("{} {}", record.ticker, date_label(record.time)).into(),
            ..Default::default()
        }
    }

    fn portfolio_display(record: &CalculatedDividend) -> RowDisplay {
        let mut row = Self::display(record);
        row.steps.truncate(1);
        row.steps.push(Step {
            title: "dividends.step_withheld",
            lines: vec![
                StepLine::Formula(conversion(record.tax_paid, record.tax_paid_nbp_rate)),
                StepLine::Result(vec![pln(record.calculated_tax_paid)]),
                StepLine::ValueCaption {
                    label: t!("dividends.net").into(),
                    value: pln(record.calculated_value - record.calculated_tax_paid),
                    caption: t!(
                        "dividends.tax_year_note",
                        year = time::tax_date(record.time).year()
                    )
                    .into(),
                },
            ],
        });
        row
    }

    fn portfolio_preview(record: &CalculatedDividend) -> Preview {
        Preview {
            nbp_date: record.nbp_date,
            formula: conversion(record.value, record.nbp_rate),
            value: pln(record.calculated_value),
            results: vec![
                ("dividends.withheld", pln(record.calculated_tax_paid)),
                (
                    "dividends.net",
                    pln(record.calculated_value - record.calculated_tax_paid),
                ),
            ],
        }
    }

    async fn load_portfolio(
        app: Arc<pitpls_app::App>,
        today: NaiveDate,
    ) -> Result<(DividendPortfolioView, Vec<CalculatedDividend>), ServiceError> {
        let data = pitpls_app::use_case::portfolio::load_dividend_portfolio(&app).await?;
        Ok((
            DividendPortfolioView::new(data.years, data.payers, today),
            data.data.calculated,
        ))
    }

    fn subtotal(records: &[&CalculatedDividend]) -> Vec<(Option<&'static str>, Decimal)> {
        let value = records.iter().map(|record| record.calculated_value).sum();
        vec![(None, value)]
    }

    fn imported(output: &OutputType) -> bool {
        matches!(output, OutputType::Dividend)
    }

    fn count(year: &YearInfo) -> u32 {
        year.dividends
    }

    async fn load(
        app: Arc<pitpls_app::App>,
        year: Option<i32>,
    ) -> Result<(Vec<Decimal>, Vec<CalculatedDividend>), ServiceError> {
        let data = dividend::load_dividends(&app, year).await?;
        Ok((vec![data.income, data.to_pay, data.paid], data.calculated))
    }

    async fn save(
        app: Arc<pitpls_app::App>,
        submission: DividendSubmission,
    ) -> Result<(), ServiceError> {
        match submission {
            Submission::Create(input) => dividend::create_dividend(&app, input).await.map(drop),
            Submission::Update(input) => dividend::update_dividend(&app, input).await,
        }
        .map_err(ServiceError::from)
    }

    async fn delete(app: Arc<pitpls_app::App>, ids: Vec<String>) -> Result<u64, ServiceError> {
        dividend::delete_dividends(&app, ids)
            .await
            .map_err(ServiceError::from)
    }

    async fn preview(
        app: Arc<pitpls_app::App>,
        draft: DividendDraft,
    ) -> Result<CalculatedDividend, ServiceError> {
        let (value, value_currency) = draft.value;
        let (tax_paid, tax_paid_currency) = draft.tax_paid;
        let dividend = Dividend {
            id: String::new(),
            time: draft.time,
            ticker: String::new(),
            value: Amount {
                value,
                currency: value_currency,
            },
            tax_paid: Amount {
                value: tax_paid,
                currency: tax_paid_currency,
            },
            country: draft.country,
            provider: String::new(),
        };
        dividend::preview_dividend(&app, dividend)
            .await
            .map_err(ServiceError::from)
    }

    fn preview_display(record: &CalculatedDividend) -> Preview {
        Preview {
            nbp_date: record.nbp_date,
            formula: conversion(record.value, record.nbp_rate),
            value: pln(record.calculated_value),
            results: vec![
                ("dividends.calculated_to_pay", pln(record.calculated_to_pay)),
                ("dividends.used_tax_paid", pln(record.used_tax_paid)),
            ],
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct DividendDraft {
    time: DateTime<Utc>,
    value: (Decimal, Currency),
    tax_paid: (Decimal, Currency),
    country: Country,
}

type DividendSubmission = Submission<CreateDividendInput, UpdateDividendInput>;

pub struct DividendForm {
    existing_id: Option<String>,
    id: Entity<InputState>,
    date: Entity<DatePickerState>,
    ticker: Entity<InputState>,
    value: Entity<InputState>,
    value_currency: Entity<ChoiceState<Currency>>,
    tax_paid: Entity<InputState>,
    tax_paid_currency: Entity<ChoiceState<Currency>>,
    country: Entity<InputState>,
    provider: Entity<InputState>,
}

impl RecordForm for DividendForm {
    type Record = CalculatedDividend;
    type Submission = DividendSubmission;
    type Draft = DividendDraft;

    fn new(record: Option<&CalculatedDividend>, window: &mut Window, cx: &mut App) -> Self {
        let text = |value: fn(&CalculatedDividend) -> String| record.map(value).unwrap_or_default();
        Self {
            existing_id: record.map(|record| record.id.clone()),
            id: form::optional_id(text(|record| record.id.clone()), window, cx),
            date: form::date_picker(record.map(|record| record.time), window, cx),
            ticker: form::input(text(|record| record.ticker.clone()), window, cx),
            value: form::input(text(|record| record.value.value.to_string()), window, cx),
            value_currency: form::currency(
                record.map_or(Currency::USD, |record| record.value.currency),
                window,
                cx,
            ),
            tax_paid: form::input(
                record.map_or_else(|| "0".into(), |record| record.tax_paid.value.to_string()),
                window,
                cx,
            ),
            tax_paid_currency: form::currency(
                record.map_or(Currency::USD, |record| record.tax_paid.currency),
                window,
                cx,
            ),
            country: form::input(
                record.map_or_else(|| "US".into(), |record| record.country.to_string()),
                window,
                cx,
            ),
            provider: form::input(text(|record| record.provider.clone()), window, cx),
        }
    }

    fn title(&self) -> Cow<'static, str> {
        if self.existing_id.is_some() {
            t!("dividends.edit")
        } else {
            t!("dividends.add")
        }
    }

    fn existing_id(&self) -> Option<&str> {
        self.existing_id.as_deref()
    }

    fn id_input(&self) -> &Entity<InputState> {
        &self.id
    }

    fn first_input(&self) -> &Entity<InputState> {
        &self.ticker
    }

    fn submission(&self, cx: &App) -> Result<DividendSubmission, String> {
        let time = form::selected_time(&self.date, &t!("field.date"), cx)?;
        let ticker = form::required(&self.ticker, &t!("field.ticker"), cx)?;
        let value = form::amount(&self.value, &t!("field.value"), cx)?;
        let value_currency = form::selected(&self.value_currency, &t!("field.value_currency"), cx)?;
        let tax_paid = form::amount(&self.tax_paid, &t!("field.tax_paid"), cx)?;
        let tax_paid_currency =
            form::selected(&self.tax_paid_currency, &t!("field.tax_paid_currency"), cx)?;
        let country = form::required(&self.country, &t!("field.country_code"), cx)?
            .parse()
            .map_err(crate::messages::country_error)?;
        let provider = form::required(&self.provider, &t!("field.provider"), cx)?;
        let fields = DividendFields {
            time,
            ticker,
            value,
            value_currency,
            tax_paid,
            tax_paid_currency,
            country,
            provider,
        };
        Ok(match self.existing_id.clone() {
            Some(id) => Submission::Update(UpdateDividendInput { id, fields }),
            None => Submission::Create(CreateDividendInput {
                id: form::optional(&self.id, cx),
                fields,
            }),
        })
    }

    fn draft(&self, cx: &App) -> Option<DividendDraft> {
        Some(DividendDraft {
            time: form::picked_time(&self.date, cx)?,
            value: form::parsed_amount(&self.value, &self.value_currency, cx)?,
            tax_paid: form::parsed_amount(&self.tax_paid, &self.tax_paid_currency, cx)?,
            country: form::text(&self.country, cx).parse().ok()?,
        })
    }

    fn watch<V: 'static>(
        &self,
        window: &mut Window,
        cx: &mut Context<V>,
        changed: fn(&mut V, &mut Window, &mut Context<V>),
    ) -> Vec<Subscription> {
        vec![
            form::watch(&self.date, window, cx, changed),
            form::watch(&self.value, window, cx, changed),
            form::watch(&self.value_currency, window, cx, changed),
            form::watch(&self.tax_paid, window, cx, changed),
            form::watch(&self.tax_paid_currency, window, cx, changed),
            form::watch(&self.country, window, cx, changed),
        ]
    }

    fn render(&self, busy: bool, cx: &App) -> Div {
        form::grid()
            .child(form::date_field(t!("field.date"), &self.date, busy, cx))
            .child(form::input_field(
                t!("field.ticker"),
                &self.ticker,
                busy,
                cx,
            ))
            .child(form::amount_field(
                t!("field.value"),
                t!("field.value_currency"),
                &self.value,
                &self.value_currency,
                busy,
                cx,
            ))
            .child(form::amount_field(
                t!("field.tax_paid"),
                t!("field.tax_paid_currency"),
                &self.tax_paid,
                &self.tax_paid_currency,
                busy,
                cx,
            ))
            .child(form::input_field(
                t!("field.country_code"),
                &self.country,
                busy,
                cx,
            ))
            .child(form::input_field(
                t!("field.provider"),
                &self.provider,
                busy,
                cx,
            ))
    }
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use chrono::{DateTime, Utc};
    use gpui_kit::{AppContext as _, TestAppContext};
    use pitpls_core::{
        common::Country,
        dividend::{CalculatedDividend, Dividend, calculate},
        rate::NbpRateProvider,
        settings::DividendRounding,
    };
    use rust_decimal::{Decimal, dec};

    use super::{Dividends, RecordKind};
    use crate::{
        components::records::{EntryStyle, Preview, RowDisplay, Step, StepLine},
        fixtures::{self, Event, REVOLUT, T212, Ui, on, rates, usd},
        format::DisplayText,
        navigation::Page,
        pages::records::RecordsPage,
    };

    // 30% withheld, so the credit stops at the US treaty's 15%.
    fn record() -> CalculatedDividend {
        let dividend = Dividend {
            id: "ko".to_owned(),
            time: on(2025, 5, 15),
            ticker: "KO".to_owned(),
            value: usd(dec!(100)),
            tax_paid: usd(dec!(30)),
            country: Country::USA,
            provider: REVOLUT.to_owned(),
        };
        let rates = NbpRateProvider::new(rates(2025..=2025));
        calculate(vec![dividend], &rates, DividendRounding::SumToGroszy)
            .unwrap()
            .calculated
            .pop()
            .unwrap()
    }

    fn texts(parts: &[DisplayText]) -> Vec<&str> {
        parts.iter().map(|part| part.full.as_ref()).collect()
    }

    fn titles(row: &RowDisplay) -> Vec<&str> {
        row.steps.iter().map(|step| step.title).collect()
    }

    fn results(step: &Step) -> Vec<Vec<&str>> {
        step.lines
            .iter()
            .filter_map(|line| match line {
                StepLine::Result(value) | StepLine::Entry { value, .. } => Some(texts(value)),
                StepLine::ValueCaption { value, .. } => Some(vec![value.full.as_ref()]),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tax_year_row_shows_conversion_tax_and_credit() {
        let row = Dividends::display(&record());

        assert_eq!(
            texts(&row.cells),
            ["15.05", "KO", "Revolut", "US", "100.00 USD", "30.00 USD"]
        );
        assert_eq!(
            titles(&row),
            [
                "records.step_conversion",
                "records.step_polish_tax",
                "dividends.step_credit"
            ]
        );
        let [conversion, tax, credit] = &row.steps[..] else {
            panic!("expected three steps");
        };
        let StepLine::Formula(formula) = &conversion.lines[0] else {
            panic!("expected the conversion formula first");
        };
        assert_eq!(texts(formula), ["100.00 USD", "× 4"]);
        assert_eq!(results(conversion), [["400.00 PLN"]]);
        assert_eq!(results(tax), [["76.00 PLN"]]);
        assert_eq!(
            results(credit),
            [["120.00 PLN"], ["60.00 PLN"], ["60.00 PLN"]]
        );
        let strong = credit
            .lines
            .iter()
            .filter_map(|line| match line {
                StepLine::Entry { value, style, .. } if *style == EntryStyle::Strong => {
                    Some(texts(value))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(strong, [["60.00 PLN"]]);
    }

    #[test]
    fn all_years_row_has_no_polish_tax_or_credit() {
        let record = record();

        let row = Dividends::portfolio_display(&record);
        assert_eq!(
            titles(&row),
            ["records.step_conversion", "dividends.step_withheld"]
        );
        assert_eq!(results(&row.steps[1]), [["120.00 PLN"], ["280.00 PLN"]]);

        let keys = |preview: Preview| {
            preview
                .results
                .into_iter()
                .map(|(key, value)| (key, value.full.to_string()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            keys(Dividends::portfolio_preview(&record)),
            [
                ("dividends.withheld", "120.00 PLN".to_owned()),
                ("dividends.net", "280.00 PLN".to_owned()),
            ]
        );
        assert_eq!(
            keys(Dividends::preview_display(&record)),
            [
                ("dividends.calculated_to_pay", "76.00 PLN".to_owned()),
                ("dividends.used_tax_paid", "60.00 PLN".to_owned()),
            ]
        );
    }

    fn paid(id: &str, time: DateTime<Utc>, provider: &str, value: Decimal) -> Dividend {
        Dividend {
            id: id.to_owned(),
            time,
            ticker: id.to_uppercase(),
            value: usd(value),
            tax_paid: usd(value * dec!(0.15)),
            country: Country::USA,
            provider: provider.to_owned(),
        }
    }

    fn open(
        cx: &mut TestAppContext,
        rates: Option<RangeInclusive<i32>>,
        dividends: Vec<Dividend>,
        year: Option<i32>,
    ) -> Ui<RecordsPage<Dividends>> {
        let runtime = fixtures::runtime();
        let app = runtime.block_on(async {
            let app = match rates {
                Some(years) => fixtures::app(years).await,
                None => fixtures::app_without_rates().await,
            };
            for dividend in &dividends {
                app.db.dividend_repo().insert(dividend).await.unwrap();
            }
            app
        });
        let ui = fixtures::open(cx, runtime, app, move |context, window, cx| {
            cx.new(|cx| RecordsPage::<Dividends>::new(context, year, None, window, cx))
        });
        ui.wait_loaded(1, cx);
        ui
    }

    // KO in May and AAPL in March 2025, at USD 4.
    fn open_2025(cx: &mut TestAppContext) -> Ui<RecordsPage<Dividends>> {
        let dividends = vec![
            paid("ko", on(2025, 5, 15), REVOLUT, dec!(100)),
            paid("aapl", on(2025, 3, 10), T212, dec!(50)),
        ];
        open(cx, Some(2025..=2025), dividends, Some(2025))
    }

    fn stored(ui: &Ui<RecordsPage<Dividends>>, cx: &TestAppContext) -> Vec<(String, Decimal)> {
        let services = ui.read(cx, |page, _| page.context.services.clone());
        let mut stored = services
            .runtime
            .block_on(services.app.db.dividend_repo().get_by_year(None))
            .unwrap()
            .into_iter()
            .map(|dividend| (dividend.id, dividend.value.value))
            .collect::<Vec<_>>();
        stored.sort();
        stored
    }

    fn totals(ui: &Ui<RecordsPage<Dividends>>, cx: &TestAppContext) -> Vec<(&'static str, String)> {
        ui.read(cx, |page, _| {
            page.totals
                .iter()
                .map(|(key, value)| (*key, value.full.to_string()))
                .collect()
        })
    }

    #[gpui_kit::test]
    fn year_page_lists_its_records_by_month_with_totals(cx: &mut TestAppContext) {
        let ui = open_2025(cx);

        assert_eq!(
            totals(&ui, cx),
            [
                ("tax.income_i65", "600.00 PLN".to_owned()),
                ("tax.to_pay_short_g47", "114.00 PLN".to_owned()),
                ("tax.paid_short_g48", "90.00 PLN".to_owned()),
            ]
        );
        let groups = ui.read(cx, |page, _| {
            page.table
                .groups
                .iter()
                .map(|group| (group.title.to_string(), group.count.to_string()))
                .collect::<Vec<_>>()
        });
        assert_eq!(
            groups,
            [
                ("March".to_owned(), "1 record".to_owned()),
                ("May".to_owned(), "1 record".to_owned()),
            ]
        );
        assert!(ui.find("select-ko", cx).is_some_and(|row| row.visible()));
        assert!(ui.find("select-aapl", cx).is_some_and(|row| row.visible()));
    }

    #[gpui_kit::test]
    fn clicking_a_row_opens_and_closes_its_calculation(cx: &mut TestAppContext) {
        let ui = open_2025(cx);
        assert_eq!(ui.find("line-ko", cx).unwrap().expanded(), Some(false));
        assert!(ui.find("edit-ko", cx).is_none());

        ui.click("line-ko", cx);
        assert_eq!(ui.find("line-ko", cx).unwrap().expanded(), Some(true));
        assert!(
            ui.find("edit-ko", cx)
                .is_some_and(|button| button.visible())
        );
        assert!(ui.find("delete-ko", cx).is_some());

        ui.click("line-ko", cx);
        assert!(ui.find("edit-ko", cx).is_none());
    }

    #[gpui_kit::test]
    fn edited_value_is_saved_and_recalculated(cx: &mut TestAppContext) {
        let ui = open_2025(cx);
        ui.click("line-ko", cx);
        ui.click("edit-ko", cx);

        let (ticker, value) = ui.read(cx, |page, cx| {
            let form = &page.editor.as_ref().expect("the editor is open").form;
            (form.ticker.read(cx).value(), form.value.clone())
        });
        assert_eq!(ticker.as_ref(), "KO");
        assert_eq!(
            value.read_with(cx, |value, _| value.value()).as_ref(),
            "100"
        );

        ui.type_into(&value, "200", cx);
        ui.click("dialog-submit", cx);
        ui.wait_loaded(2, cx);

        assert!(ui.read(cx, |page, _| page.editor.is_none()));
        assert_eq!(
            stored(&ui, cx),
            [("aapl".to_owned(), dec!(50)), ("ko".to_owned(), dec!(200))]
        );
        assert_eq!(totals(&ui, cx)[0].1, "1\u{202f}000.00 PLN");
        assert!(
            ui.events()
                .iter()
                .any(|event| matches!(event, Event::YearsChanged))
        );
    }

    #[gpui_kit::test]
    fn selected_records_are_deleted_after_confirming(cx: &mut TestAppContext) {
        let ui = open_2025(cx);
        ui.click("select-ko", cx);
        assert_eq!(ui.find("select-ko", cx).unwrap().checked(), Some(true));

        ui.click("delete-selected", cx);
        assert_eq!(
            stored(&ui, cx).len(),
            2,
            "nothing is deleted before confirming"
        );
        ui.click("dialog-confirm", cx);
        ui.wait_loaded(2, cx);

        assert_eq!(stored(&ui, cx), [("aapl".to_owned(), dec!(50))]);
        assert!(ui.read(cx, |page, _| page.table.selected.is_empty()));
        assert!(ui.find("select-ko", cx).is_none());
    }

    #[gpui_kit::test]
    fn filter_narrows_the_table(cx: &mut TestAppContext) {
        let ui = open_2025(cx);
        let filter = ui.read(cx, |page, _| page.filter.clone());

        ui.type_into(&filter, "aapl", cx);

        assert_eq!(ui.read(cx, |page, _| page.table.shown()), 1);
        assert!(ui.find("select-ko", cx).is_none());
        assert!(ui.find("select-aapl", cx).is_some());
    }

    #[gpui_kit::test]
    fn missing_rates_offer_to_download_the_year(cx: &mut TestAppContext) {
        let ui = open(
            cx,
            Some(2024..=2024),
            vec![paid("ko", on(2025, 5, 15), REVOLUT, dec!(100))],
            Some(2025),
        );
        assert!(ui.read(cx, |page, _| page.missing_rate.is_some()));
        assert_eq!(ui.read(cx, |page, _| page.missing_rate_year), Some(2025));

        ui.click("missing-import", cx);

        let year = ui.read(cx, |page, cx| {
            page.nbp_year
                .as_ref()
                .map(|input| input.read(cx).value().to_string())
        });
        assert_eq!(year.as_deref(), Some("2025"));
    }

    #[gpui_kit::test]
    fn empty_year_links_to_the_nearest_year_with_records(cx: &mut TestAppContext) {
        let ui = open(
            cx,
            Some(2024..=2024),
            vec![paid("ko", on(2024, 5, 15), REVOLUT, dec!(100))],
            Some(2025),
        );
        assert_eq!(ui.read(cx, |page, _| page.other_year), Some((2024, 1)));

        ui.click("other-year", cx);
        ui.click("empty-import", cx);

        let events = ui.events();
        assert!(matches!(
            events[..],
            [
                Event::SelectYear(Some(2024)),
                Event::Navigate(Page::Imports)
            ]
        ));
    }
}
