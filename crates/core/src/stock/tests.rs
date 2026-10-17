use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use rust_decimal::{Decimal, dec};

use super::*;
use crate::{
    common::{Amount, Currency},
    rate::{NbpRateProvider, Rate},
};

const T212: &str = "Trading 212";
const REVOLUT: &str = "Revolut";
const IBKR: &str = "Interactive Brokers";

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn on(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 15, 0, 0).unwrap()
}

fn usd(value: Decimal) -> Amount {
    Amount {
        value,
        currency: Currency::USD,
    }
}

fn eur(value: Decimal) -> Amount {
    Amount {
        value,
        currency: Currency::EUR,
    }
}

fn pln(value: Decimal) -> Amount {
    Amount {
        value,
        currency: Currency::PLN,
    }
}

fn buy(
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
        action: StockAction::Buy(StockBuy {
            number: shares,
            price,
            amount: None,
            fee: Amount {
                value: Decimal::ZERO,
                currency: price.currency,
            },
            provider: provider.to_owned(),
            settlement_date: None,
        }),
    }
}

fn sell(
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
        action: StockAction::Sell(StockSell {
            number: shares,
            price,
            amount: None,
            fee: Amount {
                value: Decimal::ZERO,
                currency: price.currency,
            },
            provider: provider.to_owned(),
            settlement_date: None,
            tax_paid: None,
        }),
    }
}

fn split(id: &str, time: DateTime<Utc>, ticker: &str, before: u32, after: u32) -> Stock {
    Stock {
        id: id.to_owned(),
        time,
        ticker: ticker.to_owned(),
        action: StockAction::Split(StockSplit { before, after }),
    }
}

fn with_fee(mut stock: Stock, fee: Amount) -> Stock {
    match &mut stock.action {
        StockAction::Buy(buy) => buy.fee = fee,
        StockAction::Sell(sell) => sell.fee = fee,
        StockAction::Split(_) => panic!("a split has no fee"),
    }
    stock
}

fn with_settlement(mut stock: Stock, date: NaiveDate) -> Stock {
    match &mut stock.action {
        StockAction::Buy(buy) => buy.settlement_date = Some(date),
        StockAction::Sell(sell) => sell.settlement_date = Some(date),
        StockAction::Split(_) => panic!("a split has no settlement"),
    }
    stock
}

fn with_tax_paid(mut stock: Stock, tax_paid: Amount) -> Stock {
    match &mut stock.action {
        StockAction::Sell(sell) => sell.tax_paid = Some(tax_paid),
        _ => panic!("only a sale has tax paid"),
    }
    stock
}

fn with_amount(mut stock: Stock, amount: Decimal) -> Stock {
    match &mut stock.action {
        StockAction::Buy(buy) => buy.amount = Some(amount),
        StockAction::Sell(sell) => sell.amount = Some(amount),
        StockAction::Split(_) => panic!("a split has no amount"),
    }
    stock
}

fn rate(date: NaiveDate, currency: Currency, rate: Decimal) -> Rate {
    Rate {
        date,
        currency,
        rate,
    }
}

// The same rate on every day of 2023–2026, so the tests that aren't about rates stay simple.
fn flat(currency: Currency, value: Decimal) -> Vec<Rate> {
    date(2023, 1, 1)
        .iter_days()
        .take_while(|day| day.year() <= 2026)
        .map(|day| rate(day, currency, value))
        .collect()
}

fn usd_at_4() -> NbpRateProvider {
    NbpRateProvider::new(flat(Currency::USD, dec!(4)))
}

fn listed(ticker: &str, provider: &str, mic: &str, country: &str) -> InstrumentExchange {
    InstrumentExchange {
        ticker: ticker.to_owned(),
        provider: provider.to_owned(),
        mic: mic.parse().unwrap(),
        country: country.parse().unwrap(),
    }
}

fn sale<'a>(data: &'a StockTaxData, id: &str) -> &'a CalculatedSale {
    data.sales
        .iter()
        .find(|sale| sale.id == id)
        .unwrap_or_else(|| panic!("no sale {id}"))
}

fn open_lot<'a>(data: &'a StockTaxData, buy_id: &str) -> &'a OpenLot {
    data.open_lots
        .iter()
        .find(|lot| lot.buy_id == buy_id)
        .unwrap_or_else(|| panic!("no open lot {buy_id}"))
}

fn sold_without_buy(data: &StockTaxData, id: &str) -> Option<Decimal> {
    data.warnings.iter().find_map(|warning| match warning {
        Warning::SellWithoutBuy { sell_id, shares } if sell_id == id => Some(*shares),
        _ => None,
    })
}

