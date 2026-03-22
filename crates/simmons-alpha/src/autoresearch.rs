//! Autoresearch Agent
//!
//! Autonomous alpha discovery through pattern mining, hypothesis generation,
//! and statistical validation.

use crate::patterns::{Pattern, PatternDatabase, PatternMiner, PatternState, PatternStats, PatternType};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::{MarketState, Regime, Trade};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Autoresearch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoresearchConfig {
    /// Minimum sample size for validation
    pub min_sample_size: usize,
    /// Minimum win rate to consider
    pub min_win_rate: Decimal,
    /// Minimum profit factor
    pub min_profit_factor: Decimal,
    /// Maximum active patterns
    pub max_active_patterns: usize,
    /// Hypothesis generation enabled
    pub generate_hypotheses: bool,
    /// Pattern decay half-life (days)
    pub pattern_decay_days: u32,
}

impl Default for AutoresearchConfig {
    fn default() -> Self {
        Self {
            min_sample_size: 30,
            min_win_rate: dec!(0.55),
            min_profit_factor: dec!(1.3),
            max_active_patterns: 20,
            generate_hypotheses: true,
            pattern_decay_days: 30,
        }
    }
}

/// Research hypothesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub description: String,
    pub conditions: HypothesisConditions,
    pub expected_edge_bps: Decimal,
    pub confidence: Decimal,
    pub status: HypothesisStatus,
    pub created_at: i64,
    pub tested_trades: usize,
    pub validation_result: Option<ValidationResult>,
}

/// Hypothesis conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisConditions {
    pub regime: Option<Regime>,
    pub volatility_range: Option<(Decimal, Decimal)>,
    pub time_of_day: Option<(u8, u8)>,
    pub price_action: Option<String>,
    pub custom: HashMap<String, String>,
}

/// Hypothesis status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisStatus {
    Pending,
    Testing,
    Validated,
    Rejected,
    Expired,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub win_rate: Decimal,
    pub profit_factor: Decimal,
    pub sample_size: usize,
    pub p_value: Decimal,
    pub reason: String,
}

/// Alpha score for an opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaScore {
    pub score: Decimal,
    pub contributing_patterns: Vec<String>,
    pub confidence: Decimal,
    pub expected_return_bps: Decimal,
    pub risk_adjusted_score: Decimal,
}

/// Autoresearch agent
pub struct AutoresearchAgent {
    config: AutoresearchConfig,
    pattern_db: PatternDatabase,
    pattern_miner: PatternMiner,
    hypotheses: Vec<Hypothesis>,
    alpha_history: Vec<AlphaScoreHistory>,
}

/// Alpha score historical entry
#[derive(Debug, Clone)]
struct AlphaScoreHistory {
    score: AlphaScore,
    timestamp: i64,
    actual_return: Option<Decimal>,
}

