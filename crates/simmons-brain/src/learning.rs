//! Learning Engine
//!
//! Continuous learning from trade outcomes to improve strategies.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::{Trade, TradeOutcome};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Learning engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Minimum trades to analyze pattern
    pub min_pattern_trades: usize,
    /// Weight decay for older trades
    pub weight_decay_per_day: Decimal,
    /// Maximum history to keep
    pub max_history_size: usize,
    /// Significance threshold for patterns
    pub significance_threshold: Decimal,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            min_pattern_trades: 10,
            weight_decay_per_day: dec!(0.95),
            max_history_size: 1000,
            significance_threshold: dec!(0.05),
        }
    }
}

/// Learning engine for continuous improvement
pub struct LearningEngine {
    config: LearningConfig,
    trade_history: Vec<TradeRecord>,
    pattern_effectiveness: HashMap<String, PatternStats>,
    strategy_performance: HashMap<String, StrategyStats>,
    daily_performance: Vec<DailyPerformance>,
}

/// Extended trade record for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub trade: Trade,
    pub reasoning: String,
    pub signals_used: Vec<String>,
    pub pattern_ids: Vec<String>,
    pub market_conditions: MarketConditions,
    pub timestamp: i64,
}

/// Market conditions at time of trade
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketConditions {
    pub regime: String,
    pub volatility: Decimal,
    pub spread_bps: Decimal,
    pub volume_relative: Decimal,
    pub time_of_day: u8,
    pub day_of_week: u8,
}

/// Pattern effectiveness statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatternStats {
    pub pattern_id: String,
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub total_pnl: Decimal,
    pub avg_pnl: Decimal,
    pub win_rate: Decimal,
    pub profit_factor: Decimal,
    pub max_consecutive_losses: usize,
    pub recent_performance: Decimal,
    pub is_effective: bool,
}

/// Strategy performance statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyStats {
    pub strategy_name: String,
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub total_pnl: Decimal,
    pub avg_pnl: Decimal,
    pub win_rate: Decimal,
    pub sharpe_ratio: Decimal,
    pub max_drawdown: Decimal,
    pub current_streak: i32,
    pub best_conditions: Vec<String>,
    pub worst_conditions: Vec<String>,
}

/// Daily performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPerformance {
    pub date: String,
    pub trades: usize,
    pub wins: usize,
    pub pnl: Decimal,
    pub win_rate: Decimal,
    pub best_strategy: Option<String>,
    pub worst_strategy: Option<String>,
}

/// Learning insights for feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsights {
    pub period_trades: usize,
    pub period_win_rate: Decimal,
    pub period_pnl: Decimal,
    pub improving_patterns: Vec<String>,
    pub declining_patterns: Vec<String>,
    pub best_strategies: Vec<StrategyRanking>,
    pub worst_strategies: Vec<StrategyRanking>,
    pub recommendations: Vec<String>,
    pub weight_adjustments: HashMap<String, Decimal>,
}

/// Strategy ranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRanking {
    pub name: String,
    pub score: Decimal,
    pub win_rate: Decimal,
    pub pnl: Decimal,
}