#[test]
fn fifo_is_separate_per_provider() {
    let records = [
        buy(
            "rev-buy",
            on(2025, 3, 3),
            "NVDA",
            REVOLUT,
            dec!(1),
            usd(dec!(200)),
        ),
        sell(
            "rev-sell",
            on(2025, 6, 2),
            "NVDA",
            REVOLUT,
            dec!(1),
            usd(dec!(300)),
        ),
        buy(
            "t212-buy",
            on(2025, 3, 3),
            "NVDA",
            T212,
            dec!(2),
            usd(dec!(200)),
        ),
        sell(
            "t212-sell",
            on(2025, 6, 2),
            "NVDA",
            T212,
            dec!(1),
            usd(dec!(350)),
        ),
    ];

    let data = calculate_stocks(&records, &[], 2025, &usd_at_4()).unwrap();

    let revolut = sale(&data, "rev-sell");
    assert_eq!(revolut.costs, dec!(800));
    assert_eq!(revolut.profit, dec!(400));
    assert_eq!(revolut.matches[0].buy_id, "rev-buy");

    let t212 = sale(&data, "t212-sell");
    assert_eq!(t212.costs, dec!(800));
    assert_eq!(t212.profit, dec!(600));
    assert_eq!(t212.matches[0].buy_id, "t212-buy");

    assert_eq!(data.open_lots.len(), 1);
    assert_eq!(open_lot(&data, "t212-buy").open, dec!(1));
}

#[test]
fn sale_takes_oldest_lots_first() {
    let records = [
        buy("old", on(2025, 2, 3), "AAPL", T212, dec!(3), usd(dec!(10))),
        buy("new", on(2025, 2, 10), "AAPL", T212, dec!(4), usd(dec!(20))),
        sell("sell", on(2025, 5, 5), "AAPL", T212, dec!(5), usd(dec!(30))),
    ];

    let data = calculate_stocks(&records, &[], 2025, &usd_at_4()).unwrap();
    let sale = sale(&data, "sell");

    let [old, new] = &sale.matches[..] else {
        panic!("expected two matches");
    };
    assert_eq!(
        (old.buy_id.as_str(), old.before, old.taken, old.after),
        ("old", dec!(3), dec!(3), dec!(0))
    );
    assert_eq!(
        (new.buy_id.as_str(), new.before, new.taken, new.after),
        ("new", dec!(4), dec!(2), dec!(2))
    );
    assert_eq!(sale.income, dec!(600));
    assert_eq!(sale.lots_cost, dec!(280));
    assert_eq!(sale.profit, dec!(320));

    let open = open_lot(&data, "new");
    assert_eq!(open.open, dec!(2));
    assert_eq!(open.cost, dec!(160));
}

#[test]
fn buy_fee_counts_by_shares_taken_and_sale_fee_in_full() {
    let records = [
        with_fee(
            buy(
                "buy",
                on(2025, 2, 3),
                "MSFT",
                T212,
                dec!(10),
                usd(dec!(100)),
            ),
            usd(dec!(1.59)),
        ),
        with_fee(
            sell(
                "sell",
                on(2025, 5, 5),
                "MSFT",
                T212,
                dec!(5),
                usd(dec!(120)),
            ),
            usd(dec!(2)),
        ),
    ];

    let data = calculate_stocks(&records, &[], 2025, &usd_at_4()).unwrap();
    let sale = sale(&data, "sell");

    assert_eq!(sale.lots_cost, dec!(2000));
    assert_eq!(sale.buy_fees, dec!(3.18));
    assert_eq!(sale.fee_pln, dec!(8));
    assert_eq!(sale.costs, dec!(2011.18));
    assert_eq!(open_lot(&data, "buy").fee_part, dec!(3.18));
}

#[test]
fn split_changes_shares_but_not_lot_cost() {
    let records = [
        buy(
            "buy",
            on(2024, 1, 10),
            "NVDA",
            T212,
            dec!(3),
            usd(dec!(433.44)),
        ),
        split("split", on(2024, 6, 10), "NVDA", 1, 10),
        sell(
            "sell",
            on(2025, 1, 7),
            "NVDA",
            T212,
            dec!(20),
            usd(dec!(140)),
        ),
    ];

    let data = calculate_stocks(&records, &[], 2025, &usd_at_4()).unwrap();
    let lot = &sale(&data, "sell").matches[0];

    assert_eq!(lot.lot_shares, dec!(30));
    assert_eq!(
        (lot.before, lot.taken, lot.after),
        (dec!(30), dec!(20), dec!(10))
    );
    assert_eq!(lot.price.value, dec!(43.344));
    assert_eq!(lot.cost, dec!(3467.52));
    let [step] = &lot.splits[..] else {
        panic!("expected one split");
    };
    assert_eq!(step.split_id, "split");
    assert_eq!(
        (step.lot_shares_before, step.lot_shares_after),
        (dec!(3), dec!(30))
    );

    let open = open_lot(&data, "buy");
    assert_eq!(open.open, dec!(10));
    assert_eq!(open.cost, dec!(1733.76));
}

