use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use pitpls_core::{
    common::{Amount, Currency},
    crypto::{Action, Crypto},
};

use crate::{ImportError, Result};

const PROVIDER: &str = "Coinbase";

struct ColumnMap {
    id: usize,
    timestamp: usize,
    tx_type: usize,
    asset: usize,
    price_currency: usize,
    subtotal: usize,
    fees: usize,
    notes: usize,
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

        Ok(Self {
            id: find("ID")?,
            timestamp: find("Timestamp")?,
            tx_type: find("Transaction Type")?,
            asset: find("Asset")?,
            price_currency: find("Price Currency")?,
            subtotal: find("Subtotal")?,
            fees: find("Fees and/or Spread")?,
            notes: find("Notes")?,
        })
    }
}

pub fn parse(csv_content: String) -> Result<Vec<Crypto>> {
    let mut lines = csv_content.lines();

    let header = loop {
        let line = lines
            .next()
            .ok_or(ImportError::MissingHeader)?
            .trim_start_matches('\u{feff}')
            .trim_end();

        if line.starts_with("ID,") {
            break line;
        }
    };

    let columns = ColumnMap::from_header(header)?;

    let mut out = Vec::new();

    for line in lines {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        let fields = split_row(line);

        if let Some(crypto) = parse_row(&fields, &columns, line)? {
            out.push(crypto);
        }
    }

    Ok(out)
}

fn parse_row(fields: &[String], columns: &ColumnMap, row: &str) -> Result<Option<Crypto>> {
    let id = field(fields, columns.id, row)?;
    let timestamp = field(fields, columns.timestamp, row)?;
    let tx_type = field(fields, columns.tx_type, row)?;
    let asset = field(fields, columns.asset, row)?;
    let price_currency = field(fields, columns.price_currency, row)?;
    let subtotal = field(fields, columns.subtotal, row)?;
    let fees = field(fields, columns.fees, row)?;
    let notes = field(fields, columns.notes, row)?;

    let Some(action) = classify(tx_type, asset, notes) else {
        return Ok(None);
    };

    let value = Amount {
        value: parse_decimal(subtotal)?.abs(),
        currency: parse_currency(price_currency)?,
    };

    let fee = Amount {
        value: parse_decimal(fees)?.abs(),
        currency: value.currency,
    };

    Ok(Some(Crypto {
        id: id.to_string(),
        value,
        fee,
        action,
        date: parse_date(timestamp)?,
        provider: PROVIDER.to_string(),
    }))
}

fn classify(tx_type: &str, asset: &str, notes: &str) -> Option<Action> {
    let asset_is_fiat = is_fiat(asset);

    match tx_type {
        "Buy" => (!asset_is_fiat).then_some(Action::FiatBuy),
        "Sell" => (!asset_is_fiat).then_some(Action::FiatSell),
        "Advanced Trade Buy" => match (asset_is_fiat, quote_currency(notes).map(is_fiat)) {
            (false, Some(true)) => Some(Action::FiatBuy),
            (true, Some(false)) => Some(Action::FiatSell),
            _ => None,
        },
        "Advanced Trade Sell" => match (asset_is_fiat, quote_currency(notes).map(is_fiat)) {
            (false, Some(true)) => Some(Action::FiatSell),
            (true, Some(false)) => Some(Action::FiatBuy),
            _ => None,
        },
        _ => None,
    }
}

fn is_fiat(symbol: &str) -> bool {
    Currency::from_str(symbol).is_ok()
}

fn quote_currency(notes: &str) -> Option<&str> {
    notes.split_once(" for ")?.1.split_whitespace().nth(1)
}

fn field<'a>(fields: &'a [String], index: usize, row: &str) -> Result<&'a str> {
    fields
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| ImportError::malformed_row(index + 1, fields.len(), row))
}

fn parse_date(timestamp: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(timestamp.get(..10).unwrap_or(""), "%Y-%m-%d")
        .map_err(|source| ImportError::invalid_timestamp(timestamp, source))
}

fn parse_currency(currency: &str) -> Result<Currency> {
    Currency::from_str(currency).map_err(|source| ImportError::invalid_currency(currency, source))
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

fn split_row(line: &str) -> Vec<String> {
    let mut parts = Vec::with_capacity(11);
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
            ',' if !in_quotes => parts.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }

    parts.push(current);
    parts
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn parse_fixture(content: &str) -> Vec<Crypto> {
        parse(content.to_owned()).expect("fixture should parse")
    }

    #[test]
    fn parses_only_fiat_crypto_transactions() {
        let cryptos = parse_fixture(include_str!(
            "../../tests/fixtures/coinbase/transactions.csv"
        ));

        assert_ids(
            &cryptos,
            &[
                "advanced-buy-fiat-1",
                "simple-buy-fiat-1",
                "advanced-sell-fiat-1",
            ],
        );

        for ignored_id in [
            "staking-income-1",
            "deposit-1",
            "advanced-buy-crypto-1",
            "advanced-sell-crypto-1",
        ] {
            assert!(!cryptos.iter().any(|crypto| crypto.id == ignored_id));
        }

        assert_crypto(
            &cryptos[0],
            ExpectedCrypto {
                id: "advanced-buy-fiat-1",
                action: ExpectedAction::FiatBuy,
                date: NaiveDate::from_ymd_opt(2025, 11, 28).unwrap(),
                value: "413.32342",
                fee: "0.00",
                currency: Currency::EUR,
            },
        );
        assert_crypto(
            &cryptos[1],
            ExpectedCrypto {
                id: "simple-buy-fiat-1",
                action: ExpectedAction::FiatBuy,
                date: NaiveDate::from_ymd_opt(2025, 4, 9).unwrap(),
                value: "9.70683",
                fee: "0.2931714115440003565444",
                currency: Currency::EUR,
            },
        );
        assert_crypto(
            &cryptos[2],
            ExpectedCrypto {
                id: "advanced-sell-fiat-1",
                action: ExpectedAction::FiatSell,
                date: NaiveDate::from_ymd_opt(2025, 3, 11).unwrap(),
                value: "12.81876",
                fee: "0.1538250674048000055985248",
                currency: Currency::EUR,
            },
        );
    }

    struct ExpectedCrypto<'a> {
        id: &'a str,
        action: ExpectedAction,
        date: NaiveDate,
        value: &'a str,
        fee: &'a str,
        currency: Currency,
    }

    enum ExpectedAction {
        FiatBuy,
        FiatSell,
    }

    fn assert_ids(cryptos: &[Crypto], expected: &[&str]) {
        let ids = cryptos
            .iter()
            .map(|crypto| crypto.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, expected);
    }

    fn assert_crypto(crypto: &Crypto, expected: ExpectedCrypto<'_>) {
        assert_eq!(crypto.id, expected.id);
        assert_action(crypto.action, expected.action);
        assert_eq!(crypto.date, expected.date);
        assert_eq!(
            crypto.value.value,
            Decimal::from_str(expected.value).unwrap()
        );
        assert_eq!(crypto.value.currency.as_str(), expected.currency.as_str());
        assert_eq!(crypto.fee.value, Decimal::from_str(expected.fee).unwrap());
        assert_eq!(crypto.fee.currency.as_str(), expected.currency.as_str());
        assert_eq!(crypto.provider, PROVIDER);
    }

    fn assert_action(actual: Action, expected: ExpectedAction) {
        assert!(matches!(
            (actual, expected),
            (Action::FiatBuy, ExpectedAction::FiatBuy)
                | (Action::FiatSell, ExpectedAction::FiatSell)
        ));
    }
}
