use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize, Default)]
pub enum DividendRounding {
    #[default]
    SumToGroszy,
    SumToPayToZlote,
    SumBothToZlote,
    AllToZlote,
}

#[derive(Clone, Copy, Default)]
pub struct Settings {
    pub dividend_rounding: DividendRounding,
}