#[test]
fn split_applies_to_every_provider() {
    let records = [
        buy(
            "t212",
            on(2024, 1, 10),
            "NVDA",
            T212,
            dec!(3),
            usd(dec!(400)),
        ),
        buy(
            "rev",
            on(2024, 2, 12),
            "NVDA",
            REVOLUT,
            dec!(2),
            usd(dec!(500)),
        ),
        split("split", on(2024, 6, 10), "NVDA", 1, 10),
    ];

    let data = calculate_stocks(&records, &[], 2024, &usd_at_4()).unwrap();

    assert_eq!(open_lot(&data, "t212").open, dec!(30));
    assert_eq!(open_lot(&data, "rev").open, dec!(20));
}

#[test]
fn reverse_split_closes_lot_exactly() {
    let records = [
        buy("buy", on(2025, 2, 3), "XYZ", T212, dec!(9), usd(dec!(10))),
        split("split", on(2025, 3, 3), "XYZ", 3, 1),
        sell("sell", on(2025, 4, 7), "XYZ", T212, dec!(3), usd(dec!(40))),
    ];

    let data = calculate_stocks(&records, &[], 2025, &usd_at_4()).unwrap();
    let sale = sale(&data, "sell");

    assert_eq!(sale.matches[0].after, Decimal::ZERO);
    assert_eq!(sale.matches[0].price.value, dec!(30));
    assert_eq!(sale.uncovered, Decimal::ZERO);
    assert_eq!(sale.lots_cost, dec!(360));
    assert!(data.open_lots.is_empty());
}

#[test]
fn sale_without_buy_counts_at_zero_cost() {
    let records = [
        buy("buy", on(2025, 2, 3), "KO", REVOLUT, dec!(4), usd(dec!(50))),
        sell(
            "short",
            on(2025, 5, 5),
            "KO",
            REVOLUT,
            dec!(7),
            usd(dec!(60)),
        ),
        sell(
            "nothing",
            on(2025, 5, 5),
            "PEP",
            REVOLUT,
            dec!(2),
            usd(dec!(10)),
        ),
    ];

    let data = calculate_stocks(&records, &[], 2025, &usd_at_4()).unwrap();

    let short = sale(&data, "short");
    assert_eq!(short.matches.len(), 1);
    assert_eq!(short.uncovered, dec!(3));
    assert_eq!(short.income, dec!(1680));
    assert_eq!(short.costs, dec!(800));
    assert_eq!(sold_without_buy(&data, "short"), Some(dec!(3)));

    let nothing = sale(&data, "nothing");
    assert!(nothing.matches.is_empty());
    assert_eq!(nothing.uncovered, dec!(2));
    assert_eq!(nothing.costs, Decimal::ZERO);
    assert_eq!(sold_without_buy(&data, "nothing"), Some(dec!(2)));
}

#[test]
fn tax_year_follows_settlement_date() {
    let records = [
        buy(
            "buy",
            on(2025, 6, 2),
            "VWCE",
            T212,
            dec!(10),
            eur(dec!(100)),
        ),
        // Monday 29.12 settles Wednesday 31.12; Tuesday 30.12 settles Thursday 01.01.
        sell(
            "dec-29",
            on(2025, 12, 29),
            "VWCE",
            T212,
            dec!(1),
            eur(dec!(110)),
        ),
        sell(
            "dec-30",
            on(2025, 12, 30),
            "VWCE",
            T212,
            dec!(4),
            eur(dec!(110)),
        ),
    ];
    let exchanges = [listed("VWCE", T212, "XETR", "DE")];
    let rates = NbpRateProvider::new(flat(Currency::EUR, dec!(4.5)));

    let data_2025 = calculate_stocks(&records, &exchanges, 2025, &rates).unwrap();
    assert_eq!(data_2025.sales.len(), 1);
    assert_eq!(
        sale(&data_2025, "dec-29").settlement_date,
        date(2025, 12, 31)
    );
    let [dec_30] = &data_2025.next_year_sales[..] else {
        panic!("expected the 30.12 sale to count in 2026");
    };
    assert_eq!(dec_30.settlement_date, date(2026, 1, 1));
    assert!(dec_30.settlement_worked_out);
    assert_eq!(dec_30.nbp_date, date(2025, 12, 31));
    assert_eq!(data_2025.income, dec!(495));
    assert_eq!(data_2025.costs, dec!(450));
    // Holdings on 31.12 follow the trade date.
    assert_eq!(open_lot(&data_2025, "buy").open, dec!(5));

    let data_2026 = calculate_stocks(&records, &exchanges, 2026, &rates).unwrap();
    assert_eq!(sale(&data_2026, "dec-30").settlement_date, date(2026, 1, 1));
    assert!(data_2026.next_year_sales.is_empty());
    assert_eq!(data_2026.income, dec!(1980));
    assert_eq!(data_2026.costs, dec!(1800));
}