impl LearningEngine {
    pub fn new(config: LearningConfig) -> Self {
        Self {
            config,
            trade_history: Vec::new(),
            pattern_effectiveness: HashMap::new(),
            strategy_performance: HashMap::new(),
            daily_performance: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(LearningConfig::default())
    }

    /// Record a trade outcome
    pub fn record_outcome(&mut self, trade: Trade, reasoning: &str, signals: Vec<String>, patterns: Vec<String>, conditions: MarketConditions) {
        let record = TradeRecord {
            trade: trade.clone(),
            reasoning: reasoning.to_string(),
            signals_used: signals.clone(),
            pattern_ids: patterns.clone(),
            market_conditions: conditions,
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.trade_history.push(record);

        // Trim history if too large
        if self.trade_history.len() > self.config.max_history_size {
            self.trade_history.remove(0);
        }

        // Update pattern stats
        for pattern_id in &patterns {
            self.update_pattern_stats(pattern_id, &trade);
        }

        // Update strategy stats
        for signal in &signals {
            self.update_strategy_stats(signal, &trade);
        }

        // Update daily performance
        self.update_daily_performance(&trade);

        info!(
            "Recorded trade outcome: {} PnL: {} ({:?})",
            trade.symbol, trade.pnl, trade.outcome
        );
    }

    /// Update pattern statistics
    fn update_pattern_stats(&mut self, pattern_id: &str, trade: &Trade) {
        let stats = self
            .pattern_effectiveness
            .entry(pattern_id.to_string())
            .or_insert_with(|| PatternStats {
                pattern_id: pattern_id.to_string(),
                ..Default::default()
            });

        stats.total_trades += 1;

        match trade.outcome {
            TradeOutcome::Win => {
                stats.wins += 1;
            }
            TradeOutcome::Loss => {
                stats.losses += 1;
            }
            TradeOutcome::Breakeven => {}
        }

        stats.total_pnl += trade.pnl;
        stats.avg_pnl = stats.total_pnl / Decimal::from(stats.total_trades);
        stats.win_rate = Decimal::from(stats.wins) / Decimal::from(stats.total_trades);

        // Calculate profit factor
        let total_wins: Decimal = self
            .trade_history
            .iter()
            .filter(|r| r.pattern_ids.contains(&pattern_id.to_string()) && r.trade.pnl > Decimal::ZERO)
            .map(|r| r.trade.pnl)
            .sum();

        let total_losses: Decimal = self
            .trade_history
            .iter()
            .filter(|r| r.pattern_ids.contains(&pattern_id.to_string()) && r.trade.pnl < Decimal::ZERO)
            .map(|r| r.trade.pnl.abs())
            .sum();

        stats.profit_factor = if total_losses.is_zero() {
            Decimal::from(10)
        } else {
            total_wins / total_losses
        };

        // Determine effectiveness
        stats.is_effective = stats.total_trades >= self.config.min_pattern_trades
            && stats.win_rate >= dec!(0.55)
            && stats.profit_factor >= dec!(1.3);
    }

    /// Update strategy statistics
    fn update_strategy_stats(&mut self, strategy: &str, trade: &Trade) {
        let stats = self
            .strategy_performance
            .entry(strategy.to_string())
            .or_insert_with(|| StrategyStats {
                strategy_name: strategy.to_string(),
                ..Default::default()
            });

        stats.total_trades += 1;

        match trade.outcome {
            TradeOutcome::Win => {
                stats.wins += 1;
                if stats.current_streak >= 0 {
                    stats.current_streak += 1;
                } else {
                    stats.current_streak = 1;
                }
            }
            TradeOutcome::Loss => {
                stats.losses += 1;
                if stats.current_streak <= 0 {
                    stats.current_streak -= 1;
                } else {
                    stats.current_streak = -1;
                }
            }
            TradeOutcome::Breakeven => {}
        }

        stats.total_pnl += trade.pnl;
        stats.avg_pnl = stats.total_pnl / Decimal::from(stats.total_trades);
        stats.win_rate = Decimal::from(stats.wins) / Decimal::from(stats.total_trades);
    }

    /// Update daily performance
    fn update_daily_performance(&mut self, trade: &Trade) {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        if let Some(daily) = self.daily_performance.iter_mut().find(|d| d.date == date) {
            daily.trades += 1;
            daily.pnl += trade.pnl;
            if trade.outcome == TradeOutcome::Win {
                daily.wins += 1;
            }
            daily.win_rate = Decimal::from(daily.wins) / Decimal::from(daily.trades);
        } else {
            let wins = if trade.outcome == TradeOutcome::Win { 1 } else { 0 };
            self.daily_performance.push(DailyPerformance {
                date,
                trades: 1,
                wins,
                pnl: trade.pnl,
                win_rate: Decimal::from(wins),
                best_strategy: None,
                worst_strategy: None,
            });
        }
    }

    /// Analyze patterns and generate insights
    pub fn analyze_patterns(&self) -> PatternAnalysis {
        let mut effective: Vec<_> = self
            .pattern_effectiveness
            .values()
            .filter(|p| p.is_effective)
            .cloned()
            .collect();

        effective.sort_by(|a, b| b.profit_factor.cmp(&a.profit_factor));

        let mut ineffective: Vec<_> = self
            .pattern_effectiveness
            .values()
            .filter(|p| !p.is_effective && p.total_trades >= self.config.min_pattern_trades)
            .cloned()
            .collect();

        ineffective.sort_by(|a, b| a.profit_factor.cmp(&b.profit_factor));

        PatternAnalysis {
            total_patterns: self.pattern_effectiveness.len(),
            effective_patterns: effective.len(),
            ineffective_patterns: ineffective.len(),
            top_patterns: effective.into_iter().take(5).collect(),
            worst_patterns: ineffective.into_iter().take(5).collect(),
        }
    }

    /// Generate learning insights for Claude
    pub fn generate_insights(&self) -> LearningInsights {
        // Recent period analysis (last 50 trades)
        let recent: Vec<_> = self.trade_history.iter().rev().take(50).collect();

        let period_trades = recent.len();
        let period_wins = recent.iter().filter(|r| r.trade.outcome == TradeOutcome::Win).count();
        let period_win_rate = if period_trades > 0 {
            Decimal::from(period_wins) / Decimal::from(period_trades)
        } else {
            Decimal::ZERO
        };
        let period_pnl: Decimal = recent.iter().map(|r| r.trade.pnl).sum();

        // Find improving and declining patterns
        let mut improving = Vec::new();
        let mut declining = Vec::new();

        for (id, stats) in &self.pattern_effectiveness {
            if stats.total_trades >= self.config.min_pattern_trades {
                if stats.recent_performance > Decimal::ZERO && stats.is_effective {
                    improving.push(id.clone());
                } else if stats.recent_performance < Decimal::ZERO {
                    declining.push(id.clone());
                }
            }
        }

        // Strategy rankings
        let mut strategy_rankings: Vec<_> = self
            .strategy_performance
            .values()
            .map(|s| StrategyRanking {
                name: s.strategy_name.clone(),
                score: s.win_rate * s.profit_factor(),
                win_rate: s.win_rate,
                pnl: s.total_pnl,
            })
            .collect();

        strategy_rankings.sort_by(|a, b| b.score.cmp(&a.score));

        let best = strategy_rankings.iter().take(3).cloned().collect();
        let worst = strategy_rankings.iter().rev().take(3).cloned().collect();

        // Generate recommendations
        let mut recommendations = Vec::new();

        if period_win_rate < dec!(0.5) {
            recommendations.push("Win rate below 50% - consider reducing position sizes".to_string());
        }

        if !declining.is_empty() {
            recommendations.push(format!("Declining patterns: {} - review or disable", declining.join(", ")));
        }

        if improving.len() > 3 {
            recommendations.push("Multiple patterns improving - consider increasing allocation".to_string());
        }

        // Weight adjustments
        let weight_adjustments = self.calculate_weight_adjustments();

        LearningInsights {
            period_trades,
            period_win_rate,
            period_pnl,
            improving_patterns: improving,
            declining_patterns: declining,
            best_strategies: best,
            worst_strategies: worst,
            recommendations,
            weight_adjustments,
        }
    }

    /// Adjust strategy weights based on recent performance
    pub fn adjust_weights(&self, current: &HashMap<String, Decimal>) -> HashMap<String, Decimal> {
        let mut adjusted = current.clone();
        let adjustments = self.calculate_weight_adjustments();

        for (strategy, adjustment) in adjustments {
            if let Some(weight) = adjusted.get_mut(&strategy) {
                *weight = (*weight * adjustment).max(dec!(0.05)).min(dec!(0.40));
            }
        }

        adjusted
    }

    /// Calculate weight adjustments
    fn calculate_weight_adjustments(&self) -> HashMap<String, Decimal> {
        let mut adjustments = HashMap::new();

        for (strategy, stats) in &self.strategy_performance {
            if stats.total_trades < self.config.min_pattern_trades {
                continue;
            }

            let pf = stats.profit_factor();
            let wr = stats.win_rate;

            // Adjustment based on profit factor and win rate
            let adjustment = if pf > dec!(1.5) && wr > dec!(0.6) {
                dec!(1.2) // Increase weight
            } else if pf < dec!(1.0) || wr < dec!(0.45) {
                dec!(0.8) // Decrease weight
            } else {
                dec!(1.0) // Keep same
            };

            adjustments.insert(strategy.clone(), adjustment);
        }

        adjustments
    }

    /// Get pattern effectiveness
    pub fn pattern_effectiveness(&self) -> &HashMap<String, PatternStats> {
        &self.pattern_effectiveness
    }

    /// Get strategy performance
    pub fn strategy_performance(&self) -> &HashMap<String, StrategyStats> {
        &self.strategy_performance
    }

    /// Get trade history
    pub fn history(&self) -> &[TradeRecord] {
        &self.trade_history
    }

    /// Get recent daily performance
    pub fn recent_daily_performance(&self, days: usize) -> Vec<&DailyPerformance> {
        self.daily_performance.iter().rev().take(days).collect()
    }
}

impl StrategyStats {
    fn profit_factor(&self) -> Decimal {
        if self.losses == 0 {
            return Decimal::from(10);
        }
        let avg_win = self.total_pnl / Decimal::from(self.wins.max(1));
        let avg_loss = self.total_pnl.abs() / Decimal::from(self.losses);
        if avg_loss.is_zero() {
            Decimal::from(10)
        } else {
            (self.win_rate * avg_win) / ((Decimal::ONE - self.win_rate) * avg_loss)
        }
    }
}

/// Pattern analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalysis {
    pub total_patterns: usize,
    pub effective_patterns: usize,
    pub ineffective_patterns: usize,
    pub top_patterns: Vec<PatternStats>,
    pub worst_patterns: Vec<PatternStats>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use simmons_core::Side;

    fn make_trade(pnl: Decimal, outcome: TradeOutcome) -> Trade {
        Trade {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: "BTC-USDT".to_string(),
            side: Side::Long,
            size: dec!(0.1),
            entry_price: dec!(67000),
            exit_price: dec!(67000) + pnl * dec!(10),
            pnl,
            outcome,
            reason: "test".to_string(),
            opened_at: Utc::now(),
            closed_at: Utc::now(),
        }
    }

    #[test]
    fn test_record_outcome() {
        let mut engine = LearningEngine::with_defaults();

        let trade = make_trade(dec!(100), TradeOutcome::Win);
        engine.record_outcome(
            trade,
            "Test trade",
            vec!["momentum".to_string()],
            vec!["pattern1".to_string()],
            MarketConditions::default(),
        );

        assert_eq!(engine.trade_history.len(), 1);
        assert!(engine.pattern_effectiveness.contains_key("pattern1"));
        assert!(engine.strategy_performance.contains_key("momentum"));
    }

    #[test]
    fn test_pattern_stats() {
        let mut engine = LearningEngine::with_defaults();

        // Record 20 trades - 15 wins, 5 losses
        for i in 0..20 {
            let (pnl, outcome) = if i < 15 {
                (dec!(50), TradeOutcome::Win)
            } else {
                (dec!(-30), TradeOutcome::Loss)
            };
            let trade = make_trade(pnl, outcome);
            engine.record_outcome(
                trade,
                "Test",
                vec![],
                vec!["test_pattern".to_string()],
                MarketConditions::default(),
            );
        }

        let stats = engine.pattern_effectiveness.get("test_pattern").unwrap();
        assert_eq!(stats.total_trades, 20);
        assert_eq!(stats.wins, 15);
        assert_eq!(stats.losses, 5);
        assert!(stats.win_rate > dec!(0.7));
        assert!(stats.is_effective);
    }

    #[test]
    fn test_generate_insights() {
        let mut engine = LearningEngine::with_defaults();

        // Record some trades
        for i in 0..30 {
            let (pnl, outcome) = if i % 3 != 0 {
                (dec!(40), TradeOutcome::Win)
            } else {
                (dec!(-25), TradeOutcome::Loss)
            };
            let trade = make_trade(pnl, outcome);
            engine.record_outcome(
                trade,
                "Test",
                vec!["momentum".to_string()],
                vec!["pattern1".to_string()],
                MarketConditions::default(),
            );
        }

        let insights = engine.generate_insights();
        assert!(insights.period_trades > 0);
        assert!(insights.period_win_rate > dec!(0.5));
    }

    #[test]
    fn test_weight_adjustments() {
        let mut engine = LearningEngine::with_defaults();

        // Record good performance for momentum
        for _ in 0..20 {
            let trade = make_trade(dec!(50), TradeOutcome::Win);
            engine.record_outcome(
                trade,
                "Test",
                vec!["momentum".to_string()],
                vec![],
                MarketConditions::default(),
            );
        }

        // Record bad performance for mean_rev
        for _ in 0..15 {
            let trade = make_trade(dec!(-30), TradeOutcome::Loss);
            engine.record_outcome(
                trade,
                "Test",
                vec!["mean_rev".to_string()],
                vec![],
                MarketConditions::default(),
            );
        }

        let mut current = HashMap::new();
        current.insert("momentum".to_string(), dec!(0.25));
        current.insert("mean_rev".to_string(), dec!(0.25));

        let adjusted = engine.adjust_weights(&current);

        // Momentum should increase, mean_rev should decrease
        assert!(adjusted.get("momentum").unwrap() > &dec!(0.25));
        assert!(adjusted.get("mean_rev").unwrap() < &dec!(0.25));
    }
}
