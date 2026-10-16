use crate::{
    components::{ButtonText, spinner},
    format::{date, shares},
    i18n::plural_key,
    services::Services,
    theme::palette,
};
use chrono::NaiveDate;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{
        IndexPath,
        button::*,
        select::{SearchableVec, Select, SelectDelegate, SelectItem, SelectState},
        *,
    },
    *,
};
use pitpls_app::use_case::exchange as exchanges;
use pitpls_core::{common::Mic, exchange::Exchange};
use rust_decimal::Decimal;
use rust_i18n::t;
use std::{cell::Cell, rc::Rc};

#[derive(Clone)]
pub struct ExchangeChoice {
    mic: Mic,
    name: SharedString,
    detail: SharedString,
    picked: SharedString,
    search: String,
    faint: Hsla,
}

impl ExchangeChoice {
    fn new(exchange: &Exchange, faint: Hsla) -> Self {
        let picked = format!("· {} · {}", exchange.mic, exchange.country);
        let mut detail = format!("{} · {}", exchange.mic, exchange.country);
        if let Some(city) = exchange.display_city() {
            detail.push_str(" · ");
            detail.push_str(&city);
        }
        let search = [
            exchange.acronym.as_deref().unwrap_or_default(),
            &exchange.name,
            exchange.mic.as_str(),
            exchange.city.as_deref().unwrap_or_default(),
            &exchange.country.to_string(),
        ]
        .join("\n")
        .to_lowercase();
        Self {
            mic: exchange.mic,
            name: exchange.display_name().into(),
            detail: detail.into(),
            picked: picked.into(),
            search,
            faint,
        }
    }
}

impl SelectItem for ExchangeChoice {
    type Value = Mic;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(
            h_flex()
                .min_w_0()
                .gap(px(4.))
                .child(self.name.clone())
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(self.faint)
                        .child(self.picked.clone()),
                )
                .into_any_element(),
        )
    }

    fn value(&self) -> &Mic {
        &self.mic
    }

    fn matches(&self, query: &str) -> bool {
        self.search.contains(&query.trim().to_lowercase())
    }
}

pub enum CatalogStatus {
    Loading,
    Downloading,
    Failed(SharedString),
    Ready,
}

pub type RefreshFn = Rc<dyn Fn(&mut Window, &mut App)>;

// The kit's select has no footer, so the list's header carries the download date and Refresh.
pub struct ExchangeList {
    items: SearchableVec<ExchangeChoice>,
    total: usize,
    downloaded: Option<NaiveDate>,
    refreshing: Rc<Cell<bool>>,
    refresh: Option<RefreshFn>,
}

impl ExchangeList {
    fn empty() -> Self {
        Self {
            items: SearchableVec::new(Vec::new()),
            total: 0,
            downloaded: None,
            refreshing: Rc::default(),
            refresh: None,
        }
    }
}

impl SelectDelegate for ExchangeList {
    type Item = ExchangeChoice;

    fn items_count(&self, section: usize) -> usize {
        self.items.items_count(section)
    }

    fn item(&self, ix: IndexPath) -> Option<&ExchangeChoice> {
        self.items.item(ix)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq,
    {
        self.items.position(value)
    }

    fn perform_search(&mut self, query: &str, window: &mut Window, cx: &mut App) -> Task<()> {
        self.items.perform_search(query, window, cx)
    }

    fn render_item(
        &self,
        _: IndexPath,
        item: &ExchangeChoice,
        checked: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let p = *palette(cx);
        Some(
            h_flex()
                .flex_1()
                .min_w_0()
                .gap(px(8.))
                .child(div().w(px(14.)).flex_shrink_0().when(checked, |slot| {
                    slot.child(Icon::new(IconName::Check).size(px(14.)))
                }))
                .child(div().flex_shrink_0().child(item.name.clone()))
                .child(div().flex_1())
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(item.detail.clone()),
                )
                .into_any_element(),
        )
    }

