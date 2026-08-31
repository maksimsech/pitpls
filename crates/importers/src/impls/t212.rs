use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use pitpls_core::{
    common::{Amount, Country, Currency},
    dividend::Dividend,
    interest::Interest,
};

use crate::{ImportError, Result};

const PROVIDER: &str = "Trading 212";

struct ColumnMap {
    common: CommonColumns,
    dividend: DividendColumns,
    interest: InterestColumns,
}

struct CommonColumns {
    action: usize,
    time: usize,
    id: Option<usize>,
}

struct DividendColumns {
    isin: Option<usize>,
    ticker: Option<usize>,
    shares: Option<usize>,
    price: Option<usize>,
    price_currency: Option<usize>,
    tax: Option<usize>,
    tax_currency: Option<usize>,
}

struct InterestColumns {
    total: Option<usize>,
    total_currency: Option<usize>,
}

impl ColumnMap {
    fn from_header(header: &str) -> Result<Self> {
        let columns = split_row(header);
        let find = |name: &str| {
            columns
                .iter()
                .position(|c| c == name)
                .ok_or_else(|| ImportError::MissingColumn(name.to_string()))
        };
        let find_optional = |name: &str| columns.iter().position(|c| c == name);

        Ok(Self {
            common: CommonColumns {
                action: find("Action")?,
                time: find("Time")?,
                id: find_optional("ID"),
            },
            dividend: DividendColumns {
                isin: find_optional("ISIN"),
                ticker: find_optional("Ticker"),
                shares: find_optional("No. of shares"),
                price: find_optional("Price / share"),
                price_currency: find_optional("Currency (Price / share)"),
                tax: find_optional("Withholding tax"),
                tax_currency: find_optional("Currency (Withholding tax)"),
            },
            interest: InterestColumns {
                total: find_optional("Total"),
                total_currency: find_optional("Currency (Total)"),
            },
        })
    }
}

enum RowKind {
    Dividend,
    Interest,
    Stock,
    Unknown,
}

impl RowKind {
    fn from_action(action: &str) -> Self {
        if action.starts_with("Dividend") {
            Self::Dividend
        } else if action.starts_with("Interest on cash") {
            Self::Interest
        } else if matches!(action, "Market buy" | "Market sell") {
            Self::Stock
        } else {
            Self::Unknown
        }
    }
}

pub fn parse(csv_content: String) -> Result<(Vec<Dividend>, Vec<Interest>)> {
    let mut lines = csv_content.lines();

    let header = lines
        .next()
        .ok_or(ImportError::MissingHeader)?
        .trim_start_matches('\u{feff}');

    if !header.starts_with("Action,") {
        return Err(ImportError::UnexpectedHeader(header.to_string()));
    }

    let columns = ColumnMap::from_header(header)?;

    let mut dividends = Vec::new();
    let mut interests = Vec::new();

    for line in lines {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        let fields = split_row(line);
        let action = required_indexed_field(&fields, columns.common.action, line)?;

        match RowKind::from_action(action) {
            RowKind::Dividend => dividends.push(parse_dividend_row(&fields, &columns, line)?),
            RowKind::Interest => interests.push(parse_interest_row(&fields, &columns, line)?),
            RowKind::Stock | RowKind::Unknown => {}
        }
    }

    Ok((dividends, interests))
}

fn parse_interest_row(fields: &[String], columns: &ColumnMap, row: &str) -> Result<Interest> {
    let timestamp = required_indexed_field(fields, columns.common.time, row)?;
    let total = required_field(fields, columns.interest.total, "Total", row)?;
    let total_currency = required_field(
        fields,
        columns.interest.total_currency,
        "Currency (Total)",
        row,
    )?;

    Ok(Interest {
        id: row_id(fields, columns.common.id),
        date: parse_date(timestamp)?,
        value: parse_amount(total, total_currency)?,
        provider: PROVIDER.to_string(),
    })
}

fn parse_dividend_row(fields: &[String], columns: &ColumnMap, row: &str) -> Result<Dividend> {
    let timestamp = required_indexed_field(fields, columns.common.time, row)?;
    let isin = required_field(fields, columns.dividend.isin, "ISIN", row)?;
    let ticker = required_field(fields, columns.dividend.ticker, "Ticker", row)?;
    let shares = required_field(fields, columns.dividend.shares, "No. of shares", row)?;
    let price = required_field(fields, columns.dividend.price, "Price / share", row)?;
    let price_currency = required_field(
        fields,
        columns.dividend.price_currency,
        "Currency (Price / share)",
        row,
    )?;
    let tax = required_field(fields, columns.dividend.tax, "Withholding tax", row)?;
    let tax_currency = required_field(
        fields,
        columns.dividend.tax_currency,
        "Currency (Withholding tax)",
        row,
    )?;

    let date = parse_date(timestamp)?;
    let country =
        Country::from_isin(isin).map_err(|source| ImportError::invalid_isin(isin, source))?;
    let value_currency = parse_currency(price_currency)?;
    let tax_currency = parse_currency(tax_currency)?;

    if value_currency != tax_currency {
        return Err(ImportError::other(format!(
            "price and tax currency mismatch: {price_currency} vs {tax_currency}"
        )));
    }

    let tax_value = parse_decimal(tax)?;

    let value = Amount {
        value: parse_decimal(shares)? * parse_decimal(price)? + tax_value,
        currency: value_currency,
    };

    let tax_paid = Amount {
        value: tax_value,
        currency: tax_currency,
    };

    Ok(Dividend {
        id: row_id(fields, columns.common.id),
        date,
        ticker: ticker.to_owned(),
        value,
        tax_paid,
        country,
        provider: PROVIDER.to_owned(),
    })
}

