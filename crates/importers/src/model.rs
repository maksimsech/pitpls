use pitpls_core::{crypto::Crypto, dividend::Dividend, interest::Interest};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImporterKind {
    T212,
    Revolut,
    Coinbase,
}

#[derive(Serialize)]
pub enum InputType {
    Csv,
    Pdf,
}

#[derive(Serialize)]
pub enum OutputType {
    Dividend,
    Crypto,
    Interest,
}

#[derive(Serialize)]
pub struct Importer {
    pub kind: ImporterKind,
    pub name: &'static str,
    pub input: &'static [InputType],
    pub output: &'static [OutputType],
}

pub struct ImportData {
    pub dividends: Vec<Dividend>,
    pub cryptos: Vec<Crypto>,
    pub interests: Vec<Interest>,
}
