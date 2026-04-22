use pitpls_core::{crypto::Crypto, dividend::Dividend, interest::Interest};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Type)]
pub enum ImporterKind {
    T212,
    Revolut,
    Coinbase,
}

#[derive(Serialize, Type)]
pub enum InputType {
    Csv,
    Pdf,
}

#[derive(Serialize, Type)]
pub enum OutputType {
    Dividend,
    Crypto,
    Interest,
}

#[derive(Serialize, Type)]
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