impl AutoresearchAgent {
    pub fn new(config: AutoresearchConfig) -> Self {
        Self {
            pattern_miner: PatternMiner::new(config.min_sample_size, dec!(0.6)),
            config,
            pattern_db: PatternDatabase::new(),
            hypotheses: Vec::new(),
            alpha_history: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(AutoresearchConfig::default())
    }

    /// Discover patterns from historical trades
    pub fn discover_patterns(&mut self, history: &[Trade]) -> Vec<Pattern> {
        info!("Mining patterns from {} trades", history.len());

        let patterns = self.pattern_miner.mine_patterns(history);

        // Validate and add to database
        let mut validated = Vec::new();
        for pattern in patterns {
            if self.validate_pattern(&pattern) {
                let mut p = pattern.clone();
                p.state = PatternState::Validated;
                self.pattern_db.upsert(p.clone());
                validated.push(p);
            }
        }

        info!("Discovered {} validated patterns", validated.len());
        validated
    }

    /// Validate a pattern meets minimum requirements
    fn validate_pattern(&self, pattern: &Pattern) -> bool {
        let stats = &pattern.statistics;

        stats.sample_size >= self.config.min_sample_size
            && stats.win_rate >= self.config.min_win_rate
            && stats.profit_factor >= self.config.min_profit_factor
    }

    /// Score potential alpha from current market state
    pub fn score_alpha(&self, market_state: &MarketState) -> AlphaScore {
        let active_patterns = self.pattern_db.active_patterns();
        let pattern_count = active_patterns.len();

        if active_patterns.is_empty() {
            return AlphaScore {
                score: Decimal::ZERO,
                contributing_patterns: vec![],
                confidence: Decimal::ZERO,
                expected_return_bps: Decimal::ZERO,
                risk_adjusted_score: Decimal::ZERO,
            };
        }

        let mut total_score = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;
        let mut contributing = Vec::new();
        let mut expected_return = Decimal::ZERO;

        for pattern in active_patterns {
            if self.pattern_matches(pattern, market_state) {
                let weight = pattern.statistics.confidence * pattern.statistics.profit_factor;
                total_score += pattern.edge_bps() * weight;
                total_weight += weight;
                expected_return += pattern.signal.expected_return_bps * weight;
                contributing.push(pattern.name.clone());
            }
        }

        let score = if total_weight.is_zero() {
            Decimal::ZERO
        } else {
            total_score / total_weight
        };

        let confidence = if contributing.is_empty() {
            Decimal::ZERO
        } else {
            (Decimal::from(contributing.len()) / Decimal::from(pattern_count))
                .min(Decimal::ONE)
        };

        let expected = if total_weight.is_zero() {
            Decimal::ZERO
        } else {
            expected_return / total_weight
        };

        // Risk-adjusted: score / volatility (simplified)
        let risk_adjusted = score * confidence / market_state.volatility_1h.max(dec!(0.01));

        AlphaScore {
            score,
            contributing_patterns: contributing,
            confidence,
            expected_return_bps: expected,
            risk_adjusted_score: risk_adjusted,
        }
    }

    /// Check if a pattern matches current market state
    fn pattern_matches(&self, pattern: &Pattern, state: &MarketState) -> bool {
        for condition in &pattern.conditions {
            let value = self.get_state_value(state, &condition.field);

            let matches = match condition.operator {
                crate::patterns::ConditionOperator::GreaterThan => value > condition.value,
                crate::patterns::ConditionOperator::LessThan => value < condition.value,
                crate::patterns::ConditionOperator::Equals => value == condition.value,
                crate::patterns::ConditionOperator::NotEquals => value != condition.value,
                crate::patterns::ConditionOperator::Between => {
                    // Use value as center, ±10%
                    let low = condition.value * dec!(0.9);
                    let high = condition.value * dec!(1.1);
                    value >= low && value <= high
                }
            };

            if !matches {
                return false;
            }
        }
        true
    }

    /// Get value from market state by field name
    fn get_state_value(&self, state: &MarketState, field: &str) -> Decimal {
        match field {
            "price" => state.price,
            "spread_bps" => state.spread_bps,
            "volatility_1h" => state.volatility_1h,
            "hour" => {
                let hour = chrono::Utc::now().format("%H").to_string();
                Decimal::from(hour.parse::<u32>().unwrap_or(0))
            }
            _ => Decimal::ZERO,
        }
    }

    /// Generate trading hypotheses based on patterns and market state
    pub fn generate_hypotheses(&mut self, market_state: &MarketState) -> Vec<Hypothesis> {
        if !self.config.generate_hypotheses {
            return vec![];
        }

        let mut hypotheses = Vec::new();

        // Regime-based hypothesis
        let regime_hypo = Hypothesis {
            id: format!("regime_{}_{}", regime_to_str(&market_state.regime), chrono::Utc::now().timestamp()),
            description: format!(
                "Market in {} regime with {} volatility - expect continuation",
                regime_to_str(&market_state.regime),
                if market_state.volatility_1h > dec!(0.02) { "high" } else { "low" }
            ),
            conditions: HypothesisConditions {
                regime: Some(market_state.regime),
                volatility_range: Some((
                    market_state.volatility_1h * dec!(0.8),
                    market_state.volatility_1h * dec!(1.2),
                )),
                time_of_day: None,
                price_action: None,
                custom: HashMap::new(),
            },
            expected_edge_bps: dec!(20),
            confidence: dec!(0.5),
            status: HypothesisStatus::Pending,
            created_at: chrono::Utc::now().timestamp(),
            tested_trades: 0,
            validation_result: None,
        };
        hypotheses.push(regime_hypo);

        // Spread-based hypothesis
        if market_state.spread_bps > dec!(30) {
            let spread_hypo = Hypothesis {
                id: format!("wide_spread_{}", chrono::Utc::now().timestamp()),
                description: "Wide spread detected - potential mean reversion opportunity".to_string(),
                conditions: HypothesisConditions {
                    regime: None,
                    volatility_range: None,
                    time_of_day: None,
                    price_action: Some("wide_spread".to_string()),
                    custom: {
                        let mut m = HashMap::new();
                        m.insert("min_spread_bps".to_string(), market_state.spread_bps.to_string());
                        m
                    },
                },
                expected_edge_bps: dec!(15),
                confidence: dec!(0.4),
                status: HypothesisStatus::Pending,
                created_at: chrono::Utc::now().timestamp(),
                tested_trades: 0,
                validation_result: None,
            };
            hypotheses.push(spread_hypo);
        }

        // Store and return
        self.hypotheses.extend(hypotheses.clone());
        hypotheses
    }

    /// Test a hypothesis against a trade
    pub fn test_hypothesis(&mut self, hypothesis_id: &str, trade: &Trade) {
        if let Some(hypo) = self.hypotheses.iter_mut().find(|h| h.id == hypothesis_id) {
            hypo.tested_trades += 1;
            hypo.status = HypothesisStatus::Testing;

            // After enough trades, validate
            if hypo.tested_trades >= self.config.min_sample_size {
                // This is simplified - real implementation would track all trades
                let result = ValidationResult {
                    passed: trade.pnl > Decimal::ZERO,
                    win_rate: dec!(0.5), // Would calculate from all trades
                    profit_factor: dec!(1.0),
                    sample_size: hypo.tested_trades,
                    p_value: dec!(0.05),
                    reason: "Reached minimum sample size".to_string(),
                };

                hypo.validation_result = Some(result.clone());
                hypo.status = if result.passed {
                    HypothesisStatus::Validated
                } else {
                    HypothesisStatus::Rejected
                };
            }
        }
    }

    /// Record alpha score with actual outcome
    pub fn record_outcome(&mut self, alpha_score: AlphaScore, actual_return: Decimal) {
        self.alpha_history.push(AlphaScoreHistory {
            score: alpha_score,
            timestamp: chrono::Utc::now().timestamp(),
            actual_return: Some(actual_return),
        });

        // Keep only last 1000 entries
        if self.alpha_history.len() > 1000 {
            self.alpha_history.remove(0);
        }
    }

    /// Analyze prediction accuracy
    pub fn analyze_accuracy(&self) -> AccuracyReport {
        let with_outcomes: Vec<_> = self
            .alpha_history
            .iter()
            .filter(|h| h.actual_return.is_some())
            .collect();

        if with_outcomes.is_empty() {
            return AccuracyReport::default();
        }

        let mut correct_direction = 0;
        let mut total_predicted = Decimal::ZERO;
        let mut total_actual = Decimal::ZERO;

        for entry in &with_outcomes {
            let predicted_positive = entry.score.score > Decimal::ZERO;
            let actual_positive = entry.actual_return.unwrap() > Decimal::ZERO;

            if predicted_positive == actual_positive {
                correct_direction += 1;
            }

            total_predicted += entry.score.expected_return_bps;
            total_actual += entry.actual_return.unwrap();
        }

        let direction_accuracy = Decimal::from(correct_direction) / Decimal::from(with_outcomes.len());
        let prediction_error = (total_predicted - total_actual).abs() / total_predicted.abs().max(Decimal::ONE);

        AccuracyReport {
            sample_size: with_outcomes.len(),
            direction_accuracy,
            average_predicted_return: total_predicted / Decimal::from(with_outcomes.len()),
            average_actual_return: total_actual / Decimal::from(with_outcomes.len()),
            prediction_error,
        }
    }

    /// Get pattern database reference
    pub fn patterns(&self) -> &PatternDatabase {
        &self.pattern_db
    }

    /// Get pending hypotheses
    pub fn pending_hypotheses(&self) -> Vec<&Hypothesis> {
        self.hypotheses
            .iter()
            .filter(|h| h.status == HypothesisStatus::Pending || h.status == HypothesisStatus::Testing)
            .collect()
    }

    /// Get validated hypotheses
    pub fn validated_hypotheses(&self) -> Vec<&Hypothesis> {
        self.hypotheses
            .iter()
            .filter(|h| h.status == HypothesisStatus::Validated)
            .collect()
    }

    /// Promote validated hypotheses to patterns
    pub fn promote_hypotheses(&mut self) -> Vec<Pattern> {
        let mut promoted = Vec::new();

        let validated: Vec<_> = self
            .hypotheses
            .iter()
            .filter(|h| h.status == HypothesisStatus::Validated)
            .cloned()
            .collect();

        for hypo in validated {
            if let Some(result) = &hypo.validation_result {
                let pattern = Pattern {
                    id: format!("from_hypo_{}", hypo.id),
                    name: hypo.description.clone(),
                    pattern_type: PatternType::Composite,
                    conditions: vec![], // Would convert from hypothesis conditions
                    signal: crate::patterns::PatternSignal {
                        side: simmons_core::Side::Long,
                        confidence: hypo.confidence,
                        expected_return_bps: hypo.expected_edge_bps,
                        hold_time_secs: 300,
                    },
                    statistics: PatternStats {
                        sample_size: result.sample_size,
                        win_rate: result.win_rate,
                        profit_factor: result.profit_factor,
                        confidence: Decimal::ONE - result.p_value,
                        ..Default::default()
                    },
                    state: PatternState::Active,
                    created_at: chrono::Utc::now().timestamp(),
                    last_seen_at: chrono::Utc::now().timestamp(),
                };

                self.pattern_db.upsert(pattern.clone());
                promoted.push(pattern);
            }
        }

        // Remove promoted hypotheses
        self.hypotheses.retain(|h| h.status != HypothesisStatus::Validated);

        promoted
    }
}

/// Accuracy report
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccuracyReport {
    pub sample_size: usize,
    pub direction_accuracy: Decimal,
    pub average_predicted_return: Decimal,
    pub average_actual_return: Decimal,
    pub prediction_error: Decimal,
}

/// Helper to convert regime to string
fn regime_to_str(regime: &Regime) -> &'static str {
    match regime {
        Regime::TrendingUp => "trending_up",
        Regime::TrendingDown => "trending_down",
        Regime::MeanReverting => "mean_reverting",
        Regime::HighVolatility => "high_vol",
        Regime::LowVolatility => "low_vol",
        Regime::Choppy => "choppy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simmons_core::TradeOutcome;

    fn make_market_state() -> MarketState {
        MarketState {
            symbol: "BTC-USDT".to_string(),
            price: dec!(67000),
            spread_bps: dec!(15),
            volatility_1h: dec!(0.025),
            regime: Regime::TrendingUp,
            cex_price: Some(dec!(67000)),
            dex_price: Some(dec!(67050)),
        }
    }

    #[test]
    fn test_score_alpha_no_patterns() {
        let agent = AutoresearchAgent::with_defaults();
        let state = make_market_state();

        let score = agent.score_alpha(&state);
        assert_eq!(score.score, Decimal::ZERO);
        assert!(score.contributing_patterns.is_empty());
    }

    #[test]
    fn test_generate_hypotheses() {
        let mut agent = AutoresearchAgent::with_defaults();
        let state = make_market_state();

        let hypotheses = agent.generate_hypotheses(&state);
        assert!(!hypotheses.is_empty());
        assert_eq!(hypotheses[0].status, HypothesisStatus::Pending);
    }

    #[test]
    fn test_accuracy_report_empty() {
        let agent = AutoresearchAgent::with_defaults();
        let report = agent.analyze_accuracy();
        assert_eq!(report.sample_size, 0);
    }
}