#[test]
fn recorded_settlement_date_wins_over_worked_out() {
    let records = [
        buy(
            "buy",
            on(2025, 6, 2),
            "VWCE",
            T212,
            dec!(10),
            eur(dec!(100)),
        ),
        with_settlement(
            sell(
                "sell",
                on(2025, 12, 30),
                "VWCE",
                T212,
                dec!(4),
                eur(dec!(110)),
            ),
            date(2025, 12, 31),
        ),
    ];
    let exchanges = [listed("VWCE", T212, "XETR", "DE")];
    let rates = NbpRateProvider::new(flat(Currency::EUR, dec!(4.5)));

    let data = calculate_stocks(&records, &exchanges, 2025, &rates).unwrap();
    let sale = sale(&data, "sell");

    assert_eq!(sale.settlement_date, date(2025, 12, 31));
    assert!(!sale.settlement_worked_out);
    assert_eq!(sale.nbp_date, date(2025, 12, 30));
}

#[test]
fn records_after_tax_year_are_ignored() {
    let records = [
        buy("2025", on(2025, 3, 3), "INTC", T212, dec!(5), usd(dec!(10))),
        buy("2026", on(2026, 1, 5), "INTC", T212, dec!(3), usd(dec!(12))),
        sell("sell", on(2026, 2, 2), "INTC", T212, dec!(2), usd(dec!(15))),
    ];

    let data = calculate_stocks(&records, &[], 2025, &usd_at_4()).unwrap();

    assert!(data.sales.is_empty());
    assert_eq!(data.income, Decimal::ZERO);
    let [lot] = &data.open_lots[..] else {
        panic!("expected one open lot");
    };
    assert_eq!((lot.buy_id.as_str(), lot.open), ("2025", dec!(5)));
}

#[test]
fn holding_after_each_record() {
    let records = [
        buy(
            "buy-1",
            on(2024, 1, 10),
            "NVDA",
            T212,
            dec!(3),
            usd(dec!(433.44)),
        ),
        split("split", on(2024, 6, 10), "NVDA", 1, 10),
        buy(
            "rev-buy",
            on(2024, 8, 1),
            "NVDA",
            REVOLUT,
            dec!(1),
            usd(dec!(110)),
        ),
        buy(
            "buy-2",
            on(2024, 9, 6),
            "NVDA",
            T212,
            dec!(10),
            usd(dec!(106.21)),
        ),
        sell(
            "sell",
            on(2025, 1, 7),
            "NVDA",
            T212,
            dec!(20),
            usd(dec!(140)),
        ),
    ];

    let replay = replay(&records, &[], &usd_at_4()).unwrap();
    let holding = |record_id: &str, provider: &str| {
        let holding = replay
            .holdings
            .iter()
            .find(|holding| holding.record_id == record_id && holding.provider == provider)
            .unwrap_or_else(|| panic!("no holding for {record_id} in {provider}"));
        (holding.shares, holding.lots)
    };

    assert_eq!(holding("buy-1", T212), (dec!(3), 1));
    assert_eq!(holding("split", T212), (dec!(30), 1));
    assert_eq!(holding("buy-2", T212), (dec!(40), 2));
    assert_eq!(holding("sell", T212), (dec!(20), 2));
    // Revolut buys only after the split, but still gets a row for it.
    assert_eq!(holding("split", REVOLUT), (dec!(0), 0));
    assert_eq!(holding("rev-buy", REVOLUT), (dec!(1), 1));
}

#[test]
fn sale_without_exchange_warns_and_stays_out_of_countries() {
    let records = [
        buy(
            "aapl-buy",
            on(2025, 2, 3),
            "AAPL",
            T212,
            dec!(1),
            usd(dec!(100)),
        ),
        sell(
            "aapl-sell",
            on(2025, 5, 5),
            "AAPL",
            T212,
            dec!(1),
            usd(dec!(110)),
        ),
        buy(
            "ko-buy",
            on(2025, 2, 3),
            "KO",
            REVOLUT,
            dec!(2),
            usd(dec!(50)),
        ),
        sell(
            "ko-sell-1",
            on(2025, 5, 5),
            "KO",
            REVOLUT,
            dec!(1),
            usd(dec!(60)),
        ),
        sell(
            "ko-sell-2",
            on(2025, 6, 2),
            "KO",
            REVOLUT,
            dec!(1),
            usd(dec!(60)),
        ),
    ];
    let exchanges = [listed("AAPL", T212, "XNAS", "US")];

    let data = calculate_stocks(&records, &exchanges, 2025, &usd_at_4()).unwrap();

    assert_eq!(data.income, dec!(920));
    assert_eq!(sale(&data, "ko-sell-1").exchange, None);
    let [
        Warning::MissingExchange {
            ticker,
            provider,
            sales,
        },
    ] = &data.warnings[..]
    else {
        panic!("expected one missing exchange warning");
    };
    assert_eq!(
        (ticker.as_str(), provider.as_str(), *sales),
        ("KO", REVOLUT, 2)
    );
    let [us] = &data.countries[..] else {
        panic!("expected one country");
    };
    assert_eq!(us.sales, 1);
}

