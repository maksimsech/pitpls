use super::records::{
    RecordData, RecordDefinition, RecordRow, RecordsPage, edit_base, record_form,
};
use crate::{
    components::{SummaryGroup, form::Form, table::Column},
    format::{amount, exact_pln, money, pln},
    navigation::Page,
};
use gpui_kit::{App, Window};
use pitpls_app::use_case::interest;
use serde_json::{Value, json};

pub type InterestsPage = RecordsPage<Interests>;

pub struct Interests;
impl RecordDefinition for Interests {
    const PAGE: Page = Page::Interests;

    async fn load(app: &pitpls_app::App, year: Option<i32>) -> Result<RecordData, String> {
        let mut out = RecordData::default();
        let data = interest::load_interests(app, year).await?;
        out.columns = vec![
            Column::text("Date", 120.),
            Column::text("Provider", 160.),
            Column::number("Value", 160.),
            Column::number("Value (PLN)", 160.),
            Column::number("Calculated tax (PLN)", 185.),
        ];
        out.summaries.push(SummaryGroup {
            title: "Interest totals",
            values: vec![
                ("Income", pln(data.income)),
                ("Calculated tax", pln(data.to_pay)),
            ],
        });
        for record in data.calculated {
            let edit = edit_base(&record.id, record.date, record.value, &record.provider);
            out.rows.push(RecordRow {
                cells: vec![
                    record.date.to_string().into(),
                    record.provider.into(),
                    amount(record.value),
                    money(record.calculated_value),
                    money(record.to_pay),
                ],
                details: vec![
                    ("Record ID".into(), record.id.clone().into()),
                    ("NBP date".into(), record.nbp_date.to_string().into()),
                    ("Original value".into(), amount(record.value)),
                    (
                        "Converted value (full precision)".into(),
                        exact_pln(record.calculated_value),
                    ),
                    (
                        "Calculated tax (full precision)".into(),
                        exact_pln(record.to_pay),
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
            "interest",
            existing,
            json!({"value": "", "value_currency": "USD"}),
            window,
            cx,
        );
        form.input("value", "Value", &values, false, window, cx);
        form.currency("value_currency", "Value currency", &values, window, cx);

        form.input("provider", "Provider", &values, false, window, cx);
        form
    }

    async fn save(app: &pitpls_app::App, editing: bool, values: Value) -> Result<String, String> {
        if editing {
            interest::update_interest(
                app,
                serde_json::from_value(values).map_err(|e| e.to_string())?,
            )
            .await?;
        } else {
            interest::create_interest(
                app,
                serde_json::from_value(values).map_err(|e| e.to_string())?,
            )
            .await?;
        }
        Ok("Record saved.".into())
    }

    async fn delete(app: &pitpls_app::App, ids: Vec<String>) -> Result<String, String> {
        let count = interest::delete_interests(app, ids).await?;
        Ok(format!("Deleted {count} record(s)."))
    }
}
