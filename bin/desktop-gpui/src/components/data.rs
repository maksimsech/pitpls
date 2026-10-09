//! What Summary's Data card and the Imports page say about the data: how
//! current the exchange rates are, and which statement was imported last.

use crate::{format::DATE_FORMAT, theme::palette};
use chrono::{Datelike, Days, Local, NaiveDate, Weekday};
use gpui_kit::*;
use pitpls_app::use_case::{import::LastImport, rate::RateCoverage};

/// A card on the raised colour, with a 12px radius.
pub fn card(cx: &App) -> Div {
    open_card(cx).overflow_hidden()
}

/// A [`card`] that doesn't clip its content, so the kit's focus ring, drawn
/// 3px outside a control, shows around full-width rows. A last row with a
/// background rounds its bottom by [`CARD_INNER_RADIUS`] itself.
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

/// A 7px status dot.
pub fn dot(colour: Hsla) -> Div {
    div().size(px(7.)).flex_shrink_0().rounded_full().bg(colour)
}

/// Whether `year` is the calendar year still running.
pub fn is_current_year(year: Option<i32>) -> bool {
    year == Some(Local::now().year())
}

pub struct RateStatus {
    pub dot: Hsla,
    pub detail: StyledText,
    /// There are no rates, or, in the current year, the latest is too old.
    pub stale: bool,
}

/// `lead` and the latest rate's date. The age follows only when `year` is
/// the current year: it only matters while the year is still running.
pub fn rate_status(
    lead: &str,
    coverage: Option<RateCoverage>,
    year: Option<i32>,
    cx: &App,
) -> RateStatus {
    let p = palette(cx);
    let Some(coverage) = coverage else {
        return RateStatus {
            dot: p.warning,
            detail: StyledText::new("No NBP rates yet"),
            stale: true,
        };
    };
    let latest = format!("{lead} {}", coverage.last.format(DATE_FORMAT));
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
    /// Provider · file · counts.
    pub detail: SharedString,
    /// When it was imported, in local time.
    pub date: Option<SharedString>,
}

pub fn import_status(last: Option<&LastImport>, cx: &App) -> ImportStatus {
    let p = palette(cx);
    match last {
        Some(last) => ImportStatus {
            dot: p.ok,
            detail: format!(
                "{} · {} · {}",
                last.provider,
                last.file_name,
                import_counts(last)
            )
            .into(),
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
            detail: "No statement imported yet".into(),
            date: None,
        },
    }
}

/// The imported records, without the kinds the file had none of.
fn import_counts(last: &LastImport) -> String {
    let counts = [
        (last.dividends, "dividend", "dividends"),
        (last.interests, "interest", "interest"),
        (last.cryptos, "crypto record", "crypto records"),
    ]
    .into_iter()
    .filter(|(count, _, _)| *count > 0)
    .map(|(count, one, many)| format!("{count} {}", if count == 1 { one } else { many }))
    .collect::<Vec<_>>();
    if counts.is_empty() {
        "no records".to_owned()
    } else {
        counts.join(", ")
    }
}

/// How old the latest rate is, and whether it is stale: older than the
/// last working day before `today`, whose rate a record dated today needs.
/// Weekends are skipped; public holidays aren't known, so the day after one
/// shows as stale until NBP publishes again.
fn rate_age(latest: NaiveDate, today: NaiveDate) -> (String, bool) {
    let days = (today - latest).num_days().max(0);
    let age = match days {
        0 => "today".to_owned(),
        1 => "1 day old".to_owned(),
        days => format!("{days} days old"),
    };
    let mut expected = today - Days::new(1);
    while matches!(expected.weekday(), Weekday::Sat | Weekday::Sun) {
        expected = expected - Days::new(1);
    }
    (age, latest < expected)
}