fn required_field<'a>(
    fields: &'a [String],
    index: Option<usize>,
    name: &str,
    row: &str,
) -> Result<&'a str> {
    let index = index.ok_or_else(|| ImportError::MissingColumn(name.to_string()))?;
    required_indexed_field(fields, index, row)
}

fn required_indexed_field<'a>(fields: &'a [String], index: usize, row: &str) -> Result<&'a str> {
    fields
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| ImportError::malformed_row(index + 1, fields.len(), row))
}

fn row_id(fields: &[String], id: Option<usize>) -> String {
    id.and_then(|i| fields.get(i))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

fn parse_date(timestamp: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(timestamp.get(..10).unwrap_or(""), "%Y-%m-%d")
        .map_err(|source| ImportError::invalid_timestamp(timestamp, source))
}

fn parse_amount(value: &str, currency: &str) -> Result<Amount> {
    Ok(Amount {
        value: parse_decimal(value)?,
        currency: parse_currency(currency)?,
    })
}

fn parse_currency(currency: &str) -> Result<Currency> {
    Currency::from_str(currency).map_err(|source| ImportError::invalid_currency(currency, source))
}

fn parse_decimal(s: &str) -> Result<Decimal> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(Decimal::ZERO);
    }
    Decimal::from_str(trimmed).map_err(|source| ImportError::invalid_decimal(s, source))
}

fn split_row(line: &str) -> Vec<String> {
    let mut parts = Vec::with_capacity(13);
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }

    parts.push(current);
    parts
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use pitpls_core::common::Country;

    use super::*;

    fn parse_fixture(content: &str) -> (Vec<Dividend>, Vec<Interest>) {
        parse(content.to_owned()).expect("fixture should parse")
    }

    #[test]
    fn parses_dividents_interest_stock_format() {
        let (dividends, interests) = parse_fixture(include_str!(
            "../../tests/fixtures/t212/dividents_interest_stock.csv"
        ));

        assert_eq!(dividends.len(), 2);
        assert_eq!(interests.len(), 2);

        assert_first_dividend(&dividends[0]);
        assert_first_interest(&interests[0]);
    }

    #[test]
    fn parses_dividents_interest_format() {
        let (dividends, interests) = parse_fixture(include_str!(
            "../../tests/fixtures/t212/dividents_interest.csv"
        ));

        assert_eq!(dividends.len(), 2);
        assert_eq!(interests.len(), 2);

        assert_first_dividend(&dividends[0]);
        assert_first_interest(&interests[0]);
    }

    #[test]
    fn parses_dividends_only_format() {
        let (dividends, interests) =
            parse_fixture(include_str!("../../tests/fixtures/t212/dividends_only.csv"));

        assert_eq!(dividends.len(), 2);
        assert_eq!(interests.len(), 0);
        assert_first_dividend(&dividends[0]);
    }

    #[test]
    fn parses_interest_only_format() {
        let (dividends, interests) =
            parse_fixture(include_str!("../../tests/fixtures/t212/interest_only.csv"));

        assert_eq!(dividends.len(), 0);
        assert_eq!(interests.len(), 2);
        assert_first_interest(&interests[0]);
    }

    fn assert_first_dividend(dividend: &Dividend) {
        let expected_value = Decimal::from_str("34.8301912300").unwrap()
            * Decimal::from_str("0.204000").unwrap()
            + Decimal::from_str("1.25").unwrap();

        assert_eq!(dividend.date, NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
        assert_eq!(dividend.ticker, "st1");
        assert_eq!(dividend.value.value, expected_value);
        assert_eq!(dividend.value.currency.as_str(), "USD");
        assert_eq!(dividend.tax_paid.value, Decimal::from_str("1.25").unwrap());
        assert_eq!(dividend.tax_paid.currency.as_str(), "USD");
        assert_eq!(dividend.country, Country::USA);
        assert_eq!(dividend.provider, PROVIDER);
    }

    fn assert_first_interest(interest: &Interest) {
        assert_eq!(interest.id, "interest-1");
        assert_eq!(interest.date, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(interest.value.value, Decimal::from_str("0.02").unwrap());
        assert_eq!(interest.value.currency.as_str(), "EUR");
        assert_eq!(interest.provider, PROVIDER);
    }
}
