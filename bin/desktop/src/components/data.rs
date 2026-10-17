use crate::{config::LastImport, format::DATE_FORMAT, i18n::plural, theme::palette};
use chrono::{Datelike, Days, Local, NaiveDate, Weekday};
use gpui_kit::*;
use pitpls_app::use_case::rate::RateCoverage;
use rust_i18n::t;

pub fn card(cx: &App) -> Div {
    open_card(cx).overflow_hidden()
}

/// Unclipped, so the kit's focus ring, drawn outside a control, shows around
/// full-width rows. A last row with a background rounds its own bottom by
/// [`CARD_INNER_RADIUS`].
pub fn open_card(cx: &App) -> Div {
    let p = palette(cx);
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(p.line)
        .rounded(px(12.))
        .bg(p.raised)
}

/// The card's radius inside its 1px border.
pub const CARD_INNER_RADIUS: Pixels = px(11.);

pub fn dot(colour: Hsla) -> Div {
    div().size(px(7.)).flex_shrink_0().rounded_full().bg(colour)
}

pub fn is_current_year(year: Option<i32>) -> bool {
    year == Some(Local::now().year())
}

pub struct RateStatus {
    pub dot: Hsla,
    pub detail: StyledText,
    pub stale: bool,
}

/// The age shows only for the current year: it only matters while the year is
/// still running. `latest` is a message key with the rate's `%{date}`.
pub fn rate_status(
    latest: &str,
    coverage: Option<RateCoverage>,
    year: Option<i32>,
    cx: &App,
) -> RateStatus {
    let p = palette(cx);
    let Some(coverage) = coverage else {
        return RateStatus {
            dot: p.warning,
            detail: StyledText::new(t!("data.no_rates")),
            stale: true,
        };
    };
    let latest = t!(latest, date = coverage.last.format(DATE_FORMAT)).into_owned();
    if !is_current_year(year) {
        return RateStatus {
            dot: p.ok,
            detail: StyledText::new(latest),
            stale: false,
        };
    }
    let (age, stale) = rate_age(coverage.last, Local::now().date_naive());
    let colour = if stale { p.warning } else { p.text };
    let text = format!("{latest} · {age}");
    let start = latest.len() + " · ".len();
    RateStatus {
        dot: if stale { p.warning } else { p.ok },
        detail: StyledText::new(text.clone())
            .with_highlights([(start..text.len(), HighlightStyle::color(colour))]),
        stale,
    }
}

pub struct ImportStatus {
    pub dot: Hsla,
    pub source: SharedString,
    pub counts: Option<SharedString>,
    pub date: Option<SharedString>,
}

pub fn import_status(last: Option<&LastImport>, cx: &App) -> ImportStatus {
    let p = palette(cx);
    match last {
        Some(last) => ImportStatus {
            dot: p.ok,
            source: format!("{} · {}", last.provider, last.file_name).into(),
            counts: Some(import_counts(last).into()),
            date: Some(
                last.imported_at
                    .with_timezone(&Local)
                    .format(DATE_FORMAT)
                    .to_string()
                    .into(),
            ),
        },
        None => ImportStatus {
            dot: p.faint,
            source: t!("data.no_import").into(),
            counts: None,
            date: None,
        },
    }
}

// Statement file names are long; they give way so the counts stay readable.
pub fn import_detail(status: &ImportStatus) -> Div {
    div()
        .flex()
        .gap(px(4.))
        .child(div().min_w_0().truncate().child(status.source.clone()))
        .children(status.counts.clone().map(|counts| {
            div()
                .flex_shrink_0()
                .child(SharedString::from(format!("· {counts}")))
        }))
}

fn import_counts(last: &LastImport) -> String {
    let counts = [
        (last.dividends, "count.dividends"),
        (last.interests, "count.interests"),
        (last.cryptos, "count.cryptos"),
        (last.stocks, "count.stocks"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, key)| plural(key, count))
    .collect::<Vec<_>>();
    if counts.is_empty() {
        t!("records.none").into_owned()
    } else {
        counts.join(", ")
    }
}

/// Stale means older than the last working day before `today`, whose rate a
/// record dated today needs. Public holidays aren't known, so the day after one
/// shows as stale until NBP publishes again.
fn rate_age(latest: NaiveDate, today: NaiveDate) -> (String, bool) {
    let days = (today - latest).num_days().max(0);
    let age = match days {
        0 => t!("data.rate_today").into_owned(),
        days => plural("data.rate_age", days as u64).into_owned(),
    };
    let mut expected = today - Days::new(1);
    while matches!(expected.weekday(), Weekday::Sat | Weekday::Sun) {
        expected = expected - Days::new(1);
    }
    (age, latest < expected)
}
