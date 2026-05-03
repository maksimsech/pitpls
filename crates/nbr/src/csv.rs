use std::path::Path;

use chrono::NaiveDate;
use pitpls_core::{common::Currency, rate::Rate};
use rust_decimal::Decimal;
use tokio::fs::read;

use crate::CsvImportError;

struct RateColumn {
    index: usize,
    currency: Currency,
    unit: Decimal,
}

pub async fn load_csv_rates(path: &str) -> Result<Vec<Rate>, CsvImportError> {
    let path = Path::new(path);

    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
    {
        return Err(CsvImportError::InvalidExtension);
    }

    let bytes = read(path).await?;
    let content = String::from_utf8_lossy(&bytes);

    let mut lines = content.lines().enumerate().filter_map(|(i, l)| {
        let line = l.trim();
        (!line.is_empty()).then_some((i + 1, line))
    });

    let (_, header_line) = lines.next().ok_or(CsvImportError::Empty)?;

    let headers = split_fields(header_line.trim_start_matches('\u{feff}'));
    let rate_columns = rate_columns(&headers)?;

    if lines.next().is_none() {
        return Err(CsvImportError::InvalidFormat);
    }

    let mut rates = vec![];

    for (line_number, line) in lines {
        let fields = split_fields(line);
        let is_rate_row = fields
            .first()
            .is_some_and(|f| f.len() == 8 && f.chars().all(|c| c.is_ascii_digit()));
        if !is_rate_row {
            continue;
        }

        parse_row(&headers, &rate_columns, &fields, line_number, &mut rates)?;
    }

    Ok(rates)
}

fn rate_columns(headers: &[&str]) -> Result<Vec<RateColumn>, CsvImportError> {
    if headers.len() < 3 {
        return Err(CsvImportError::InvalidFormat);
    }

    let table_number_index = headers.len() - 2;
    let mut rate_columns = Vec::new();

    for (index, header) in headers[1..table_number_index].iter().enumerate() {
        let Some(currency) = parse_currency(header) else {
            continue;
        };

        rate_columns.push(RateColumn {
            index: index + 1,
            currency,
            unit: parse_unit(header).map_err(|source| CsvImportError::InvalidUnit {
                header: (*header).to_string(),
                source,
            })?,
        });
    }

    Ok(rate_columns)
}

fn parse_row(
    headers: &[&str],
    rate_columns: &[RateColumn],
    fields: &[&str],
    line_number: usize,
    rates: &mut Vec<Rate>,
) -> Result<(), CsvImportError> {
    if fields.len() != headers.len() {
        return Err(CsvImportError::InvalidFormat);
    }

    let date = NaiveDate::parse_from_str(fields[0], "%Y%m%d").map_err(|source| {
        CsvImportError::InvalidDate {
            line: line_number,
            source,
        }
    })?;

    for column in rate_columns {
        let currency = column.currency;

        if rates
            .iter()
            .any(|r| r.date == date && r.currency == currency)
        {
            return Err(CsvImportError::DuplicateRate {
                currency: currency.to_string(),
                date,
                line: line_number,
            });
        }

        let rate: Decimal = fields[column.index]
            .replace(',', ".")
            .parse()
            .map_err(|source| CsvImportError::InvalidRate {
                line: line_number,
                column: headers[column.index].to_string(),
                source,
            })?;
        rates.push(Rate {
            date,
            currency,
            rate: rate / column.unit,
        });
    }

    Ok(())
}

fn parse_currency(code: &str) -> Option<Currency> {
    let symbol = code.trim_start_matches(|c: char| c.is_ascii_digit());

    match symbol {
        "USD" => Some(Currency::USD),
        "EUR" => Some(Currency::EUR),
        _ => None,
    }
}

fn split_fields(line: &str) -> Vec<&str> {
    line.trim().split_terminator(';').collect()
}

fn parse_unit(code: &str) -> Result<Decimal, rust_decimal::Error> {
    let digits = code
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return Ok(Decimal::ONE);
    }

    digits.parse()
}
