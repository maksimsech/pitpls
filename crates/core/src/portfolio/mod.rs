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
