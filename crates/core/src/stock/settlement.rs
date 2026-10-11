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
