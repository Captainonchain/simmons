//! Nunchi Signal Aggregation
//!
//! Aggregates signals from multiple trading strategies into a unified score.
//! Named after the Korean concept of "nunchi" - social awareness and reading situations.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::{Regime, Signal, StrategySignal};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Nunchi aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NunchiConfig {
    /// Strategy weights
    pub weights: HashMap<String, Decimal>,
    /// Minimum confidence to include signal
    pub min_confidence: Decimal,
    /// Regime-specific weight adjustments
    pub regime_adjustments: HashMap<String, Decimal>,
    /// Enable signal decay over time
    pub signal_decay: bool,
    /// Decay half-life in seconds
    pub decay_half_life_secs: u64,
}

impl Default for NunchiConfig {
    fn default() -> Self {
        let mut weights = HashMap::new();
        weights.insert("momentum".to_string(), dec!(0.25));
        weights.insert("mean_reversion".to_string(), dec!(0.20));
        weights.insert("regime".to_string(), dec!(0.20));
        weights.insert("arbitrage".to_string(), dec!(0.15));
        weights.insert("sentiment".to_string(), dec!(0.10));
        weights.insert("volume".to_string(), dec!(0.10));

        let mut regime_adjustments = HashMap::new();
        regime_adjustments.insert("momentum_trending".to_string(), dec!(1.5));
        regime_adjustments.insert("mean_reversion_ranging".to_string(), dec!(1.3));
        regime_adjustments.insert("momentum_choppy".to_string(), dec!(0.5));

        Self {
            weights,
            min_confidence: dec!(0.3),
            regime_adjustments,
            signal_decay: true,
            decay_half_life_secs: 300, // 5 minutes
        }
    }
}

/// Aggregated Nunchi score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NunchiScore {
    /// Overall score (-1 to +1)
    pub score: Decimal,
    /// Confidence in the score (0 to 1)
    pub confidence: Decimal,
    /// Recommended action
    pub recommendation: NunchiRecommendation,
    /// Contributing signals
    pub signal_contributions: Vec<SignalContribution>,
    /// Active regime
    pub regime: Regime,
    /// Timestamp
    pub timestamp: i64,
}

impl NunchiScore {
    /// Is the score actionable (high confidence)?
    pub fn is_actionable(&self) -> bool {
        self.confidence > dec!(0.6) && self.score.abs() > dec!(0.3)
    }

    /// Direction strength (0 to 1)
    pub fn direction_strength(&self) -> Decimal {
        self.score.abs()
    }

    /// Is bullish?
    pub fn is_bullish(&self) -> bool {
        self.score > dec!(0.1)
    }

    /// Is bearish?
    pub fn is_bearish(&self) -> bool {
        self.score < dec!(-0.1)
    }
}

/// Nunchi recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NunchiRecommendation {
    StrongBuy,
    Buy,
    Hold,
    Sell,
    StrongSell,
    Wait,
}

impl NunchiRecommendation {
    pub fn from_score(score: Decimal, confidence: Decimal) -> Self {
        if confidence < dec!(0.5) {
            return NunchiRecommendation::Wait;
        }

        match score {
            s if s > dec!(0.6) => NunchiRecommendation::StrongBuy,
            s if s > dec!(0.3) => NunchiRecommendation::Buy,
            s if s < dec!(-0.6) => NunchiRecommendation::StrongSell,
            s if s < dec!(-0.3) => NunchiRecommendation::Sell,
            _ => NunchiRecommendation::Hold,
        }
    }
}

/// Individual signal contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalContribution {
    pub strategy: String,
    pub raw_signal: Decimal,
    pub weight: Decimal,
    pub regime_adjustment: Decimal,
    pub weighted_contribution: Decimal,
    pub confidence: Decimal,
}

/// Nunchi signal aggregator
pub struct NunchiSignals {
    config: NunchiConfig,
    recent_signals: Vec<TimestampedSignal>,
    current_regime: Regime,
}

