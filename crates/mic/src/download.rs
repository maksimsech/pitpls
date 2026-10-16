use pitpls_core::exchange::Exchange;
use reqwest::{Client, StatusCode};
use thiserror::Error;

use crate::parse::{ParseError, parse_exchanges};

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum DownloadError {
    Request {
        #[source]
        source: reqwest::Error,
    },
    HttpStatus {
        status: StatusCode,
    },
    Response {
        #[source]
        source: reqwest::Error,
    },
    Parse(#[from] ParseError),
}

const MIC_CSV_URL: &str =
    "https://www.iso20022.org/sites/default/files/ISO10383_MIC/ISO10383_MIC.csv";

pub async fn load_exchanges(client: &Client) -> Result<Vec<Exchange>, DownloadError> {
    let response = client
        .get(MIC_CSV_URL)
        .send()
        .await
        .map_err(|source| DownloadError::Request { source })?;
    let status = response.status();

    if !status.is_success() {
        return Err(DownloadError::HttpStatus { status });
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|source| DownloadError::Response { source })?;
    Ok(parse_exchanges(&bytes)?)
}
