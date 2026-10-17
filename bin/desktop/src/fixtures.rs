use std::{
    cell::{Cell, RefCell},
    ops::RangeInclusive,
    rc::Rc,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, ElementId, Entity, Global, Point, Render, TestAppContext,
    Window, WindowBounds, WindowOptions,
    component::input::InputState,
    px, size,
    test::{ElementSnapshot, TestWindowExt},
};
use pitpls_app::App;
use pitpls_core::{
    common::{Amount, Currency, Mic},
    exchange::{Exchange, ExchangeKind, ExchangeStatus},
    rate::Rate,
    stock::{Stock, StockAction, StockBuy, StockSell, StockSplit},
};
use pitpls_db::Database;
use rust_decimal::{Decimal, dec};
use tokio::runtime::Runtime;

use crate::{
    i18n::{Language, apply_language},
    navigation::{Page, PageContext, PageEvent, PageEvents},
    services::Services,
    theme,
};

pub const T212: &str = "Trading 212";
pub const REVOLUT: &str = "Revolut";

pub fn on(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap()
}

pub fn usd(value: Decimal) -> Amount {
    Amount {
        value,
        currency: Currency::USD,
    }
}

pub fn eur(value: Decimal) -> Amount {
    Amount {
        value,
        currency: Currency::EUR,
    }
}

// USD at 4 and EUR at 4.5 on every day.
pub fn rates(years: RangeInclusive<i32>) -> Vec<Rate> {
    NaiveDate::from_ymd_opt(*years.start(), 1, 1)
        .unwrap()
        .iter_days()
        .take_while(|day| day.year() <= *years.end())
        .flat_map(|date| {
            [(Currency::USD, dec!(4)), (Currency::EUR, dec!(4.5))].map(|(currency, rate)| Rate {
                date,
                currency,
                rate,
            })
        })
        .collect()
}

// NASDAQ and XETRA in the exchanges table.
pub async fn app(years: RangeInclusive<i32>) -> App {
    let app = app_without_rates().await;
    app.db
        .rate_repo()
        .upload(rates(years).into_iter())
        .await
        .unwrap();
    app
}

pub async fn app_without_rates() -> App {
    let app = App::new(Database::open_in_memory().await.unwrap());
    let exchange = |mic: &str, acronym: &str, country: &str| Exchange {
        mic: mic.parse().unwrap(),
        operating_mic: mic.parse().unwrap(),
        kind: ExchangeKind::Operating,
        name: format!("{acronym} market"),
        acronym: Some(acronym.to_owned()),
        country: country.parse().unwrap(),
        city: None,
        status: ExchangeStatus::Active,
        last_update: None,
    };
    app.db
        .exchange_repo()
        .replace_all(&[
            exchange("XNAS", "NASDAQ", "US"),
            exchange("XETR", "XETRA", "DE"),
        ])
        .await
        .unwrap();
    app
}

pub async fn listed(app: &App, ticker: &str, provider: &str, mic: &str) {
    app.db
        .instrument_repo()
        .set_exchange(ticker, provider, Some(mic.parse::<Mic>().unwrap()))
        .await
        .unwrap();
}

pub async fn add_stocks(app: &App, records: &[Stock]) {
    for record in records {
        app.db.stock_repo().insert(record).await.unwrap();
    }
}

fn trade(provider: &str, shares: Decimal, price: Amount) -> StockBuy {
    StockBuy {
        number: shares,
        price,
        amount: None,
        fee: Amount {
            value: Decimal::ZERO,
            currency: price.currency,
        },
        provider: provider.to_owned(),
        settlement_date: None,
    }
}

pub fn buy(
    id: &str,
    time: DateTime<Utc>,
    ticker: &str,
    provider: &str,
    shares: Decimal,
    price: Amount,
) -> Stock {
    Stock {
        id: id.to_owned(),
        time,
        ticker: ticker.to_owned(),
        action: StockAction::Buy(trade(provider, shares, price)),
    }
}

pub fn sell(
    id: &str,
    time: DateTime<Utc>,
    ticker: &str,
    provider: &str,
    shares: Decimal,
    price: Amount,
) -> Stock {
    let trade = trade(provider, shares, price);
    Stock {
        id: id.to_owned(),
        time,
        ticker: ticker.to_owned(),
        action: StockAction::Sell(StockSell {
            number: trade.number,
            price: trade.price,
            amount: None,
            fee: trade.fee,
            provider: trade.provider,
            settlement_date: None,
            tax_paid: None,
        }),
    }
}

// AAPL on NASDAQ, bought with an imported total and partly sold; KO sells a share it never
// bought, with no exchange.
pub async fn stock_app() -> App {
    let app = app(2025..=2025).await;
    listed(&app, "AAPL", T212, "XNAS").await;
    let mut imported = buy(
        "aapl-1",
        on(2025, 2, 3),
        "AAPL",
        T212,
        dec!(10),
        usd(dec!(100.025)),
    );
    if let StockAction::Buy(buy) = &mut imported.action {
        buy.amount = Some(dec!(1000.25));
    }
    add_stocks(
        &app,
        &[
            imported,
            sell(
                "aapl-s",
                on(2025, 5, 5),
                "AAPL",
                T212,
                dec!(4),
                usd(dec!(150)),
            ),
            buy(
                "ko-1",
                on(2025, 2, 3),
                "KO",
                REVOLUT,
                dec!(2),
                usd(dec!(50)),
            ),
            sell(
                "ko-s",
                on(2025, 6, 2),
                "KO",
                REVOLUT,
                dec!(3),
                usd(dec!(60)),
            ),
        ],
    )
    .await;
    app
}

