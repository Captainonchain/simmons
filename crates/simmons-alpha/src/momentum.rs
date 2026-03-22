//! Momentum-based trading signals

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_core::{Signal, StrategySignal};

/// Momentum signal generator
pub struct MomentumEngine {
    /// Lookback period for calculations
    pub lookback: usize,
    /// RSI period
    pub rsi_period: usize,
    /// RSI overbought threshold
    pub rsi_overbought: Decimal,
    /// RSI oversold threshold
    pub rsi_oversold: Decimal,
}

impl Default for MomentumEngine {
    fn default() -> Self {
        Self {
            lookback: 14,
            rsi_period: 14,
            rsi_overbought: dec!(70),
            rsi_oversold: dec!(30),
        }
    }
}

impl MomentumEngine {
    pub fn new(lookback: usize, rsi_period: usize) -> Self {
        Self {
            lookback,
            rsi_period,
            ..Default::default()
        }
    }

    /// Calculate Rate of Change (ROC)
    pub fn calculate_roc(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.lookback + 1 {
            return None;
        }

        let current = *prices.last()?;
        let past = prices[prices.len() - self.lookback - 1];

        if past.is_zero() {
            return None;
        }

        Some((current - past) / past * dec!(100))
    }

    /// Calculate Relative Strength Index (RSI)
    pub fn calculate_rsi(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.rsi_period + 1 {
            return None;
        }

        let changes: Vec<Decimal> = prices
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect();

        let recent_changes = &changes[changes.len().saturating_sub(self.rsi_period)..];

        let mut gains = Decimal::ZERO;
        let mut losses = Decimal::ZERO;

        for change in recent_changes {
            if *change > Decimal::ZERO {
                gains += *change;
            } else {
                losses += change.abs();
            }
        }

        let n = Decimal::from(recent_changes.len());
        let avg_gain = gains / n;
        let avg_loss = losses / n;

        if avg_loss.is_zero() {
            return Some(dec!(100));
        }

        let rs = avg_gain / avg_loss;
        Some(dec!(100) - (dec!(100) / (dec!(1) + rs)))
    }

    /// Calculate momentum (simple price change percentage)
    pub fn calculate_momentum(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < 2 {
            return None;
        }

        let n = prices.len().min(self.lookback);
        let start = prices[prices.len() - n];
        let end = *prices.last()?;

        if start.is_zero() {
            return None;
        }

        Some((end - start) / start * dec!(100))
    }

    /// Generate trading signal
    pub fn generate_signal(&self, prices: &[Decimal]) -> Option<StrategySignal> {
        let rsi = self.calculate_rsi(prices)?;
        let roc = self.calculate_roc(prices)?;
        let momentum = self.calculate_momentum(prices)?;

        let (signal, confidence, reason) = if rsi < self.rsi_oversold && roc > dec!(-5) {
            // Oversold but not in freefall
            (
                Signal::Buy,
                dec!(0.6) + (self.rsi_oversold - rsi) / dec!(100),
                format!("RSI {:.1} oversold, ROC {:.2}%", rsi, roc),
            )
        } else if rsi > self.rsi_overbought && roc < dec!(5) {
            // Overbought but not surging
            (
                Signal::Sell,
                dec!(0.6) + (rsi - self.rsi_overbought) / dec!(100),
                format!("RSI {:.1} overbought, ROC {:.2}%", rsi, roc),
            )
        } else if momentum > dec!(3) && rsi < dec!(65) {
            // Strong momentum with room to run
            (
                Signal::Buy,
                dec!(0.5) + momentum / dec!(20),
                format!("Momentum +{:.2}%, RSI {:.1}", momentum, rsi),
            )
        } else if momentum < dec!(-3) && rsi > dec!(35) {
            // Strong downward momentum with room to fall
            (
                Signal::Sell,
                dec!(0.5) + momentum.abs() / dec!(20),
                format!("Momentum {:.2}%, RSI {:.1}", momentum, rsi),
            )
        } else {
            (
                Signal::Hold,
                dec!(0.3),
                format!("RSI {:.1}, ROC {:.2}%", rsi, roc),
            )
        };

        Some(StrategySignal {
            strategy: "momentum".to_string(),
            signal,
            confidence: confidence.min(dec!(0.95)),
            reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_prices(base: i32, changes: &[i32]) -> Vec<Decimal> {
        let mut prices = vec![Decimal::from(base)];
        for change in changes {
            let last = *prices.last().unwrap();
            prices.push(last + Decimal::from(*change));
        }
        prices
    }

    #[test]
    fn test_rsi_calculation() {
        let engine = MomentumEngine::default();

        // Uptrending prices
        let prices: Vec<Decimal> = (100..120).map(Decimal::from).collect();
        let rsi = engine.calculate_rsi(&prices).unwrap();
        assert!(rsi > dec!(50)); // Should be bullish

        // Downtrending prices
        let prices: Vec<Decimal> = (100..120).rev().map(Decimal::from).collect();
        let rsi = engine.calculate_rsi(&prices).unwrap();
        assert!(rsi < dec!(50)); // Should be bearish
    }

    #[test]
    fn test_roc_calculation() {
        let engine = MomentumEngine::new(5, 14);
        let prices: Vec<Decimal> = vec![100, 102, 104, 106, 108, 110]
            .into_iter()
            .map(Decimal::from)
            .collect();

        let roc = engine.calculate_roc(&prices).unwrap();
        assert_eq!(roc, dec!(10)); // 110/100 - 1 = 10%
    }

    #[test]
    fn test_signal_generation() {
        let engine = MomentumEngine::default();

        // Strong uptrend
        let prices: Vec<Decimal> = (100..130).map(Decimal::from).collect();
        let signal = engine.generate_signal(&prices).unwrap();
        assert!(signal.signal.is_bullish() || signal.signal == Signal::Hold);
    }
}