/// Signal with timestamp
#[derive(Debug, Clone)]
struct TimestampedSignal {
    signal: StrategySignal,
    timestamp: i64,
}

impl NunchiSignals {
    pub fn new(config: NunchiConfig) -> Self {
        Self {
            config,
            recent_signals: Vec::new(),
            current_regime: Regime::default(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(NunchiConfig::default())
    }

    /// Set current market regime
    pub fn set_regime(&mut self, regime: Regime) {
        self.current_regime = regime;
    }

    /// Add a new signal
    pub fn add_signal(&mut self, signal: StrategySignal) {
        let timestamped = TimestampedSignal {
            signal,
            timestamp: chrono::Utc::now().timestamp(),
        };
        self.recent_signals.push(timestamped);

        // Prune old signals
        let cutoff = chrono::Utc::now().timestamp() - (self.config.decay_half_life_secs * 4) as i64;
        self.recent_signals.retain(|s| s.timestamp > cutoff);
    }

    /// Aggregate all signals into a Nunchi score
    pub fn aggregate(&self, signals: &[StrategySignal]) -> NunchiScore {
        let mut contributions = Vec::new();
        let mut total_weight = Decimal::ZERO;
        let mut weighted_sum = Decimal::ZERO;
        let mut confidence_sum = Decimal::ZERO;

        for signal in signals {
            // Skip low confidence signals
            if signal.confidence < self.config.min_confidence {
                continue;
            }

            // Get base weight
            let base_weight = self
                .config
                .weights
                .get(&signal.strategy)
                .copied()
                .unwrap_or(dec!(0.1));

            // Apply regime adjustment
            let regime_key = format!("{}_{}", signal.strategy, regime_to_str(&self.current_regime));
            let regime_adj = self
                .config
                .regime_adjustments
                .get(&regime_key)
                .copied()
                .unwrap_or(Decimal::ONE);

            let adjusted_weight = base_weight * regime_adj;

            // Convert signal to numeric
            let raw_signal = signal_to_decimal(&signal.signal);

            // Calculate weighted contribution
            let contribution = raw_signal * adjusted_weight * signal.confidence;

            contributions.push(SignalContribution {
                strategy: signal.strategy.clone(),
                raw_signal,
                weight: base_weight,
                regime_adjustment: regime_adj,
                weighted_contribution: contribution,
                confidence: signal.confidence,
            });

            weighted_sum += contribution;
            total_weight += adjusted_weight * signal.confidence;
            confidence_sum += signal.confidence;
        }

        // Calculate final score
        let score = if total_weight.is_zero() {
            Decimal::ZERO
        } else {
            weighted_sum / total_weight
        };

        // Calculate overall confidence
        let confidence = if contributions.is_empty() {
            Decimal::ZERO
        } else {
            confidence_sum / Decimal::from(contributions.len())
        };

        // Generate recommendation
        let recommendation = NunchiRecommendation::from_score(score, confidence);

        NunchiScore {
            score,
            confidence,
            recommendation,
            signal_contributions: contributions,
            regime: self.current_regime,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Should we trade based on the score?
    pub fn should_trade(&self, score: &NunchiScore, threshold: Decimal) -> TradeDecision {
        // Check confidence
        if score.confidence < dec!(0.5) {
            return TradeDecision {
                should_trade: false,
                reason: "Confidence too low".to_string(),
                suggested_size_pct: Decimal::ZERO,
            };
        }

        // Check score magnitude
        if score.score.abs() < threshold {
            return TradeDecision {
                should_trade: false,
                reason: format!("Score {} below threshold {}", score.score.abs(), threshold),
                suggested_size_pct: Decimal::ZERO,
            };
        }

        // Check for conflicting signals
        let bullish_count = score
            .signal_contributions
            .iter()
            .filter(|c| c.weighted_contribution > Decimal::ZERO)
            .count();
        let bearish_count = score
            .signal_contributions
            .iter()
            .filter(|c| c.weighted_contribution < Decimal::ZERO)
            .count();

        if bullish_count > 0 && bearish_count > 0 {
            let agreement_ratio = (bullish_count.max(bearish_count) as f64)
                / (bullish_count + bearish_count) as f64;
            if agreement_ratio < 0.7 {
                return TradeDecision {
                    should_trade: false,
                    reason: "Signals conflicting".to_string(),
                    suggested_size_pct: Decimal::ZERO,
                };
            }
        }

        // Calculate suggested size based on confidence and score strength
        let size_multiplier = score.confidence * score.score.abs();
        let suggested_size = (size_multiplier * dec!(100)).min(dec!(100));

        TradeDecision {
            should_trade: true,
            reason: format!(
                "{:?} with {:.0}% confidence",
                score.recommendation,
                score.confidence * dec!(100)
            ),
            suggested_size_pct: suggested_size,
        }
    }

    /// Get regime-adjusted weights
    pub fn get_effective_weights(&self) -> HashMap<String, Decimal> {
        let mut effective = HashMap::new();

        for (strategy, base_weight) in &self.config.weights {
            let regime_key = format!("{}_{}", strategy, regime_to_str(&self.current_regime));
            let regime_adj = self
                .config
                .regime_adjustments
                .get(&regime_key)
                .copied()
                .unwrap_or(Decimal::ONE);

            effective.insert(strategy.clone(), *base_weight * regime_adj);
        }

        effective
    }

    /// Analyze signal correlation
    pub fn analyze_correlation(&self, signals: &[StrategySignal]) -> SignalCorrelation {
        let numerics: Vec<Decimal> = signals.iter().map(|s| signal_to_decimal(&s.signal)).collect();

        if numerics.len() < 2 {
            return SignalCorrelation::default();
        }

        // Count agreement
        let bullish = numerics.iter().filter(|&&n| n > Decimal::ZERO).count();
        let bearish = numerics.iter().filter(|&&n| n < Decimal::ZERO).count();
        let neutral = numerics.iter().filter(|&&n| n == Decimal::ZERO).count();

        let total = numerics.len();
        let max_agreement = bullish.max(bearish).max(neutral);
        let agreement_pct = Decimal::from(max_agreement) / Decimal::from(total) * dec!(100);

        // Determine dominant direction
        let dominant = if bullish > bearish && bullish > neutral {
            DominantDirection::Bullish
        } else if bearish > bullish && bearish > neutral {
            DominantDirection::Bearish
        } else {
            DominantDirection::Mixed
        };

        SignalCorrelation {
            bullish_count: bullish,
            bearish_count: bearish,
            neutral_count: neutral,
            agreement_pct,
            dominant,
        }
    }
}

/// Trade decision from Nunchi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeDecision {
    pub should_trade: bool,
    pub reason: String,
    pub suggested_size_pct: Decimal,
}

/// Signal correlation analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignalCorrelation {
    pub bullish_count: usize,
    pub bearish_count: usize,
    pub neutral_count: usize,
    pub agreement_pct: Decimal,
    pub dominant: DominantDirection,
}

/// Dominant signal direction
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DominantDirection {
    Bullish,
    Bearish,
    #[default]
    Mixed,
}

/// Convert Signal to numeric value
fn signal_to_decimal(signal: &Signal) -> Decimal {
    match signal {
        Signal::StrongBuy => dec!(1.0),
        Signal::Buy => dec!(0.5),
        Signal::Hold => dec!(0.0),
        Signal::Sell => dec!(-0.5),
        Signal::StrongSell => dec!(-1.0),
    }
}

/// Convert Regime to string key
fn regime_to_str(regime: &Regime) -> &'static str {
    match regime {
        Regime::TrendingUp => "trending",
        Regime::TrendingDown => "trending",
        Regime::MeanReverting => "ranging",
        Regime::HighVolatility => "volatile",
        Regime::LowVolatility => "calm",
        Regime::Choppy => "choppy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal(strategy: &str, signal: Signal, confidence: Decimal) -> StrategySignal {
        StrategySignal {
            strategy: strategy.to_string(),
            signal,
            confidence,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn test_aggregate_bullish_signals() {
        let nunchi = NunchiSignals::with_defaults();

        let signals = vec![
            make_signal("momentum", Signal::Buy, dec!(0.8)),
            make_signal("mean_reversion", Signal::Buy, dec!(0.7)),
            make_signal("regime", Signal::StrongBuy, dec!(0.9)),
        ];

        let score = nunchi.aggregate(&signals);

        assert!(score.score > dec!(0.3));
        assert!(score.is_bullish());
        assert!(score.is_actionable());
        assert!(matches!(
            score.recommendation,
            NunchiRecommendation::Buy | NunchiRecommendation::StrongBuy
        ));
    }

    #[test]
    fn test_aggregate_mixed_signals() {
        let nunchi = NunchiSignals::with_defaults();

        let signals = vec![
            make_signal("momentum", Signal::Buy, dec!(0.8)),
            make_signal("mean_reversion", Signal::Sell, dec!(0.7)),
            make_signal("regime", Signal::Hold, dec!(0.6)),
        ];

        let score = nunchi.aggregate(&signals);

        // Should be close to neutral
        assert!(score.score.abs() < dec!(0.3));
    }

    #[test]
    fn test_low_confidence_filtered() {
        let nunchi = NunchiSignals::with_defaults();

        let signals = vec![
            make_signal("momentum", Signal::StrongBuy, dec!(0.2)), // Below min_confidence
            make_signal("mean_reversion", Signal::Buy, dec!(0.7)),
        ];

        let score = nunchi.aggregate(&signals);

        // Only one signal should contribute
        assert_eq!(score.signal_contributions.len(), 1);
    }

    #[test]
    fn test_should_trade() {
        let nunchi = NunchiSignals::with_defaults();

        let high_score = NunchiScore {
            score: dec!(0.7),
            confidence: dec!(0.85),
            recommendation: NunchiRecommendation::Buy,
            signal_contributions: vec![
                SignalContribution {
                    strategy: "momentum".to_string(),
                    raw_signal: dec!(0.5),
                    weight: dec!(0.25),
                    regime_adjustment: dec!(1.0),
                    weighted_contribution: dec!(0.125),
                    confidence: dec!(0.8),
                },
            ],
            regime: Regime::TrendingUp,
            timestamp: 0,
        };

        let decision = nunchi.should_trade(&high_score, dec!(0.3));
        assert!(decision.should_trade);
        assert!(decision.suggested_size_pct > Decimal::ZERO);
    }

    #[test]
    fn test_correlation_analysis() {
        let nunchi = NunchiSignals::with_defaults();

        let signals = vec![
            make_signal("a", Signal::Buy, dec!(0.8)),
            make_signal("b", Signal::Buy, dec!(0.7)),
            make_signal("c", Signal::StrongBuy, dec!(0.9)),
            make_signal("d", Signal::Sell, dec!(0.6)),
        ];

        let corr = nunchi.analyze_correlation(&signals);

        assert_eq!(corr.bullish_count, 3);
        assert_eq!(corr.bearish_count, 1);
        assert_eq!(corr.dominant, DominantDirection::Bullish);
        assert_eq!(corr.agreement_pct, dec!(75));
    }

    #[test]
    fn test_regime_adjustment() {
        let mut nunchi = NunchiSignals::with_defaults();
        nunchi.set_regime(Regime::TrendingUp);

        let weights = nunchi.get_effective_weights();

        // Momentum should be boosted in trending
        let momentum_weight = weights.get("momentum").copied().unwrap_or_default();
        assert!(momentum_weight > dec!(0.25)); // Base is 0.25, should be 1.5x
    }
}
