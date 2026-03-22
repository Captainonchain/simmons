//! Gas Optimizer - AI-powered gas price optimization
//!
//! Predicts optimal gas prices and timing for transactions.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Transaction priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// Time-sensitive - must execute now
    Urgent,
    /// Standard priority - within a few blocks
    Normal,
    /// Can wait for lower gas
    Low,
    /// No time pressure - execute when cheapest
    Deferred,
}

impl Priority {
    /// Maximum acceptable delay in seconds
    pub fn max_delay_secs(&self) -> u64 {
        match self {
            Priority::Urgent => 10,
            Priority::Normal => 60,
            Priority::Low => 300,
            Priority::Deferred => 3600,
        }
    }

    /// Maximum gas premium multiplier
    pub fn max_premium_multiplier(&self) -> Decimal {
        match self {
            Priority::Urgent => dec!(2.0),
            Priority::Normal => dec!(1.3),
            Priority::Low => dec!(1.1),
            Priority::Deferred => dec!(1.0),
        }
    }
}

/// Wait decision from optimizer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum WaitDecision {
    /// Execute immediately
    ExecuteNow { gas_price: Decimal },
    /// Wait for better price
    Wait {
        estimated_wait_secs: u64,
        expected_price: Decimal,
        current_price: Decimal,
        savings_pct: Decimal,
    },
    /// Gas too high, skip
    Skip { reason: String },
}

/// Gas price history entry
#[derive(Debug, Clone)]
struct GasHistoryEntry {
    timestamp: Instant,
    gas_price: Decimal,
    block_number: u64,
}

/// Gas optimizer
pub struct GasOptimizer {
    config: GasOptimizerConfig,
    history: VecDeque<GasHistoryEntry>,
    predictions: Option<GasPrediction>,
}

/// Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasOptimizerConfig {
    /// Maximum gas price to ever pay (gwei)
    pub max_gas_gwei: Decimal,
    /// History window size
    pub history_window: usize,
    /// Enable time-of-day optimization
    pub use_time_patterns: bool,
    /// Enable block space predictions
    pub use_block_predictions: bool,
}

impl Default for GasOptimizerConfig {
    fn default() -> Self {
        Self {
            max_gas_gwei: dec!(100),
            history_window: 100,
            use_time_patterns: true,
            use_block_predictions: true,
        }
    }
}

/// Gas prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPrediction {
    pub current: Decimal,
    pub predicted_5min: Decimal,
    pub predicted_15min: Decimal,
    pub predicted_1hr: Decimal,
    pub trend: GasTrend,
    pub confidence: Decimal,
}

/// Gas price trend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GasTrend {
    Rising,
    Falling,
    Stable,
    Volatile,
}

