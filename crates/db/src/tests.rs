use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pitpls_core::{
    common::{Amount, Country, Currency, Mic},
    dividend::Dividend,
    rate::Rate,
    stock::{Stock, StockAction, StockBuy, StockSell, StockSplit},
};
use rust_decimal::{Decimal, dec};

use super::*;

const T212: &str = "Trading 212";
const REVOLUT: &str = "Revolut";

fn on(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap()
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn usd(value: Decimal) -> Amount {
    Amount {
        value,
        currency: Currency::USD,
    }
}

fn dividend(id: &str, time: DateTime<Utc>, provider: &str, value: Decimal) -> Dividend {
    Dividend {
        id: id.to_owned(),
        time,
        ticker: "KO".to_owned(),
        value: usd(value),
        tax_paid: usd(dec!(1.5)),
        country: Country::USA,
        provider: provider.to_owned(),
    }
}

fn buy(id: &str, time: DateTime<Utc>, ticker: &str, provider: &str) -> Stock {
    Stock {
        id: id.to_owned(),
        time,
        ticker: ticker.to_owned(),
        action: StockAction::Buy(StockBuy {
            number: dec!(1),
            price: usd(dec!(100)),
            amount: None,
            fee: usd(dec!(0)),
            provider: provider.to_owned(),
            settlement_date: None,
        }),
    }
}

fn ids(stocks: &[Stock]) -> Vec<&str> {
    stocks.iter().map(|stock| stock.id.as_str()).collect()
}

#[tokio::test]
async fn open_uses_wal_and_delete_removes_every_file() {
    let path = std::env::temp_dir().join(format!("pitpls-test-{}.db", std::process::id()));
    let db = Database::open(&path).await.unwrap();
    db.year_repo().add(2025).await.unwrap();

    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(mode, "wal");

    db.pool.close().await;
    Database::delete(&path).unwrap();
    assert!(!path.exists());
}

#[tokio::test]
async fn stock_records_read_back_unchanged() {
    let db = Database::open_in_memory().await.unwrap();
    let records = [
        Stock {
            id: "buy".to_owned(),
            time: on(2025, 3, 3),
            ticker: "AAPL".to_owned(),
            action: StockAction::Buy(StockBuy {
                number: dec!(0.8565163700),
                price: usd(dec!(145.94)),
                amount: Some(dec!(125.00)),
                fee: Amount {
                    value: dec!(0.19),
                    currency: Currency::PLN,
                },
                provider: T212.to_owned(),
                settlement_date: Some(date(2025, 3, 4)),
            }),
        },
        Stock {
            id: "sell".to_owned(),
            time: on(2025, 6, 2),
            ticker: "AAPL".to_owned(),
            action: StockAction::Sell(StockSell {
                number: dec!(0.5),
                price: usd(dec!(160)),
                amount: None,
                fee: usd(dec!(0)),
                provider: T212.to_owned(),
                settlement_date: None,
                tax_paid: Some(Amount {
                    value: dec!(1.25),
                    currency: Currency::EUR,
                }),
            }),
        },
        Stock {
            id: "split".to_owned(),
            time: on(2025, 7, 1),
            ticker: "AAPL".to_owned(),
            action: StockAction::Split(StockSplit {
                before: 1,
                after: 4,
            }),
        },
    ];
    for record in &records {
        db.stock_repo().insert(record).await.unwrap();
    }

    let stored = db.stock_repo().get_by_year(Some(2025)).await.unwrap();

    assert_eq!(ids(&stored), ["buy", "sell", "split"]);
    let StockAction::Buy(buy) = &stored[0].action else {
        panic!("expected a buy");
    };
    assert_eq!(buy.number.to_string(), "0.8565163700");
    assert_eq!(
        (buy.price.value, buy.price.currency, buy.amount),
        (dec!(145.94), Currency::USD, Some(dec!(125)))
    );
    assert_eq!(
        (buy.fee.value, buy.fee.currency),
        (dec!(0.19), Currency::PLN)
    );
    assert_eq!(
        (buy.provider.as_str(), buy.settlement_date),
        (T212, Some(date(2025, 3, 4)))
    );
    let StockAction::Sell(sell) = &stored[1].action else {
        panic!("expected a sale");
    };
    let tax_paid = sell.tax_paid.unwrap();
    assert_eq!(
        (sell.number, sell.amount, sell.settlement_date),
        (dec!(0.5), None, None)
    );
    assert_eq!(
        (tax_paid.value, tax_paid.currency),
        (dec!(1.25), Currency::EUR)
    );
    let StockAction::Split(split) = &stored[2].action else {
        panic!("expected a split");
    };
    assert_eq!((split.before, split.after), (1, 4));
    assert_eq!(stored[2].time, on(2025, 7, 1));
}

#[tokio::test]
async fn year_is_the_warsaw_year() {
    let db = Database::open_in_memory().await.unwrap();
    let repo = db.dividend_repo();
    // 31.12 23:30 and 01.01 00:30 in Warsaw.
    let december = Utc.with_ymd_and_hms(2025, 12, 31, 22, 30, 0).unwrap();
    let january = Utc.with_ymd_and_hms(2025, 12, 31, 23, 30, 0).unwrap();
    repo.insert(&dividend("december", december, T212, dec!(10)))
        .await
        .unwrap();
    repo.insert(&dividend("january", january, T212, dec!(10)))
        .await
        .unwrap();

    let in_year = |dividends: Vec<Dividend>| {
        dividends
            .into_iter()
            .map(|dividend| dividend.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        in_year(repo.get_by_year(Some(2025)).await.unwrap()),
        ["december"]
    );
    assert_eq!(
        in_year(repo.get_by_year(Some(2026)).await.unwrap()),
        ["january"]
    );

    let years = db
        .year_repo()
        .list_with_records()
        .await
        .unwrap()
        .into_iter()
        .map(|year| (year.year, year.dividends))
        .collect::<Vec<_>>();
    assert_eq!(years, [(2026, 1), (2025, 1)]);
}

#[tokio::test]
async fn save_updates_by_id_only_for_the_same_provider() {
    let db = Database::open_in_memory().await.unwrap();
    let dividends = db.dividend_repo();
    let at = on(2025, 5, 15);

    assert_eq!(
        dividends
            .save(&[dividend("x", at, T212, dec!(10))])
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        dividends
            .save(&[dividend("x", at, T212, dec!(12))])
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        dividends
            .save(&[dividend("x", at, REVOLUT, dec!(99))])
            .await
            .unwrap(),
        0
    );
    let [stored] = &dividends.get_by_year(None).await.unwrap()[..] else {
        panic!("expected one dividend");
    };
    assert_eq!(
        (stored.provider.as_str(), stored.value.value),
        (T212, dec!(12))
    );

    let stocks = db.stock_repo();
    stocks.save(&[buy("s", at, "AAPL", T212)]).await.unwrap();
    assert_eq!(stocks.save(&[buy("s", at, "MSFT", T212)]).await.unwrap(), 1);
    assert_eq!(
        stocks.save(&[buy("s", at, "KO", REVOLUT)]).await.unwrap(),
        0
    );
    let [stored] = &stocks.get_by_year(None).await.unwrap()[..] else {
        panic!("expected one stock");
    };
    assert_eq!(stored.ticker, "MSFT");
}

#[tokio::test]
async fn ticker_history_keeps_splits_of_every_provider() {
    let db = Database::open_in_memory().await.unwrap();
    let repo = db.stock_repo();
    let records = [
        buy("t212", on(2024, 2, 1), "NVDA", T212),
        buy("revolut", on(2024, 3, 1), "NVDA", REVOLUT),
        Stock {
            id: "split".to_owned(),
            time: on(2024, 6, 10),
            ticker: "NVDA".to_owned(),
            action: StockAction::Split(StockSplit {
                before: 1,
                after: 10,
            }),
        },
        buy("later", on(2026, 1, 5), "NVDA", T212),
        buy("other", on(2024, 2, 1), "AAPL", T212),
    ];
    for record in &records {
        repo.insert(record).await.unwrap();
    }

    let t212 = repo
        .get_by_ticker_through_year("NVDA", Some(T212), Some(2025))
        .await
        .unwrap();
    assert_eq!(ids(&t212), ["t212", "split"]);

    let all = repo
        .get_by_ticker_through_year("NVDA", None, None)
        .await
        .unwrap();
    assert_eq!(ids(&all), ["t212", "revolut", "split", "later"]);
}

#[tokio::test]
async fn rate_range_adds_the_earliest_day() {
    let db = Database::open_in_memory().await.unwrap();
    let repo = db.rate_repo();
    let rate = |date| Rate {
        date,
        currency: Currency::USD,
        rate: dec!(4),
    };
    repo.upload(
        [
            date(2020, 1, 2),
            date(2025, 3, 3),
            date(2025, 3, 4),
            date(2025, 3, 5),
        ]
        .into_iter()
        .map(rate),
    )
    .await
    .unwrap();

    let dates = |rates: Vec<Rate>| {
        let mut dates = rates.into_iter().map(|rate| rate.date).collect::<Vec<_>>();
        dates.sort();
        dates
    };
    assert_eq!(
        dates(
            repo.load_range(date(2025, 3, 4), date(2025, 3, 5))
                .await
                .unwrap()
        ),
        [date(2020, 1, 2), date(2025, 3, 4), date(2025, 3, 5)]
    );
    assert_eq!(
        dates(
            repo.load_range(date(2020, 1, 1), date(2020, 1, 3))
                .await
                .unwrap()
        ),
        [date(2020, 1, 2)]
    );
}

#[tokio::test]
async fn import_fills_isin_but_keeps_the_chosen_exchange() {
    let db = Database::open_in_memory().await.unwrap();
    let repo = db.instrument_repo();
    let xnas: Mic = "XNAS".parse().unwrap();

    repo.set_exchange("AAPL", T212, Some(xnas)).await.unwrap();
    repo.upsert_from_import("AAPL", T212, "US0378331005")
        .await
        .unwrap();
    repo.upsert_from_import("AAPL", T212, "US0000000009")
        .await
        .unwrap();
    repo.add("AAPL", T212).await.unwrap();

    let instrument = repo.get("AAPL", T212).await.unwrap().unwrap();
    assert_eq!(instrument.exchange, Some(xnas));
    assert_eq!(instrument.isin.as_deref(), Some("US0378331005"));
}
