//! Pattern Mining
//!
//! Discovers and validates trading patterns from historical data.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::{Regime, Side, Trade, TradeOutcome};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Pattern type classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    PriceAction,
    Volume,
    Momentum,
    MeanReversion,
    Regime,
    TimeOfDay,
    DayOfWeek,
    Composite,
}

/// Pattern state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternState {
    Hypothesis,
    Validated,
    Active,
    Deprecated,
}

/// Discovered pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub pattern_type: PatternType,
    pub conditions: Vec<PatternCondition>,
    pub signal: PatternSignal,
    pub statistics: PatternStats,
    pub state: PatternState,
    pub created_at: i64,
    pub last_seen_at: i64,
}

impl Pattern {
    /// Check if pattern is profitable
    pub fn is_profitable(&self) -> bool {
        self.statistics.profit_factor > dec!(1.2) && self.statistics.win_rate > dec!(0.55)
    }

    /// Check if pattern has enough samples
    pub fn is_validated(&self) -> bool {
        self.statistics.sample_size >= 30 && self.statistics.confidence > dec!(0.7)
    }

    /// Edge in basis points
    pub fn edge_bps(&self) -> Decimal {
        if self.statistics.sample_size == 0 {
            return Decimal::ZERO;
        }
        (self.statistics.avg_win * self.statistics.win_rate
            - self.statistics.avg_loss * (Decimal::ONE - self.statistics.win_rate))
            * dec!(10000)
    }
}

/// Condition for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: Decimal,
    pub weight: Decimal,
}

/// Condition operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    GreaterThan,
    LessThan,
    Equals,
    Between,
    NotEquals,
}

/// Pattern signal output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSignal {
    pub side: Side,
    pub confidence: Decimal,
    pub expected_return_bps: Decimal,
    pub hold_time_secs: u64,
}

/// Pattern statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatternStats {
    pub sample_size: usize,
    pub win_rate: Decimal,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub profit_factor: Decimal,
    pub max_drawdown: Decimal,
    pub sharpe_ratio: Decimal,
    pub confidence: Decimal,
}

/// Pattern database for storing discovered patterns
pub struct PatternDatabase {
    patterns: HashMap<String, Pattern>,
    feature_importance: HashMap<String, Decimal>,
}

impl PatternDatabase {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            feature_importance: HashMap::new(),
        }
    }

    /// Add or update a pattern
    pub fn upsert(&mut self, pattern: Pattern) {
        self.patterns.insert(pattern.id.clone(), pattern);
    }

    /// Get a pattern by ID
    pub fn get(&self, id: &str) -> Option<&Pattern> {
        self.patterns.get(id)
    }

    /// Get all active patterns
    pub fn active_patterns(&self) -> Vec<&Pattern> {
        self.patterns
            .values()
            .filter(|p| p.state == PatternState::Active)
            .collect()
    }

    /// Get profitable patterns
    pub fn profitable_patterns(&self) -> Vec<&Pattern> {
        self.patterns.values().filter(|p| p.is_profitable()).collect()
    }

    /// Deprecate underperforming patterns
    pub fn deprecate_underperforming(&mut self, min_win_rate: Decimal, min_profit_factor: Decimal) {
        for pattern in self.patterns.values_mut() {
            if pattern.state == PatternState::Active
                && pattern.statistics.sample_size >= 50
                && (pattern.statistics.win_rate < min_win_rate
                    || pattern.statistics.profit_factor < min_profit_factor)
            {
                pattern.state = PatternState::Deprecated;
                info!("Deprecated pattern: {} (WR: {}, PF: {})",
                    pattern.name, pattern.statistics.win_rate, pattern.statistics.profit_factor);
            }
        }
    }

    /// Update feature importance
    pub fn update_feature_importance(&mut self, feature: &str, importance: Decimal) {
        self.feature_importance.insert(feature.to_string(), importance);
    }

    /// Get top features by importance
    pub fn top_features(&self, n: usize) -> Vec<(&String, &Decimal)> {
        let mut features: Vec<_> = self.feature_importance.iter().collect();
        features.sort_by(|a, b| b.1.cmp(a.1));
        features.into_iter().take(n).collect()
    }
}

impl Default for PatternDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern miner for discovering patterns from trades
pub struct PatternMiner {
    min_sample_size: usize,
    min_confidence: Decimal,
    discovered: Vec<Pattern>,
}

