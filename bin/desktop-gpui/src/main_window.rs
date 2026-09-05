//! The main pitpls window.
//!
//! This module owns window-local navigation, asynchronous page state, and the
//! Crypto/Rates rendering tree. Process initialization and native app commands
//! intentionally stay in `main` and `menus`.

use std::{collections::HashSet, ops::Range, sync::Arc};

use chrono::{Datelike as _, Local};
use gpui::{
    AnyElement, App, AppContext, Context, Div, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, PathPromptOptions, Render, Styled as _, Task, TextAlign, Window, div, img,
    prelude::FluentBuilder as _, px, uniform_list,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    scroll::ScrollableElement as _,
    select::Select,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem, SidebarToggleButton},
    v_flex,
};
use pitpls_app::use_case::rate::{RateDay, RatesViewModel};
use pitpls_core::{
    common::Currency,
    crypto::{Action, CryptoTaxData},
};

use crate::{crypto_form::CryptoForm, services::Services};

const NBP_MIN_YEAR: i32 = 2002;
const COMPACT_WINDOW_WIDTH: f32 = 1050.;
const SPLIT_FORM_WINDOW_WIDTH: f32 = 1350.;
const CRYPTO_TABLE_MIN_WIDTH: f32 = 960.;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Crypto,
    Rates,
}

enum LoadState<T> {
    Loading,
    Ready(T),
    Error(String),
}

pub struct MainWindow {
    services: Option<Arc<Services>>,
    page: Page,
    sidebar_collapsed: bool,
    crypto: LoadState<CryptoTaxData>,
    rates: LoadState<RatesViewModel>,
    crypto_generation: u64,
    rates_generation: u64,
    crypto_task: Task<()>,
    rates_task: Task<()>,
    selected: HashSet<String>,
    inspected: Option<String>,
    form: Option<CryptoForm>,
    year_filter: Entity<InputState>,
    nbp_year: Entity<InputState>,
    busy: Option<&'static str>,
    error: Option<String>,
}

