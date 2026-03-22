//! Mean reversion trading signals

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_core::{Signal, StrategySignal};

/// Mean reversion signal generator
pub struct MeanReversionEngine {
    /// Lookback period for mean calculation
    pub lookback: usize,
    /// Z-score threshold for signals
    pub zscore_threshold: Decimal,
    /// Bollinger band standard deviation multiplier
    pub bb_std_mult: Decimal,
}

impl Default for MeanReversionEngine {
    fn default() -> Self {
        Self {
            lookback: 20,
            zscore_threshold: dec!(2),
            bb_std_mult: dec!(2),
        }
    }
}

impl MeanReversionEngine {
    pub fn new(lookback: usize, zscore_threshold: Decimal) -> Self {
        Self {
            lookback,
            zscore_threshold,
            ..Default::default()
        }
    }

    /// Calculate simple moving average
    pub fn calculate_sma(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.lookback {
            return None;
        }

        let recent = &prices[prices.len() - self.lookback..];
        let sum: Decimal = recent.iter().sum();
        Some(sum / Decimal::from(self.lookback))
    }

    /// Calculate standard deviation
    pub fn calculate_std(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.lookback {
            return None;
        }

        let recent = &prices[prices.len() - self.lookback..];
        let mean = self.calculate_sma(prices)?;
        let n = Decimal::from(recent.len());

        let variance: Decimal = recent
            .iter()
            .map(|p| {
                let diff = *p - mean;
                diff * diff
            })
            .sum::<Decimal>()
            / n;

        // Approximate sqrt using Newton-Raphson
        Some(self.decimal_sqrt(variance))
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

    /// Calculate Z-score
    pub fn calculate_zscore(&self, prices: &[Decimal]) -> Option<Decimal> {
        let current = *prices.last()?;
        let mean = self.calculate_sma(prices)?;
        let std = self.calculate_std(prices)?;

        if std.is_zero() {
            return Some(Decimal::ZERO);
        }

        Some((current - mean) / std)
    }

    /// Calculate Bollinger Bands
    pub fn calculate_bollinger_bands(&self, prices: &[Decimal]) -> Option<(Decimal, Decimal, Decimal)> {
        let sma = self.calculate_sma(prices)?;
        let std = self.calculate_std(prices)?;

        let upper = sma + self.bb_std_mult * std;
        let lower = sma - self.bb_std_mult * std;

        Some((upper, sma, lower))
    }

    /// Calculate percent B (position within Bollinger Bands)
    pub fn calculate_percent_b(&self, prices: &[Decimal]) -> Option<Decimal> {
        let current = *prices.last()?;
        let (upper, _, lower) = self.calculate_bollinger_bands(prices)?;

        let range = upper - lower;
        if range.is_zero() {
            return Some(dec!(0.5));
        }

        Some((current - lower) / range)
    }

    /// Generate trading signal
    pub fn generate_signal(&self, prices: &[Decimal]) -> Option<StrategySignal> {
        let zscore = self.calculate_zscore(prices)?;
        let percent_b = self.calculate_percent_b(prices)?;

        let (signal, confidence, reason) = if zscore < -self.zscore_threshold {
            // Price significantly below mean - buy signal
            let conf = dec!(0.6) + zscore.abs() / dec!(10);
            (
                Signal::Buy,
                conf,
                format!("Z-score {:.2} (oversold), %B {:.2}", zscore, percent_b),
            )
        } else if zscore > self.zscore_threshold {
            // Price significantly above mean - sell signal
            let conf = dec!(0.6) + zscore.abs() / dec!(10);
            (
                Signal::Sell,
                conf,
                format!("Z-score {:.2} (overbought), %B {:.2}", zscore, percent_b),
            )
        } else if percent_b < dec!(0.2) {
            // Near lower Bollinger band
            (
                Signal::Buy,
                dec!(0.55),
                format!("Near lower BB, %B {:.2}, Z {:.2}", percent_b, zscore),
            )
        } else if percent_b > dec!(0.8) {
            // Near upper Bollinger band
            (
                Signal::Sell,
                dec!(0.55),
                format!("Near upper BB, %B {:.2}, Z {:.2}", percent_b, zscore),
            )
        } else {
            (
                Signal::Hold,
                dec!(0.3),
                format!("Z-score {:.2}, %B {:.2}", zscore, percent_b),
            )
        };

        Some(StrategySignal {
            strategy: "mean_reversion".to_string(),
            signal,
            confidence: confidence.min(dec!(0.95)),
            reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_calculation() {
        let engine = MeanReversionEngine::new(5, dec!(2));
        let prices: Vec<Decimal> = vec![10, 11, 12, 13, 14]
            .into_iter()
            .map(Decimal::from)
            .collect();

        let sma = engine.calculate_sma(&prices).unwrap();
        assert_eq!(sma, dec!(12)); // (10+11+12+13+14)/5 = 12
    }

    #[test]
    fn test_zscore_calculation() {
        let engine = MeanReversionEngine::new(10, dec!(2));

        // Create prices centered around 100 with small deviation, then spike
        let mut prices: Vec<Decimal> = vec![100, 99, 101, 100, 99, 101, 100, 99, 101, 100]
            .into_iter()
            .map(Decimal::from)
            .collect();

        let zscore_normal = engine.calculate_zscore(&prices).unwrap();
        assert!(zscore_normal.abs() < dec!(1)); // Normal range

        // Add spike
        prices.push(dec!(110));
        let zscore_spike = engine.calculate_zscore(&prices).unwrap();
        assert!(zscore_spike > dec!(1)); // Should be elevated
    }

    #[test]
    fn test_bollinger_bands() {
        let engine = MeanReversionEngine::new(5, dec!(2));
        let prices: Vec<Decimal> = vec![100, 101, 102, 101, 100]
            .into_iter()
            .map(Decimal::from)
            .collect();

        let (upper, middle, lower) = engine.calculate_bollinger_bands(&prices).unwrap();
        assert!(upper > middle);
        assert!(middle > lower);
        assert_eq!(middle, dec!(100.8)); // SMA
    }
}
