use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use pitpls_core::{
    common::{Amount, Country, Currency},
    dividend::Dividend,
};

use crate::{ImportAmounts, ImportContext, ImportError, ImportField, ImporterKind, Result};

const PROVIDER: &str = crate::ImporterKind::Revolut.provider();
const OTHER_INCOME_TABLE: ImportContext = ImportContext::RevolutOtherIncome;
const TABLE_HEADER: &str =
    "Date Description Security name ISIN Country Gross Amount Withholding Tax Net Amount";

#[derive(Clone, Copy)]
struct AmountColumns {
    currency: Currency,
    gross: Decimal,
    tax: Decimal,
    net: Decimal,
}

#[derive(Default)]
struct SectionSums {
    currency: Option<Currency>,
    gross: Decimal,
    tax: Decimal,
    net: Decimal,
}

struct ParsedRow {
    amounts: AmountColumns,
    dividend: Option<Dividend>,
}

pub fn parse(pdf_bytes: Vec<u8>) -> Result<Vec<Dividend>> {
    let text = pdf_extract::extract_text_from_mem(&pdf_bytes)?;
    parse_text(&text)
}

fn parse_text(text: &str) -> Result<Vec<Dividend>> {
    if !text.contains("Profit and Loss Statement")
        || !text.contains("Revolut Securities Europe UAB")
    {
        return Err(ImportError::UnexpectedFormat {
            expected: ImporterKind::Revolut,
        });
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut found_table = false;
    let mut i = 0;

    while i < lines.len() {
        if is_other_income_heading(lines[i])
            && let Some(header_idx) = next_non_empty_line(&lines, i + 1)
            && is_table_header(lines[header_idx])
        {
            found_table = true;
            let (mut dividends, next_idx) = parse_other_income_table(&lines, header_idx + 1)?;
            out.append(&mut dividends);
            i = next_idx;
            continue;
        }

        i += 1;
    }

    if !found_table {
        return Err(ImportError::MissingSection(OTHER_INCOME_TABLE));
    }

    Ok(out)
}

fn parse_other_income_table(lines: &[&str], start_idx: usize) -> Result<(Vec<Dividend>, usize)> {
    let mut out = Vec::new();
    let mut sums = SectionSums::default();
    let mut i = start_idx;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.is_empty() || is_table_header(line) {
            i += 1;
            continue;
        }

        if is_total_line(line) {
            let (total, consumed) = parse_total(&lines[i..])?;
            sums.validate(total)?;
            return Ok((out, i + consumed));
        }

        if starts_with_date(line) {
            let (row_lines, next_idx) = collect_row_lines(lines, i);
            if let Some(row) = parse_table_row(&row_lines)? {
                sums.add(row.amounts)?;
                if let Some(dividend) = row.dividend {
                    out.push(dividend);
                }
            }
            i = next_idx;
            continue;
        }

        return Err(ImportError::invalid_field(
            ImportField::Line,
            line,
            OTHER_INCOME_TABLE,
        ));
    }

    Err(ImportError::missing_field(
        ImportField::TotalRow,
        OTHER_INCOME_TABLE,
    ))
}

fn collect_row_lines<'a>(lines: &'a [&str], start_idx: usize) -> (Vec<&'a str>, usize) {
    let mut out = Vec::new();
    let mut i = start_idx;

    while i < lines.len() {
        let line = lines[i].trim();

        if i > start_idx && (starts_with_date(line) || is_table_header(line) || is_total_line(line))
        {
            break;
        }

        if !line.is_empty() {
            out.push(line);
        }

        i += 1;
    }

    (out, i)
}