#[test]
fn rates_come_from_working_day_before_each_settlement() {
    // Buy Thursday 06.02 settles Friday 07.02; sell Friday 07.03 settles Monday 10.03.
    let records = [
        with_fee(
            buy("buy", on(2025, 2, 6), "AAPL", T212, dec!(2), usd(dec!(100))),
            eur(dec!(1)),
        ),
        with_fee(
            sell(
                "sell",
                on(2025, 3, 7),
                "AAPL",
                T212,
                dec!(2),
                usd(dec!(120)),
            ),
            usd(dec!(0.12)),
        ),
    ];
    let rates = NbpRateProvider::new(vec![
        rate(date(2025, 2, 5), Currency::USD, dec!(9.99)),
        rate(date(2025, 2, 6), Currency::USD, dec!(4.05)),
        rate(date(2025, 2, 6), Currency::EUR, dec!(4.30)),
        rate(date(2025, 2, 7), Currency::USD, dec!(9.99)),
        rate(date(2025, 3, 6), Currency::USD, dec!(9.99)),
        rate(date(2025, 3, 7), Currency::USD, dec!(3.85)),
        rate(date(2025, 3, 10), Currency::USD, dec!(9.99)),
    ]);

    let data = calculate_stocks(&records, &[], 2025, &rates).unwrap();
    let sale = sale(&data, "sell");
    let lot = &sale.matches[0];

    assert_eq!((lot.nbp_date, lot.nbp_rate), (date(2025, 2, 6), dec!(4.05)));
    assert_eq!(lot.cost, dec!(810));
    assert_eq!(lot.fee_part, dec!(4.30));
    assert_eq!(
        (sale.nbp_date, sale.nbp_rate),
        (date(2025, 3, 7), dec!(3.85))
    );
    assert_eq!(sale.income, dec!(924));
    assert_eq!(sale.fee_pln, dec!(0.462));
    assert_eq!(sale.costs, dec!(814.762));
    assert_eq!(sale.profit, dec!(109.238));
}

#[test]
fn missing_rate_is_an_error() {
    let price = [
        buy("buy", on(2025, 2, 3), "AAPL", T212, dec!(1), usd(dec!(100))),
        sell(
            "sell",
            on(2025, 5, 5),
            "AAPL",
            T212,
            dec!(1),
            usd(dec!(110)),
        ),
    ];
    let error = calculate_stocks(&price, &[], 2025, &NbpRateProvider::new(Vec::new())).err();
    assert!(matches!(
        error,
        Some(CalculateStocksError::PriceConversion(_))
    ));

    let fee = [with_fee(
        buy("buy", on(2025, 2, 3), "AAPL", T212, dec!(1), usd(dec!(100))),
        eur(dec!(1)),
    )];
    let error = calculate_stocks(&fee, &[], 2025, &usd_at_4()).err();
    assert!(matches!(
        error,
        Some(CalculateStocksError::FeeConversion(_))
    ));

    let tax_paid = [
        buy("buy", on(2025, 2, 3), "AAPL", T212, dec!(1), usd(dec!(100))),
        with_tax_paid(
            sell(
                "sell",
                on(2025, 5, 5),
                "AAPL",
                T212,
                dec!(1),
                usd(dec!(110)),
            ),
            eur(dec!(1)),
        ),
    ];
    let error = calculate_stocks(&tax_paid, &[], 2025, &usd_at_4()).err();
    assert!(matches!(
        error,
        Some(CalculateStocksError::TaxPaidConversion(_))
    ));
}

#[test]
fn trade_total_is_the_value_over_shares_times_price() {
    let records = [
        with_amount(
            buy(
                "buy",
                on(2025, 2, 3),
                "AAPL",
                T212,
                dec!(3),
                usd(dec!(33.333)),
            ),
            dec!(100),
        ),
        with_amount(
            sell("sell", on(2025, 5, 5), "AAPL", T212, dec!(3), usd(dec!(40))),
            dec!(120.01),
        ),
    ];

    let data = calculate_stocks(&records, &[], 2025, &usd_at_4()).unwrap();
    let sale = sale(&data, "sell");

    assert_eq!(sale.value.value, dec!(120.01));
    assert!(sale.from_total);
    assert_eq!(sale.income, dec!(480.04));
    assert!(sale.matches[0].from_total);
    assert_eq!(sale.lots_cost, dec!(400));
}

#[test]
fn records_at_one_time_go_split_buy_sell() {
    let at = on(2025, 6, 2);
    let records = [
        buy(
            "early",
            on(2025, 2, 3),
            "NVDA",
            T212,
            dec!(2),
            usd(dec!(100)),
        ),
        sell("sell", at, "NVDA", T212, dec!(5), usd(dec!(60))),
        buy("buy", at, "NVDA", T212, dec!(1), usd(dec!(50))),
        split("split", at, "NVDA", 1, 2),
    ];

    let data = calculate_stocks(&records, &[], 2025, &usd_at_4()).unwrap();
    let sale = sale(&data, "sell");

    assert_eq!(sale.uncovered, Decimal::ZERO);
    assert_eq!(
        sale.matches
            .iter()
            .map(|lot| (lot.buy_id.as_str(), lot.taken))
            .collect::<Vec<_>>(),
        [("early", dec!(4)), ("buy", dec!(1))]
    );
    assert!(data.open_lots.is_empty());
}

