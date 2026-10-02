use super::records::{
    RecordData, RecordDefinition, RecordRow, RecordsPage, edit_base, record_form,
};
use crate::{
    components::{SummaryGroup, form::Form, table::Column},
    format::{amount, exact_pln, pln},
    navigation::Page,
};
use gpui_kit::{App, Window};
use pitpls_app::use_case::dividend;
use serde_json::{Value, json};

pub type DividendsPage = RecordsPage<Dividends>;

pub struct Dividends;
impl RecordDefinition for Dividends {
    const PAGE: Page = Page::Dividends;

    async fn load(app: &pitpls_app::App, year: Option<i32>) -> Result<RecordData, String> {
        let mut out = RecordData::default();
        let data = dividend::load_dividends(app, year).await?;
        out.columns = vec![
            Column::text("Date", 120.),
            Column::text("Ticker", 95.),
            Column::text("Provider", 140.),
            Column::number("Value", 155.),
            Column::number("Tax paid", 155.),
            Column::text("Country", 85.),
        ];
        out.summaries.push(SummaryGroup {
            title: "Dividend totals",
            values: vec![
                ("Income", pln(data.income)),
                ("Calculated tax", pln(data.to_pay)),
                ("Creditable foreign tax", pln(data.paid)),
            ],
        });
        for record in data.calculated {
            let mut edit = edit_base(&record.id, record.date, record.value, &record.provider);
            edit["ticker"] = json!(record.ticker);
            edit["tax_paid"] = json!(record.tax_paid.value.to_string());
            edit["tax_paid_currency"] = json!(record.tax_paid.currency);
            edit["country"] = json!(record.country);
            out.rows.push(RecordRow {
                cells: vec![
                    record.date.to_string().into(),
                    record.ticker.into(),
                    record.provider.into(),
                    amount(record.value),
                    amount(record.tax_paid),
                    record.country.to_string().into(),
                ],
                details: vec![
                    ("Record ID".into(), record.id.clone().into()),
                    ("NBP date".into(), record.nbp_date.to_string().into()),
                    ("Original value".into(), amount(record.value)),
                    ("Original tax paid".into(), amount(record.tax_paid)),
                    ("Value in PLN".into(), exact_pln(record.calculated_value)),
                    ("Calculated tax".into(), exact_pln(record.calculated_to_pay)),
                    (
                        "Foreign tax paid in PLN".into(),
                        exact_pln(record.calculated_tax_paid),
                    ),
                    ("Maximum credit".into(), exact_pln(record.max_tax_paid)),
                    (
                        "Creditable foreign tax".into(),
                        exact_pln(record.used_tax_paid),
                    ),
                ],
                id: record.id,
                edit,
            });
        }

        Ok(out)
    }

    fn form(existing: Option<Value>, window: &mut Window, cx: &mut App) -> Form {
        let (mut form, values) = record_form(
            "dividend",
            existing,
            json!({"value": "", "value_currency": "USD", "tax_paid": "0", "tax_paid_currency": "USD", "country": "US"}),
            window,
            cx,
        );
        form.input("ticker", "Ticker", &values, false, window, cx);
        form.input("value", "Value", &values, false, window, cx);
        form.currency("value_currency", "Value currency", &values, window, cx);
        form.input("tax_paid", "Tax paid", &values, false, window, cx);
        form.currency(
            "tax_paid_currency",
            "Tax paid currency",
            &values,
            window,
            cx,
        );
        form.input(
            "country",
            "Country code (two letters)",
            &values,
            false,
            window,
            cx,
        );

        form.input("provider", "Provider", &values, false, window, cx);
        form
    }

    async fn save(
        app: &pitpls_app::App,
        editing: bool,
        mut values: Value,
    ) -> Result<String, String> {
        if let Some(country) = values["country"].as_str() {
            values["country"] = json!(country.to_uppercase());
        }
        if editing {
            dividend::update_dividend(
                app,
                serde_json::from_value(values).map_err(|e| e.to_string())?,
            )
            .await?;
        } else {
            dividend::create_dividend(
                app,
                serde_json::from_value(values).map_err(|e| e.to_string())?,
            )
            .await?;
        }
        Ok("Record saved.".into())
    }

    async fn delete(app: &pitpls_app::App, ids: Vec<String>) -> Result<String, String> {
        let count = dividend::delete_dividends(app, ids).await?;
        Ok(format!("Deleted {count} record(s)."))
    }
}
