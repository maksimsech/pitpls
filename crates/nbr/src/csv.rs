use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, anyhow, ensure};
use chrono::NaiveDate;
use pitpls_core::{common::Currency, rate::RateExport};
use rust_decimal::Decimal;
use tokio::fs::read;

struct RateCsv {
    date: String,
    rate_map: BTreeMap<String, Decimal>,
}

pub async fn load_csv_rates(path: &str) -> Result<Vec<RateExport>> {
    let csv_rates = read_csv(path).await?;

    let rates_by_date = csv_rates
        .into_iter()
        .map(|r| {
            let date = NaiveDate::parse_from_str(&r.date, "%Y%m%d")?;
            let rates = r
                .rate_map
                .into_iter()
                .filter_map(|(code, rate)| {
                    let symbol = code.trim_start_matches(|c: char| c.is_ascii_digit());
                    let currency = match symbol {
                        "USD" => Currency::USD,
                        "EUR" => Currency::EUR,
                        _ => return None,
                    };

                    Some(parse_unit(code.as_str()).map(|unit| (currency, rate / unit)))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;

            Ok((date, rates))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    Ok(rates_by_date
        .into_iter()
        .flat_map(|(date, rates)| {
            rates.into_iter().map(move |(currency, rate)| RateExport {
                date,
                currency,
                rate,
            })
        })
        .collect())
}

async fn read_csv(path: &str) -> Result<Vec<RateCsv>> {
    let path = Path::new(path);

    ensure!(
        path.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("csv")),
        "Invalid extension"
    );

    parse_file(path).await
}

async fn parse_file(path: &Path) -> Result<Vec<RateCsv>> {
    let bytes = read(path).await?;
    let content = String::from_utf8_lossy(&bytes);

    let mut lines = content.lines().enumerate().filter_map(|(i, l)| {
        let line = l.trim();
        (!line.is_empty()).then_some((i + 1, line))
    });

    let (_, header_line) = lines.next().ok_or_else(|| anyhow!("empty rates CSV"))?;

    let headers = split_fields(header_line.trim_start_matches('\u{feff}'));

    ensure!(lines.next().is_some(), "invalid format");

    let mut rates = vec![];

    for (_number, line) in lines {
        let is_rate_row = split_fields(line)
            .first()
            .is_some_and(|f| f.len() == 8 && f.chars().all(|c| c.is_ascii_digit()));
        if !is_rate_row {
            continue;
        }

        rates.push(parse_row(&headers, line)?);
    }

    Ok(rates)
}

fn parse_row(headers: &[&str], line: &str) -> Result<RateCsv> {
    let fields = split_fields(line);

    ensure!(fields.len() == headers.len(), "invalid format");

    let table_number_index = headers.len() - 2;
    let mut rate_map = BTreeMap::new();

    for (header, value) in headers[1..table_number_index]
        .iter()
        .zip(&fields[1..table_number_index])
    {
        let rate: Decimal = value.replace(',', ".").parse()?;

        rate_map.insert((*header).to_string(), rate);
    }

    Ok(RateCsv {
        date: fields[0].to_string(),
        rate_map,
    })
}

fn split_fields(line: &str) -> Vec<&str> {
    line.trim().split_terminator(';').collect()
}

fn parse_unit(code: &str) -> Result<Decimal> {
    let digits = code
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return Ok(Decimal::ONE);
    }

    Ok(digits.parse()?)
}
