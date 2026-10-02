use super::PageView;
use crate::{
    components::{self, Status, SummaryGroup},
    format::pln,
    navigation::{Page, PageContext},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{button::*, *},
    *,
};
use pitpls_app::use_case::{tax, warnings};

pub struct HomePage {
    context: PageContext,
    year: Option<i32>,
    status: Status,
    summaries: Vec<SummaryGroup>,
    warning: bool,
}

impl HomePage {
    pub fn new(
        context: PageContext,
        year: Option<i32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut page = Self {
            context,
            year,
            status: Status::default(),
            summaries: vec![],
            warning: false,
        };
        page.refresh(window, cx);
        page
    }
}

impl PageView for HomePage {
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status.begin_load();
        self.warning = false;
        let year = self.year;
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let warnings = warnings::get_warnings(&app, year).await?;
                let summary = tax::load_tax_summary(&app, year).await;
                Ok((
                    warnings.rates_empty && warnings.has_records_in_year,
                    summary,
                ))
            },
            |this, result, _, cx| {
                if let Some((warning, summary)) = this.status.loaded(result) {
                    this.warning = warning;
                    if let Some(summary) = this.status.loaded(summary) {
                        this.summaries = vec![
                            SummaryGroup {
                                title: "Crypto",
                                values: vec![
                                    ("Income (E-36)", pln(summary.crypto.income)),
                                    ("Costs (E-37)", pln(summary.crypto.costs)),
                                ],
                            },
                            SummaryGroup {
                                title: "Foreign dividends and interest",
                                values: vec![
                                    ("Income (I-65)", pln(summary.foreign.income)),
                                    ("Tax to pay (G-47)", pln(summary.foreign.tax_to_pay)),
                                    ("Paid tax (G-48)", pln(summary.foreign.tax_paid)),
                                ],
                            },
                        ];
                    }
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for HomePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        components::scroll(
            v_flex()
                .gap_4()
                .p_5()
                .child(self.status.render(cx))
                .when(self.warning, |view| {
                    view.child(components::notice(
                        "No rates loaded for this period. Open Rates to import exchange rates.",
                        cx,
                    ))
                })
                .when(self.warning || self.status.error.is_some(), |view| {
                    view.child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("open-rates")
                                    .label("Open rates")
                                    .outline()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.context.navigate(Page::Rates, cx)
                                    })),
                            )
                            .child(Button::new("retry").label("Retry").outline().on_click(
                                cx.listener(|this, _, window, cx| this.refresh(window, cx)),
                            )),
                    )
                })
                .when(
                    !self.status.loading && self.status.error.is_none(),
                    |view| view.child(components::summaries(&self.summaries, cx)),
                ),
        )
    }
}
