use chrono::NaiveDate;
use csv::{ReaderBuilder, StringRecord, Trim};
use pitpls_core::{
    common::{Country, Mic},
    exchange::{Exchange, ExchangeKind, ExchangeStatus},
};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum ParseError {
    Csv(#[from] csv::Error),
    InvalidMic {
        line: Option<u64>,
        value: String,
    },
    InvalidDate {
        line: Option<u64>,
        value: String,
        #[source]
        source: chrono::ParseError,
    },
    Empty,
}

#[derive(Deserialize)]
struct Row {
    #[serde(rename = "MIC")]
    mic: String,
    #[serde(rename = "OPERATING MIC")]
    operating_mic: String,
    #[serde(rename = "OPRT/SGMT")]
    kind: ExchangeKind,
    #[serde(rename = "MARKET NAME-INSTITUTION DESCRIPTION")]
    name: String,
    #[serde(rename = "ACRONYM")]
    acronym: String,
    #[serde(rename = "ISO COUNTRY CODE (ISO 3166)")]
    country: Country,
    #[serde(rename = "CITY")]
    city: String,
    #[serde(rename = "STATUS")]
    status: ExchangeStatus,
    #[serde(rename = "LAST UPDATE DATE")]
    last_update: String,
}

pub fn parse_exchanges(bytes: &[u8]) -> Result<Vec<Exchange>, ParseError> {
    let mut reader = ReaderBuilder::new().trim(Trim::All).from_reader(bytes);
    let headers = reader.headers()?.clone();

    let mut exchanges = Vec::new();
    let mut record = StringRecord::new();
    while reader.read_record(&mut record)? {
        let line = record.position().map(|position| position.line());
        let row: Row = record.deserialize(Some(&headers))?;
        exchanges.push(exchange(row, line)?);
    }

    // An empty list would replace the stored one with nothing.
    if exchanges.is_empty() {
        return Err(ParseError::Empty);
    }

    Ok(exchanges)
}

fn exchange(row: Row, line: Option<u64>) -> Result<Exchange, ParseError> {
    let last_update = non_empty(row.last_update)
        .map(|value| {
            NaiveDate::parse_from_str(&value, "%Y%m%d").map_err(|source| ParseError::InvalidDate {
                line,
                value,
                source,
            })
        })
        .transpose()?;

    Ok(Exchange {
        mic: mic(&row.mic, line)?,
        operating_mic: mic(&row.operating_mic, line)?,
        kind: row.kind,
        name: row.name,
        acronym: non_empty(row.acronym),
        country: row.country,
        city: non_empty(row.city),
        status: row.status,
        last_update,
    })
}

fn mic(value: &str, line: Option<u64>) -> Result<Mic, ParseError> {
    value.parse().map_err(|_| ParseError::InvalidMic {
        line,
        value: value.to_owned(),
    })
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}
