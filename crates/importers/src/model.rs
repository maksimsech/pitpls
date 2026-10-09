use pitpls_core::{crypto::Crypto, dividend::Dividend, interest::Interest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImporterKind {
    T212,
    Revolut,
    Coinbase,
}

impl ImporterKind {
    /// Stable provider values persisted with imported records. These are data,
    /// not UI labels; changing them would change existing provider identities.
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
}

pub struct Importer {
    pub kind: ImporterKind,
    pub input: &'static [InputType],
    pub output: &'static [OutputType],
}

pub struct ImportData {
    pub dividends: Vec<Dividend>,
    pub cryptos: Vec<Crypto>,
    pub interests: Vec<Interest>,
}
