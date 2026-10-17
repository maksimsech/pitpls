use super::exchange::{self, Catalog, PickerState};
use crate::{
    components::{Status, dialog, form},
    i18n::plural,
    services::Services,
    theme::palette,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{dialog::Dialog, *},
    *,
};
use pitpls_app::use_case::stock::{self, SetExchangeInput};
use pitpls_core::common::Mic;
use rust_i18n::t;

#[derive(Clone)]
pub struct ExchangeRow {
    pub ticker: SharedString,
    pub provider: SharedString,
    pub detail: SharedString,
    pub exchange: Option<Mic>,
}

// A save closes the dialog too, so it's the only event the host gets then.
pub enum ExchangeDialogEvent {
    Saved(SharedString),
    Closed,
}

pub struct ExchangeDialog {
    services: Services,
    title: SharedString,
    description: &'static str,
    rows: Vec<(ExchangeRow, Entity<PickerState>)>,
    catalog: Entity<Catalog>,
    filled: Option<usize>,
    status: Status,
    _subscription: Subscription,
}

impl EventEmitter<ExchangeDialogEvent> for ExchangeDialog {}

impl ExchangeDialog {
    pub fn open(
        services: Services,
        catalog: Entity<Catalog>,
        title: impl Into<SharedString>,
        description: &'static str,
        rows: Vec<ExchangeRow>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let view = cx.new(|cx| {
            let rows = rows
                .into_iter()
                .map(|row| (row, exchange::picker_state(window, cx)))
                .collect();
            let subscription = cx.observe_in(&catalog, window, |this: &mut Self, _, window, cx| {
                this.fill(window, cx)
            });
            Self {
                services,
                title: title.into(),
                description,
                rows,
                catalog,
                filled: None,
                status: Status::default(),
                _subscription: subscription,
            }
        });
        view.update(cx, |this, cx| {
            this.fill(window, cx);
            dialog::open(window, cx, Self::render_dialog);
            if let Some((_, picker)) = this.rows.first() {
                picker.update(cx, |picker, cx| picker.focus(window, cx));
            }
            this.catalog
                .update(cx, |catalog, cx| catalog.load(window, cx));
        });
        view
    }

    fn fill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let catalog = self.catalog.read(cx);
        let (empty, revision) = (catalog.is_empty(), catalog.revision);
        if !empty && self.filled != Some(revision) {
            let first = self.filled.is_none();
            self.filled = Some(revision);
            for (row, picker) in &self.rows {
                exchange::fill(picker, &self.catalog, window, cx);
                if first {
                    exchange::select(picker, row.exchange, window, cx);
                }
            }
        }
        cx.notify();
    }

    // Only rows whose exchange changed are saved; one left empty keeps what the instrument has.
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let inputs = self
            .rows
            .iter()
            .filter_map(|(row, picker)| {
                let picked = exchange::picked(picker, cx);
                (picked != row.exchange).then(|| SetExchangeInput {
                    ticker: row.ticker.to_string(),
                    provider: row.provider.to_string(),
                    exchange: picked,
                })
            })
            .collect::<Vec<_>>();
        if inputs.is_empty() {
            window.close_dialog(cx);
            self.close(cx);
            return;
        }
        let message = plural("exchange.saved", inputs.len() as u64);
        self.status.begin_save();
        self.status.task = Some(self.services.run(
            window,
            cx,
            move |app| async move {
                stock::set_exchanges(&app, inputs).await?;
                Ok(message.into_owned())
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    let message = this
                        .status
                        .message
                        .take()
                        .map_or_else(SharedString::default, |message| message.text());
                    window.close_dialog(cx);
                    cx.emit(ExchangeDialogEvent::Saved(message));
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        cx.emit(ExchangeDialogEvent::Closed);
    }

    fn render_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let busy = self.status.busy;
        let p = *palette(cx);
        let refresh_error = self.catalog.read(cx).refresh_error.clone();
        let rows = self.rows.iter().map(|(row, picker)| {
            h_flex()
                .gap(px(16.))
                .py(px(12.))
                .border_t_1()
                .border_color(p.line)
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(div().font_semibold().child(row.ticker.clone()))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(row.detail.clone()),
                        ),
                )
                .child(
                    div().w(px(250.)).h_8().flex_shrink_0().child(
                        exchange::picker(
                            picker,
                            &self.catalog,
                            t!("exchange.for", ticker = row.ticker, provider = row.provider),
                            busy,
                            cx,
                        )
                        .cleanable(true),
                    ),
                )
        });
        let body = v_flex()
            .child(
                div()
                    .pb(px(14.))
                    .text_color(p.muted)
                    .child(t!(self.description)),
            )
            .children(rows)
            .when_some(refresh_error, |body, error| {
                body.child(form::hint(error, cx).text_color(p.warning))
            })
            .when_some(self.status.error.as_ref(), |body, error| {
                body.child(form::error(error.text(), cx).mt(px(8.)))
            });
        dialog::editor(
            self,
            dialog,
            body,
            if busy {
                t!("common.saving")
            } else {
                t!("common.save")
            },
            |this| &this.status,
            Self::save,
            Self::close,
            cx,
        )
        .title(div().text_size(px(15.)).child(self.title.clone()))
        .w(px(620.))
        .bg(p.popover)
        .border_color(p.strong_line)
        .px(px(20.))
        .pb(px(14.))
    }
}