fn parse_table_row(lines: &[&str]) -> Result<Option<ParsedRow>> {
    let row = lines.join(" ");
    let tokens: Vec<&str> = row.split_whitespace().collect();

    if tokens.len() < 2 {
        return Err(ImportError::malformed_row(2, tokens.len(), row));
    }

    let date = NaiveDate::parse_from_str(tokens[0], "%Y-%m-%d")
        .map_err(|source| ImportError::invalid_timestamp(tokens[0], source))?;
    let ticker = tokens[1];
    let row_mentions_dividend = contains_dividend(&tokens);

    if !is_ticker(ticker) {
        if row_mentions_dividend {
            return Err(ImportError::invalid_field(
                ImportField::Ticker,
                ticker,
                ImportContext::RevolutRow { ticker: None, date },
            ));
        }
        return Ok(None);
    }

    let Some(isin_idx) = tokens.iter().position(|token| is_isin(token)) else {
        if row_mentions_dividend {
            return Err(ImportError::missing_field(
                ImportField::Isin,
                ImportContext::RevolutRow {
                    ticker: Some(ticker.to_owned()),
                    date,
                },
            ));
        }
        return Ok(None);
    };

    let is_dividend = contains_dividend(&tokens[1..isin_idx]);
    let isin = tokens[isin_idx];
    let country =
        Country::from_isin(isin).map_err(|source| ImportError::invalid_isin(isin, source))?;
    let mut i = isin_idx + 1;

    if tokens.get(i).is_some_and(|token| is_country_code(token)) {
        let country_code = tokens[i];
        if !country_code.eq_ignore_ascii_case(country.code().as_str()) {
            return Err(ImportError::CountryMismatch {
                context: ImportContext::RevolutRow {
                    ticker: Some(ticker.to_owned()),
                    date,
                },
                isin: isin.to_owned(),
                expected: country,
                actual: country_code.to_owned(),
            });
        }
        i += 1;
    }

    let (amounts, consumed) = parse_amount_columns(
        &tokens,
        i,
        &ImportContext::RevolutRow {
            ticker: Some(ticker.to_owned()),
            date,
        },
    )?;
    if consumed != tokens.len() {
        return Err(ImportError::invalid_field(
            ImportField::TrailingTokens,
            tokens[consumed..].join(" "),
            ImportContext::RevolutRow {
                ticker: Some(ticker.to_owned()),
                date,
            },
        ));
    }

    let dividend = is_dividend.then(|| Dividend {
        id: uuid::Uuid::new_v4().to_string(),
        date,
        ticker: ticker.to_string(),
        value: Amount {
            value: amounts.gross,
            currency: amounts.currency,
        },
        tax_paid: Amount {
            value: amounts.tax,
            currency: amounts.currency,
        },
        country,
        provider: PROVIDER.to_string(),
    });

    Ok(Some(ParsedRow { amounts, dividend }))
}

fn parse_total(lines: &[&str]) -> Result<(AmountColumns, usize)> {
    let mut merged = String::new();

    for (offset, line) in lines.iter().enumerate().take(6) {
        let line = line.trim();

        if offset > 0 && (line.is_empty() || starts_with_date(line) || is_table_header(line)) {
            break;
        }

        if line.is_empty() {
            continue;
        }

        if !merged.is_empty() {
            merged.push(' ');
        }
        merged.push_str(line);

        match try_parse_total_line(&merged)? {
            Some(total) => return Ok((total, offset + 1)),
            None => continue,
        }
    }

    Err(ImportError::invalid_field(
        ImportField::TotalRow,
        merged,
        OTHER_INCOME_TABLE,
    ))
}

