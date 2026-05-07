use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use pitpls_core::{
    common::{Amount, Currency},
    crypto::{Action, Crypto},
};

use crate::{ImportError, Result};

const FIAT_SYMBOLS: &[&str] = &["EUR", "USD", "PLN"];

pub fn parse(csv_content: String) -> Result<Vec<Crypto>> {
    let mut lines = csv_content.lines();

    loop {
        let line = lines
            .next()
            .ok_or(ImportError::MissingHeader)?
            .trim_start_matches('\u{feff}');

        if line.starts_with("ID,") {
            break;
        }
    }

    let mut out = Vec::new();

    for line in lines {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        let fields = split_row(line);
        if fields.len() < 10 {
            return Err(ImportError::malformed_row(10, fields.len(), line));
        }

        if let Some(crypto) = parse_row(&fields)? {
            out.push(crypto);
        }
    }

    Ok(out)
}

fn parse_row(fields: &[&str]) -> Result<Option<Crypto>> {
    let id = fields[0];
    let timestamp = fields[1];
    let tx_type = fields[2];
    let asset = fields[3];
    let price_currency = fields[5];
    let subtotal = fields[7];
    let fees = fields[9];
    let notes = fields.get(10).copied().unwrap_or("");

    let Some(action) = classify(tx_type, asset, notes) else {
        return Ok(None);
    };

    let currency = Currency::from_str(price_currency)
        .map_err(|source| ImportError::invalid_currency(price_currency, source))?;

    let date = NaiveDate::parse_from_str(timestamp.get(..10).unwrap_or(""), "%Y-%m-%d")
        .map_err(|source| ImportError::invalid_timestamp(timestamp, source))?;

    let value = Amount {
        value: parse_decimal(subtotal)?.abs(),
        currency,
    };

    let fee = Amount {
        value: parse_decimal(fees)?.abs(),
        currency,
    };

    Ok(Some(Crypto {
        id: id.to_string(),
        value,
        fee,
        action,
        date,
        provider: "Coinbase".to_string(),
    }))
}

fn classify(tx_type: &str, asset: &str, notes: &str) -> Option<Action> {
    match tx_type {
        "Buy" => (!is_fiat(asset)).then_some(Action::FiatBuy),
        "Sell" => (!is_fiat(asset)).then_some(Action::FiatSell),
        "Advanced Trade Buy" => match (is_fiat(asset), parse_quote(notes).map(|q| is_fiat(&q))) {
            (false, Some(true)) => Some(Action::FiatBuy),
            (true, Some(false)) => Some(Action::FiatSell),
            _ => None,
        },
        "Advanced Trade Sell" => match (is_fiat(asset), parse_quote(notes).map(|q| is_fiat(&q))) {
            (false, Some(true)) => Some(Action::FiatSell),
            (true, Some(false)) => Some(Action::FiatBuy),
            _ => None,
        },
        _ => None,
    }
}

fn is_fiat(symbol: &str) -> bool {
    FIAT_SYMBOLS.contains(&symbol)
}

fn parse_quote(notes: &str) -> Option<String> {
    let after_for = notes.split_once(" for ")?.1;
    let quote = after_for.split_whitespace().nth(1)?;
    Some(quote.to_string())
}

fn parse_decimal(s: &str) -> Result<Decimal> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();

    if cleaned.is_empty() || cleaned == "-" {
        return Ok(Decimal::ZERO);
    }

    Decimal::from_str(&cleaned).map_err(|source| ImportError::invalid_decimal(s, source))
}

fn split_row(line: &str) -> Vec<&str> {
    let mut parts = Vec::with_capacity(11);
    let mut start = 0;
    let mut commas = 0;

    for (i, b) in line.bytes().enumerate() {
        if b == b',' {
            parts.push(&line[start..i]);
            start = i + 1;
            commas += 1;
            if commas == 10 {
                break;
            }
        }
    }

    parts.push(&line[start..]);
    parts
}