pub fn split(id: &str, time: DateTime<Utc>, ticker: &str, before: u32, after: u32) -> Stock {
    Stock {
        id: id.to_owned(),
        time,
        ticker: ticker.to_owned(),
        action: StockAction::Split(StockSplit { before, after }),
    }
}

// What a page asked the shell to do.
pub enum Event {
    Navigate(Page),
    SelectYear(Option<i32>),
    YearsChanged,
}

// A page in a test window, with the app's background work on a real runtime.
pub struct Ui<V> {
    pub window: AnyWindowHandle,
    pub view: Entity<V>,
    loads: Rc<Cell<usize>>,
    events: Rc<RefCell<Vec<Event>>>,
}

impl<V: 'static> Ui<V> {
    // Runs the window's tasks until the page has reported `count` loads.
    #[track_caller]
    pub fn wait_loaded(&self, count: usize, cx: &mut TestAppContext) {
        let loads = self.loads.clone();
        self.wait_until(cx, move |_| loads.get() >= count);
    }

    #[track_caller]
    pub fn wait_until(
        &self,
        cx: &mut TestAppContext,
        mut done: impl FnMut(&mut TestAppContext) -> bool,
    ) {
        let start = Instant::now();
        loop {
            cx.run_until_parked();
            if done(cx) {
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "the page didn't get there within 10 s ({} loads)",
                self.loads.get()
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn read<R>(&self, cx: &TestAppContext, read: impl FnOnce(&V, &gpui_kit::App) -> R) -> R {
        self.view.read_with(cx, read)
    }

    pub fn window<R>(
        &self,
        cx: &mut TestAppContext,
        update: impl FnOnce(&mut Window, &mut gpui_kit::App) -> R,
    ) -> R {
        let result = cx
            .update_window(self.window, |_, window, cx| update(window, cx))
            .unwrap();
        cx.run_until_parked();
        result
    }

    pub fn click(&self, id: impl Into<ElementId>, cx: &mut TestAppContext) {
        let id = id.into();
        self.window(cx, |window, cx| window.click(id, cx));
    }

    // For an ID that repeats on the page, such as a radio's index in its group.
    pub fn click_within(
        &self,
        scope: impl Into<ElementId>,
        id: impl Into<ElementId>,
        cx: &mut TestAppContext,
    ) {
        let (scope, id) = (scope.into(), id.into());
        self.window(cx, |window, cx| window.within(scope).click(id, cx));
    }

    // After a fresh frame, so it reflects the latest state.
    pub fn find(
        &self,
        id: impl Into<ElementId>,
        cx: &mut TestAppContext,
    ) -> Option<ElementSnapshot> {
        let id = id.into();
        self.window(cx, |window, cx| {
            window.render_frame(cx);
            window.try_find(id)
        })
    }

    // Replaces the text the way a user does: focus, select all, type.
    pub fn type_into(&self, input: &Entity<InputState>, text: &str, cx: &mut TestAppContext) {
        let select_all = if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        };
        self.window(cx, |window, cx| {
            input.update(cx, |input, cx| input.focus(window, cx));
            window.press(select_all, cx);
            window.input(text, cx);
        });
    }

    pub fn events(&self) -> std::cell::Ref<'_, Vec<Event>> {
        self.events.borrow()
    }
}

struct Initialized;

impl Global for Initialized {}

pub fn runtime() -> Arc<Runtime> {
    Arc::new(Runtime::new().unwrap())
}

pub fn open<V: Render>(
    cx: &mut TestAppContext,
    runtime: Arc<Runtime>,
    app: impl Into<Arc<App>>,
    build: impl FnOnce(PageContext, &mut Window, &mut gpui_kit::App) -> Entity<V>,
) -> Ui<V> {
    // The app's work runs on the tokio runtime and wakes the window's tasks from its threads.
    cx.executor().allow_parking();
    let loads = Rc::new(Cell::new(0));
    let events = Rc::new(RefCell::new(Vec::new()));
    let (counter, sink) = (loads.clone(), events.clone());
    let (window, view) = cx.update(|cx| {
        if !cx.has_global::<Initialized>() {
            gpui_kit::init(cx);
            theme::configure_theme(cx);
            apply_language(Some(Language::English), cx);
            // Dialogs slide in on the wall clock, so a click could land where one was a frame ago.
            cx.set_reduce_motion(true);
            cx.set_global(Initialized);
        }
        let page_events = cx.new(|_| PageEvents);
        cx.subscribe(&page_events, move |_, event, _| match event {
            PageEvent::Loaded(_) => counter.set(counter.get() + 1),
            PageEvent::Navigate(page) => sink.borrow_mut().push(Event::Navigate(*page)),
            PageEvent::SelectYear(year) => sink.borrow_mut().push(Event::SelectYear(*year)),
            PageEvent::YearsChanged => sink.borrow_mut().push(Event::YearsChanged),
            _ => {}
        })
        .detach();
        let context = PageContext::new(
            Services {
                app: app.into(),
                runtime,
            },
            page_events,
        );
        let bounds = Bounds {
            origin: Point::default(),
            size: size(px(1180.), px(780.)),
        };
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            |window, cx| build(context, window, cx),
        )
        .expect("open test window")
    });
    Ui {
        window,
        view,
        loads,
        events,
    }
}
