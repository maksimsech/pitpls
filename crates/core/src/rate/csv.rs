use anyhow::{Result, anyhow, ensure};
use std::collections::BTreeMap;
use std::path::Path;
use tokio::fs::read;

use rust_decimal::Decimal;

pub struct RateCsv {
    pub date: String,
    pub rate_map: BTreeMap<String, Decimal>,
}

pub async fn read_csv(path: &str) -> Result<Vec<RateCsv>> {
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
