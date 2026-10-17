use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use pitpls_core::{
    common::{Amount, Country, Currency, Mic},
    dividend::Dividend,
    exchange::{Exchange, ExchangeKind, ExchangeStatus},
    rate::Rate,
    stock::{Stock, StockAction, StockBuy, StockPreview, StockSell, StockSplit, Warning},
};
use pitpls_db::Database;
use pitpls_importers::model::ImporterKind;
use rust_decimal::{Decimal, dec};

use crate::{
    App,
    use_case::{
        AmountField, TextField, ValidationError, import, portfolio,
        stock::{self, CreateStockInput, Error, StockActionFields, StockFields, TradeFields},
        year,
    },
};

const T212: &str = "Trading 212";
const REVOLUT: &str = "Revolut";

async fn app() -> App {
    App::new(Database::open_in_memory().await.unwrap())
}

fn on(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 15, 0, 0).unwrap()
}

fn mic(code: &str) -> Mic {
    code.parse().unwrap()
}

fn amount(value: Decimal, currency: Currency) -> Amount {
    Amount { value, currency }
}

async fn add_rates(app: &App, years: std::ops::RangeInclusive<i32>) {
    let rates = NaiveDate::from_ymd_opt(*years.start(), 1, 1)
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
        .collect::<Vec<_>>();
    app.db.rate_repo().upload(rates.into_iter()).await.unwrap();
}

async fn add_exchanges(app: &App) {
    let exchange = |code: &str, operating: &str, kind, acronym: &str, country: &str| Exchange {
        mic: mic(code),
        operating_mic: mic(operating),
        kind,
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
            exchange("XNAS", "XNAS", ExchangeKind::Operating, "NASDAQ", "US"),
            exchange("XNGS", "XNAS", ExchangeKind::Segment, "NGS", "US"),
            exchange("XNYS", "XNYS", ExchangeKind::Operating, "NYSE", "US"),
            exchange("XETR", "XETR", ExchangeKind::Operating, "XETRA", "DE"),
        ])
        .await
        .unwrap();
}

async fn add_stocks(app: &App, records: &[Stock]) {
    for record in records {
        app.db.stock_repo().insert(record).await.unwrap();
    }
}

fn trade(provider: &str, shares: Decimal, price: Amount) -> StockBuy {
    StockBuy {
        number: shares,
        price,
        amount: None,
        fee: amount(Decimal::ZERO, price.currency),
        provider: provider.to_owned(),
        settlement_date: None,
    }
}

fn buy(id: &str, time: DateTime<Utc>, ticker: &str, provider: &str, price: Amount) -> Stock {
    Stock {
        id: id.to_owned(),
        time,
        ticker: ticker.to_owned(),
        action: StockAction::Buy(trade(provider, dec!(10), price)),
    }
}

