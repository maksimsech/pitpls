use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use chrono::NaiveDate;
use rust_decimal::Decimal;

use pitpls_core::{
    common::{Amount, Country, Currency},
    dividend::Dividend,
    interest::Interest,
};

struct ColumnMap {
    action: usize,
    time: usize,
    isin: usize,
    ticker: usize,
    shares: usize,
    price: usize,
    price_currency: usize,
    tax: usize,
    tax_currency: usize,
    total: usize,
    total_currency: usize,
    id: Option<usize>,
}

impl ColumnMap {
    fn from_header(header: &str) -> Result<Self> {
        let columns = split_row(header);
        let find = |name: &str| {
            columns
                .iter()
                .position(|c| c == name)
                .ok_or_else(|| anyhow!("missing column: {name}"))
        };

        Ok(Self {
            action: find("Action")?,
            time: find("Time")?,
            isin: find("ISIN")?,
            ticker: find("Ticker")?,
            shares: find("No. of shares")?,
            price: find("Price / share")?,
            price_currency: find("Currency (Price / share)")?,
            tax: find("Withholding tax")?,
            tax_currency: find("Currency (Withholding tax)")?,
            total: find("Total")?,
            total_currency: find("Currency (Total)")?,
            id: columns.iter().position(|c| c == "ID"),
        })
    }

    fn max_index(&self) -> usize {
        *[
            self.action,
            self.time,
            self.isin,
            self.ticker,
            self.shares,
            self.price,
            self.price_currency,
            self.tax,
            self.tax_currency,
        ]
        .iter()
        .max()
        .unwrap()
    }
}

pub fn parse(csv_content: String) -> Result<(Vec<Dividend>, Vec<Interest>)> {
    let mut lines = csv_content.lines();

    let header = lines
        .next()
        .ok_or_else(|| anyhow!("missing header row"))?
        .trim_start_matches('\u{feff}');

    if !header.starts_with("Action,") {
        return Err(anyhow!("unexpected header: {header}"));
    }

    let columns = ColumnMap::from_header(header)?;
    let min_fields = columns.max_index() + 1;

    let mut dividends = Vec::new();
    let mut interests = Vec::new();

    for line in lines {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        let fields = split_row(line);
        if fields.len() < min_fields {
            return Err(anyhow!(
                "malformed row: expected at least {min_fields} fields, got {}: {line}",
                fields.len()
            ));
        }

        if let Some(dividend) = parse_dividend_row(&fields, &columns)? {
            dividends.push(dividend);
        } else if let Some(interest) = parse_interest_row(&fields, &columns)? {
            interests.push(interest);
        }
    }

    Ok((dividends, interests))
}

fn parse_interest_row(fields: &[String], columns: &ColumnMap) -> Result<Option<Interest>> {
    let action = &fields[columns.action];
    if !action.starts_with("Interest on cash") {
        return Ok(None);
    }

    let timestamp = &fields[columns.time];
    let total = &fields[columns.total];
    let total_currency = &fields[columns.total_currency];

    let date = NaiveDate::parse_from_str(timestamp.get(..10).unwrap_or(""), "%Y-%m-%d")
        .with_context(|| format!("invalid timestamp: {timestamp}"))?;

    let value_price_currency =
        Currency::from_str(&total_currency.to_ascii_lowercase()).map_err(|e| anyhow!(e))?;

    let value = Amount {
        value: parse_decimal(total)?,
        currency: value_price_currency,
    };

    let id = columns
        .id
        .and_then(|i| fields.get(i))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    Ok(Some(Interest {
        id,
        date,
        value,
        provider: "Trading 212".to_string(),
    }))
}

fn parse_dividend_row(fields: &[String], columns: &ColumnMap) -> Result<Option<Dividend>> {
    let action = &fields[columns.action];
    if !action.starts_with("Dividend") {
        return Ok(None);
    }

    let timestamp = &fields[columns.time];
    let isin = &fields[columns.isin];
    let ticker = &fields[columns.ticker];
    let shares = &fields[columns.shares];
    let price = &fields[columns.price];
    let price_currency = &fields[columns.price_currency];
    let tax = &fields[columns.tax];
    let tax_currency = &fields[columns.tax_currency];

    let date = NaiveDate::parse_from_str(timestamp.get(..10).unwrap_or(""), "%Y-%m-%d")
        .with_context(|| format!("invalid timestamp: {timestamp}"))?;

    let country =
        country_from_isin(isin).ok_or_else(|| anyhow!("unsupported ISIN country: {isin}"))?;

    let value_currency =
        Currency::from_str(&price_currency.to_ascii_lowercase()).map_err(|e| anyhow!(e))?;
    let tax_currency =
        Currency::from_str(&tax_currency.to_ascii_lowercase()).map_err(|e| anyhow!(e))?;

    if value_currency != tax_currency {
        return Err(anyhow!(
            "price and tax currency mismatch: {price_currency} vs {tax_currency}"
        ));
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

    let id = columns
        .id
        .and_then(|i| fields.get(i))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    Ok(Some(Dividend {
        id,
        date,
        ticker: ticker.to_owned(),
        value,
        tax_paid,
        country,
        provider: "Trading 212".to_owned(),
    }))
}

fn country_from_isin(isin: &str) -> Option<Country> {
    match isin.get(..2)? {
        "US" => Some(Country::USA),
        "JP" => Some(Country::Japan),
        _ => None,
    }
}

fn parse_decimal(s: &str) -> Result<Decimal> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(Decimal::ZERO);
    }
    Decimal::from_str(trimmed).with_context(|| format!("invalid decimal: {s}"))
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
