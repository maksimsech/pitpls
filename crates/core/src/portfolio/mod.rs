use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

use crate::{
    common::{Amount, Currency},
    crypto::{Action, CalculatedCrypto},
    dividend::CalculatedDividend,
    interest::CalculatedInterest,
    stock::{CalculatedSale, OpenLot},
    time::tax_date,
};

mod model;

pub use model::{
    CryptoTotals, CryptoYear, DividendPayer, DividendYear, EarnedYear, InterestMonth,
    InterestProvider, InterestYear, Last12Months, Position, RealizedInstrument, RealizedYear,
};

pub fn positions(open_lots: &[OpenLot]) -> Vec<Position> {
    let mut groups: BTreeMap<_, Vec<&OpenLot>> = BTreeMap::new();
    for lot in open_lots.iter().filter(|lot| lot.open > Decimal::ZERO) {
        groups
            .entry((
                lot.ticker.as_str(),
                lot.provider.as_str(),
                lot.price.currency,
            ))
            .or_default()
            .push(lot);
    }

    let mut positions: Vec<_> = groups
        .into_iter()
        .map(|((ticker, provider, currency), lots)| {
            let shares: Decimal = lots.iter().map(|lot| lot.open).sum();
            let weighted_price: Decimal = lots.iter().map(|lot| lot.open * lot.price.value).sum();
            Position {
                ticker: ticker.to_owned(),
                provider: provider.to_owned(),
                currency,
                shares,
                avg_price: weighted_price / shares,
                cost: lots.iter().map(|lot| lot.cost + lot.fee_part).sum(),
                since: lots.iter().map(|lot| lot.bought).min().unwrap(),
                lots: lots.len(),
            }
        })
        .collect();
    positions.sort_by(|a, b| {
        b.cost.cmp(&a.cost).then_with(|| {
            (&a.ticker, &a.provider, a.currency.as_str()).cmp(&(
                &b.ticker,
                &b.provider,
                b.currency.as_str(),
            ))
        })
    });
    positions
}

pub fn realized_by_year(sales: &[CalculatedSale]) -> Vec<RealizedYear> {
    let mut years = BTreeMap::new();
    for sale in sales {
        let year = sale.settlement_date.year();
        let sum = years.entry(year).or_insert_with(|| RealizedYear {
            year,
            ..Default::default()
        });
        sum.sales += 1;
        sum.income += sale.income;
        sum.costs += sale.costs;
        sum.profit += sale.profit;
    }
    years.into_values().collect()
}

pub fn realized_by_instrument(
    sales: &[CalculatedSale],
    open_lots: &[OpenLot],
) -> Vec<RealizedInstrument> {
    let held: BTreeSet<_> = open_lots
        .iter()
        .filter(|lot| lot.open > Decimal::ZERO)
        .map(|lot| (lot.ticker.as_str(), lot.provider.as_str()))
        .collect();
    let mut instruments = BTreeMap::new();
    for sale in sales {
        let key = (sale.ticker.as_str(), sale.provider.as_str());
        let sum = instruments
            .entry(key)
            .or_insert_with(|| RealizedInstrument {
                ticker: sale.ticker.clone(),
                provider: sale.provider.clone(),
                sales: 0,
                shares: Decimal::ZERO,
                income: Decimal::ZERO,
                costs: Decimal::ZERO,
                profit: Decimal::ZERO,
                first_sale: sale.time,
                last_sale: sale.time,
                held: held.contains(&key),
            });
        sum.sales += 1;
        sum.shares += sale.shares;
        sum.income += sale.income;
        sum.costs += sale.costs;
        sum.profit += sale.profit;
        sum.first_sale = sum.first_sale.min(sale.time);
        sum.last_sale = sum.last_sale.max(sale.time);
    }
    let mut instruments: Vec<_> = instruments.into_values().collect();
    instruments.sort_by(|a, b| {
        b.profit
            .cmp(&a.profit)
            .then_with(|| (&a.ticker, &a.provider).cmp(&(&b.ticker, &b.provider)))
    });
    instruments
}

