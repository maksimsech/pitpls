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

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
    }

    fn local_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
    }

    #[test]
    fn tax_date_is_warsaw_day() {
        assert_eq!(
            tax_date(utc(2025, 12, 31, 23, 30)),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
        assert_eq!(
            tax_date(utc(2025, 6, 30, 22, 30)),
            NaiveDate::from_ymd_opt(2025, 7, 1).unwrap()
        );
        assert_eq!(
            tax_date(utc(2025, 6, 30, 21, 30)),
            NaiveDate::from_ymd_opt(2025, 6, 30).unwrap()
        );
    }

    #[test]
    fn year_range_is_warsaw_midnight() {
        assert_eq!(
            year_range(2025),
            Some((utc(2024, 12, 31, 23, 0), utc(2025, 12, 31, 23, 0)))
        );
    }

    #[test]
    fn from_local_handles_daylight_saving_changes() {
        // 30.03.2025 skips 02:00–03:00; 26.10.2025 has 02:00–03:00 twice.
        assert_eq!(from_local(local_time(2025, 3, 30, 2, 30)), None);
        assert_eq!(
            from_local(local_time(2025, 10, 26, 2, 30)),
            Some(utc(2025, 10, 26, 0, 30))
        );
    }
}
