use tokio::fs::{read, read_to_string};

mod error;
mod impls;
pub mod model;

use crate::impls::{coinbase, revolut, t212};
pub use error::{ImportAmounts, ImportContext, ImportError, ImportField, Result};
pub use model::{ImportData, ImportedInstrument, Importer, ImporterKind, InputType, OutputType};

pub async fn import(kind: ImporterKind, path: &str) -> Result<ImportData> {
    Ok(match kind {
        ImporterKind::Coinbase => {
            let content = read_to_string(path).await?;
            let cryptos = coinbase::parse(content)?;

            ImportData {
                cryptos,
                ..ImportData::default()
            }
        }
        ImporterKind::Revolut => {
            let bytes = read(path).await?;
            let dividends = revolut::parse(bytes)?;

            ImportData {
                dividends,
                ..ImportData::default()
            }
        }
        ImporterKind::T212 => {
            let content = read_to_string(path).await?;
            t212::parse(content)?
        }
    })
}

pub const IMPORTERS: &[Importer] = &[
    Importer {
        kind: ImporterKind::T212,
        input: &[InputType::Csv],
        output: &[
            OutputType::Dividend,
            OutputType::Interest,
            OutputType::Stock,
        ],
    },
    Importer {
        kind: ImporterKind::Revolut,
        input: &[InputType::Pdf],
        output: &[OutputType::Dividend],
    },
    Importer {
        kind: ImporterKind::Coinbase,
        input: &[InputType::Csv],
        output: &[OutputType::Crypto],
    },
];
