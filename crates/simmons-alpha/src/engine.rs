//! Unified alpha engine combining all signal sources

use crate::arbitrage::ArbitrageEngine;
use crate::mean_reversion::MeanReversionEngine;
use crate::momentum::MomentumEngine;
use crate::regime::RegimeDetector;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_core::{ArbOpportunity, Regime, Signal, StrategySignal};

/// Unified alpha/signal generation engine
pub struct AlphaEngine {
    pub momentum: MomentumEngine,
    pub mean_reversion: MeanReversionEngine,
    pub regime_detector: RegimeDetector,
    pub arbitrage: ArbitrageEngine,
    /// Minimum confidence to include signal
    pub min_confidence: Decimal,
}

impl Default for AlphaEngine {
    fn default() -> Self {
        Self {
            momentum: MomentumEngine::default(),
            mean_reversion: MeanReversionEngine::default(),
            regime_detector: RegimeDetector::default(),
            arbitrage: ArbitrageEngine::default(),
            min_confidence: dec!(0.5),
        }
    }
}

impl AlphaEngine {
    pub fn new(
        momentum: MomentumEngine,
        mean_reversion: MeanReversionEngine,
        regime_detector: RegimeDetector,
        arbitrage: ArbitrageEngine,
    ) -> Self {
        Self {
            momentum,
            mean_reversion,
            regime_detector,
            arbitrage,
            min_confidence: dec!(0.5),
        }
    }

    /// Detect current market regime
    pub fn detect_regime(&self, prices: &[Decimal]) -> Regime {
        self.regime_detector.detect(prices)
    }

    /// Generate all signals for a symbol
    pub fn generate_signals(&self, symbol: &str, prices: &[Decimal]) -> Vec<StrategySignal> {
        let mut signals = Vec::new();
        let regime = self.detect_regime(prices);

        // Momentum signal
        if let Some(mut sig) = self.momentum.generate_signal(prices) {
            // Adjust confidence based on regime
            sig.confidence = self.adjust_for_regime(sig.confidence, &sig.signal, regime);
            if sig.confidence >= self.min_confidence {
                signals.push(sig);
            }
        }

        // Mean reversion signal
        if let Some(mut sig) = self.mean_reversion.generate_signal(prices) {
            sig.confidence = self.adjust_for_regime(sig.confidence, &sig.signal, regime);
            if sig.confidence >= self.min_confidence {
                signals.push(sig);
            }
        }

        // Add regime signal
        let regime_signal = self.regime_to_signal(regime, prices);
        signals.push(regime_signal);

        signals
    }

    /// Adjust signal confidence based on market regime
    fn adjust_for_regime(&self, confidence: Decimal, signal: &Signal, regime: Regime) -> Decimal {
        let multiplier = match (regime, signal) {
            // Momentum works well in trends
            (Regime::TrendingUp, Signal::Buy) | (Regime::TrendingUp, Signal::StrongBuy) => dec!(1.2),
            (Regime::TrendingDown, Signal::Sell) | (Regime::TrendingDown, Signal::StrongSell) => dec!(1.2),

            // Mean reversion works in ranging markets
            (Regime::MeanReverting, _) => dec!(1.1),

            // Reduce confidence in choppy markets
            (Regime::Choppy, _) => dec!(0.6),

            // Reduce position sizing in high volatility
            (Regime::HighVolatility, _) => dec!(0.8),

            // Contrarian signals in trends are risky
            (Regime::TrendingUp, Signal::Sell) | (Regime::TrendingUp, Signal::StrongSell) => dec!(0.7),
            (Regime::TrendingDown, Signal::Buy) | (Regime::TrendingDown, Signal::StrongBuy) => dec!(0.7),

            _ => dec!(1.0),
        };

        (confidence * multiplier).min(dec!(0.95))
    }

