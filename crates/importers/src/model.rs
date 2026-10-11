use pitpls_core::{crypto::Crypto, dividend::Dividend, interest::Interest, stock::Stock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImporterKind {
    T212,
    Revolut,
    Coinbase,
}

impl ImporterKind {
    pub const fn provider(self) -> &'static str {
        match self {
            Self::T212 => "Trading 212",
            Self::Revolut => "Revolut",
            Self::Coinbase => "Coinbase",
        }
    }
}

pub enum InputType {
    Csv,
    Pdf,
}

pub enum OutputType {
    Dividend,
    Crypto,
    Interest,
    Stock,
}

pub struct Importer {
    pub kind: ImporterKind,
    pub input: &'static [InputType],
    pub output: &'static [OutputType],
}

#[derive(Default)]
pub struct ImportData {
    pub dividends: Vec<Dividend>,
    pub cryptos: Vec<Crypto>,
    pub interests: Vec<Interest>,
    pub stocks: Vec<Stock>,
    pub instruments: Vec<ImportedInstrument>,
}

pub struct ImportedInstrument {
    pub ticker: String,
    pub provider: String,
    pub isin: String,
}
