//! Feedback Loop
//!
//! Connects trade outcomes back to learning and Claude brain.

use crate::learning::{LearningEngine, LearningInsights, MarketConditions, TradeRecord};
use crate::bridge::BrainBridge;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::Trade;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Feedback loop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackConfig {
    /// Minimum trades before syncing learnings
    pub min_trades_before_sync: usize,
    /// Sync interval (seconds)
    pub sync_interval_secs: u64,
    /// Enable auto weight adjustment
    pub auto_adjust_weights: bool,
    /// Performance report interval (seconds)
    pub report_interval_secs: u64,
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        Self {
            min_trades_before_sync: 10,
            sync_interval_secs: 3600, // 1 hour
            auto_adjust_weights: true,
            report_interval_secs: 86400, // Daily
        }
    }
}

/// Feedback loop connecting learning to brain
pub struct FeedbackLoop {
    config: FeedbackConfig,
    learning: LearningEngine,
    pending_trades: usize,
    last_sync: i64,
    last_report: i64,
    strategy_weights: HashMap<String, Decimal>,
}

impl FeedbackLoop {
    pub fn new(config: FeedbackConfig) -> Self {
        let mut weights = HashMap::new();
        weights.insert("momentum".to_string(), dec!(0.25));
        weights.insert("mean_reversion".to_string(), dec!(0.20));
        weights.insert("regime".to_string(), dec!(0.20));
        weights.insert("arbitrage".to_string(), dec!(0.15));
        weights.insert("sentiment".to_string(), dec!(0.10));
        weights.insert("volume".to_string(), dec!(0.10));

        Self {
            config,
            learning: LearningEngine::with_defaults(),
            pending_trades: 0,
            last_sync: 0,
            last_report: 0,
            strategy_weights: weights,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(FeedbackConfig::default())
    }

    /// Called when a trade completes
    pub fn on_trade_complete(
        &mut self,
        trade: Trade,
        reasoning: &str,
        signals: Vec<String>,
        patterns: Vec<String>,
        conditions: MarketConditions,
    ) {
        // Record to learning engine
        self.learning.record_outcome(trade.clone(), reasoning, signals, patterns, conditions);
        self.pending_trades += 1;

        info!("Feedback: Trade recorded. Pending: {}", self.pending_trades);

        // Check if we should sync
        if self.should_sync() {
            self.sync_learnings();
        }
    }

    /// Check if sync is needed
    fn should_sync(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        let time_since_sync = (now - self.last_sync) as u64;

        self.pending_trades >= self.config.min_trades_before_sync
            || time_since_sync >= self.config.sync_interval_secs
    }

    /// Sync learnings (would notify Claude in full implementation)
    pub fn sync_learnings(&mut self) {
        let insights = self.learning.generate_insights();

        info!(
            "Syncing learnings: {} trades, {:.1}% win rate, PnL: {}",
            insights.period_trades,
            insights.period_win_rate * dec!(100),
            insights.period_pnl
        );

        // Auto-adjust weights if enabled
        if self.config.auto_adjust_weights && insights.period_trades >= 10 {
            self.strategy_weights = self.learning.adjust_weights(&self.strategy_weights);
            info!("Adjusted strategy weights: {:?}", self.strategy_weights);
        }

        // Log recommendations
        for rec in &insights.recommendations {
            info!("Learning recommendation: {}", rec);
        }

        self.pending_trades = 0;
        self.last_sync = chrono::Utc::now().timestamp();
    }

    /// Generate performance report
    pub fn performance_report(&self) -> PerformanceReport {
        let insights = self.learning.generate_insights();
        let pattern_analysis = self.learning.analyze_patterns();

        // Calculate overall metrics
        let total_trades = self.learning.history().len();
        let wins = self.learning.history()
            .iter()
            .filter(|r| r.trade.outcome == simmons_core::TradeOutcome::Win)
            .count();
        let total_pnl: Decimal = self.learning.history()
            .iter()
            .map(|r| r.trade.pnl)
            .sum();

        let win_rate = if total_trades > 0 {
            Decimal::from(wins) / Decimal::from(total_trades)
        } else {
            Decimal::ZERO
        };

        // Daily performance summary
        let daily = self.learning.recent_daily_performance(7);
        let daily_avg_pnl = if daily.is_empty() {
            Decimal::ZERO
        } else {
            daily.iter().map(|d| d.pnl).sum::<Decimal>() / Decimal::from(daily.len())
        };

        // Strategy breakdown
        let strategy_breakdown: Vec<StrategyBreakdown> = self
            .learning
            .strategy_performance()
            .values()
            .map(|s| StrategyBreakdown {
                name: s.strategy_name.clone(),
                trades: s.total_trades,
                win_rate: s.win_rate,
                pnl: s.total_pnl,
                current_weight: self.strategy_weights.get(&s.strategy_name).copied().unwrap_or(Decimal::ZERO),
                streak: s.current_streak,
            })
            .collect();

        PerformanceReport {
            period: "All Time".to_string(),
            total_trades,
            wins,
            losses: total_trades - wins,
            win_rate,
            total_pnl,
            daily_avg_pnl,
            effective_patterns: pattern_analysis.effective_patterns,
            ineffective_patterns: pattern_analysis.ineffective_patterns,
            strategy_breakdown,
            recommendations: insights.recommendations,
            generated_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Get current strategy weights
    pub fn weights(&self) -> &HashMap<String, Decimal> {
        &self.strategy_weights
    }

    /// Set strategy weight
    pub fn set_weight(&mut self, strategy: &str, weight: Decimal) {
        self.strategy_weights.insert(strategy.to_string(), weight);
    }

    /// Get learning engine reference
    pub fn learning(&self) -> &LearningEngine {
        &self.learning
    }

    /// Get mutable learning engine reference
    pub fn learning_mut(&mut self) -> &mut LearningEngine {
        &mut self.learning
    }

    /// Should generate report?
    pub fn should_report(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        (now - self.last_report) as u64 >= self.config.report_interval_secs
    }

    /// Mark report as generated
    pub fn mark_reported(&mut self) {
        self.last_report = chrono::Utc::now().timestamp();
    }

    /// Get recent insights
    pub fn get_insights(&self) -> LearningInsights {
        self.learning.generate_insights()
    }

    /// Evaluate strategy health
    pub fn evaluate_strategy_health(&self) -> HealthReport {
        let performance = self.learning.strategy_performance();

        let mut healthy = Vec::new();
        let mut warning = Vec::new();
        let mut critical = Vec::new();

        for (name, stats) in performance {
            if stats.total_trades < 10 {
                continue; // Not enough data
            }

            let health = if stats.win_rate >= dec!(0.55) && stats.current_streak >= 0 {
                StrategyHealth::Healthy
            } else if stats.win_rate >= dec!(0.45) || stats.current_streak > -3 {
                StrategyHealth::Warning
            } else {
                StrategyHealth::Critical
            };

            let entry = StrategyHealthEntry {
                name: name.clone(),
                health,
                win_rate: stats.win_rate,
                streak: stats.current_streak,
                pnl: stats.total_pnl,
            };

            match health {
                StrategyHealth::Healthy => healthy.push(entry),
                StrategyHealth::Warning => warning.push(entry),
                StrategyHealth::Critical => critical.push(entry),
            }
        }

        let overall = if !critical.is_empty() {
            StrategyHealth::Critical
        } else if !warning.is_empty() && warning.len() > healthy.len() {
            StrategyHealth::Warning
        } else {
            StrategyHealth::Healthy
        };

        HealthReport {
            overall,
            healthy,
            warning,
            critical,
        }
    }
}

/// Performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub period: String,
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: Decimal,
    pub total_pnl: Decimal,
    pub daily_avg_pnl: Decimal,
    pub effective_patterns: usize,
    pub ineffective_patterns: usize,
    pub strategy_breakdown: Vec<StrategyBreakdown>,
    pub recommendations: Vec<String>,
    pub generated_at: i64,
}

/// Strategy breakdown in report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyBreakdown {
    pub name: String,
    pub trades: usize,
    pub win_rate: Decimal,
    pub pnl: Decimal,
    pub current_weight: Decimal,
    pub streak: i32,
}

/// Strategy health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrategyHealth {
    Healthy,
    Warning,
    Critical,
}