    /// Convert regime to a strategy signal
    fn regime_to_signal(&self, regime: Regime, prices: &[Decimal]) -> StrategySignal {
        let (signal, confidence, reason) = match regime {
            Regime::TrendingUp => (
                Signal::Buy,
                dec!(0.6),
                "Bullish regime detected".to_string(),
            ),
            Regime::TrendingDown => (
                Signal::Sell,
                dec!(0.6),
                "Bearish regime detected".to_string(),
            ),
            Regime::MeanReverting => (
                Signal::Hold,
                dec!(0.5),
                "Range-bound market".to_string(),
            ),
            Regime::HighVolatility => (
                Signal::Hold,
                dec!(0.4),
                "High volatility - caution".to_string(),
            ),
            Regime::LowVolatility => (
                Signal::Hold,
                dec!(0.5),
                "Low volatility - watch for breakout".to_string(),
            ),
            Regime::Choppy => (
                Signal::Hold,
                dec!(0.3),
                "Choppy market - avoid trading".to_string(),
            ),
        };

        StrategySignal {
            strategy: "regime".to_string(),
            signal,
            confidence,
            reason,
        }
    }

    /// Combine signals to get overall recommendation
    pub fn combine_signals(&self, signals: &[StrategySignal]) -> (Signal, Decimal) {
        if signals.is_empty() {
            return (Signal::Hold, dec!(0.0));
        }

        // Weight signals by confidence
        let mut buy_score = Decimal::ZERO;
        let mut sell_score = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for sig in signals {
            let weight = sig.confidence;
            total_weight += weight;

            match sig.signal {
                Signal::StrongBuy => buy_score += weight * dec!(2),
                Signal::Buy => buy_score += weight,
                Signal::StrongSell => sell_score += weight * dec!(2),
                Signal::Sell => sell_score += weight,
                Signal::Hold => {}
            }
        }

        if total_weight.is_zero() {
            return (Signal::Hold, dec!(0.0));
        }

        let net_score = (buy_score - sell_score) / total_weight;
        let confidence = (buy_score + sell_score) / total_weight / dec!(2);

        let signal = if net_score > dec!(0.5) {
            Signal::StrongBuy
        } else if net_score > dec!(0.2) {
            Signal::Buy
        } else if net_score < dec!(-0.5) {
            Signal::StrongSell
        } else if net_score < dec!(-0.2) {
            Signal::Sell
        } else {
            Signal::Hold
        };

        (signal, confidence.min(dec!(0.95)))
    }

    /// Check for arbitrage opportunities
    pub fn check_arbitrage(
        &self,
        symbol: &str,
        cex_price: Decimal,
        dex_price: Option<Decimal>,
        capital: Decimal,
    ) -> Vec<ArbOpportunity> {
        let mut opportunities = Vec::new();

        if let Some(dex) = dex_price {
            if let Some(arb) = self.arbitrage.check_cedefi_arb(symbol, cex_price, dex, capital) {
                opportunities.push(arb);
            }
        }

        opportunities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_signals() {
        let engine = AlphaEngine::default();

        // Create uptrending prices
        let prices: Vec<Decimal> = (100..130).map(Decimal::from).collect();
        let signals = engine.generate_signals("BTC-USDT", &prices);

        assert!(!signals.is_empty());

        // Should have regime signal at minimum
        let regime_sig = signals.iter().find(|s| s.strategy == "regime");
        assert!(regime_sig.is_some());
    }

    #[test]
    fn test_combine_signals() {
        let engine = AlphaEngine::default();

        let signals = vec![
            StrategySignal {
                strategy: "momentum".to_string(),
                signal: Signal::Buy,
                confidence: dec!(0.8),
                reason: "test".to_string(),
            },
            StrategySignal {
                strategy: "mean_reversion".to_string(),
                signal: Signal::Buy,
                confidence: dec!(0.8),
                reason: "test".to_string(),
            },
        ];

        let (combined, confidence) = engine.combine_signals(&signals);
        assert!(combined.is_bullish());
        assert!(confidence > dec!(0.3)); // Confidence is weighted
    }

    #[test]
    fn test_regime_adjustment() {
        let engine = AlphaEngine::default();

        // In trending up market, buy signals should be boosted
        let boosted = engine.adjust_for_regime(dec!(0.7), &Signal::Buy, Regime::TrendingUp);
        assert!(boosted > dec!(0.7));

        // In choppy market, all signals should be reduced
        let reduced = engine.adjust_for_regime(dec!(0.7), &Signal::Buy, Regime::Choppy);
        assert!(reduced < dec!(0.7));
    }
}
