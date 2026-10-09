use crate::{
    components::{self, ButtonText, Status, data, file_picker, header, nbp},
    navigation::{Page, PageContext},
    theme::{palette, tabular_digits},
};
use chrono::{Datelike, Local};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{alert::Alert, button::*, dialog::Dialog, input::InputState, *},
    *,
};
use pitpls_app::use_case::{
    import::{self, LastImport},
    rate::{self, RateCoverage},
};
use pitpls_importers::{IMPORTERS, InputType, OutputType, model::Importer};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Active {
    Statement(usize),
    Rates,
}

struct Data {
    coverage: Option<RateCoverage>,
    last_import: Option<LastImport>,
}

pub struct ImportsPage {
    context: PageContext,
    year: Option<i32>,
    status: Status,
    /// Apart from `status`, so a reload keeps the last import's outcome.
    load: Status,
    data: Option<Data>,
    active: Option<Active>,
    file_name: Option<SharedString>,
    nbp_year: Option<Entity<InputState>>,
    focus: FocusHandle,
}

impl ImportsPage {
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
            load: Status::default(),
            data: None,
            active: None,
            file_name: None,
            nbp_year: None,
            focus: cx.focus_handle(),
        };
        page.refresh(window, cx);
        page
    }

    fn locked(&self) -> bool {
        self.status.busy || self.nbp_year.is_some()
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(self.locked(), cx);
        cx.notify();
    }

    fn pick_file(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        self.active = Some(Active::Statement(index));
        self.file_name = None;
        self.status.begin_save();
        let extension = match IMPORTERS[index].input[0] {
            InputType::Csv => "csv",
            InputType::Pdf => "pdf",
        };
        self.status.task = Some(file_picker::pick(
            extension,
            window,
            cx,
            move |this, result, window, cx| {
                match result {
                    Ok(Some(file)) => this.import_statement(index, file, window, cx),
                    Ok(None) => {
                        this.status.busy = false;
                        this.active = None;
                    }
                    Err(error) => {
                        this.status.busy = false;
                        this.status.error = Some(error.into());
                    }
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn import_statement(
        &mut self,
        index: usize,
        file: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.file_name = Path::new(&file)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned().into());
        let kind = IMPORTERS[index].kind;
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let result = import::run_import(&app, kind, file).await?;
                let counts = IMPORTERS[index]
                    .output
                    .iter()
                    .map(|output| match output {
                        OutputType::Dividend => count(result.dividends, "dividend"),
                        OutputType::Crypto => count(result.cryptos, "crypto record"),
                        OutputType::Interest => count(result.interests, "interest record"),
                    })
                    .collect::<Vec<_>>()
                    .join(" and ");
                Ok(format!("Imported {counts}."))
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    this.refresh(window, cx);
                }
                // Importers commit each record type separately, so even a
                // failed import may have added reporting years.
                this.context.years_changed(cx);
                this.notify(cx);
            },
        ));
    }

    fn open_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        let year = self.year.unwrap_or_else(|| Local::now().year());
        self.nbp_year = Some(nbp::open(year, Self::nbp_dialog, window, cx));
        self.active = Some(Active::Rates);
        self.file_name = None;
        self.status.error = None;
        self.status.message = None;
        self.notify(cx);
    }

    fn close_nbp(&mut self, cx: &mut Context<Self>) {
        self.nbp_year = None;
        self.status.error = None;
        self.notify(cx);
    }

    fn import_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(input) = &self.nbp_year else {
            return;
        };
        match nbp::year(input, cx) {
            Ok(year) => {
                self.status.begin_save();
                self.status.task = Some(self.context.services.run(
                    window,
                    cx,
                    move |app| nbp::import(app, year),
                    |this, result, window, cx| {
                        if this.status.saved(result) {
                            window.close_dialog(cx);
                            this.close_nbp(cx);
                            window.focus(&this.focus, cx);
                            this.refresh(window, cx);
                        }
                        this.notify(cx);
                    },
                ));
                self.notify(cx);
            }
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
            }
        }
    }

    fn nbp_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(input) = &self.nbp_year else {
            return dialog;
        };
        nbp::dialog(
            self,
            dialog,
            input,
            |this| &this.status,
            Self::import_nbp,
            Self::close_nbp,
            cx,
        )
    }

    /// A running load starts over.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.load.begin_load(window, cx, |this| &mut this.load);
        self.load.task = Some(self.context.services.run(
            window,
            cx,
            |app| async move {
                Ok(Data {
                    coverage: rate::rate_coverage(&app).await?,
                    last_import: import::load_last_import(&app).await?,
                })
            },
            |this, result, _, cx| {
                if let Some(data) = this.load.loaded(result) {
                    this.data = Some(data);
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for ImportsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("imports-page")
            .track_focus(&self.focus)
            .size_full()
            .min_h_0()
            .text_size(px(13.))
            .child(header::page(Page::Imports.title(), None, window, cx))
            .child(components::scroll(
                div().w_full().px(px(20.)).child(
                    v_flex()
                        .w_full()
                        .max_w(px(720.))
                        .mx_auto()
                        .pt(px(6.))
                        .pb(px(28.))
                        .gap(px(14.))
                        .child(self.providers(cx))
                        .children(self.data_cards(cx)),
                ),
            ))
    }
}

impl ImportsPage {
    fn providers(&self, cx: &mut Context<Self>) -> Div {
        data::card(cx)
            .child(
                h_flex()
                    .h(px(42.))
                    .pl(px(16.))
                    .pr(px(10.))
                    .child(div().font_semibold().child("Providers")),
            )
            .children(
                IMPORTERS
                    .iter()
                    .enumerate()
                    .map(|(index, importer)| self.provider(index, importer, cx)),
            )
    }

    fn provider(&self, index: usize, importer: &Importer, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let active = self.active == Some(Active::Statement(index));
        let formats = importer
            .input
            .iter()
            .map(|value| match value {
                InputType::Csv => "CSV",
                InputType::Pdf => "PDF",
            })
            .collect::<Vec<_>>()
            .join(" / ");
        let outputs = importer
            .output
            .iter()
            .map(|value| match value {
                OutputType::Dividend => "Dividends",
                OutputType::Crypto => "Crypto",
                OutputType::Interest => "Interest",
            })
            .collect::<Vec<_>>()
            .join(" & ");
        let file = match &self.file_name {
            Some(file_name) if active => file_name.to_string(),
            _ => formats.clone(),
        };
        // The picker is open or the file is importing.
        let busy = active && self.status.busy;
        let label = if busy && self.file_name.is_some() {
            "Importing…".to_owned()
        } else if busy {
            "Choosing file…".to_owned()
        } else if active && self.status.error.is_some() {
            "Try another file".to_owned()
        } else {
            format!("Choose {formats}")
        };
        let button = header::task_button(("import-file", index), None, label, busy)
            .when(!busy, |button| {
                button.accessibility_label(format!("Choose {formats} file for {}", importer.name))
            })
            .disabled(self.locked())
            .on_click(cx.listener(move |this, _, window, cx| this.pick_file(index, window, cx)));
        v_flex()
            .border_t_1()
            .border_color(p.line)
            .child(
                h_flex()
                    .h(px(54.))
                    .gap(px(14.))
                    .pl(px(16.))
                    .pr(px(10.))
                    .child(
                        div()
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .justify_center()
                            .size(px(30.))
                            .rounded(px(8.))
                            .bg(p.selected)
                            .text_size(px(11.5))
                            .font_semibold()
                            .text_color(p.muted)
                            .child(initials(importer.name)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(1.))
                            .child(div().truncate().font_medium().child(importer.name))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("{file} · {outputs}")),
                            ),
                    )
                    .child(div().flex_shrink_0().child(button)),
            )
            .when(active, |row| {
                row.children(self.outcome("Couldn't import file", px(60.), cx))
            })
    }

    fn data_cards(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut cards = vec![];
        if let Some(error) = &self.load.error {
            cards.push(
                v_flex()
                    .gap_3()
                    .child(
                        Alert::error("imports-load-error", error.clone())
                            .title("Couldn't load the exchange rate and import status"),
                    )
                    .child(
                        h_flex().child(
                            Button::new("retry")
                                .text_label("Retry")
                                .outline()
                                .disabled(self.load.loading)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.refresh(window, cx)),
                                ),
                        ),
                    )
                    .into_any_element(),
            );
        }
        match &self.data {
            Some(data) => {
                cards.push(self.rates_card(data, cx).into_any_element());
                cards.push(self.import_card(data, cx).into_any_element());
            }
            None if self.load.loading_visible => cards.extend(
                ["Exchange rates", "Last import"]
                    .map(|title| status_row_skeleton(title, cx).into_any_element()),
            ),
            None => {}
        }
        cards
    }

    fn rates_card(&self, data: &Data, cx: &mut Context<Self>) -> Div {
        let rates = data::rate_status("NBP table A · latest rate", data.coverage, self.year, cx);
        let import = header::button("import-nbp", None, "Import from NBP")
            .primary()
            .disabled(self.locked())
            .on_click(cx.listener(|this, _, window, cx| this.open_nbp(window, cx)));
        // While the dialog is open, it shows the outcome itself.
        let outcome = if self.active == Some(Active::Rates) && self.nbp_year.is_none() {
            self.outcome("Couldn't import rates", px(35.), cx)
        } else {
            None
        };
        data::card(cx)
            .child(
                status_row(rates.dot, "Exchange rates", rates.detail, cx)
                    .pr(px(10.))
                    .child(div().flex_shrink_0().child(import)),
            )
            .children(outcome)
    }

    fn import_card(&self, data: &Data, cx: &App) -> Div {
        let p = palette(cx);
        let import = data::import_status(data.last_import.as_ref(), cx);
        data::card(cx).child(
            status_row(
                import.dot,
                "Last import",
                StyledText::new(import.detail),
                cx,
            )
            .pr(px(16.))
            .children(import.date.map(|date| {
                div()
                    .flex_shrink_0()
                    .text_color(p.faint)
                    .font_features(tabular_digits())
                    .child(date)
            })),
        )
    }

    fn outcome(&self, failure: &'static str, indent: Pixels, cx: &App) -> Option<Div> {
        let p = palette(cx);
        let band = h_flex()
            .items_start()
            .gap(px(8.))
            .pl(indent)
            .pr(px(16.))
            .pb(px(12.))
            .text_size(px(12.));
        if let Some(error) = self.status.error.clone() {
            Some(
                band.text_color(p.danger)
                    .child(Icon::new(IconName::CircleX).size(px(14.)).mt(px(1.)))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(2.))
                            .child(div().font_medium().child(failure))
                            .child(error),
                    ),
            )
        } else {
            self.status.message.clone().map(|message| {
                band.child(
                    Icon::new(IconName::Check)
                        .size(px(14.))
                        .mt(px(1.))
                        .text_color(p.ok),
                )
                .child(div().flex_1().min_w_0().text_color(p.muted).child(message))
            })
        }
    }
}

fn status_row(dot: Hsla, title: &'static str, detail: StyledText, cx: &App) -> Div {
    h_flex()
        .h(px(56.))
        .gap(px(12.))
        .pl(px(16.))
        .child(data::dot(dot))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(px(1.))
                .child(div().font_medium().child(title))
                .child(
                    div()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(palette(cx).muted)
                        .child(detail),
                ),
        )
}

fn status_row_skeleton(title: &'static str, cx: &App) -> Div {
    data::card(cx).child(
        h_flex()
            .h(px(56.))
            .gap(px(12.))
            .pl(px(16.))
            .child(data::dot(cx.theme().skeleton))
            .child(
                v_flex()
                    .gap(px(4.))
                    .child(div().font_medium().child(title))
                    .child(
                        div()
                            .h(px(12.))
                            .w(px(220.))
                            .rounded(px(4.))
                            .bg(cx.theme().skeleton),
                    ),
            ),
    )
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect()
}

fn count(count: u64, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}