fn sell(id: &str, time: DateTime<Utc>, ticker: &str, provider: &str, price: Amount) -> Stock {
    let trade = trade(provider, dec!(1), price);
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

fn ids(records: &[Stock]) -> Vec<&str> {
    records.iter().map(|record| record.id.as_str()).collect()
}

fn usd(value: Decimal) -> Amount {
    amount(value, Currency::USD)
}

fn eur(value: Decimal) -> Amount {
    amount(value, Currency::EUR)
}

#[tokio::test]
async fn tax_year_uses_earlier_lots_but_lists_only_its_records() {
    let app = app().await;
    add_rates(&app, 2024..=2026).await;
    add_stocks(
        &app,
        &[
            buy("2024", on(2024, 3, 4), "AAPL", T212, usd(dec!(10))),
            sell("2025", on(2025, 3, 3), "AAPL", T212, usd(dec!(15))),
            buy("2026", on(2026, 1, 5), "AAPL", T212, usd(dec!(20))),
        ],
    )
    .await;

    let data = stock::load_stocks(&app, 2025).await.unwrap();

    assert_eq!(ids(&data.records), ["2025"]);
    let [sale] = &data.tax.sales[..] else {
        panic!("expected one sale");
    };
    assert_eq!(sale.matches[0].buy_id, "2024");
    assert_eq!((sale.income, sale.costs), (dec!(60), dec!(40)));
    let [lot] = &data.tax.open_lots[..] else {
        panic!("expected one open lot");
    };
    assert_eq!((lot.buy_id.as_str(), lot.open), ("2024", dec!(9)));
}

async fn year_end_sales(app: &App) {
    add_exchanges(app).await;
    app.db
        .instrument_repo()
        .set_exchange("VWCE", T212, Some(mic("XETR")))
        .await
        .unwrap();
    add_stocks(
        app,
        &[
            buy("buy", on(2025, 6, 2), "VWCE", T212, eur(dec!(100))),
            // Monday 29.12 settles on 31.12; Tuesday 30.12 on 01.01.2026.
            sell("dec-29", on(2025, 12, 29), "VWCE", T212, eur(dec!(110))),
            sell("dec-30", on(2025, 12, 30), "VWCE", T212, eur(dec!(110))),
        ],
    )
    .await;
}

#[tokio::test]
async fn year_list_has_the_year_a_sale_settles_in() {
    let app = app().await;
    year_end_sales(&app).await;

    let years = year::list_year_info(&app)
        .await
        .unwrap()
        .into_iter()
        .map(|info| (info.year, info.stocks, info.custom))
        .collect::<Vec<_>>();

    assert_eq!(years, [(2026, 0, false), (2025, 3, true)]);
}

#[tokio::test]
async fn deleting_a_year_deletes_the_sales_that_settle_in_it() {
    let app = app().await;
    year_end_sales(&app).await;

    assert_eq!(year::delete_year(&app, 2026).await.unwrap(), 1);

    let left = app.db.stock_repo().get_by_year(None).await.unwrap();
    assert_eq!(ids(&left), ["buy", "dec-29"]);
}

#[tokio::test]
async fn segment_mic_counts_as_its_operating_mic() {
    let app = app().await;
    add_rates(&app, 2025..=2025).await;
    add_exchanges(&app).await;
    let instruments = app.db.instrument_repo();
    instruments
        .set_exchange("AAPL", T212, Some(mic("XNGS")))
        .await
        .unwrap();
    instruments
        .set_exchange("KO", REVOLUT, Some(mic("ZZZZ")))
        .await
        .unwrap();
    add_stocks(
        &app,
        &[
            buy("aapl-buy", on(2025, 2, 3), "AAPL", T212, usd(dec!(100))),
            sell("aapl-sell", on(2025, 5, 5), "AAPL", T212, usd(dec!(110))),
            buy("ko-buy", on(2025, 2, 3), "KO", REVOLUT, usd(dec!(50))),
            sell("ko-sell", on(2025, 5, 5), "KO", REVOLUT, usd(dec!(60))),
        ],
    )
    .await;

    let data = stock::load_stocks(&app, 2025).await.unwrap();

    let listing = data.listings.get("AAPL", T212).unwrap();
    assert_eq!(
        (listing.mic, listing.country, listing.name.as_str()),
        (mic("XNAS"), Country::USA, "NASDAQ")
    );
    assert!(data.listings.get("KO", REVOLUT).is_none());
    let [us] = &data.tax.countries[..] else {
        panic!("expected one country");
    };
    assert_eq!(
        (us.exchanges.as_slice(), us.sales),
        ([mic("XNAS")].as_slice(), 1)
    );
    assert!(matches!(
        &data.tax.warnings[..],
        [Warning::MissingExchange { ticker, provider, sales: 1 }]
            if ticker == "KO" && provider == REVOLUT
    ));
}

#[tokio::test]
async fn year_history_stops_at_the_end_of_the_year() {
    let app = app().await;
    add_rates(&app, 2024..=2026).await;
    add_stocks(
        &app,
        &[
            buy("2024", on(2024, 3, 4), "NVDA", T212, usd(dec!(100))),
            buy("revolut", on(2024, 3, 4), "NVDA", REVOLUT, usd(dec!(100))),
            Stock {
                id: "split".to_owned(),
                time: on(2025, 6, 10),
                ticker: "NVDA".to_owned(),
                action: StockAction::Split(StockSplit {
                    before: 1,
                    after: 10,
                }),
            },
            buy("2026", on(2026, 1, 5), "NVDA", T212, usd(dec!(15))),
        ],
    )
    .await;

    let history = stock::load_history(&app, "NVDA", T212, Some(2025))
        .await
        .unwrap();

    assert_eq!(ids(&history.records), ["2024", "split"]);
    let [lot] = &history.replay.open_lots[..] else {
        panic!("expected one open lot");
    };
    assert_eq!((lot.buy_id.as_str(), lot.open), ("2024", dec!(100)));

    let all = stock::load_history(&app, "NVDA", T212, None).await.unwrap();
    assert_eq!(ids(&all.records), ["2024", "split", "2026"]);
}

#[tokio::test]
async fn preview_settles_by_the_exchange_picked_in_the_dialog() {
    let app = app().await;
    add_rates(&app, 2025..=2025).await;
    add_exchanges(&app).await;
    add_stocks(
        &app,
        &[buy("buy", on(2025, 2, 3), "VWCE", T212, eur(dec!(100)))],
    )
    .await;
    // Monday 03.03.2025: T+2 by the EUR currency, T+1 on a US exchange.
    let draft = sell("draft", on(2025, 3, 3), "VWCE", T212, eur(dec!(110)));

    let settles = |preview| match preview {
        StockPreview::Sell { sale, .. } => sale.settlement_date,
        _ => panic!("expected a sale preview"),
    };
    let by_currency = stock::preview_stock(&app, draft.clone(), None)
        .await
        .unwrap();
    assert_eq!(
        settles(by_currency),
        NaiveDate::from_ymd_opt(2025, 3, 5).unwrap()
    );
    let picked = stock::preview_stock(&app, draft, Some(mic("XNYS")))
        .await
        .unwrap();
    assert_eq!(
        settles(picked),
        NaiveDate::from_ymd_opt(2025, 3, 4).unwrap()
    );
}

#[tokio::test]
async fn all_years_error_names_the_earliest_year_without_rates() {
    let app = app().await;
    add_rates(&app, 2025..=2025).await;
    app.db
        .dividend_repo()
        .insert(&Dividend {
            id: "dividend".to_owned(),
            time: on(2023, 6, 1),
            ticker: "KO".to_owned(),
            value: usd(dec!(10)),
            tax_paid: usd(dec!(1.5)),
            country: Country::USA,
            provider: REVOLUT.to_owned(),
        })
        .await
        .unwrap();
    add_stocks(
        &app,
        &[buy("buy", on(2021, 6, 1), "AAPL", T212, usd(dec!(100)))],
    )
    .await;
    let today = NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();

    let error = portfolio::load_summary(&app, today).await.err().unwrap();

    assert_eq!(
        error.rate_error().and_then(|error| error.missing_year()),
        Some(2021)
    );
}

#[tokio::test]
async fn t212_import_saves_records_instruments_and_years_once() {
    let app = app().await;
    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../importers/tests/fixtures/t212/stocks.csv"
    );

    for _ in 0..2 {
        let result = import::run_import(&app, ImporterKind::T212, file.to_owned())
            .await
            .unwrap();
        assert_eq!((result.stocks, result.skipped_dividend_adjustments), (7, 1));
    }

    assert_eq!(
        app.db.stock_repo().get_by_year(None).await.unwrap().len(),
        7
    );
    let isins = app
        .db
        .instrument_repo()
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|instrument| (instrument.ticker, instrument.isin))
        .collect::<Vec<_>>();
    assert_eq!(
        isins,
        [
            ("AAPL".to_owned(), Some("US0378331005".to_owned())),
            ("MSFT".to_owned(), Some("US5949181045".to_owned())),
            ("SAP".to_owned(), Some("DE0007164600".to_owned())),
        ]
    );
    let years = year::list_year_info(&app).await.unwrap();
    assert_eq!(
        years.iter().map(|info| info.year).collect::<Vec<_>>(),
        [2025]
    );
}