impl GasOptimizer {
    pub fn new(config: GasOptimizerConfig) -> Self {
        Self {
            config,
            history: VecDeque::with_capacity(100),
            predictions: None,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(GasOptimizerConfig::default())
    }

    /// Record a gas price observation
    pub fn record_gas_price(&mut self, gas_price: Decimal, block_number: u64) {
        let entry = GasHistoryEntry {
            timestamp: Instant::now(),
            gas_price,
            block_number,
        };

        if self.history.len() >= self.config.history_window {
            self.history.pop_front();
        }
        self.history.push_back(entry);

        // Update predictions
        self.update_predictions();
    }

    /// Get optimal gas price for priority level
    pub fn optimal_gas(&self, priority: Priority, current_gas: Decimal) -> Decimal {
        let base_price = current_gas;

        // Apply priority multiplier
        let with_priority = base_price * priority.max_premium_multiplier();

        // Cap at max
        with_priority.min(self.config.max_gas_gwei)
    }

    /// Decide whether to wait for lower gas
    pub fn should_wait(&self, priority: Priority, current_gas: Decimal) -> WaitDecision {
        // Never wait for urgent
        if priority == Priority::Urgent {
            return WaitDecision::ExecuteNow {
                gas_price: self.optimal_gas(priority, current_gas),
            };
        }

        // Check if gas is too high
        if current_gas > self.config.max_gas_gwei {
            return WaitDecision::Skip {
                reason: format!(
                    "Gas {} gwei exceeds max {} gwei",
                    current_gas, self.config.max_gas_gwei
                ),
            };
        }

        // Use predictions if available
        if let Some(pred) = &self.predictions {
            let (expected, wait_secs) = match priority {
                Priority::Urgent => (current_gas, 0),
                Priority::Normal => (pred.predicted_5min, 300),
                Priority::Low => (pred.predicted_15min, 900),
                Priority::Deferred => (pred.predicted_1hr, 3600),
            };

            // Check if waiting is worthwhile (>10% savings)
            if expected < current_gas * dec!(0.9) {
                let savings_pct = ((current_gas - expected) / current_gas) * dec!(100);

                return WaitDecision::Wait {
                    estimated_wait_secs: wait_secs,
                    expected_price: expected,
                    current_price: current_gas,
                    savings_pct,
                };
            }
        }

        // Execute now if no benefit to waiting
        WaitDecision::ExecuteNow {
            gas_price: self.optimal_gas(priority, current_gas),
        }
    }

    /// Estimate transaction cost
    pub fn estimate_cost(&self, gas_units: u64, gas_price: Decimal) -> TxCostEstimate {
        let gas_cost_gwei = Decimal::from(gas_units) * gas_price;
        let gas_cost_eth = gas_cost_gwei / dec!(1_000_000_000);

        // Estimate USD cost (assume ETH = $3000)
        let eth_price = dec!(3000);
        let cost_usd = gas_cost_eth * eth_price;

        TxCostEstimate {
            gas_units,
            gas_price_gwei: gas_price,
            cost_eth: gas_cost_eth,
            cost_usd,
        }
    }

    /// Batch multiple transactions for gas efficiency
    pub fn batch_transactions(&self, tx_gas_estimates: &[u64]) -> BatchAnalysis {
        let total_gas: u64 = tx_gas_estimates.iter().sum();

        // Batching overhead (transaction base cost)
        let base_cost_per_tx: u64 = 21000;
        let unbatched_gas: u64 = tx_gas_estimates.iter().sum::<u64>()
            + (tx_gas_estimates.len() as u64 * base_cost_per_tx);

        // Batched saves on base costs (only pay once)
        let batched_gas = total_gas + base_cost_per_tx;

        let savings = unbatched_gas.saturating_sub(batched_gas);
        let savings_pct = if unbatched_gas > 0 {
            Decimal::from(savings) / Decimal::from(unbatched_gas) * dec!(100)
        } else {
            Decimal::ZERO
        };

        BatchAnalysis {
            transaction_count: tx_gas_estimates.len(),
            unbatched_gas,
            batched_gas,
            gas_savings: savings,
            savings_pct,
            recommend_batch: savings_pct > dec!(5), // >5% savings
        }
    }

    /// Update internal predictions based on history
    fn update_predictions(&mut self) {
        if self.history.len() < 10 {
            return;
        }

        // Simple moving average and trend analysis
        let recent: Vec<Decimal> = self
            .history
            .iter()
            .rev()
            .take(20)
            .map(|e| e.gas_price)
            .collect();

        let current = recent.first().copied().unwrap_or_default();

        // Calculate averages
        let avg_5 = avg(&recent[..recent.len().min(5)]);
        let avg_20 = avg(&recent);

        // Simple trend detection
        let trend = if recent.len() >= 5 {
            let recent_avg = avg(&recent[..5]);
            let older_avg = if recent.len() >= 10 {
                avg(&recent[5..10])
            } else {
                recent_avg
            };

            if recent_avg > older_avg * dec!(1.1) {
                GasTrend::Rising
            } else if recent_avg < older_avg * dec!(0.9) {
                GasTrend::Falling
            } else {
                GasTrend::Stable
            }
        } else {
            GasTrend::Stable
        };

        // Simple predictions based on trend
        let (pred_5, pred_15, pred_1hr) = match trend {
            GasTrend::Rising => (
                current * dec!(1.1),
                current * dec!(1.2),
                current * dec!(1.3),
            ),
            GasTrend::Falling => (
                current * dec!(0.95),
                current * dec!(0.85),
                current * dec!(0.75),
            ),
            GasTrend::Stable => (current, avg_5, avg_20),
            GasTrend::Volatile => (current, avg_5, avg_20),
        };

        // Confidence based on volatility
        let stddev = std_dev(&recent);
        let cv = if avg_20.is_zero() {
            Decimal::ZERO
        } else {
            stddev / avg_20
        };
        let confidence = (Decimal::ONE - cv.min(Decimal::ONE)).max(dec!(0.3));

        self.predictions = Some(GasPrediction {
            current,
            predicted_5min: pred_5,
            predicted_15min: pred_15,
            predicted_1hr: pred_1hr,
            trend,
            confidence,
        });
    }

    /// Get current predictions
    pub fn predictions(&self) -> Option<&GasPrediction> {
        self.predictions.as_ref()
    }

    /// Get history stats
    pub fn history_stats(&self) -> HistoryStats {
        let prices: Vec<Decimal> = self.history.iter().map(|e| e.gas_price).collect();

        if prices.is_empty() {
            return HistoryStats::default();
        }

        let min = prices.iter().copied().min().unwrap_or_default();
        let max = prices.iter().copied().max().unwrap_or_default();
        let mean = avg(&prices);
        let std = std_dev(&prices);

        // Percentiles
        let mut sorted = prices.clone();
        sorted.sort();
        let p25 = percentile(&sorted, 25);
        let p50 = percentile(&sorted, 50);
        let p75 = percentile(&sorted, 75);

        HistoryStats {
            count: prices.len(),
            min,
            max,
            mean,
            std_dev: std,
            p25,
            p50,
            p75,
        }
    }
}

/// Transaction cost estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxCostEstimate {
    pub gas_units: u64,
    pub gas_price_gwei: Decimal,
    pub cost_eth: Decimal,
    pub cost_usd: Decimal,
}

