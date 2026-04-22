use rust_decimal::{Decimal, RoundingStrategy};

pub trait DecimalExt {
    fn round_amount(self) -> Decimal;
}

impl DecimalExt for Decimal {
    fn round_amount(self) -> Decimal {
        self.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::DecimalExt;

    #[test]
    fn round_amount_rounds_positive_midpoints_away_from_zero() {
        assert_eq!(dec!(1.005).round_amount(), dec!(1.01));
    }

    #[test]
    fn round_amount_rounds_negative_midpoints_away_from_zero() {
        assert_eq!(dec!(-1.005).round_amount(), dec!(-1.01));
    }
}
