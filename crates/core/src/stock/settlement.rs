use chrono::{DateTime, Datelike, Days, NaiveDate, Utc, Weekday};

use crate::{
    common::{Country, Currency, IsinCountryCode},
    time::tax_date,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Market {
    Us,
    Canada,
    Mexico,
    Germany,
    Other,
}

pub fn market(country: Option<Country>, currency: Currency) -> Market {
    match (country.map(|country| country.code()), currency) {
        (Some(IsinCountryCode::US), _) | (None, Currency::USD) => Market::Us,
        (Some(IsinCountryCode::CA), _) | (None, Currency::CAD) => Market::Canada,
        (Some(IsinCountryCode::MX), _) | (None, Currency::MXN) => Market::Mexico,
        (Some(IsinCountryCode::DE), _) => Market::Germany,
        _ => Market::Other,
    }
}

// A recorded date wins; otherwise the date is worked out (true) from the exchange country or currency.
pub fn settles_on(
    time: DateTime<Utc>,
    recorded: Option<NaiveDate>,
    country: Option<Country>,
    currency: Currency,
) -> (NaiveDate, bool) {
    match recorded {
        Some(date) => (date, false),
        None => (settlement_date(time, market(country, currency)), true),
    }
}

// TODO: Do we need there holidays?
pub fn settlement_date(time: DateTime<Utc>, market: Market) -> NaiveDate {
    // The Polish calendar day, like every other date in the return.
    let trade = tax_date(time);
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
        // 27.05.2024 was Memorial Day, so the US moved a day after Canada and Mexico.
        Market::Us if trade >= date(2024, 5, 28) => 1,
        Market::Canada | Market::Mexico if trade >= date(2024, 5, 27) => 1,
        Market::Us | Market::Canada if trade >= date(2017, 9, 5) => 2,
        // Germany was already T+2 when the rest of the EU moved on 06.10.2014. Mexico's move to T+2
        // isn't dated, so it's T+2 for every date, as podatekgieldowy.pl has it.
        Market::Germany | Market::Mexico => 2,
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
        assert_eq!(market(Some(germany()), Currency::USD), Market::Germany);
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
    fn settlement_uses_warsaw_trade_date() {
        // Midnight of Monday 10.02.2025 in Warsaw is still Sunday in UTC.
        assert_eq!(
            settlement_date(at(2025, 2, 9, 23, 0), Market::Us),
            date(2025, 2, 11)
        );
    }

    #[test]
    fn year_end_sale_settles_next_year() {
        // 01.01 is a holiday, so the real date is 02.01.2026. Either way the sale counts in 2026,
        // its settlement year (docs/stocks/IMPORTANT-QUESTIONS.md, Q1).
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