/// Strategy health entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyHealthEntry {
    pub name: String,
    pub health: StrategyHealth,
    pub win_rate: Decimal,
    pub streak: i32,
    pub pnl: Decimal,
}

/// Overall health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall: StrategyHealth,
    pub healthy: Vec<StrategyHealthEntry>,
    pub warning: Vec<StrategyHealthEntry>,
    pub critical: Vec<StrategyHealthEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use simmons_core::{Side, TradeOutcome};

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
    fn test_on_trade_complete() {
        let mut feedback = FeedbackLoop::with_defaults();
        // Set last_sync to now to avoid time-based trigger
        feedback.last_sync = chrono::Utc::now().timestamp();

        let trade = make_trade(dec!(100), TradeOutcome::Win);
        feedback.on_trade_complete(
            trade,
            "Test trade",
            vec!["momentum".to_string()],
            vec![],
            MarketConditions::default(),
        );

        assert_eq!(feedback.pending_trades, 1);
        assert_eq!(feedback.learning.history().len(), 1);
    }

    #[test]
    fn test_sync_triggers() {
        let mut feedback = FeedbackLoop::with_defaults();
        // Set last_sync to now to avoid time-based trigger
        feedback.last_sync = chrono::Utc::now().timestamp();

        // Record enough trades to trigger sync
        for _ in 0..feedback.config.min_trades_before_sync {
            let trade = make_trade(dec!(50), TradeOutcome::Win);
            feedback.on_trade_complete(
                trade,
                "Test",
                vec!["momentum".to_string()],
                vec![],
                MarketConditions::default(),
            );
        }

        // Should have synced and reset pending
        assert_eq!(feedback.pending_trades, 0);
    }

    #[test]
    fn test_performance_report() {
        let mut feedback = FeedbackLoop::with_defaults();

        // Record some trades
        for i in 0..20 {
            let (pnl, outcome) = if i % 3 != 0 {
                (dec!(50), TradeOutcome::Win)
            } else {
                (dec!(-30), TradeOutcome::Loss)
            };
            let trade = make_trade(pnl, outcome);
            feedback.on_trade_complete(
                trade,
                "Test",
                vec!["momentum".to_string()],
                vec![],
                MarketConditions::default(),
            );
        }

        let report = feedback.performance_report();
        assert_eq!(report.total_trades, 20);
        assert!(report.win_rate > dec!(0.5));
    }

    #[test]
    fn test_strategy_health() {
        let mut feedback = FeedbackLoop::with_defaults();

        // Good performance for momentum
        for _ in 0..15 {
            let trade = make_trade(dec!(40), TradeOutcome::Win);
            feedback.on_trade_complete(
                trade,
                "Test",
                vec!["momentum".to_string()],
                vec![],
                MarketConditions::default(),
            );
        }

        // Poor performance for mean_rev
        for _ in 0..12 {
            let trade = make_trade(dec!(-35), TradeOutcome::Loss);
            feedback.on_trade_complete(
                trade,
                "Test",
                vec!["mean_rev".to_string()],
                vec![],
                MarketConditions::default(),
            );
        }

        let health = feedback.evaluate_strategy_health();
        assert!(health.healthy.iter().any(|s| s.name == "momentum"));
        assert!(health.critical.iter().any(|s| s.name == "mean_rev"));
    }

    #[test]
    fn test_weight_adjustment() {
        let mut feedback = FeedbackLoop::with_defaults();

        let initial_momentum = *feedback.weights().get("momentum").unwrap();

        // Good momentum performance
        for _ in 0..20 {
            let trade = make_trade(dec!(50), TradeOutcome::Win);
            feedback.on_trade_complete(
                trade,
                "Test",
                vec!["momentum".to_string()],
                vec![],
                MarketConditions::default(),
            );
        }

        feedback.sync_learnings();

        // Momentum weight should have increased
        let new_momentum = *feedback.weights().get("momentum").unwrap();
        assert!(new_momentum >= initial_momentum);
    }
}
