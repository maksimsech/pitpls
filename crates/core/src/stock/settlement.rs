use chrono::{DateTime, Datelike, Days, NaiveDate, Utc, Weekday};

use crate::common::{Country, Currency};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Market {
    Us,
    Other,
}

pub fn market(country: Option<Country>, currency: Currency) -> Market {
    match (country, currency) {
        (Some(Country::USA), _) | (None, Currency::USD) => Market::Us,
        _ => Market::Other,
    }
}

// TODO: Do we need there holidays?
// The trade date is the UTC date, not the Warsaw tax date: regular hours on US, European and
// Asian exchanges fall on the same UTC date as the local one. US after-hours trades after
// 20:00 New York time land on the next UTC date.
pub fn settlement_date(time: DateTime<Utc>, market: Market) -> NaiveDate {
    let trade = time.date_naive();
    let mut date = trade;
    // Monday to Friday only, so a date that falls on an exchange holiday is a day early.
    for _ in 0..cycle(trade, market) {
        date = date + Days::new(1);
        while matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            date = date + Days::new(1);
        }
    }
    date
}

fn cycle(trade: NaiveDate, market: Market) -> u8 {
    match market {
        Market::Us if trade >= date(2024, 5, 28) => 1,
        Market::Us if trade >= date(2017, 9, 5) => 2,
        Market::Other if trade >= date(2014, 10, 6) => 2,
        _ => 3,
    }
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
    }

    fn germany() -> Country {
        "DE".parse().unwrap()
    }

    #[test]
    fn market_follows_exchange_country_over_currency() {
        assert_eq!(market(Some(Country::USA), Currency::EUR), Market::Us);
        assert_eq!(market(Some(germany()), Currency::USD), Market::Other);
    }

    #[test]
    fn market_falls_back_to_currency_without_exchange() {
        assert_eq!(market(None, Currency::USD), Market::Us);
        assert_eq!(market(None, Currency::EUR), Market::Other);
    }

    #[test]
    fn settlement_skips_weekend() {
        // Friday 24.05.2024, T+2.
        assert_eq!(
            settlement_date(at(2024, 5, 24, 15, 0), Market::Us),
            date(2024, 5, 28)
        );
        // Friday 06.09.2024, T+1.
        assert_eq!(
            settlement_date(at(2024, 9, 6, 15, 0), Market::Us),
            date(2024, 9, 9)
        );
    }

    #[test]
    fn us_cycle_changes_on_switch_days() {
        // T+3 → T+2 from 05.09.2017.
        assert_eq!(
            settlement_date(at(2017, 9, 4, 15, 0), Market::Us),
            date(2017, 9, 7)
        );
        assert_eq!(
            settlement_date(at(2017, 9, 5, 15, 0), Market::Us),
            date(2017, 9, 7)
        );
        // T+2 → T+1 from 28.05.2024.
        assert_eq!(
            settlement_date(at(2024, 5, 27, 15, 0), Market::Us),
            date(2024, 5, 29)
        );
        assert_eq!(
            settlement_date(at(2024, 5, 28, 15, 0), Market::Us),
            date(2024, 5, 29)
        );
    }

    #[test]
    fn other_cycle_changes_on_6_october_2014() {
        // Friday, T+3.
        assert_eq!(
            settlement_date(at(2014, 10, 3, 9, 0), Market::Other),
            date(2014, 10, 8)
        );
        // Monday, T+2.
        assert_eq!(
            settlement_date(at(2014, 10, 6, 9, 0), Market::Other),
            date(2014, 10, 8)
        );
    }

    #[test]
    fn settlement_uses_utc_trade_date_not_warsaw() {
        // 23:30 UTC on Monday 06.01.2025 is already Tuesday in Warsaw.
        assert_eq!(
            settlement_date(at(2025, 1, 6, 23, 30), Market::Us),
            date(2025, 1, 7)
        );
    }

    #[test]
    fn year_end_sale_settles_next_year() {
        // 01.01 is a holiday, so the real date is 02.01.2026; either way the tax year is 2026.
        assert_eq!(
            settlement_date(at(2025, 12, 30, 10, 0), Market::Other),
            date(2026, 1, 1)
        );
    }

    #[test]
    fn matches_handover_examples() {
        let cases = [
            (at(2023, 8, 17, 15, 0), Market::Us, date(2023, 8, 21)),
            (at(2025, 1, 7, 15, 0), Market::Us, date(2025, 1, 8)),
            (at(2025, 2, 10, 15, 0), Market::Us, date(2025, 2, 11)),
            (at(2025, 3, 3, 10, 0), Market::Other, date(2025, 3, 5)),
            (at(2025, 5, 19, 15, 0), Market::Us, date(2025, 5, 20)),
        ];
        for (time, market, settles) in cases {
            assert_eq!(settlement_date(time, market), settles, "trade {time}");
        }
    }
}
