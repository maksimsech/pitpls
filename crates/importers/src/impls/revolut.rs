use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use chrono::NaiveDate;
use rust_decimal::Decimal;

use pitpls_core::{
    common::{Amount, Country, Currency},
    dividend::Dividend,
};

pub fn parse(pdf_bytes: Vec<u8>) -> Result<Vec<Dividend>> {
    let text =
        pdf_extract::extract_text_from_mem(&pdf_bytes).context("failed to extract PDF text")?;

    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match try_parse_dividend(&tokens[i..])? {
            Some((dividend, consumed)) => {
                out.push(dividend);
                i += consumed;
            }
            None => i += 1,
        }
    }

    Ok(out)
}

fn try_parse_dividend(tokens: &[&str]) -> Result<Option<(Dividend, usize)>> {
    if tokens.len() < 8 {
        return Ok(None);
    }

    let Ok(date) = NaiveDate::parse_from_str(tokens[0], "%Y-%m-%d") else {
        return Ok(None);
    };

    let ticker = tokens[1];
    if !is_ticker(ticker) {
        return Ok(None);
    }

    let mut isin_idx = None;
    for j in 2..tokens.len().min(10) {
        if is_isin(tokens[j]) {
            isin_idx = Some(j);
            break;
        }
        if NaiveDate::parse_from_str(tokens[j], "%Y-%m-%d").is_ok() {
            return Ok(None);
        }
    }
    let Some(isin_idx) = isin_idx else {
        return Ok(None);
    };

    let country = Country::from_isin(tokens[isin_idx]).map_err(|e| {
        anyhow!(
            "{e} (ISIN {}, ticker {ticker}, date {date})",
            tokens[isin_idx]
        )
    })?;

    let mut i = isin_idx + 1;

    if tokens.get(i).is_some_and(|t| is_country_code(t)) {
        i += 1;
    }

    let gross_token = tokens
        .get(i)
        .ok_or_else(|| anyhow!("missing gross amount for {ticker} on {date}"))?;
    let (currency, gross) = parse_currency_amount(gross_token)
        .with_context(|| format!("invalid gross amount for {ticker} on {date}: {gross_token}"))?;
    i += 1;

    if tokens.get(i + 1).copied() == Some("PLN") {
        i += 2;
    }

    if tokens.get(i).copied() == Some("Rate:") {
        i += 2;
    }

    let tax = match tokens.get(i).copied() {
        Some("-") => {
            i += 1;
            Decimal::ZERO
        }
        Some(t) => match parse_currency_amount(t) {
            Some((tax_currency, value)) => {
                if tax_currency != currency {
                    return Err(anyhow!(
                        "tax currency mismatch for {ticker} on {date}: {currency} vs {tax_currency}"
                    ));
                }
                i += 1;
                value
            }
            None => Decimal::ZERO,
        },
        None => Decimal::ZERO,
    };

    Ok(Some((
        Dividend {
            id: uuid::Uuid::new_v4().to_string(),
            date,
            ticker: ticker.to_string(),
            value: Amount {
                value: gross,
                currency,
            },
            tax_paid: Amount {
                value: tax,
                currency,
            },
            country,
            provider: "Revolut".to_string(),
        },
        i,
    )))
}

fn is_ticker(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 6
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '.')
}

fn is_country_code(s: &str) -> bool {
    s.len() == 2 && s.chars().all(|c| c.is_ascii_uppercase())
}

fn is_isin(s: &str) -> bool {
    s.len() == 12
        && s.as_bytes()[..2].iter().all(|b| b.is_ascii_uppercase())
        && s.as_bytes()[2..].iter().all(|b| b.is_ascii_alphanumeric())
}

fn parse_currency_amount(s: &str) -> Option<(Currency, Decimal)> {
    if let Some(rest) = s.strip_prefix("US$") {
        return Decimal::from_str(rest).ok().map(|v| (Currency::USD, v));
    }
    if let Some(rest) = s.strip_prefix('€') {
        return Decimal::from_str(rest).ok().map(|v| (Currency::EUR, v));
    }
    None
}
