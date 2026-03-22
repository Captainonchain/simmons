//! Kelly Criterion position sizing

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Kelly Criterion calculator
pub struct KellyCriterion {
    /// Fraction of Kelly to use (0.25 = quarter Kelly)
    pub fraction: Decimal,
    /// Maximum position size (percentage of capital)
    pub max_position: Decimal,
    /// Minimum position size (percentage of capital)
    pub min_position: Decimal,
}

impl Default for KellyCriterion {
    fn default() -> Self {
        Self {
            fraction: dec!(0.25),      // Quarter Kelly
            max_position: dec!(0.15),  // Max 15% of capital
            min_position: dec!(0.01),  // Min 1% of capital
        }
    }
}

impl KellyCriterion {
    pub fn new(fraction: Decimal) -> Self {
        Self {
            fraction,
            ..Default::default()
        }
    }

    /// Calculate optimal position size using Kelly criterion
    ///
    /// Kelly formula: f* = (bp - q) / b
    /// where:
    /// - f* = fraction of bankroll to bet
    /// - b = odds received (win_amount / loss_amount)
    /// - p = probability of winning
    /// - q = probability of losing (1 - p)
    pub fn calculate(
        &self,
        win_probability: Decimal,
        win_amount: Decimal,
        loss_amount: Decimal,
    ) -> Decimal {
        if loss_amount.is_zero() || win_amount.is_zero() {
            return Decimal::ZERO;
        }

        let p = win_probability;
        let q = dec!(1) - p;
        let b = win_amount / loss_amount;

        // Kelly formula: (bp - q) / b
        let kelly = (b * p - q) / b;

        // Apply fraction (e.g., quarter Kelly)
        let position = kelly * self.fraction;

        // Clamp to min/max
        if position < self.min_position {
            Decimal::ZERO // Too small, don't trade
        } else {
            position.min(self.max_position)
        }
    }

    /// Calculate position size from historical win rate and risk/reward
    pub fn from_history(
        &self,
        wins: u32,
        losses: u32,
        avg_win: Decimal,
        avg_loss: Decimal,
    ) -> Decimal {
        let total = wins + losses;
        if total == 0 {
            return self.min_position; // Default for no history
        }

        let win_prob = Decimal::from(wins) / Decimal::from(total);
        self.calculate(win_prob, avg_win, avg_loss)
    }

    /// Calculate position size with confidence adjustment
    pub fn with_confidence(
        &self,
        base_position: Decimal,
        confidence: Decimal,
    ) -> Decimal {
        // Scale position by confidence (0.5 to 1.0 range matters most)
        let scaled_conf = if confidence < dec!(0.5) {
            dec!(0.5)
        } else {
            confidence
        };

        let adjusted = base_position * scaled_conf;
        adjusted.max(self.min_position).min(self.max_position)
    }
}

/// Calculate expected value
pub fn expected_value(
    win_probability: Decimal,
    win_amount: Decimal,
    loss_amount: Decimal,
) -> Decimal {
    let q = dec!(1) - win_probability;
    win_probability * win_amount - q * loss_amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kelly_positive_edge() {
        let kelly = KellyCriterion::default();

        // 60% win rate, 1:1 risk/reward
        let position = kelly.calculate(dec!(0.6), dec!(100), dec!(100));

        // Full Kelly would be 0.2 (20%)
        // Quarter Kelly = 0.05 (5%)
        assert!(position > dec!(0.04) && position < dec!(0.06));
    }

    #[test]
    fn test_kelly_negative_edge() {
        let kelly = KellyCriterion::default();

        // 40% win rate, 1:1 risk/reward - negative edge
        let position = kelly.calculate(dec!(0.4), dec!(100), dec!(100));

        assert_eq!(position, Decimal::ZERO); // Should not bet
    }

    #[test]
    fn test_kelly_high_risk_reward() {
        let kelly = KellyCriterion::default();

        // 30% win rate but 3:1 risk/reward
        let position = kelly.calculate(dec!(0.3), dec!(300), dec!(100));

        // EV positive, should have small position
        assert!(position > Decimal::ZERO);
    }

    #[test]
    fn test_kelly_max_cap() {
        let kelly = KellyCriterion::new(dec!(1.0)); // Full Kelly

        // Very high edge
        let position = kelly.calculate(dec!(0.9), dec!(200), dec!(100));

        // Should be capped at max_position
        assert!(position <= dec!(0.15));
    }

    #[test]
    fn test_from_history() {
        let kelly = KellyCriterion::default();

        // 70 wins, 30 losses, avg $10 win, avg $8 loss
        let position = kelly.from_history(70, 30, dec!(10), dec!(8));

        assert!(position > Decimal::ZERO);
    }

    #[test]
    fn test_expected_value() {
        // 60% win rate, win $100, lose $100
        let ev = expected_value(dec!(0.6), dec!(100), dec!(100));
        assert_eq!(ev, dec!(20)); // $60 - $40 = $20

        // 50% win rate, win $150, lose $100
        let ev = expected_value(dec!(0.5), dec!(150), dec!(100));
        assert_eq!(ev, dec!(25)); // $75 - $50 = $25
    }
}