impl PatternMiner {
    pub fn new(min_sample_size: usize, min_confidence: Decimal) -> Self {
        Self {
            min_sample_size,
            min_confidence,
            discovered: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(30, dec!(0.6))
    }

    /// Mine patterns from trade history
    pub fn mine_patterns(&mut self, trades: &[Trade]) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Time-of-day patterns
        patterns.extend(self.mine_time_patterns(trades));

        // Price action patterns
        patterns.extend(self.mine_price_patterns(trades));

        // Filter by confidence
        patterns.retain(|p| p.statistics.confidence >= self.min_confidence);

        self.discovered.extend(patterns.clone());
        patterns
    }

    /// Mine time-based patterns
    fn mine_time_patterns(&self, trades: &[Trade]) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Group by hour of day
        let mut by_hour: HashMap<u32, Vec<&Trade>> = HashMap::new();
        for trade in trades {
            let hour = trade.opened_at.format("%H").to_string().parse().unwrap_or(0);
            by_hour.entry(hour).or_insert_with(Vec::new).push(trade);
        }

        // Check each hour for statistical edge
        for (hour, hour_trades) in by_hour {
            if hour_trades.len() < self.min_sample_size {
                continue;
            }

            let stats = self.calculate_stats(&hour_trades);

            if stats.win_rate > dec!(0.6) && stats.profit_factor > dec!(1.5) {
                patterns.push(Pattern {
                    id: format!("tod_hour_{}", hour),
                    name: format!("Hour {} Edge", hour),
                    pattern_type: PatternType::TimeOfDay,
                    conditions: vec![PatternCondition {
                        field: "hour".to_string(),
                        operator: ConditionOperator::Equals,
                        value: Decimal::from(hour),
                        weight: Decimal::ONE,
                    }],
                    signal: PatternSignal {
                        side: if stats.avg_win > Decimal::ZERO {
                            Side::Long
                        } else {
                            Side::Short
                        },
                        confidence: stats.confidence,
                        expected_return_bps: stats.avg_win * dec!(10000),
                        hold_time_secs: 3600,
                    },
                    statistics: stats,
                    state: PatternState::Hypothesis,
                    created_at: chrono::Utc::now().timestamp(),
                    last_seen_at: chrono::Utc::now().timestamp(),
                });
            }
        }

        patterns
    }

    /// Mine price action patterns
    fn mine_price_patterns(&self, trades: &[Trade]) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Group by price change buckets
        let mut by_move: HashMap<i32, Vec<&Trade>> = HashMap::new();
        for trade in trades {
            let pct_move = ((trade.exit_price - trade.entry_price) / trade.entry_price * dec!(100))
                .round()
                .to_string()
                .parse()
                .unwrap_or(0);
            let bucket = pct_move / 10 * 10; // Round to nearest 10%
            by_move.entry(bucket).or_insert_with(Vec::new).push(trade);
        }

        for (bucket, bucket_trades) in by_move {
            if bucket_trades.len() < self.min_sample_size {
                continue;
            }

            let stats = self.calculate_stats(&bucket_trades);

            if stats.profit_factor > dec!(1.3) {
                patterns.push(Pattern {
                    id: format!("price_move_{}", bucket),
                    name: format!("{}% Move Pattern", bucket),
                    pattern_type: PatternType::PriceAction,
                    conditions: vec![PatternCondition {
                        field: "price_change_pct".to_string(),
                        operator: ConditionOperator::Between,
                        value: Decimal::from(bucket),
                        weight: Decimal::ONE,
                    }],
                    signal: PatternSignal {
                        side: Side::Long,
                        confidence: stats.confidence,
                        expected_return_bps: stats.avg_win * dec!(10000),
                        hold_time_secs: 300,
                    },
                    statistics: stats,
                    state: PatternState::Hypothesis,
                    created_at: chrono::Utc::now().timestamp(),
                    last_seen_at: chrono::Utc::now().timestamp(),
                });
            }
        }

        patterns
    }

    /// Calculate statistics from trades
    fn calculate_stats(&self, trades: &[&Trade]) -> PatternStats {
        if trades.is_empty() {
            return PatternStats::default();
        }

        let wins: Vec<_> = trades.iter().filter(|t| t.outcome == TradeOutcome::Win).collect();
        let losses: Vec<_> = trades.iter().filter(|t| t.outcome == TradeOutcome::Loss).collect();

        let win_rate = Decimal::from(wins.len()) / Decimal::from(trades.len());

        let avg_win = if wins.is_empty() {
            Decimal::ZERO
        } else {
            wins.iter().map(|t| t.pnl).sum::<Decimal>() / Decimal::from(wins.len())
        };

        let avg_loss = if losses.is_empty() {
            Decimal::ZERO
        } else {
            losses.iter().map(|t| t.pnl.abs()).sum::<Decimal>() / Decimal::from(losses.len())
        };

        let profit_factor = if avg_loss.is_zero() {
            Decimal::from(10)
        } else {
            (avg_win * win_rate) / (avg_loss * (Decimal::ONE - win_rate))
        };

        // Calculate confidence based on sample size and consistency
        let n = trades.len();
        let z_score = dec!(1.96); // 95% confidence
        let confidence = if n > 0 {
            let p = win_rate;
            let se = ((p * (Decimal::ONE - p)) / Decimal::from(n)).sqrt();
            if se.is_zero() {
                Decimal::ONE
            } else {
                (Decimal::ONE - se * z_score / p).max(Decimal::ZERO).min(Decimal::ONE)
            }
        } else {
            Decimal::ZERO
        };

        PatternStats {
            sample_size: trades.len(),
            win_rate,
            avg_win,
            avg_loss,
            profit_factor,
            max_drawdown: Decimal::ZERO, // Would need equity curve
            sharpe_ratio: Decimal::ZERO, // Would need returns series
            confidence,
        }
    }

    /// Get all discovered patterns
    pub fn discovered(&self) -> &[Pattern] {
        &self.discovered
    }

    /// Clear discovered patterns
    pub fn clear(&mut self) {
        self.discovered.clear();
    }
}