    fn render_section_header(&self, _: usize, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let p = *palette(cx);
        let shown = self.items.items_count(0);
        let source = match self.downloaded {
            Some(day) => t!("exchange.source_downloaded", date = date(day).main),
            None => t!("exchange.source"),
        };
        let refreshing = self.refreshing.get();
        let refresh = self.refresh.clone();
        Some(
            v_flex()
                .px_2()
                .pt(px(6.))
                .pb(px(4.))
                .gap(px(2.))
                .text_size(px(11.5))
                .text_color(p.faint)
                .child(div().font_medium().child(t!(
                    plural_key("exchange.shown", self.total as u64),
                    shown = count(shown),
                    total = count(self.total)
                )))
                .child(
                    h_flex()
                        .gap(px(6.))
                        .child(source)
                        .when(refreshing, |line| line.child(spinner()))
                        .when_some(refresh.filter(|_| !refreshing), |line, refresh| {
                            line.child(
                                Button::new("exchanges-refresh")
                                    .ghost()
                                    .h(px(18.))
                                    .px(px(4.))
                                    .text_color(p.muted)
                                    .accessibility_label(t!("exchange.refresh_label"))
                                    .child(div().text_size(px(11.5)).child(t!("common.refresh")))
                                    .on_click(move |_, window, cx| refresh(window, cx)),
                            )
                        }),
                )
                .into_any_element(),
        )
    }
}

fn count(value: usize) -> SharedString {
    shares(Decimal::from(value)).main
}

pub struct Catalog {
    services: Services,
    exchanges: Vec<Exchange>,
    downloaded: Option<NaiveDate>,
    pub status: CatalogStatus,
    refreshing: Rc<Cell<bool>>,
    pub refresh_error: Option<SharedString>,
    // Goes up whenever the list changes, so pickers know when to refill.
    pub revision: usize,
    running: bool,
    _task: Option<Task<()>>,
}