impl MainWindow {
    pub fn new(
        bootstrap: Result<Arc<Services>, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let startup_error = bootstrap.as_ref().err().cloned();
        let services = bootstrap.ok();
        let year_filter = cx.new(|cx| InputState::new(window, cx).placeholder("All years"));
        let nbp_year = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(Local::now().year().to_string())
                .placeholder("Year")
        });
        let mut this = Self {
            services,
            page: Page::Crypto,
            sidebar_collapsed: false,
            crypto: startup_error
                .clone()
                .map_or(LoadState::Loading, LoadState::Error),
            rates: startup_error.map_or(LoadState::Loading, LoadState::Error),
            crypto_generation: 0,
            rates_generation: 0,
            crypto_task: Task::ready(()),
            rates_task: Task::ready(()),
            selected: HashSet::new(),
            inspected: None,
            form: None,
            year_filter,
            nbp_year,
            busy: None,
            error: None,
        };
        if this.services.is_some() {
            this.start_load_crypto(cx);
        }
        this
    }

    fn start_load_crypto(&mut self, cx: &mut Context<Self>) {
        let Some(services) = self.services.clone() else {
            return;
        };
        let year = match self.selected_year(cx) {
            Ok(year) => year,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };

        self.error = None;
        self.selected.clear();
        self.crypto_generation += 1;
        let generation = self.crypto_generation;
        self.crypto = LoadState::Loading;
        let request = services.load_cryptos(year);
        self.crypto_task = cx.spawn(async move |this, cx| {
            let result = request
                .await
                .map_err(|error| format!("crypto service task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                if generation != this.crypto_generation {
                    return;
                }
                this.crypto = result.map_or_else(LoadState::Error, LoadState::Ready);
                cx.notify();
            })
            .ok();
        });
        cx.notify();
    }

    fn start_load_rates(&mut self, cx: &mut Context<Self>) {
        let Some(services) = self.services.clone() else {
            return;
        };
        self.error = None;
        self.rates_generation += 1;
        let generation = self.rates_generation;
        self.rates = LoadState::Loading;
        let request = services.list_rates();
        self.rates_task = cx.spawn(async move |this, cx| {
            let result = request
                .await
                .map_err(|error| format!("rates service task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                if generation != this.rates_generation {
                    return;
                }
                this.rates = result.map_or_else(LoadState::Error, LoadState::Ready);
                cx.notify();
            })
            .ok();
        });
        cx.notify();
    }

    fn selected_year(&self, cx: &App) -> Result<Option<i32>, String> {
        let value = self.year_filter.read(cx).value();
        let value = value.trim();
        if value.is_empty() {
            return Ok(None);
        }
        let year = value
            .parse::<i32>()
            .map_err(|_| "Year must be a number or blank for all years".to_string())?;
        if !(1900..=2100).contains(&year) {
            return Err("Year must be between 1900 and 2100".to_string());
        }
        Ok(Some(year))
    }

    fn switch_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        self.form = None;
        self.error = None;
        if page == Page::Rates {
            self.start_load_rates(cx);
        }
        cx.notify();
    }

    fn open_crypto_form(
        &mut self,
        id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let record = id.as_ref().and_then(|id| match &self.crypto {
            LoadState::Ready(data) => data.calculated.iter().find(|record| &record.id == id),
            _ => None,
        });
        self.form = Some(CryptoForm::new(record, window, cx));
        self.error = None;
        cx.notify();
    }

    fn submit_crypto(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let Some(services) = self.services.clone() else {
            return;
        };
        let Some(form) = self.form.as_ref() else {
            return;
        };

        let request = if form.editing_id.is_some() {
            match form.update_input(cx) {
                Ok(input) => services.update_crypto(input),
                Err(error) => {
                    self.error = Some(error);
                    cx.notify();
                    return;
                }
            }
        } else {
            match form.create_input(cx) {
                Ok(input) => services.create_crypto(input),
                Err(error) => {
                    self.error = Some(error);
                    cx.notify();
                    return;
                }
            }
        };

        self.busy = Some("Saving…");
        self.error = None;
        cx.spawn(async move |this, cx| {
            let result = request
                .await
                .map_err(|error| format!("crypto service task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok(()) => {
                        this.form = None;
                        this.start_load_crypto(cx);
                    }
                    Err(error) => {
                        this.error = Some(error);
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn delete_cryptos(&mut self, ids: Vec<String>, cx: &mut Context<Self>) {
        if self.busy.is_some() || ids.is_empty() {
            return;
        }
        let Some(services) = self.services.clone() else {
            return;
        };
        self.busy = Some("Deleting…");
        self.error = None;
        let request = services.delete_cryptos(ids);
        cx.spawn(async move |this, cx| {
            let result = request
                .await
                .map_err(|error| format!("crypto service task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok(()) => this.start_load_crypto(cx),
                    Err(error) => {
                        this.error = Some(error);
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn upload_csv(&mut self, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let Some(services) = self.services.clone() else {
            return;
        };
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose rates CSV".into()),
        });
        self.busy = Some("Choosing CSV…");
        self.error = None;
        cx.spawn(async move |this, cx| {
            let selected = picker
                .await
                .map_err(|_| "the file picker closed unexpectedly".to_string())
                .and_then(|result| result.map_err(|error| error.to_string()));
            let path = selected.and_then(|paths| {
                paths
                    .and_then(|mut paths| paths.pop())
                    .ok_or_else(|| "CSV selection cancelled".to_string())
            });
            let path = match path {
                Ok(path)
                    if path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("csv")) =>
                {
                    path
                }
                Ok(_) => {
                    this.update(cx, |this, cx| {
                        this.busy = None;
                        this.error = Some("Choose a file with the .csv extension".to_string());
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                Err(error) => {
                    this.update(cx, |this, cx| {
                        this.busy = None;
                        if error != "CSV selection cancelled" {
                            this.error = Some(error);
                        }
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };

            let result = services
                .import_rate_csv(path.to_string_lossy().into_owned())
                .await
                .map_err(|error| format!("rates service task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok(()) => this.start_load_rates(cx),
                    Err(error) => {
                        this.error = Some(error);
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn import_nbp(&mut self, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let Some(services) = self.services.clone() else {
            return;
        };
        let value = self.nbp_year.read(cx).value();
        let current_year = Local::now().year();
        let year = match value.trim().parse::<i32>() {
            Ok(year) if (NBP_MIN_YEAR..=current_year).contains(&year) => year,
            _ => {
                self.error = Some(format!(
                    "NBP year must be between {NBP_MIN_YEAR} and {current_year}"
                ));
                cx.notify();
                return;
            }
        };
        self.busy = Some("Importing from NBP…");
        self.error = None;
        let request = services.import_nbp(year);
        cx.spawn(async move |this, cx| {
            let result = request
                .await
                .map_err(|error| format!("rates service task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok(()) => this.start_load_rates(cx),
                    Err(error) => {
                        this.error = Some(error);
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn reset_rates(&mut self, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let Some(services) = self.services.clone() else {
            return;
        };
        self.busy = Some("Resetting…");
        self.error = None;
        let request = services.reset_rates();
        cx.spawn(async move |this, cx| {
            let result = request
                .await
                .map_err(|error| format!("rates service task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.busy = None;
                match result {
                    Ok(()) => this.start_load_rates(cx),
                    Err(error) => {
                        this.error = Some(error);
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn render_sidebar(&self, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        let collapsed = compact || self.sidebar_collapsed;
        let menu = SidebarMenu::new()
            .child(
                SidebarMenuItem::new("Crypto")
                    .icon(IconName::LayoutDashboard)
                    .active(self.page == Page::Crypto)
                    .on_click(cx.listener(|this, _, _, cx| this.switch_page(Page::Crypto, cx))),
            )
            .child(
                SidebarMenuItem::new("Rates")
                    .icon(IconName::ChartPie)
                    .active(self.page == Page::Rates)
                    .on_click(cx.listener(|this, _, _, cx| this.switch_page(Page::Rates, cx))),
            );

        Sidebar::left()
            .collapsed(collapsed)
            .header(
                h_flex()
                    .w_full()
                    .h(px(36.))
                    .gap_2()
                    .child(img("images/app-icon.png").size(px(32.)))
                    .when(!collapsed, |this| {
                        this.child(div().text_lg().font_semibold().child("pitpls"))
                    }),
            )
            .child(menu)
            .when(!compact, |this| {
                this.footer(
                    h_flex()
                        .w_full()
                        .when(!collapsed, |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Crypto + Rates"),
                            )
                        })
                        .child(div().flex_1())
                        .child(SidebarToggleButton::left().collapsed(collapsed).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.sidebar_collapsed = !this.sidebar_collapsed;
                                cx.notify();
                            }),
                        )),
                )
            })
            .into_any_element()
    }

    fn render_page_header(&self, cx: &mut Context<Self>) -> Div {
        h_flex()
            .h(px(56.))
            .flex_shrink_0()
            .px_5()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().text_lg().font_semibold().child(match self.page {
                Page::Crypto => "Crypto",
                Page::Rates => "Rates",
            }))
            .child(div().flex_1())
            .when_some(self.busy, |this, busy| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(busy),
                )
            })
    }

    fn render_error(&self, cx: &App) -> Option<Div> {
        self.error.as_ref().map(|error| {
            h_flex()
                .flex_shrink_0()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().danger)
                .bg(cx.theme().danger.opacity(0.08))
                .px_3()
                .py_2()
                .text_sm()
                .text_color(cx.theme().danger)
                .child(error.clone())
        })
    }

    fn render_crypto(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let compact = window.viewport_size().width < px(COMPACT_WINDOW_WIDTH);
        let full_page_form = window.viewport_size().width < px(SPLIT_FORM_WINDOW_WIDTH);
        if full_page_form && let Some(form) = self.form.as_ref() {
            return self.render_crypto_form(form, true, cx);
        }

        let content = match &self.crypto {
            LoadState::Loading => status_panel("Loading crypto transactions…", cx),
            LoadState::Error(error) => error_panel(
                "Couldn't load crypto transactions",
                error,
                Button::new("retry-crypto")
                    .label("Retry")
                    .on_click(cx.listener(|this, _, _, cx| this.start_load_crypto(cx))),
                cx,
            ),
            LoadState::Ready(data) => self.render_crypto_ready(data, compact, cx),
        };

        h_flex()
            .items_start()
            .size_full()
            .min_h_0()
            .gap_4()
            .child(content)
            .children(
                self.form
                    .as_ref()
                    .map(|form| self.render_crypto_form(form, false, cx)),
            )
            .into_any_element()
    }

    fn render_crypto_toolbar(
        &self,
        row_count: usize,
        all_selected: bool,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let busy = self.busy.is_some();
        let filters = h_flex()
            .gap_2()
            .child(
                Button::new("add-crypto")
                    .primary()
                    .label("Add new")
                    .disabled(busy)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_crypto_form(None, window, cx)),
                    ),
            )
            .child(div().w(px(130.)).child(Input::new(&self.year_filter)))
            .child(
                Button::new("apply-year")
                    .label("Apply year")
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| this.start_load_crypto(cx))),
            );
        let selection = h_flex()
            .gap_2()
            .child(
                Button::new("select-all")
                    .label(if all_selected {
                        "Clear selection"
                    } else {
                        "Select all"
                    })
                    .disabled(row_count == 0)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if all_selected {
                            this.selected.clear();
                        } else if let LoadState::Ready(data) = &this.crypto {
                            this.selected = data
                                .calculated
                                .iter()
                                .map(|record| record.id.clone())
                                .collect();
                        }
                        cx.notify();
                    })),
            )
            .child(
                Button::new("delete-selected")
                    .danger()
                    .label(format!("Delete selected ({})", self.selected.len()))
                    .disabled(busy || self.selected.is_empty())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.delete_cryptos(this.selected.iter().cloned().collect(), cx)
                    })),
            );

        if compact {
            v_flex()
                .w_full()
                .gap_2()
                .child(filters)
                .child(selection)
                .into_any_element()
        } else {
            h_flex()
                .w_full()
                .gap_2()
                .child(filters)
                .child(div().flex_1())
                .child(selection)
                .into_any_element()
        }
    }

    fn render_crypto_ready(
        &self,
        data: &CryptoTaxData,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let row_count = data.calculated.len();
        let all_selected = row_count > 0 && self.selected.len() == row_count;
        let inspected = self
            .inspected
            .as_ref()
            .and_then(|id| data.calculated.iter().find(|record| &record.id == id));

        v_flex()
            .h_full()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap_3()
            .child(self.render_crypto_toolbar(row_count, all_selected, compact, cx))
            .child(
                h_flex()
                    .gap_3()
                    .child(summary_card(
                        "Income (E-36)",
                        format_money(&data.income.to_string(), Currency::PLN),
                        cx,
                    ))
                    .child(summary_card(
                        "Costs (E-37)",
                        format_money(&data.costs.to_string(), Currency::PLN),
                        cx,
                    )),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .overflow_x_scrollbar()
                    .child(crypto_header(cx))
                    .child(if row_count == 0 {
                        status_panel(
                            "No crypto records yet. Add a transaction to get started.",
                            cx,
                        )
                    } else {
                        uniform_list(
                            "crypto-rows",
                            row_count,
                            cx.processor(|this, range: Range<usize>, _window, cx| {
                                let mut rows = Vec::with_capacity(range.len());
                                let LoadState::Ready(data) = &this.crypto else {
                                    return rows;
                                };
                                for index in range {
                                    let record = &data.calculated[index];
                                    let id = record.id.clone();
                                    let checked = this.selected.contains(&id);
                                    let inspected = this.inspected.as_ref() == Some(&id);
                                    let action = match record.action {
                                        Action::FiatBuy => "Buy",
                                        Action::FiatSell => "Sell",
                                    };
                                    rows.push(
                                        h_flex()
                                            .id(("crypto-row", index))
                                            .w_full()
                                            .min_w(px(CRYPTO_TABLE_MIN_WIDTH))
                                            .h(px(44.))
                                            .px_2()
                                            .gap_2()
                                            .border_b_1()
                                            .border_color(cx.theme().border)
                                            .when(checked || inspected, |row| {
                                                row.bg(cx.theme().muted)
                                            })
                                            .child(
                                                Checkbox::new(("crypto-check", index))
                                                    .checked(checked)
                                                    .on_click({
                                                        let id = id.clone();
                                                        cx.listener(move |this, checked, _, cx| {
                                                            if *checked {
                                                                this.selected.insert(id.clone());
                                                            } else {
                                                                this.selected.remove(&id);
                                                            }
                                                            cx.notify();
                                                        })
                                                    }),
                                            )
                                            .child(table_cell(short_id(&record.id), 90., false))
                                            .child(table_cell(record.date.to_string(), 100., false))
                                            .child(table_cell(action, 70., false))
                                            .child(table_cell(
                                                format_money(
                                                    &record.value.value.to_string(),
                                                    record.value.currency,
                                                ),
                                                140.,
                                                true,
                                            ))
                                            .child(table_cell(
                                                format_money(
                                                    &record.fee.value.to_string(),
                                                    record.fee.currency,
                                                ),
                                                120.,
                                                true,
                                            ))
                                            .child(table_cell(record.provider.clone(), 110., true))
                                            .child(
                                                h_flex()
                                                    .w(px(240.))
                                                    .flex_shrink_0()
                                                    .justify_end()
                                                    .gap_1()
                                                    .child(
                                                        Button::new(("inspect-crypto", index))
                                                            .small()
                                                            .label(if inspected {
                                                                "Hide details"
                                                            } else {
                                                                "Details"
                                                            })
                                                            .on_click({
                                                                let id = id.clone();
                                                                cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        if this.inspected.as_ref()
                                                                            == Some(&id)
                                                                        {
                                                                            this.inspected = None;
                                                                        } else {
                                                                            this.inspected =
                                                                                Some(id.clone());
                                                                        }
                                                                        cx.notify();
                                                                    },
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        Button::new(("edit-crypto", index))
                                                            .small()
                                                            .label("Edit")
                                                            .on_click({
                                                                let id = id.clone();
                                                                cx.listener(
                                                                    move |this, _, window, cx| {
                                                                        this.open_crypto_form(
                                                                            Some(id.clone()),
                                                                            window,
                                                                            cx,
                                                                        )
                                                                    },
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        Button::new(("delete-crypto", index))
                                                            .small()
                                                            .danger()
                                                            .label("Delete")
                                                            .disabled(this.busy.is_some())
                                                            .on_click(cx.listener(
                                                                move |this, _, _, cx| {
                                                                    this.delete_cryptos(
                                                                        vec![id.clone()],
                                                                        cx,
                                                                    )
                                                                },
                                                            )),
                                                    ),
                                            ),
                                    );
                                }
                                rows
                            }),
                        )
                        .w_full()
                        .min_w(px(CRYPTO_TABLE_MIN_WIDTH))
                        .h_full()
                        .into_any_element()
                    })
                    .children(inspected.map(|record| {
                        h_flex()
                            .w_full()
                            .min_w(px(CRYPTO_TABLE_MIN_WIDTH))
                            .flex_shrink_0()
                            .gap_6()
                            .px_4()
                            .py_3()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().muted.opacity(0.45))
                            .child(detail_value(
                                "Calculated value",
                                format_money(&record.calculated_value.to_string(), Currency::PLN),
                                cx,
                            ))
                            .child(detail_value(
                                "Calculated fee",
                                format_money(&record.calculated_fee.to_string(), Currency::PLN),
                                cx,
                            ))
                            .child(detail_value("NBP date", record.nbp_date.to_string(), cx))
                    })),
            )
            .into_any_element()
    }

    fn render_crypto_form(
        &self,
        form: &CryptoForm,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editing = form.editing_id.is_some();
        let busy = self.busy.is_some();
        v_flex()
            .w(px(370.))
            .h_full()
            .when(compact, |this| this.w_full())
            .flex_shrink_0()
            .gap_3()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .p_4()
            .overflow_y_scrollbar()
            .child(div().text_lg().font_semibold().child(if editing {
                "Edit crypto"
            } else {
                "Add crypto"
            }))
            .child(labeled_input("ID", &form.id, editing, cx))
            .child(labeled_input("Date", &form.date, false, cx))
            .child(labeled_select("Action", &form.action, cx))
            .child(
                h_flex()
                    .gap_2()
                    .child(labeled_input("Value", &form.value, false, cx))
                    .child(labeled_select("Currency", &form.value_currency, cx).w(px(120.))),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(labeled_input("Fee", &form.fee, false, cx))
                    .child(labeled_select("Currency", &form.fee_currency, cx).w(px(120.))),
            )
            .child(labeled_input("Provider", &form.provider, false, cx))
            .child(div().flex_1())
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-crypto-form")
                            .label("Cancel")
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.form = None;
                                this.error = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("save-crypto-form")
                            .primary()
                            .label(if editing { "Save" } else { "Add" })
                            .loading(busy)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.submit_crypto(window, cx)),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_rates(&self, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.busy.is_some();
        let empty = matches!(&self.rates, LoadState::Ready(data) if data.rows.is_empty());
        let content = match &self.rates {
            LoadState::Loading => status_panel("Loading exchange rates…", cx),
            LoadState::Error(error) => error_panel(
                "Couldn't load exchange rates",
                error,
                Button::new("retry-rates")
                    .label("Retry")
                    .on_click(cx.listener(|this, _, _, cx| this.start_load_rates(cx))),
                cx,
            ),
            LoadState::Ready(data) => self.render_rates_ready(data, cx),
        };
        let data_actions = h_flex()
            .gap_2()
            .child(
                Button::new("reset-rates")
                    .danger()
                    .label("Reset")
                    .disabled(busy || empty)
                    .on_click(cx.listener(|this, _, _, cx| this.reset_rates(cx))),
            )
            .child(
                Button::new("upload-rates")
                    .primary()
                    .label("Upload CSV")
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| this.upload_csv(cx))),
            );
        let import_actions = h_flex()
            .gap_2()
            .child(div().w(px(110.)).child(Input::new(&self.nbp_year)))
            .child(
                Button::new("import-nbp")
                    .label("Import from NBP")
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| this.import_nbp(cx))),
            );
        let toolbar = if compact {
            v_flex()
                .w_full()
                .items_end()
                .gap_2()
                .child(data_actions)
                .child(import_actions)
                .into_any_element()
        } else {
            h_flex()
                .w_full()
                .justify_end()
                .gap_2()
                .child(data_actions)
                .child(import_actions)
                .into_any_element()
        };

        v_flex()
            .size_full()
            .min_h_0()
            .gap_3()
            .child(toolbar)
            .child(content)
            .into_any_element()
    }

    fn render_rates_ready(&self, data: &RatesViewModel, cx: &mut Context<Self>) -> AnyElement {
        if data.rows.is_empty() {
            return status_panel("No rates yet. Upload a CSV or import from NBP.", cx);
        }
        let width = 130. + data.currencies.len() as f32 * 120.;
        let currencies = data.currencies.clone();
        v_flex()
            .flex_1()
            .min_h_0()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .overflow_x_scrollbar()
            .child(
                v_flex()
                    .w_full()
                    .min_w(px(width))
                    .h_full()
                    .child(rates_header(&currencies, cx))
                    .child(
                        uniform_list(
                            "rates-rows",
                            data.rows.len(),
                            cx.processor(move |this, range: Range<usize>, _window, cx| {
                                let mut rows = Vec::with_capacity(range.len());
                                let LoadState::Ready(data) = &this.rates else {
                                    return rows;
                                };
                                for index in range {
                                    let day = &data.rows[index];
                                    let mut row = h_flex()
                                        .id(("rate-row", index))
                                        .h(px(42.))
                                        .border_b_1()
                                        .border_color(cx.theme().border)
                                        .child(rate_cell(day.date.clone(), 130.));
                                    for currency in &currencies {
                                        row = row.child(rate_cell(
                                            rate_for(day, *currency)
                                                .unwrap_or_else(|| "—".to_string()),
                                            120.,
                                        ));
                                    }
                                    rows.push(row);
                                }
                                rows
                            }),
                        )
                        .h_full(),
                    ),
            )
            .into_any_element()
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = window.viewport_size().width < px(COMPACT_WINDOW_WIDTH);

        h_flex()
            .size_full()
            .items_start()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_sidebar(compact, cx))
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .min_h_0()
                    .child(self.render_page_header(cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .gap_3()
                            .p_4()
                            .when(compact, |this| this.p_3())
                            .children(self.render_error(cx))
                            .child(match self.page {
                                Page::Crypto => self.render_crypto(window, cx),
                                Page::Rates => self.render_rates(compact, cx),
                            }),
                    ),
            )
    }
}

fn status_panel(message: impl Into<String>, cx: &App) -> AnyElement {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(message.into())
        .into_any_element()
}

fn error_panel(title: &str, error: &str, action: Button, cx: &App) -> AnyElement {
    v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .child(div().font_semibold().child(title.to_string()))
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().danger)
                .child(error.to_string()),
        )
        .child(action)
        .into_any_element()
}

fn summary_card(label: &str, value: String, cx: &App) -> Div {
    v_flex()
        .flex_1()
        .gap_1()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border)
        .p_3()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(div().text_lg().font_semibold().child(value))
}

fn detail_value(label: &str, value: String, cx: &App) -> Div {
    v_flex()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(div().text_sm().font_medium().child(value))
}

fn crypto_header(cx: &App) -> Div {
    h_flex()
        .w_full()
        .min_w(px(CRYPTO_TABLE_MIN_WIDTH))
        .h(px(38.))
        .flex_shrink_0()
        .px_2()
        .gap_2()
        .bg(cx.theme().muted)
        .border_b_1()
        .border_color(cx.theme().border)
        .text_xs()
        .font_semibold()
        .child(div().w(px(20.)))
        .child(table_cell("ID", 90., false))
        .child(table_cell("Date", 100., false))
        .child(table_cell("Action", 70., false))
        .child(table_cell("Value", 140., true))
        .child(table_cell("Fee", 120., true))
        .child(table_cell("Provider", 110., true))
        .child(
            div()
                .w(px(240.))
                .flex_shrink_0()
                .text_align(TextAlign::Right)
                .child("Actions"),
        )
}

fn table_cell(value: impl Into<String>, width: f32, right: bool) -> Div {
    div()
        .w(px(width))
        .flex_shrink_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .when(right, |cell| cell.text_align(TextAlign::Right))
        .child(value.into())
}

fn labeled_input(label: &str, input: &Entity<InputState>, disabled: bool, cx: &App) -> Div {
    v_flex()
        .flex_1()
        .gap_1()
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(Input::new(input).disabled(disabled))
}

fn labeled_select(label: &str, select: &Entity<crate::crypto_form::ChoiceState>, cx: &App) -> Div {
    v_flex()
        .flex_1()
        .gap_1()
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(Select::new(select))
}

fn rates_header(currencies: &[Currency], cx: &App) -> Div {
    let mut row = h_flex()
        .h(px(38.))
        .flex_shrink_0()
        .bg(cx.theme().muted)
        .border_b_1()
        .border_color(cx.theme().border)
        .text_xs()
        .font_semibold()
        .child(rate_cell("Date", 130.));
    for currency in currencies {
        row = row.child(rate_cell(currency.as_str(), 120.));
    }
    row
}

fn rate_cell(value: impl Into<String>, width: f32) -> Div {
    div()
        .w(px(width))
        .flex_shrink_0()
        .px_3()
        .text_align(TextAlign::Right)
        .child(value.into())
}

fn rate_for(day: &RateDay, currency: Currency) -> Option<String> {
    day.rates
        .iter()
        .find(|rate| rate.currency == currency)
        .map(|rate| rate.rate.clone())
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn format_money(value: &str, currency: Currency) -> String {
    let value = trim_zeros(value);
    match currency {
        Currency::USD => format!("${value}"),
        Currency::EUR => format!("€{value}"),
        Currency::GBP => format!("£{value}"),
        Currency::JPY => format!("¥{value}"),
        Currency::PLN => format!("{value} zł"),
        Currency::CHF => format!("CHF {value}"),
        Currency::CAD => format!("C${value}"),
        Currency::AUD => format!("A${value}"),
        currency => format!("{value} {}", currency.as_str()),
    }
}

fn trim_zeros(value: &str) -> String {
    if !value.contains('.') {
        return value.to_string();
    }
    value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
