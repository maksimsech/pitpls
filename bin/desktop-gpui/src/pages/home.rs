use super::PageView;
use crate::{
    components::{self, Status, SummaryGroup},
    format::pln,
    navigation::{Page, PageContext},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{alert::Alert, button::*, *},
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
        if self.status.loading {
            return;
        }
        self.status.begin_load(window, cx, |this| &mut this.status);
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
        v_flex()
            .relative()
            .size_full()
            .min_h_0()
            .child(components::scroll(
                components::page_content()
                    .gap_4()
                    .when(self.status.is_visible(), |view| {
                        view.child(self.status.render())
                    })
                    .when(self.warning, |view| {
                        view.child(Alert::warning(
                            "page-notice",
                            "No rates loaded for this period. Open Rates to import exchange rates.",
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
                    .when(self.status.ready, |view| {
                        view.child(components::summaries(&self.summaries, cx))
                    })
                    .when(!self.status.ready && self.status.loading, |view| {
                        view.child(
                            v_flex()
                                .gap_5()
                                .child(components::summary_skeleton(
                                    "Crypto",
                                    &["Income (E-36)", "Costs (E-37)"],
                                    self.status.loading_visible,
                                    cx,
                                ))
                                .child(components::summary_skeleton(
                                    "Foreign dividends and interest",
                                    &["Income (I-65)", "Tax to pay (G-47)", "Paid tax (G-48)"],
                                    self.status.loading_visible,
                                    cx,
                                )),
                        )
                    }),
            ))
            .child(self.status.refreshing(cx))
    }
}
