use rust_decimal::{Decimal, RoundingStrategy};

use crate::settings::DividendRounding;

pub trait DecimalExt {
    fn round_groszy(self) -> Decimal;
    fn round_zloty(self) -> Decimal;
    fn maybe_round_dividend(self, rounding: DividendRounding) -> Decimal;
}

impl DecimalExt for Decimal {
    fn round_groszy(self) -> Decimal {
        self.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
    }

    fn round_zloty(self) -> Decimal {
        self.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
    }

    fn maybe_round_dividend(self, rounding: DividendRounding) -> Decimal {
        if matches!(rounding, DividendRounding::AllToZlote) {
            return self.round_zloty();
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::DecimalExt;

    #[test]
    fn round_groszy_rounds_positive_midpoints_away_from_zero() {
        assert_eq!(dec!(1.005).round_groszy(), dec!(1.01));
    }

    #[test]
    fn zloty_rounds_negative_midpoints_away_from_zero() {
        assert_eq!(dec!(1.5).round_zloty(), dec!(2));
    }
}
