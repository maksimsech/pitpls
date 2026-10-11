use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Warsaw;

pub fn local(time: DateTime<Utc>) -> NaiveDateTime {
    time.with_timezone(&Warsaw).naive_local()
}

// Polish tax law counts income on the Polish calendar day.
pub fn tax_date(time: DateTime<Utc>) -> NaiveDate {
    local(time).date()
}

pub fn from_local(local: NaiveDateTime) -> Option<DateTime<Utc>> {
    Warsaw
        .from_local_datetime(&local)
        .earliest()
        .map(|time| time.with_timezone(&Utc))
}

pub fn start_of_day(date: NaiveDate) -> Option<DateTime<Utc>> {
    from_local(date.and_time(NaiveTime::MIN))
}

pub fn year_range(year: i32) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let start = |year| start_of_day(NaiveDate::from_ymd_opt(year, 1, 1)?);
    Some((start(year)?, start(year + 1)?))
}