impl Catalog {
    pub fn new(services: Services) -> Self {
        Self {
            services,
            exchanges: Vec::new(),
            downloaded: None,
            status: CatalogStatus::Loading,
            refreshing: Rc::default(),
            refresh_error: None,
            revision: 0,
            running: false,
            _task: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.exchanges.is_empty()
    }

    pub fn contains(&self, mic: Mic) -> bool {
        self.exchanges.iter().any(|exchange| exchange.mic == mic)
    }

    fn set(&mut self, exchanges: Vec<Exchange>, downloaded: Option<NaiveDate>) {
        self.exchanges = exchanges;
        self.downloaded = downloaded;
        self.status = CatalogStatus::Ready;
        self.revision += 1;
    }

    // Reloads on every open, so exchanges an instrument started using since move to the top.
    // An empty table downloads the list first.
    pub fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        if self.is_empty() {
            self.status = CatalogStatus::Loading;
        }
        self.refresh_error = None;
        self.running = true;
        self._task = Some(self.services.run(
            window,
            cx,
            |app| async move {
                let list = exchanges::list_exchanges(&app).await?;
                let downloaded = exchanges::exchanges_downloaded(&app).await?;
                Ok((list, downloaded))
            },
            |this, result, window, cx| {
                this.running = false;
                match result {
                    Ok((list, _)) if list.is_empty() => this.download(window, cx),
                    Ok((list, downloaded)) => this.set(list, downloaded),
                    Err(error) => this.failed(error.into()),
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    // A failed refresh keeps the list it had.
    pub fn download(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        if self.is_empty() {
            self.status = CatalogStatus::Downloading;
        } else {
            self.refreshing.set(true);
        }
        self.refresh_error = None;
        self.running = true;
        self._task = Some(self.services.run(
            window,
            cx,
            |app| async move {
                exchanges::refresh_exchanges(&app).await?;
                let list = exchanges::list_exchanges(&app).await?;
                let downloaded = exchanges::exchanges_downloaded(&app).await?;
                Ok((list, downloaded))
            },
            |this, result, _, cx| {
                this.running = false;
                this.refreshing.set(false);
                match result {
                    Ok((list, downloaded)) => this.set(list, downloaded),
                    Err(error) => this.failed(error.into()),
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    fn failed(&mut self, error: SharedString) {
        if self.is_empty() {
            self.status = CatalogStatus::Failed(error);
        } else {
            self.refresh_error = Some(error);
        }
    }

    fn list(&self, refresh: Option<RefreshFn>, cx: &App) -> ExchangeList {
        let faint = palette(cx).faint;
        let choices = self
            .exchanges
            .iter()
            .map(|exchange| ExchangeChoice::new(exchange, faint))
            .collect::<Vec<_>>();
        ExchangeList {
            total: choices.len(),
            items: SearchableVec::new(choices),
            downloaded: self.downloaded,
            refreshing: self.refreshing.clone(),
            refresh,
        }
    }
}

fn catalog_action(
    catalog: &Entity<Catalog>,
    action: fn(&mut Catalog, &mut Window, &mut Context<Catalog>),
) -> RefreshFn {
    let catalog = catalog.downgrade();
    Rc::new(move |window, cx| {
        let _ = catalog.update(cx, |catalog, cx| action(catalog, window, cx));
    })
}

pub type PickerState = SelectState<ExchangeList>;

pub fn picker_state(window: &mut Window, cx: &mut App) -> Entity<PickerState> {
    cx.new(|cx| SelectState::new(ExchangeList::empty(), None, window, cx).searchable(true))
}

pub fn picked(state: &Entity<PickerState>, cx: &App) -> Option<Mic> {
    state.read(cx).selected_value().copied()
}

pub fn select(state: &Entity<PickerState>, mic: Option<Mic>, window: &mut Window, cx: &mut App) {
    if picked(state, cx) == mic {
        return;
    }
    state.update(cx, |state, cx| match mic {
        Some(mic) => state.set_selected_value(&mic, window, cx),
        None => state.set_selected_index(None, window, cx),
    });
}

// A MIC missing from the list, such as one an import stored before the list was downloaded,
// stays picked only while it's in the list.
pub fn fill(
    state: &Entity<PickerState>,
    catalog: &Entity<Catalog>,
    window: &mut Window,
    cx: &mut App,
) {
    let selected = picked(state, cx);
    let refresh = catalog_action(catalog, Catalog::download);
    let list = catalog.read(cx).list(Some(refresh), cx);
    state.update(cx, |state, cx| {
        state.set_items(list, window, cx);
        state.set_selected_index(None, window, cx);
        if let Some(mic) = selected {
            state.set_selected_value(&mic, window, cx);
        }
    });
}

pub fn picker(
    state: &Entity<PickerState>,
    catalog: &Entity<Catalog>,
    label: impl Into<SharedString>,
    disabled: bool,
    cx: &App,
) -> Select<ExchangeList> {
    let p = *palette(cx);
    let retry = catalog_action(catalog, Catalog::load);
    let catalog = catalog.read(cx);
    let empty: SharedString = match &catalog.status {
        CatalogStatus::Loading => t!("exchange.loading").into(),
        CatalogStatus::Downloading => t!("exchange.downloading").into(),
        CatalogStatus::Failed(error) => error.clone(),
        CatalogStatus::Ready if catalog.is_empty() => t!("exchange.list_empty").into(),
        CatalogStatus::Ready => t!("exchange.no_match").into(),
    };
    let busy = matches!(
        catalog.status,
        CatalogStatus::Loading | CatalogStatus::Downloading
    );
    let failed = matches!(catalog.status, CatalogStatus::Failed(_));
    Select::new(state)
        .accessibility_label(label)
        .placeholder(t!("exchange.placeholder"))
        .search_placeholder(t!("exchange.search_placeholder"))
        .menu_width(px(340.))
        .menu_max_h(px(320.))
        .disabled(disabled)
        .empty(move |_, _| {
            v_flex()
                .items_center()
                .gap(px(8.))
                .px_3()
                .py_5()
                .text_size(px(12.))
                .child(
                    h_flex()
                        .gap(px(8.))
                        .text_color(if failed { p.warning } else { p.muted })
                        .when(busy, |row| row.child(spinner()))
                        .child(div().min_w_0().child(empty.clone())),
                )
                .when(failed, |state| {
                    let retry = retry.clone();
                    state.child(
                        Button::new("exchanges-retry")
                            .outline()
                            .small()
                            .text_label(t!("common.retry"))
                            .on_click(move |_, window, cx| retry(window, cx)),
                    )
                })
        })
}
