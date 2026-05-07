use tokio::fs::{read, read_to_string};

mod error;
mod impls;
pub mod model;

use crate::impls::{coinbase, revolut, t212};
pub use error::{ImportError, Result};
pub use model::{ImportData, Importer, ImporterKind, InputType, OutputType};

pub async fn import(kind: ImporterKind, path: &str) -> Result<ImportData> {
    Ok(match kind {
        ImporterKind::Coinbase => {
            let content = read_to_string(path).await?;
            let cryptos = coinbase::parse(content)?;

            ImportData {
                dividends: vec![],
                cryptos,
                interests: vec![],
            }
        }
        ImporterKind::Revolut => {
            let bytes = read(path).await?;
            let dividends = revolut::parse(bytes)?;

            ImportData {
                dividends,
                cryptos: vec![],
                interests: vec![],
            }
        }
        ImporterKind::T212 => {
            let content = read_to_string(path).await?;
            let (dividends, interests) = t212::parse(content)?;

            ImportData {
                dividends,
                cryptos: vec![],
                interests,
            }
        }
    })
}

pub const IMPORTERS: &[Importer] = &[
    Importer {
        kind: ImporterKind::T212,
        name: "Trading 212",
        input: &[InputType::Csv],
        output: &[OutputType::Dividend, OutputType::Interest],
    },
    Importer {
        kind: ImporterKind::Revolut,
        name: "Revolut",
        input: &[InputType::Pdf],
        output: &[OutputType::Dividend],
    },
    Importer {
        kind: ImporterKind::Coinbase,
        name: "Coinbase",
        input: &[InputType::Csv],
        output: &[OutputType::Crypto],
    },
];