#[test]
fn preview_replaces_the_edited_record_and_leaves_out_later_ones() {
    let records = [
        buy("buy", on(2025, 2, 3), "MSFT", T212, dec!(10), usd(dec!(10))),
        sell("sell", on(2025, 5, 5), "MSFT", T212, dec!(4), usd(dec!(20))),
        buy(
            "later",
            on(2025, 8, 4),
            "MSFT",
            T212,
            dec!(5),
            usd(dec!(30)),
        ),
        buy(
            "other",
            on(2025, 2, 3),
            "AAPL",
            T212,
            dec!(7),
            usd(dec!(10)),
        ),
    ];

    let edited = sell("sell", on(2025, 5, 5), "MSFT", T212, dec!(6), usd(dec!(20)));
    let Some(StockPreview::Sell { sale, holding }) =
        preview(&records, &edited, &[], &usd_at_4()).unwrap()
    else {
        panic!("expected a sale preview");
    };
    assert_eq!((sale.shares, sale.matches[0].taken), (dec!(6), dec!(6)));
    assert_eq!((holding.shares, holding.lots), (dec!(4), 1));

    let added = buy("new", on(2025, 3, 3), "MSFT", T212, dec!(2), usd(dec!(15)));
    let Some(StockPreview::Buy { lot, holding }) =
        preview(&records, &added, &[], &usd_at_4()).unwrap()
    else {
        panic!("expected a buy preview");
    };
    assert_eq!((lot.buy_id.as_str(), lot.open), ("new", dec!(2)));
    assert_eq!((holding.shares, holding.lots), (dec!(12), 2));
}

#[test]
fn sale_years_follow_settlement() {
    let records = [
        buy(
            "buy",
            on(2025, 6, 2),
            "VWCE",
            T212,
            dec!(10),
            eur(dec!(100)),
        ),
        // Monday 29.12 settles 31.12; Tuesday 30.12 settles 01.01.
        sell(
            "dec-29",
            on(2025, 12, 29),
            "VWCE",
            T212,
            dec!(1),
            eur(dec!(110)),
        ),
        sell(
            "dec-30",
            on(2025, 12, 30),
            "VWCE",
            T212,
            dec!(1),
            eur(dec!(110)),
        ),
        with_settlement(
            sell(
                "recorded",
                on(2024, 12, 30),
                "VWCE",
                T212,
                dec!(1),
                eur(dec!(110)),
            ),
            date(2024, 12, 31),
        ),
    ];
    let exchanges = [listed("VWCE", T212, "XETR", "DE")];

    assert_eq!(
        sale_years(&records, &exchanges)
            .into_iter()
            .collect::<Vec<_>>(),
        [2024, 2025, 2026]
    );
    assert_eq!(sale_year(&records[0], &exchanges), None);
}

