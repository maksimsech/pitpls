use super::records::{
    RecordData, RecordDefinition, RecordRow, RecordsPage, edit_base, record_form,
};
use crate::components::form::Choice;
use crate::{
    components::{SummaryGroup, form::Form, table::Column},
    format::{amount, exact_pln, money, pln},
    navigation::Page,
};
use gpui_kit::{App, Window};
use pitpls_app::use_case::crypto;
use pitpls_core::crypto::Action;
use serde_json::{Value, json};

pub type CryptoPage = RecordsPage<Crypto>;

pub struct Crypto;
impl RecordDefinition for Crypto {
    const PAGE: Page = Page::Crypto;

    async fn load(app: &pitpls_app::App, year: Option<i32>) -> Result<RecordData, String> {
        let mut out = RecordData::default();
        let data = crypto::load_cryptos(app, year).await?;
        out.columns = vec![
            Column::text("Date", 120.),
            Column::text("Provider", 140.),
            Column::text("Action", 85.),
            Column::number("Value", 160.),
            Column::number("Fee", 145.),
            Column::number("Value (PLN)", 155.),
            Column::number("Fee (PLN)", 145.),
        ];
        out.summaries.push(SummaryGroup {
            title: "Crypto totals",
            values: vec![("Income", pln(data.income)), ("Costs", pln(data.costs))],
        });
        for record in data.calculated {
            let mut edit = edit_base(&record.id, record.date, record.value, &record.provider);
            edit["action"] = json!(record.action);
            edit["fee"] = json!(record.fee.value.to_string());
            edit["fee_currency"] = json!(record.fee.currency);
            out.rows.push(RecordRow {
                cells: vec![
                    record.date.to_string().into(),
                    record.provider.into(),
                    match record.action {
                        Action::FiatBuy => "Buy",
                        Action::FiatSell => "Sell",
                    }
                    .into(),
                    amount(record.value),
                    amount(record.fee),
                    money(record.calculated_value),
                    money(record.calculated_fee),
                ],
                details: vec![
                    ("Record ID".into(), record.id.clone().into()),
                    ("NBP date".into(), record.nbp_date.to_string().into()),
                    ("Original value".into(), amount(record.value)),
                    ("Original fee".into(), amount(record.fee)),
                    (
                        "Converted value (full precision)".into(),
                        exact_pln(record.calculated_value),
                    ),
                    (
                        "Converted fee (full precision)".into(),
                        exact_pln(record.calculated_fee),
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
            "crypto",
            existing,
            json!({"value": "", "value_currency": "USD", "fee": "0", "fee_currency": "USD", "action": "FiatBuy"}),
            window,
            cx,
        );
        form.choice(
            "action",
            "Action",
            vec![
                Choice::new("FiatBuy", "Buy"),
                Choice::new("FiatSell", "Sell"),
            ],
            &values,
            window,
            cx,
        );
        form.input("value", "Value", &values, false, window, cx);
        form.currency("value_currency", "Value currency", &values, window, cx);
        form.input("fee", "Fee", &values, false, window, cx);
        form.currency("fee_currency", "Fee currency", &values, window, cx);

        form.input("provider", "Provider", &values, false, window, cx);
        form
    }

    async fn save(app: &pitpls_app::App, editing: bool, values: Value) -> Result<String, String> {
        if editing {
            crypto::update_crypto(
                app,
                serde_json::from_value(values).map_err(|e| e.to_string())?,
            )
            .await?;
        } else {
            crypto::create_crypto(
                app,
                serde_json::from_value(values).map_err(|e| e.to_string())?,
            )
            .await?;
        }
        Ok("Record saved.".into())
    }

    async fn delete(app: &pitpls_app::App, ids: Vec<String>) -> Result<String, String> {
        let count = crypto::delete_cryptos(app, ids).await?;
        Ok(format!("Deleted {count} record(s)."))
    }
}