fn try_parse_total_line(line: &str) -> Result<Option<AmountColumns>> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.first().copied() != Some("Total") {
        return Err(ImportError::invalid_field(
            ImportField::TotalRow,
            line,
            OTHER_INCOME_TABLE,
        ));
    }

    match parse_amount_columns(&tokens, 1, &ImportContext::RevolutTotal) {
        Ok((total, consumed)) if consumed == tokens.len() => Ok(Some(total)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

fn parse_amount_columns(
    tokens: &[&str],
    start_idx: usize,
    context: &ImportContext,
) -> Result<(AmountColumns, usize)> {
    let mut i = start_idx;
    let (currency, gross) =
        parse_required_currency_amount(tokens.get(i), ImportField::GrossAmount, context)?;
    i += 1;

    skip_optional_local_amount(tokens, &mut i);
    skip_optional_rate(tokens, &mut i, context)?;

    let tax = match tokens.get(i).copied() {
        Some("-") => {
            i += 1;
            Decimal::ZERO
        }
        Some(token) => {
            let (tax_currency, tax) =
                parse_required_currency_amount(Some(&token), ImportField::WithholdingTax, context)?;
            if tax_currency != currency {
                return Err(ImportError::CurrencyMismatch {
                    field: ImportField::WithholdingTax,
                    context: context.clone(),
                    expected: currency,
                    actual: tax_currency,
                });
            }
            i += 1;
            tax
        }
        None => {
            return Err(ImportError::missing_field(
                ImportField::WithholdingTax,
                context.clone(),
            ));
        }
    };

    skip_optional_local_amount_or_dash(tokens, &mut i);

    let (net_currency, net) =
        parse_required_currency_amount(tokens.get(i), ImportField::NetAmount, context)?;
    if net_currency != currency {
        return Err(ImportError::CurrencyMismatch {
            field: ImportField::NetAmount,
            context: context.clone(),
            expected: currency,
            actual: net_currency,
        });
    }
    i += 1;

    skip_optional_local_amount(tokens, &mut i);

    if gross - tax != net {
        return Err(ImportError::AmountMismatch {
            context: context.clone(),
            amounts: ImportAmounts { gross, tax, net },
        });
    }

    Ok((
        AmountColumns {
            currency,
            gross,
            tax,
            net,
        },
        i,
    ))
}

fn parse_required_currency_amount(
    token: Option<&&str>,
    label: ImportField,
    context: &ImportContext,
) -> Result<(Currency, Decimal)> {
    let token = token
        .copied()
        .ok_or_else(|| ImportError::missing_field(label, context.clone()))?;

    parse_currency_amount(token)?
        .ok_or_else(|| ImportError::invalid_field(label, token, context.clone()))
}

fn skip_optional_local_amount(tokens: &[&str], i: &mut usize) -> bool {
    if tokens
        .get(*i)
        .and_then(|token| parse_decimal_token(token).ok())
        .is_some()
        && tokens
            .get(*i + 1)
            .is_some_and(|token| Currency::from_str(token).is_ok())
    {
        *i += 2;
        return true;
    }

    false
}

fn skip_optional_local_amount_or_dash(tokens: &[&str], i: &mut usize) {
    if tokens.get(*i).copied() == Some("-") {
        *i += 1;
        return;
    }

    skip_optional_local_amount(tokens, i);
}

fn skip_optional_rate(tokens: &[&str], i: &mut usize, context: &ImportContext) -> Result<()> {
    if tokens.get(*i).copied() != Some("Rate:") {
        return Ok(());
    }

    let rate = tokens.get(*i + 1).copied().ok_or_else(|| {
        ImportError::missing_field(ImportField::LocalCurrencyRate, context.clone())
    })?;
    parse_decimal_token(rate).map_err(|source| ImportError::invalid_decimal(rate, source))?;
    *i += 2;

    Ok(())
}

fn parse_currency_amount(token: &str) -> Result<Option<(Currency, Decimal)>> {
    let token = token.trim();
    let (negative, unsigned) = token
        .strip_prefix('-')
        .map_or((false, token), |rest| (true, rest));

    let prefixes = [
        ("US$", Currency::USD),
        ("AU$", Currency::AUD),
        ("A$", Currency::AUD),
        ("CA$", Currency::CAD),
        ("C$", Currency::CAD),
        ("NZ$", Currency::NZD),
        ("HK$", Currency::HKD),
        ("S$", Currency::SGD),
        ("CHF", Currency::CHF),
        ("€", Currency::EUR),
        ("£", Currency::GBP),
        ("¥", Currency::JPY),
        ("$", Currency::USD),
    ];

    for (prefix, currency) in prefixes {
        if let Some(rest) = unsigned.strip_prefix(prefix) {
            let mut value = parse_decimal_literal(rest)
                .map_err(|source| ImportError::invalid_decimal(token, source))?;
            if negative {
                value = -value;
            }
            return Ok(Some((currency, value)));
        }
    }

    Ok(None)
}

fn parse_decimal_token(token: &str) -> std::result::Result<Decimal, rust_decimal::Error> {
    parse_decimal_literal(token.trim())
}

fn parse_decimal_literal(value: &str) -> std::result::Result<Decimal, rust_decimal::Error> {
    let normalized = value.replace(',', "");
    Decimal::from_str(&normalized)
}

fn is_other_income_heading(line: &str) -> bool {
    normalize_space(line) == "Other income & fees"
}

fn is_table_header(line: &str) -> bool {
    normalize_space(line) == TABLE_HEADER
}

fn is_total_line(line: &str) -> bool {
    line.split_whitespace().next() == Some("Total")
}

fn starts_with_date(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|token| NaiveDate::parse_from_str(token, "%Y-%m-%d").is_ok())
}

fn next_non_empty_line(lines: &[&str], start_idx: usize) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .skip(start_idx)
        .find_map(|(idx, line)| (!line.trim().is_empty()).then_some(idx))
}

fn normalize_space(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn contains_dividend(tokens: &[&str]) -> bool {
    tokens
        .iter()
        .any(|token| token.to_ascii_lowercase().contains("dividend"))
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

impl SectionSums {
    fn add(&mut self, amounts: AmountColumns) -> Result<()> {
        match self.currency {
            Some(currency) if currency != amounts.currency => {
                return Err(ImportError::CurrencyMismatch {
                    field: ImportField::SourceCurrency,
                    context: OTHER_INCOME_TABLE,
                    expected: currency,
                    actual: amounts.currency,
                });
            }
            Some(_) => {}
            None => self.currency = Some(amounts.currency),
        }

        self.gross += amounts.gross;
        self.tax += amounts.tax;
        self.net += amounts.net;

        Ok(())
    }

    fn validate(&self, total: AmountColumns) -> Result<()> {
        if let Some(currency) = self.currency
            && currency != total.currency
        {
            return Err(ImportError::CurrencyMismatch {
                field: ImportField::TotalCurrency,
                context: OTHER_INCOME_TABLE,
                expected: currency,
                actual: total.currency,
            });
        }

        if self.gross != total.gross || self.tax != total.tax || self.net != total.net {
            return Err(ImportError::TotalsMismatch {
                currency: total.currency,
                expected: ImportAmounts {
                    gross: self.gross,
                    tax: self.tax,
                    net: self.net,
                },
                actual: ImportAmounts {
                    gross: total.gross,
                    tax: total.tax,
                    net: total.net,
                },
            });
        }

        Ok(())
    }
}