// The canvas samples (docs/stocks/03-fifo-calculation.md). Fees in PLN are the Trading 212
// conversion fees as the canvas has them. Rates sit on the NBP date task 2 gives each record.
fn sample_records() -> Vec<Stock> {
    vec![
        // 2025
        with_fee(
            buy(
                "nvda-1",
                on(2023, 8, 17),
                "NVDA",
                T212,
                dec!(3),
                usd(dec!(433.44)),
            ),
            pln(dec!(1.95)),
        ),
        split("nvda-split", on(2024, 6, 10), "NVDA", 1, 10),
        with_fee(
            buy(
                "nvda-2",
                on(2024, 9, 6),
                "NVDA",
                T212,
                dec!(10),
                usd(dec!(106.21)),
            ),
            pln(dec!(1.59)),
        ),
        with_fee(
            sell(
                "nvda-s1",
                on(2025, 1, 7),
                "NVDA",
                T212,
                dec!(20),
                usd(dec!(140.14)),
            ),
            pln(dec!(4.20)),
        ),
        with_fee(
            sell(
                "nvda-s2",
                on(2025, 10, 14),
                "NVDA",
                T212,
                dec!(15),
                usd(dec!(180.03)),
            ),
            pln(dec!(4.05)),
        ),
        buy(
            "vwce-1",
            on(2025, 1, 15),
            "VWCE",
            T212,
            dec!(8),
            eur(dec!(132.48)),
        ),
        buy(
            "vwce-2",
            on(2025, 3, 3),
            "VWCE",
            T212,
            dec!(4),
            eur(dec!(131.10)),
        ),
        sell(
            "vwce-s",
            on(2025, 4, 7),
            "VWCE",
            T212,
            dec!(10),
            eur(dec!(117.32)),
        ),
        buy(
            "ko-1",
            on(2023, 6, 14),
            "KO",
            REVOLUT,
            dec!(3),
            usd(dec!(58.40)),
        ),
        buy(
            "ko-2",
            on(2024, 12, 2),
            "KO",
            REVOLUT,
            dec!(4),
            usd(dec!(62.95)),
        ),
        with_fee(
            sell(
                "ko-s",
                on(2025, 5, 19),
                "KO",
                REVOLUT,
                dec!(7),
                usd(dec!(71.85)),
            ),
            usd(dec!(0.12)),
        ),
        buy(
            "aapl-1",
            on(2024, 3, 14),
            "AAPL",
            T212,
            dec!(6),
            usd(dec!(171.20)),
        ),
        with_fee(
            buy(
                "aapl-2",
                on(2024, 11, 5),
                "AAPL",
                T212,
                dec!(4),
                usd(dec!(222.80)),
            ),
            pln(dec!(1.34)),
        ),
        with_fee(
            buy(
                "aapl-3",
                on(2025, 2, 10),
                "AAPL",
                T212,
                dec!(5),
                usd(dec!(227.65)),
            ),
            pln(dec!(1.71)),
        ),
        with_fee(
            sell(
                "aapl-s",
                on(2025, 7, 22),
                "AAPL",
                T212,
                dec!(12),
                usd(dec!(214.40)),
            ),
            pln(dec!(3.86)),
        ),
        // 2026
        with_fee(
            buy(
                "et-1",
                on(2025, 4, 14),
                "ET",
                IBKR,
                dec!(200),
                usd(dec!(16.80)),
            ),
            usd(dec!(1.00)),
        ),
        with_tax_paid(
            with_fee(
                sell(
                    "et-s",
                    on(2026, 9, 2),
                    "ET",
                    IBKR,
                    dec!(200),
                    usd(dec!(17.95)),
                ),
                usd(dec!(1.00)),
            ),
            usd(dec!(359.00)),
        ),
        // 2024, fractional; the canvas has no buy dates, so these are made up.
        with_fee(
            buy(
                "nto-1",
                on(2024, 2, 5),
                "NTO",
                T212,
                dec!(0.7351942011),
                eur(dec!(49.82)),
            ),
            pln(dec!(0.16)),
        ),
        with_fee(
            buy(
                "nto-2",
                on(2024, 3, 4),
                "NTO",
                T212,
                dec!(1.1682411903),
                eur(dec!(51.36)),
            ),
            pln(dec!(0.26)),
        ),
        with_fee(
            buy(
                "nto-3",
                on(2024, 4, 8),
                "NTO",
                T212,
                dec!(0.9137021455),
                eur(dec!(48.90)),
            ),
            pln(dec!(0.19)),
        ),
        with_fee(
            sell(
                "nto-s1",
                on(2024, 8, 19),
                "NTO",
                T212,
                dec!(1.5),
                eur(dec!(52.14)),
            ),
            pln(dec!(0.34)),
        ),
        with_fee(
            sell(
                "nto-s2",
                on(2024, 11, 25),
                "NTO",
                T212,
                dec!(1.3171375369),
                eur(dec!(47.60)),
            ),
            pln(dec!(0.27)),
        ),
    ]
}

fn sample_rates() -> NbpRateProvider {
    use Currency::{EUR, USD};

    NbpRateProvider::new(vec![
        rate(date(2023, 6, 15), USD, dec!(4.1288)),
        rate(date(2023, 8, 18), USD, dec!(4.0731)),
        rate(date(2024, 3, 15), USD, dec!(3.9512)),
        rate(date(2024, 9, 6), USD, dec!(3.8456)),
        rate(date(2024, 11, 5), USD, dec!(4.0172)),
        rate(date(2024, 12, 2), USD, dec!(4.0794)),
        rate(date(2025, 1, 7), USD, dec!(4.1027)),
        rate(date(2025, 2, 10), USD, dec!(4.0581)),
        rate(date(2025, 4, 14), USD, dec!(3.7650)),
        rate(date(2025, 5, 19), USD, dec!(3.7660)),
        rate(date(2025, 7, 22), USD, dec!(3.6420)),
        rate(date(2025, 10, 14), USD, dec!(3.6680)),
        rate(date(2026, 9, 2), USD, dec!(3.6120)),
        rate(date(2024, 2, 6), EUR, dec!(4.3480)),
        rate(date(2024, 3, 5), EUR, dec!(4.3115)),
        rate(date(2024, 4, 9), EUR, dec!(4.2690)),
        rate(date(2024, 8, 20), EUR, dec!(4.2810)),
        rate(date(2024, 11, 26), EUR, dec!(4.3320)),
        rate(date(2025, 1, 16), EUR, dec!(4.2745)),
        rate(date(2025, 3, 4), EUR, dec!(4.1512)),
        rate(date(2025, 4, 8), EUR, dec!(4.2400)),
    ])
}

fn sample_exchanges() -> Vec<InstrumentExchange> {
    vec![
        listed("NVDA", T212, "XNAS", "US"),
        listed("AAPL", T212, "XNAS", "US"),
        listed("KO", REVOLUT, "XNYS", "US"),
        listed("VWCE", T212, "XETR", "DE"),
        listed("NTO", T212, "XETR", "DE"),
        listed("ET", IBKR, "XNYS", "US"),
    ]
}