pub fn dividends_by_year(dividends: &[CalculatedDividend]) -> Vec<DividendYear> {
    let mut years = BTreeMap::new();
    for dividend in dividends {
        let year = tax_date(dividend.time).year();
        let (sum, payers) = years.entry(year).or_insert_with(|| {
            (
                DividendYear {
                    year,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
        });
        sum.payments += 1;
        sum.gross += dividend.calculated_value;
        sum.withheld += dividend.calculated_tax_paid;
        payers.insert((dividend.ticker.as_str(), dividend.provider.as_str()));
    }
    years
        .into_values()
        .map(|(mut sum, payers)| {
            sum.payers = payers.len();
            sum.net = sum.gross - sum.withheld;
            sum
        })
        .collect()
}

pub fn dividends_by_payer(dividends: &[CalculatedDividend]) -> Vec<DividendPayer> {
    let mut groups: BTreeMap<_, Vec<&CalculatedDividend>> = BTreeMap::new();
    for dividend in dividends {
        groups
            .entry((dividend.ticker.as_str(), dividend.provider.as_str()))
            .or_default()
            .push(dividend);
    }
    let mut payers: Vec<_> = groups
        .into_iter()
        .map(|((ticker, provider), payments)| {
            let first = payments.iter().min_by_key(|payment| payment.time).unwrap();
            // An ID breaks ties when country changes share a payment timestamp.
            let latest = payments
                .iter()
                .max_by_key(|payment| (payment.time, &payment.id))
                .unwrap();
            let gross: Decimal = payments
                .iter()
                .map(|payment| payment.calculated_value)
                .sum();
            let withheld: Decimal = payments
                .iter()
                .map(|payment| payment.calculated_tax_paid)
                .sum();
            DividendPayer {
                ticker: ticker.to_owned(),
                provider: provider.to_owned(),
                country: latest.country,
                payments: payments.len(),
                first_year: tax_date(first.time).year(),
                last_year: tax_date(latest.time).year(),
                gross_in_currency: amounts_by_currency(
                    payments.iter().map(|payment| payment.value),
                ),
                gross,
                withheld,
                net: gross - withheld,
            }
        })
        .collect();
    payers.sort_by(|a, b| {
        b.gross
            .cmp(&a.gross)
            .then_with(|| (&a.ticker, &a.provider).cmp(&(&b.ticker, &b.provider)))
    });
    payers
}

pub fn interest_by_month(interests: &[CalculatedInterest]) -> Vec<InterestMonth> {
    let mut months = BTreeMap::new();
    for interest in interests {
        let date = tax_date(interest.time);
        let key = (date.year(), date.month());
        let (sum, providers) = months.entry(key).or_insert_with(|| {
            (
                InterestMonth {
                    year: key.0,
                    month: key.1,
                    ..Default::default()
                },
                BTreeMap::<&str, usize>::new(),
            )
        });
        sum.records += 1;
        sum.value += interest.calculated_value;
        *providers.entry(&interest.provider).or_default() += 1;
    }

    let (Some((&first, _)), Some((&last, _))) = (months.first_key_value(), months.last_key_value())
    else {
        return Vec::new();
    };
    let mut result = Vec::new();
    let mut month = first;
    loop {
        let (mut sum, providers) = months.remove(&month).unwrap_or_else(|| {
            (
                InterestMonth {
                    year: month.0,
                    month: month.1,
                    ..Default::default()
                },
                BTreeMap::new(),
            )
        });
        sum.providers = providers
            .into_iter()
            .map(|(provider, count)| (provider.to_owned(), count))
            .collect();
        result.push(sum);
        if month == last {
            break;
        }
        month = if month.1 == 12 {
            (month.0 + 1, 1)
        } else {
            (month.0, month.1 + 1)
        };
    }
    result
}

pub fn interest_by_year(interests: &[CalculatedInterest]) -> Vec<InterestYear> {
    let mut years = BTreeMap::new();
    let mut totals: BTreeMap<&str, Decimal> = BTreeMap::new();
    for interest in interests {
        let year = tax_date(interest.time).year();
        let (sum, providers) = years.entry(year).or_insert_with(|| {
            (
                InterestYear {
                    year,
                    ..Default::default()
                },
                BTreeMap::<&str, Decimal>::new(),
            )
        });
        sum.records += 1;
        sum.total += interest.calculated_value;
        *providers.entry(&interest.provider).or_default() += interest.calculated_value;
        *totals.entry(&interest.provider).or_default() += interest.calculated_value;
    }
    let mut provider_order: Vec<_> = totals.into_iter().collect();
    provider_order.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    years
        .into_values()
        .map(|(mut sum, providers)| {
            sum.by_provider = provider_order
                .iter()
                .map(|(provider, _)| {
                    (
                        (*provider).to_owned(),
                        providers.get(provider).copied().unwrap_or_default(),
                    )
                })
                .collect();
            sum
        })
        .collect()
}

pub fn interest_by_provider(interests: &[CalculatedInterest]) -> Vec<InterestProvider> {
    let mut groups: BTreeMap<_, Vec<&CalculatedInterest>> = BTreeMap::new();
    for interest in interests {
        groups
            .entry(interest.provider.as_str())
            .or_default()
            .push(interest);
    }
    let mut providers: Vec<_> = groups
        .into_iter()
        .map(|(provider, records)| InterestProvider {
            provider: provider.to_owned(),
            first: records.iter().map(|record| record.time).min().unwrap(),
            last: records.iter().map(|record| record.time).max().unwrap(),
            records: records.len(),
            received_in_currency: amounts_by_currency(records.iter().map(|record| record.value)),
            received: records.iter().map(|record| record.calculated_value).sum(),
        })
        .collect();
    providers.sort_by(|a, b| {
        b.received
            .cmp(&a.received)
            .then_with(|| a.provider.cmp(&b.provider))
    });
    providers
}

pub fn interest_last_12_months(interests: &[CalculatedInterest], today: NaiveDate) -> Last12Months {
    let from = (today.year() - 1, today.month());
    let to = if today.month() == 1 {
        (today.year() - 1, 12)
    } else {
        (today.year(), today.month() - 1)
    };
    let value = interests
        .iter()
        .filter(|interest| {
            let date = tax_date(interest.time);
            let month = (date.year(), date.month());
            from <= month && month <= to
        })
        .map(|interest| interest.calculated_value)
        .sum();
    Last12Months { value, from, to }
}

pub fn crypto_by_year(cryptos: &[CalculatedCrypto]) -> Vec<CryptoYear> {
    let mut years = BTreeMap::new();
    for crypto in cryptos {
        let year = tax_date(crypto.time).year();
        let sum = years.entry(year).or_insert_with(|| CryptoYear {
            year,
            ..Default::default()
        });
        match crypto.action {
            Action::FiatBuy => {
                sum.buys += 1;
                sum.bought += crypto.calculated_value;
            }
            Action::FiatSell => {
                sum.sells += 1;
                sum.sold += crypto.calculated_value;
            }
        }
        sum.fees += crypto.calculated_fee;
    }
    years
        .into_values()
        .map(|mut sum| {
            sum.net = sum.sold - sum.bought - sum.fees;
            sum
        })
        .collect()
}

pub fn crypto_totals(years: &[CryptoYear]) -> CryptoTotals {
    let put_in: Decimal = years.iter().map(|year| year.bought + year.fees).sum();
    let taken_out: Decimal = years.iter().map(|year| year.sold).sum();
    CryptoTotals {
        put_in,
        taken_out,
        net: taken_out - put_in,
    }
}

pub fn earned_by_year(
    realized: &[RealizedYear],
    dividends: &[DividendYear],
    interests: &[InterestYear],
) -> Vec<EarnedYear> {
    let mut years = BTreeMap::new();
    for realized in realized {
        let sum = years.entry(realized.year).or_insert_with(|| EarnedYear {
            year: realized.year,
            ..Default::default()
        });
        sum.realized += realized.profit;
    }
    for dividend in dividends {
        let sum = years.entry(dividend.year).or_insert_with(|| EarnedYear {
            year: dividend.year,
            ..Default::default()
        });
        sum.dividends += dividend.gross;
    }
    for interest in interests {
        let sum = years.entry(interest.year).or_insert_with(|| EarnedYear {
            year: interest.year,
            ..Default::default()
        });
        sum.interest += interest.total;
    }
    years
        .into_values()
        .map(|mut sum| {
            sum.total = sum.realized + sum.dividends + sum.interest;
            sum
        })
        .collect()
}

fn amounts_by_currency(amounts: impl Iterator<Item = Amount>) -> Vec<Amount> {
    let mut totals: BTreeMap<Currency, Decimal> = BTreeMap::new();
    for amount in amounts {
        *totals.entry(amount.currency).or_default() += amount.value;
    }
    let mut totals: Vec<_> = totals
        .into_iter()
        .map(|(currency, value)| Amount { value, currency })
        .collect();
    totals.sort_by_key(|amount| amount.currency.as_str());
    totals
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use rust_decimal::dec;

    use super::*;
    use crate::{
        common::Country,
        dividend::{self, Dividend},
        interest::{self, Interest},
        rate::{NbpRateProvider, Rate},
        settings::DividendRounding,
        stock::{InstrumentExchange, Stock, StockAction, StockBuy, StockSell, replay},
    };

    const T212: &str = "Trading 212";
    const REVOLUT: &str = "Revolut";

    fn on(year: i32, month: u32, day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap()
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

    fn rates() -> NbpRateProvider {
        NbpRateProvider::new(
            NaiveDate::from_ymd_opt(2023, 1, 1)
                .unwrap()
                .iter_days()
                .take_while(|day| day.year() <= 2026)
                .flat_map(|date| {
                    [(Currency::USD, dec!(4)), (Currency::EUR, dec!(4.5))].map(
                        |(currency, rate)| Rate {
                            date,
                            currency,
                            rate,
                        },
                    )
                })
                .collect(),
        )
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

    fn with_fee(mut stock: Stock, fee: Decimal) -> Stock {
        if let StockAction::Buy(buy) = &mut stock.action {
            buy.fee.value = fee;
        }
        stock
    }

    fn interest(time: DateTime<Utc>, value: Decimal, provider: &str) -> Interest {
        Interest {
            id: format!("{provider}-{time}"),
            time,
            value: usd(value),
            provider: provider.to_owned(),
        }
    }

    #[test]
    fn positions_weigh_price_by_open_shares_and_cost_includes_fees() {
        let records = [
            with_fee(
                buy("a1", on(2025, 2, 3), "AAPL", T212, dec!(10), usd(dec!(10))),
                dec!(1),
            ),
            buy("a2", on(2025, 3, 3), "AAPL", T212, dec!(30), usd(dec!(20))),
            sell("a-s", on(2025, 4, 7), "AAPL", T212, dec!(5), usd(dec!(25))),
            buy(
                "m1",
                on(2025, 2, 3),
                "MSFT",
                REVOLUT,
                dec!(1),
                usd(dec!(300)),
            ),
            buy("k1", on(2025, 2, 3), "KO", T212, dec!(2), usd(dec!(60))),
            sell("k-s", on(2025, 4, 7), "KO", T212, dec!(2), usd(dec!(70))),
        ];
        let replay = replay(&records, &[], &rates()).unwrap();

        let positions = positions(&replay.open_lots);

        let [aapl, msft] = &positions[..] else {
            panic!("expected AAPL and MSFT, KO is sold out");
        };
        assert_eq!(
            (aapl.ticker.as_str(), aapl.shares, aapl.lots, aapl.since),
            ("AAPL", dec!(35), 2, on(2025, 2, 3))
        );
        assert_eq!(aapl.avg_price, dec!(650) / dec!(35));
        // 5 × 10 × 4 + half the 4 PLN fee, and 30 × 20 × 4.
        assert_eq!(aapl.cost, dec!(2602));
        assert_eq!((msft.ticker.as_str(), msft.cost), ("MSFT", dec!(1200)));
    }

    #[test]
    fn realized_counts_in_the_settlement_year() {
        let records = [
            buy(
                "buy",
                on(2025, 6, 2),
                "VWCE",
                T212,
                dec!(10),
                eur(dec!(100)),
            ),
            sell("jun", on(2025, 6, 9), "VWCE", T212, dec!(1), eur(dec!(110))),
            // Settles on 01.01.2026.
            sell(
                "dec",
                on(2025, 12, 30),
                "VWCE",
                T212,
                dec!(2),
                eur(dec!(120)),
            ),
        ];
        let exchanges = [InstrumentExchange {
            ticker: "VWCE".to_owned(),
            provider: T212.to_owned(),
            mic: "XETR".parse().unwrap(),
            country: "DE".parse().unwrap(),
        }];
        let replay = replay(&records, &exchanges, &rates()).unwrap();

        let years = realized_by_year(&replay.sales)
            .into_iter()
            .map(|year| (year.year, year.sales, year.profit))
            .collect::<Vec<_>>();

        assert_eq!(years, [(2025, 1, dec!(45)), (2026, 1, dec!(180))]);
    }

    #[test]
    fn interest_months_fill_gaps_in_warsaw_months() {
        let interests = vec![
            interest(on(2024, 12, 5), dec!(1), T212),
            interest(on(2025, 1, 5), dec!(2), T212),
            interest(on(2025, 1, 6), dec!(3), REVOLUT),
            // 01.04 in Warsaw.
            interest(
                Utc.with_ymd_and_hms(2025, 3, 31, 22, 30, 0).unwrap(),
                dec!(4),
                T212,
            ),
        ];
        let data = interest::calculate(interests, &rates()).unwrap();

        let months = interest_by_month(&data.calculated)
            .into_iter()
            .map(|month| (month.year, month.month, month.records, month.value))
            .collect::<Vec<_>>();

        assert_eq!(
            months,
            [
                (2024, 12, 1, dec!(4)),
                (2025, 1, 2, dec!(20)),
                (2025, 2, 0, dec!(0)),
                (2025, 3, 0, dec!(0)),
                (2025, 4, 1, dec!(16)),
            ]
        );
    }

    #[test]
    fn last_12_months_leave_out_the_current_month() {
        let interests = vec![
            interest(on(2025, 9, 15), dec!(1), T212),
            interest(on(2025, 10, 15), dec!(2), T212),
            interest(on(2026, 9, 15), dec!(3), T212),
            interest(on(2026, 10, 15), dec!(4), T212),
        ];
        let data = interest::calculate(interests, &rates()).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();

        let last = interest_last_12_months(&data.calculated, today);

        assert_eq!(
            (last.from, last.to, last.value),
            ((2025, 10), (2026, 9), dec!(20))
        );
        let january = interest_last_12_months(
            &data.calculated,
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
        );
        assert_eq!((january.from, january.to), ((2025, 1), (2025, 12)));
    }

    #[test]
    fn payer_takes_the_latest_country_and_net_is_after_withholding() {
        let dividend = |id: &str, time, country| Dividend {
            id: id.to_owned(),
            time,
            ticker: "X".to_owned(),
            value: usd(dec!(10)),
            tax_paid: usd(dec!(1)),
            country,
            provider: T212.to_owned(),
        };
        let dividends = vec![
            dividend("old", on(2024, 3, 1), Country::USA),
            dividend("new", on(2025, 3, 1), "IE".parse().unwrap()),
        ];
        let data = dividend::calculate(dividends, &rates(), DividendRounding::SumToGroszy).unwrap();

        let [payer] = &dividends_by_payer(&data.calculated)[..] else {
            panic!("expected one payer");
        };

        assert_eq!(payer.country.to_string(), "IE");
        assert_eq!(
            (payer.payments, payer.first_year, payer.last_year),
            (2, 2024, 2025)
        );
        assert_eq!(
            (payer.gross, payer.withheld, payer.net),
            (dec!(80), dec!(8), dec!(72))
        );
        assert_eq!(payer.gross_in_currency[0].value, dec!(20));
    }

    #[test]
    fn earned_years_merge_every_source() {
        let realized = [RealizedYear {
            year: 2024,
            profit: dec!(100),
            ..Default::default()
        }];
        let dividends = [DividendYear {
            year: 2025,
            gross: dec!(50),
            ..Default::default()
        }];
        let interests = [
            InterestYear {
                year: 2024,
                total: dec!(10),
                ..Default::default()
            },
            InterestYear {
                year: 2025,
                total: dec!(5),
                ..Default::default()
            },
        ];

        let years = earned_by_year(&realized, &dividends, &interests)
            .into_iter()
            .map(|year| {
                (
                    year.year,
                    year.realized,
                    year.dividends,
                    year.interest,
                    year.total,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            years,
            [
                (2024, dec!(100), dec!(0), dec!(10), dec!(110)),
                (2025, dec!(0), dec!(50), dec!(5), dec!(55)),
            ]
        );
    }
}