// Helper for sqrt approximation
trait DecimalSqrt {
    fn sqrt(&self) -> Decimal;
}

impl DecimalSqrt for Decimal {
    fn sqrt(&self) -> Decimal {
        if self.is_zero() || self.is_sign_negative() {
            return Decimal::ZERO;
        }
        let mut x = *self;
        for _ in 0..15 {
            x = (x + *self / x) / dec!(2);
        }
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_trade(pnl: Decimal, outcome: TradeOutcome, hour: u32) -> Trade {
        let now = Utc::now();
        Trade {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: "BTC-USDT".to_string(),
            side: Side::Long,
            size: dec!(0.1),
            entry_price: dec!(67000),
            exit_price: dec!(67000) + pnl * dec!(100),
            pnl,
            outcome,
            reason: "test".to_string(),
            opened_at: now,
            closed_at: now,
        }
    }

    #[test]
    fn test_pattern_is_profitable() {
        let pattern = Pattern {
            id: "test".to_string(),
            name: "Test".to_string(),
            pattern_type: PatternType::TimeOfDay,
            conditions: vec![],
            signal: PatternSignal {
                side: Side::Long,
                confidence: dec!(0.8),
                expected_return_bps: dec!(50),
                hold_time_secs: 300,
            },
            statistics: PatternStats {
                sample_size: 100,
                win_rate: dec!(0.65),
                avg_win: dec!(50),
                avg_loss: dec!(30),
                profit_factor: dec!(1.8),
                max_drawdown: dec!(0.05),
                sharpe_ratio: dec!(1.5),
                confidence: dec!(0.8),
            },
            state: PatternState::Active,
            created_at: 0,
            last_seen_at: 0,
        };

        assert!(pattern.is_profitable());
        assert!(pattern.is_validated());
    }

    #[test]
    fn test_calculate_stats() {
        let miner = PatternMiner::with_defaults();

        let trades: Vec<Trade> = (0..50)
            .map(|i| {
                if i % 3 == 0 {
                    make_trade(dec!(-10), TradeOutcome::Loss, 10)
                } else {
                    make_trade(dec!(20), TradeOutcome::Win, 10)
                }
            })
            .collect();

        let trade_refs: Vec<&Trade> = trades.iter().collect();
        let stats = miner.calculate_stats(&trade_refs);

        // 33 wins, 17 losses
        assert!(stats.win_rate > dec!(0.6));
        assert!(stats.profit_factor > dec!(1.5));
    }

    #[test]
    fn test_pattern_database() {
        let mut db = PatternDatabase::new();

        let pattern = Pattern {
            id: "p1".to_string(),
            name: "Test Pattern".to_string(),
            pattern_type: PatternType::TimeOfDay,
            conditions: vec![],
            signal: PatternSignal {
                side: Side::Long,
                confidence: dec!(0.8),
                expected_return_bps: dec!(50),
                hold_time_secs: 300,
            },
            statistics: PatternStats {
                sample_size: 100,
                win_rate: dec!(0.65),
                profit_factor: dec!(1.8),
                ..Default::default()
            },
            state: PatternState::Active,
            created_at: 0,
            last_seen_at: 0,
        };

        db.upsert(pattern);

        assert_eq!(db.active_patterns().len(), 1);
        assert!(db.get("p1").is_some());
    }
}