#[test]
fn sample_2025() {
    let data = calculate_stocks(
        &sample_records(),
        &sample_exchanges(),
        2025,
        &sample_rates(),
    )
    .unwrap();

    assert_eq!(data.income, dec!(37642.91));
    assert_eq!(data.costs, dec!(24211.61));

    let expected = [
        (
            "nvda-s1",
            dec!(11499.04756),
            dec!(3536.388928),
            dec!(7962.658632),
        ),
        ("vwce-s", dec!(4974.368), dec!(5618.73072), dec!(-644.36272)),
        ("ko-s", dec!(1894.1097), dec!(1751.0106), dec!(143.0991)),
        (
            "aapl-s",
            dec!(9370.1376),
            dec!(9492.33821),
            dec!(-122.20061),
        ),
        (
            "nvda-s2",
            dec!(9905.2506),
            dec!(3813.145344),
            dec!(6092.105256),
        ),
    ];
    assert_eq!(data.sales.len(), expected.len());
    for (id, income, costs, profit) in expected {
        let sale = sale(&data, id);
        assert_eq!(
            (sale.income, sale.costs, sale.profit),
            (income, costs, profit),
            "{id}"
        );
    }

    let nvda = &sale(&data, "nvda-s2").matches;
    assert_eq!(
        nvda.iter()
            .map(|lot| (lot.buy_id.as_str(), lot.taken))
            .collect::<Vec<_>>(),
        [("nvda-1", dec!(10)), ("nvda-2", dec!(5))]
    );
    assert_eq!(nvda[1].fee_part, dec!(0.795));

    let open = |ticker: &str| {
        data.open_lots
            .iter()
            .filter(|lot| lot.ticker == ticker)
            .map(|lot| lot.open)
            .sum::<Decimal>()
    };
    assert_eq!(open("AAPL"), dec!(3));
    assert_eq!(open("NVDA"), dec!(5));
    assert_eq!(open("VWCE"), dec!(2));
    assert_eq!(open("KO"), dec!(0));

    let country = |code: &str| {
        data.countries
            .iter()
            .find(|country| country.country.to_string() == code)
            .unwrap_or_else(|| panic!("no country {code}"))
    };
    assert_eq!(country("US").income, dec!(14075.66));
    assert_eq!(country("US").sales, 4);
    assert_eq!(country("DE").profit, dec!(-644.36272));
    assert_eq!(country("DE").income, Decimal::ZERO);
    assert!(data.warnings.is_empty());
}

#[test]
fn sample_2025_without_first_ko_buy() {
    let records = sample_records()
        .into_iter()
        .filter(|record| record.id != "ko-1")
        .collect::<Vec<_>>();

    let data = calculate_stocks(&records, &sample_exchanges(), 2025, &sample_rates()).unwrap();

    assert_eq!(data.costs, dec!(23488.25));
    let ko = sale(&data, "ko-s");
    assert_eq!(ko.uncovered, dec!(3));
    assert_eq!(ko.profit, dec!(866.46486));
    assert_eq!(sold_without_buy(&data, "ko-s"), Some(dec!(3)));
}

#[test]
fn sample_2024_fractional() {
    let data = calculate_stocks(
        &sample_records(),
        &sample_exchanges(),
        2024,
        &sample_rates(),
    )
    .unwrap();

    assert_eq!(data.income, dec!(606.41));
    assert_eq!(data.costs, dec!(609.91));

    let first = sale(&data, "nto-s1");
    assert_eq!(first.income, dec!(334.81701));
    assert_eq!(
        first.matches[1].fee_part,
        dec!(0.1702127175150674020879881657)
    );
    assert_eq!(first.costs, dec!(329.28359561963565940208798817));
    assert_eq!(first.profit, dec!(5.53341438036434059791201183));

    let second = sale(&data, "nto-s2");
    assert_eq!(second.income, dec!(271.59797494889808));
    assert_eq!(second.costs, dec!(280.62504073389017859791201183));
    assert_eq!(second.profit, dec!(-9.02706578499209859791201183));
    assert!(data.open_lots.iter().all(|lot| lot.ticker != "NTO"));
}

#[test]
fn sample_2026_income_and_costs() {
    let data = calculate_stocks(
        &sample_records(),
        &sample_exchanges(),
        2026,
        &sample_rates(),
    )
    .unwrap();

    assert_eq!(data.income, dec!(12967.08));
    assert_eq!(data.costs, dec!(12657.78));
    assert_eq!(sale(&data, "et-s").tax_paid_pln, dec!(1296.708));
    let [us] = &data.countries[..] else {
        panic!("expected one country");
    };
    assert_eq!(
        (us.country, us.sales, us.tax_paid),
        (Country::USA, 1, dec!(1296.71))
    );
}
