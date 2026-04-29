use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize, Type, Default)]
pub enum DividendRounding {
    #[default]
    SumToGroszy,
    SumToPayToZlote,
    SumBothToZlote,
    AllToZlote,
}

#[derive(Clone, Copy, Deserialize, Serialize, Type, Default)]
pub struct Settings {
    pub dividend_rounding: DividendRounding,
}