fn stock_input(
    time: &str,
    ticker: &str,
    shares: &str,
    fee: &str,
    provider: &str,
) -> CreateStockInput {
    CreateStockInput {
        id: None,
        fields: StockFields {
            time: time.to_owned(),
            ticker: ticker.to_owned(),
            action: StockActionFields::Buy(TradeFields {
                shares: shares.to_owned(),
                price: "100".to_owned(),
                price_currency: Currency::USD,
                amount: None,
                fee: fee.to_owned(),
                fee_currency: Currency::USD,
                provider: provider.to_owned(),
                settlement_date: None,
                exchange: Some(mic("XNAS")),
            }),
        },
    }
}

#[tokio::test]
async fn created_stock_is_trimmed_and_validated() {
    let app = app().await;

    let id = stock::create_stock(
        &app,
        stock_input("2025-03-03 15:30:00", " AAPL ", "2", "0", " Trading 212 "),
    )
    .await
    .unwrap();

    let [stored] = &app.db.stock_repo().get_by_year(None).await.unwrap()[..] else {
        panic!("expected one stock");
    };
    assert_eq!(stored.id, id);
    assert_eq!(
        (stored.ticker.as_str(), stored.provider()),
        ("AAPL", Some(T212))
    );
    // Entered in Warsaw time.
    assert_eq!(
        stored.time,
        Utc.with_ymd_and_hms(2025, 3, 3, 14, 30, 0).unwrap()
    );
    let instrument = app.db.instrument_repo().get("AAPL", T212).await.unwrap();
    assert_eq!(
        instrument.and_then(|instrument| instrument.exchange),
        Some(mic("XNAS"))
    );

    let invalid = [
        stock_input("2025-03-03 15:30:00", "AAPL", "0", "0", T212),
        stock_input("2025-03-03 15:30:00", "AAPL", "1", "-1", T212),
        stock_input("2025-03-03 15:30:00", "AAPL", "1", "0", " "),
        stock_input("1800-03-03 15:30:00", "AAPL", "1", "0", T212),
        // Skipped by the change to summer time.
        stock_input("2025-03-30 02:30:00", "AAPL", "1", "0", T212),
    ];
    let mut errors = Vec::new();
    for input in invalid {
        match stock::create_stock(&app, input).await {
            Err(Error::Validation(error)) => errors.push(error),
            other => panic!("expected a validation error, got {:?}", other.err()),
        }
    }
    assert!(matches!(
        &errors[..],
        [
            ValidationError::NotPositive {
                field: AmountField::Shares,
                ..
            },
            ValidationError::Negative {
                field: AmountField::Fee,
                ..
            },
            ValidationError::Empty(TextField::Provider),
            ValidationError::InvalidYear { year: 1800, .. },
            ValidationError::NonexistentTime(_),
        ]
    ));
}
