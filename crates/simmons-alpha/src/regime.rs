//! Market regime detection

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_core::Regime;

/// Regime detector
pub struct RegimeDetector {
    /// Lookback period for trend detection
    pub trend_lookback: usize,
    /// Volatility lookback period
    pub vol_lookback: usize,
    /// High volatility threshold (annualized)
    pub high_vol_threshold: Decimal,
    /// Low volatility threshold (annualized)
    pub low_vol_threshold: Decimal,
    /// Trend strength threshold (percentage)
    pub trend_threshold: Decimal,
}

impl Default for RegimeDetector {
    fn default() -> Self {
        Self {
            trend_lookback: 20,
            vol_lookback: 20,
            high_vol_threshold: dec!(0.5),  // 50% annualized
            low_vol_threshold: dec!(0.15),  // 15% annualized
            trend_threshold: dec!(2),       // 2% price change
        }
    }
}

impl RegimeDetector {
    pub fn new(trend_lookback: usize, vol_lookback: usize) -> Self {
        Self {
            trend_lookback,
            vol_lookback,
            ..Default::default()
        }
    }

    /// Calculate realized volatility (annualized)
    pub fn calculate_volatility(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.vol_lookback + 1 {
            return None;
        }

        let returns: Vec<Decimal> = prices
            .windows(2)
            .filter_map(|w| {
                if w[0].is_zero() {
                    None
                } else {
                    Some((w[1] - w[0]) / w[0])
                }
            })
            .collect();

        let recent = &returns[returns.len().saturating_sub(self.vol_lookback)..];
        if recent.is_empty() {
            return None;
        }

        let n = Decimal::from(recent.len());
        let mean: Decimal = recent.iter().sum::<Decimal>() / n;

        let variance: Decimal = recent
            .iter()
            .map(|r| {
                let diff = *r - mean;
                diff * diff
            })
            .sum::<Decimal>()
            / n;

        let std = self.decimal_sqrt(variance);

        // Annualize (assuming hourly data, ~8760 hours/year)
        Some(std * self.decimal_sqrt(Decimal::from(8760)))
    }

    fn decimal_sqrt(&self, x: Decimal) -> Decimal {
        if x.is_zero() || x.is_sign_negative() {
            return Decimal::ZERO;
        }

        let mut guess = x / dec!(2);
        for _ in 0..10 {
            if guess.is_zero() {
                break;
            }
            guess = (guess + x / guess) / dec!(2);
        }
        guess
    }

    /// Calculate trend strength
    pub fn calculate_trend(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.trend_lookback {
            return None;
        }

        let start = prices[prices.len() - self.trend_lookback];
        let end = *prices.last()?;

        if start.is_zero() {
            return None;
        }

        Some((end - start) / start * dec!(100))
    }

    /// Count direction changes (for choppiness detection)
    pub fn count_direction_changes(&self, prices: &[Decimal]) -> usize {
        if prices.len() < 3 {
            return 0;
        }

        let recent = &prices[prices.len().saturating_sub(self.trend_lookback)..];

        recent
            .windows(2)
            .map(|w| (w[1] - w[0]).is_sign_positive())
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[0] != w[1])
            .count()
    }

    /// Detect current market regime
    pub fn detect(&self, prices: &[Decimal]) -> Regime {
        let volatility = self.calculate_volatility(prices).unwrap_or_default();
        let trend = self.calculate_trend(prices).unwrap_or_default();
        let changes = self.count_direction_changes(prices);

        // High volatility regime takes precedence
        if volatility > self.high_vol_threshold {
            return Regime::HighVolatility;
        }

        // Low volatility regime
        if volatility < self.low_vol_threshold {
            return Regime::LowVolatility;
        }

        // Choppy market (many direction changes)
        let expected_changes = self.trend_lookback / 3;
        if changes > expected_changes {
            return Regime::Choppy;
        }

        // Trending regimes
        if trend > self.trend_threshold {
            Regime::TrendingUp
        } else if trend < -self.trend_threshold {
            Regime::TrendingDown
        } else {
            Regime::MeanReverting
        }
    }

    /// Get regime description
    pub fn regime_description(&self, regime: Regime) -> &'static str {
        match regime {
            Regime::TrendingUp => "Bullish trend, favor momentum strategies",
            Regime::TrendingDown => "Bearish trend, favor momentum or stay flat",
            Regime::MeanReverting => "Range-bound, favor mean reversion",
            Regime::HighVolatility => "High volatility, reduce position sizes",
            Regime::LowVolatility => "Low volatility, consider breakout plays",
            Regime::Choppy => "Choppy market, avoid trading",
        }
    }

    /// Get regime trading bias
    pub fn regime_bias(&self, regime: Regime) -> i8 {
        match regime {
            Regime::TrendingUp => 1,      // Bullish
            Regime::TrendingDown => -1,   // Bearish
            Regime::MeanReverting => 0,   // Neutral
            Regime::HighVolatility => 0,  // Caution
            Regime::LowVolatility => 0,   // Neutral
            Regime::Choppy => 0,          // Avoid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_detection() {
        let detector = RegimeDetector::new(10, 10);

        // Strong uptrend
        let prices: Vec<Decimal> = (100..115).map(Decimal::from).collect();
        let regime = detector.detect(&prices);
        assert_eq!(regime, Regime::TrendingUp);

        // Strong downtrend
        let prices: Vec<Decimal> = (100..115).rev().map(Decimal::from).collect();
        let regime = detector.detect(&prices);
        assert_eq!(regime, Regime::TrendingDown);
    }

    #[test]
    fn test_choppy_detection() {
        let detector = RegimeDetector::new(10, 10);

        // Alternating prices
        let prices: Vec<Decimal> = vec![100, 101, 99, 101, 99, 101, 99, 101, 99, 100, 101]
            .into_iter()
            .map(Decimal::from)
            .collect();

        let changes = detector.count_direction_changes(&prices);
        assert!(changes > 5); // Many direction changes
    }

    #[test]
    fn test_volatility_calculation() {
        let detector = RegimeDetector::new(10, 10);

        // Stable prices
        let prices: Vec<Decimal> = vec![100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100]
            .into_iter()
            .map(Decimal::from)
            .collect();

        let vol = detector.calculate_volatility(&prices).unwrap();
        assert_eq!(vol, Decimal::ZERO); // No volatility
    }
}