/// Batch analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAnalysis {
    pub transaction_count: usize,
    pub unbatched_gas: u64,
    pub batched_gas: u64,
    pub gas_savings: u64,
    pub savings_pct: Decimal,
    pub recommend_batch: bool,
}

/// History statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryStats {
    pub count: usize,
    pub min: Decimal,
    pub max: Decimal,
    pub mean: Decimal,
    pub std_dev: Decimal,
    pub p25: Decimal,
    pub p50: Decimal,
    pub p75: Decimal,
}

/// Helper: calculate average
fn avg(values: &[Decimal]) -> Decimal {
    if values.is_empty() {
        return Decimal::ZERO;
    }
    values.iter().sum::<Decimal>() / Decimal::from(values.len())
}

/// Helper: calculate standard deviation
fn std_dev(values: &[Decimal]) -> Decimal {
    if values.len() < 2 {
        return Decimal::ZERO;
    }

    let mean = avg(values);
    let variance: Decimal = values
        .iter()
        .map(|v| {
            let diff = *v - mean;
            diff * diff
        })
        .sum::<Decimal>()
        / Decimal::from(values.len() - 1);

    // Approximate sqrt using Newton's method
    if variance.is_zero() {
        return Decimal::ZERO;
    }

    let mut x = variance;
    for _ in 0..15 {
        x = (x + variance / x) / dec!(2);
    }
    x
}

/// Helper: calculate percentile
fn percentile(sorted: &[Decimal], pct: usize) -> Decimal {
    if sorted.is_empty() {
        return Decimal::ZERO;
    }
    let idx = (sorted.len() * pct / 100).min(sorted.len() - 1);
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_gas_by_priority() {
        let optimizer = GasOptimizer::with_defaults();
        let current = dec!(30);

        let urgent = optimizer.optimal_gas(Priority::Urgent, current);
        let normal = optimizer.optimal_gas(Priority::Normal, current);
        let low = optimizer.optimal_gas(Priority::Low, current);

        assert!(urgent > normal);
        assert!(normal > low);
        assert!(urgent <= dec!(60)); // 30 * 2.0
        assert!(normal <= dec!(39)); // 30 * 1.3
    }

    #[test]
    fn test_should_wait_urgent() {
        let optimizer = GasOptimizer::with_defaults();

        let decision = optimizer.should_wait(Priority::Urgent, dec!(50));

        matches!(decision, WaitDecision::ExecuteNow { .. });
    }

    #[test]
    fn test_gas_too_high() {
        let optimizer = GasOptimizer::with_defaults();

        let decision = optimizer.should_wait(Priority::Normal, dec!(200));

        matches!(decision, WaitDecision::Skip { .. });
    }

    #[test]
    fn test_estimate_cost() {
        let optimizer = GasOptimizer::with_defaults();

        let estimate = optimizer.estimate_cost(100_000, dec!(30));

        // 100k gas * 30 gwei = 3M gwei = 0.003 ETH
        assert_eq!(estimate.cost_eth, dec!(0.003));
        // 0.003 ETH * $3000 = $9
        assert_eq!(estimate.cost_usd, dec!(9));
    }

    #[test]
    fn test_batch_analysis() {
        let optimizer = GasOptimizer::with_defaults();

        // 3 transactions of 50k gas each
        let analysis = optimizer.batch_transactions(&[50000, 50000, 50000]);

        // Unbatched: 150k + 3*21k = 213k
        // Batched: 150k + 21k = 171k
        // Savings: 42k
        assert_eq!(analysis.transaction_count, 3);
        assert!(analysis.gas_savings > 40000);
        assert!(analysis.recommend_batch);
    }

    #[test]
    fn test_predictions() {
        let mut optimizer = GasOptimizer::with_defaults();

        // Record falling gas prices
        for i in 0..20 {
            optimizer.record_gas_price(dec!(50) - Decimal::from(i), i as u64);
        }

        let pred = optimizer.predictions().unwrap();
        assert_eq!(pred.trend, GasTrend::Falling);
        assert!(pred.predicted_5min < pred.current);
    }

    #[test]
    fn test_history_stats() {
        let mut optimizer = GasOptimizer::with_defaults();

        // Add some data
        for gas in [20, 25, 30, 35, 40, 45, 50] {
            optimizer.record_gas_price(Decimal::from(gas), 0);
        }

        let stats = optimizer.history_stats();
        assert_eq!(stats.count, 7);
        assert_eq!(stats.min, dec!(20));
        assert_eq!(stats.max, dec!(50));
        assert!(stats.mean > dec!(30) && stats.mean < dec!(40));
    }
}
