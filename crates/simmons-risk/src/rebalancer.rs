//! Portfolio Rebalancer
//!
//! AI-optimized portfolio weight calculation and rebalancing trades.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::{Order, OrderType, Position, Side};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Rebalancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancerConfig {
    /// Minimum deviation to trigger rebalance (%)
    pub min_deviation_pct: Decimal,
    /// Maximum single trade size (% of portfolio)
    pub max_trade_pct: Decimal,
    /// Target correlation limit
    pub max_correlation: Decimal,
    /// Minimum position size (USD)
    pub min_position_usd: Decimal,
    /// Maximum positions
    pub max_positions: usize,
    /// Enable AI weight optimization
    pub ai_optimization: bool,
}

impl Default for RebalancerConfig {
    fn default() -> Self {
        Self {
            min_deviation_pct: dec!(5),
            max_trade_pct: dec!(25),
            max_correlation: dec!(0.7),
            min_position_usd: dec!(100),
            max_positions: 10,
            ai_optimization: true,
        }
    }
}

/// Target portfolio allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAllocation {
    pub weights: HashMap<String, Decimal>,
    pub reason: String,
    pub generated_at: i64,
}

impl TargetAllocation {
    /// Validate weights sum to 1
    pub fn is_valid(&self) -> bool {
        let sum: Decimal = self.weights.values().sum();
        (sum - Decimal::ONE).abs() < dec!(0.01)
    }

    /// Normalize weights to sum to 1
    pub fn normalize(&mut self) {
        let sum: Decimal = self.weights.values().sum();
        if !sum.is_zero() {
            for weight in self.weights.values_mut() {
                *weight = *weight / sum;
            }
        }
    }
}

/// Current portfolio state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioState {
    pub positions: HashMap<String, Decimal>,
    pub total_value: Decimal,
    pub cash: Decimal,
}

impl PortfolioState {
    /// Get current weights
    pub fn current_weights(&self) -> HashMap<String, Decimal> {
        let mut weights = HashMap::new();
        if self.total_value.is_zero() {
            return weights;
        }

        for (symbol, value) in &self.positions {
            weights.insert(symbol.clone(), *value / self.total_value);
        }
        weights.insert("cash".to_string(), self.cash / self.total_value);
        weights
    }
}

/// Rebalancing trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceTrade {
    pub symbol: String,
    pub side: Side,
    pub amount_usd: Decimal,
    pub current_weight: Decimal,
    pub target_weight: Decimal,
    pub deviation: Decimal,
    pub priority: u8,
}

/// Correlation check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationCheck {
    pub passed: bool,
    pub max_correlation: Decimal,
    pub highly_correlated_pairs: Vec<(String, String, Decimal)>,
    pub recommendations: Vec<String>,
}

/// Portfolio rebalancer
pub struct PortfolioRebalancer {
    config: RebalancerConfig,
    correlation_matrix: HashMap<(String, String), Decimal>,
}

impl PortfolioRebalancer {
    pub fn new(config: RebalancerConfig) -> Self {
        Self {
            config,
            correlation_matrix: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(RebalancerConfig::default())
    }

    /// Update correlation matrix
    pub fn update_correlation(&mut self, symbol_a: &str, symbol_b: &str, correlation: Decimal) {
        self.correlation_matrix
            .insert((symbol_a.to_string(), symbol_b.to_string()), correlation);
        self.correlation_matrix
            .insert((symbol_b.to_string(), symbol_a.to_string()), correlation);
    }

    /// Calculate optimal weights using AI (simplified model)
    pub fn calculate_weights(
        &self,
        assets: &[AssetInfo],
        risk_tolerance: Decimal,
    ) -> TargetAllocation {
        let mut weights = HashMap::new();

        if assets.is_empty() {
            return TargetAllocation {
                weights,
                reason: "No assets provided".to_string(),
                generated_at: chrono::Utc::now().timestamp(),
            };
        }

        // Simple risk-parity-like allocation
        let mut total_inverse_vol = Decimal::ZERO;
        let mut inverse_vols = HashMap::new();

        for asset in assets {
            let inverse_vol = if asset.volatility.is_zero() {
                dec!(1)
            } else {
                Decimal::ONE / asset.volatility
            };
            inverse_vols.insert(asset.symbol.clone(), inverse_vol);
            total_inverse_vol += inverse_vol;
        }

        // Adjust for risk tolerance
        let risk_adjustment = dec!(0.5) + risk_tolerance * dec!(0.5);

        for asset in assets {
            let inverse_vol = inverse_vols.get(&asset.symbol).copied().unwrap_or(Decimal::ONE);
            let base_weight = inverse_vol / total_inverse_vol;

            // Adjust based on momentum and risk tolerance
            let momentum_adjustment = if asset.momentum > Decimal::ZERO {
                Decimal::ONE + (asset.momentum * risk_adjustment).min(dec!(0.2))
            } else {
                Decimal::ONE + (asset.momentum * risk_adjustment).max(dec!(-0.2))
            };

            let weight = (base_weight * momentum_adjustment).max(Decimal::ZERO);
            weights.insert(asset.symbol.clone(), weight);
        }

        // Normalize
        let sum: Decimal = weights.values().sum();
        if !sum.is_zero() {
            for weight in weights.values_mut() {
                *weight = *weight / sum;
            }
        }

        let reason = format!(
            "Risk-parity allocation with {} risk tolerance, {} assets",
            risk_tolerance,
            assets.len()
        );

        TargetAllocation {
            weights,
            reason,
            generated_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Generate rebalancing trades
    pub fn generate_rebalance_trades(
        &self,
        current: &PortfolioState,
        target: &TargetAllocation,
    ) -> Vec<RebalanceTrade> {
        let mut trades = Vec::new();
        let current_weights = current.current_weights();

        for (symbol, target_weight) in &target.weights {
            let current_weight = current_weights.get(symbol).copied().unwrap_or(Decimal::ZERO);
            let deviation = *target_weight - current_weight;
            let deviation_pct = deviation.abs() * dec!(100);

            // Only trade if deviation exceeds threshold
            if deviation_pct < self.config.min_deviation_pct {
                continue;
            }

            let amount_usd = (deviation.abs() * current.total_value)
                .min(current.total_value * self.config.max_trade_pct / dec!(100));

            if amount_usd < self.config.min_position_usd {
                continue;
            }

            let side = if deviation > Decimal::ZERO {
                Side::Long // Buy more
            } else {
                Side::Short // Sell
            };

            // Priority: larger deviations first
            let priority = (deviation_pct / dec!(10)).min(dec!(10)).to_u64().unwrap_or(5) as u8;

            trades.push(RebalanceTrade {
                symbol: symbol.clone(),
                side,
                amount_usd,
                current_weight,
                target_weight: *target_weight,
                deviation,
                priority,
            });
        }

        // Check for positions that should be closed (not in target)
        for (symbol, current_weight) in &current_weights {
            if !target.weights.contains_key(symbol) && symbol != "cash" {
                let amount_usd = *current_weight * current.total_value;
                if amount_usd >= self.config.min_position_usd {
                    trades.push(RebalanceTrade {
                        symbol: symbol.clone(),
                        side: Side::Short,
                        amount_usd,
                        current_weight: *current_weight,
                        target_weight: Decimal::ZERO,
                        deviation: -*current_weight,
                        priority: 10, // High priority to close unwanted positions
                    });
                }
            }
        }

        // Sort by priority (descending)
        trades.sort_by(|a, b| b.priority.cmp(&a.priority));

        trades
    }

    /// Check correlation limits
    pub fn check_correlation(&self, positions: &[String]) -> CorrelationCheck {
        let mut max_corr = Decimal::ZERO;
        let mut high_pairs = Vec::new();
        let mut recommendations = Vec::new();

        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let key = (positions[i].clone(), positions[j].clone());
                if let Some(&corr) = self.correlation_matrix.get(&key) {
                    if corr > max_corr {
                        max_corr = corr;
                    }
                    if corr > self.config.max_correlation {
                        high_pairs.push((positions[i].clone(), positions[j].clone(), corr));
                    }
                }
            }
        }

        if !high_pairs.is_empty() {
            recommendations.push(format!(
                "Reduce exposure to highly correlated pairs: {}",
                high_pairs
                    .iter()
                    .map(|(a, b, c)| format!("{}/{} ({:.0}%)", a, b, c * dec!(100)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        CorrelationCheck {
            passed: high_pairs.is_empty(),
            max_correlation: max_corr,
            highly_correlated_pairs: high_pairs,
            recommendations,
        }
    }

    /// Convert rebalance trade to order
    pub fn trade_to_order(&self, trade: &RebalanceTrade, current_price: Decimal) -> Order {
        let size = if current_price.is_zero() {
            Decimal::ZERO
        } else {
            trade.amount_usd / current_price
        };

        Order {
            symbol: trade.symbol.clone(),
            side: trade.side,
            size,
            order_type: OrderType::Market,
            limit_price: None,
            stop_loss: None,
            take_profit: None,
        }
    }

    /// Calculate drift from target
    pub fn calculate_drift(&self, current: &PortfolioState, target: &TargetAllocation) -> DriftReport {
        let current_weights = current.current_weights();
        let mut total_drift = Decimal::ZERO;
        let mut max_drift = Decimal::ZERO;
        let mut drifted_assets = Vec::new();

        for (symbol, target_weight) in &target.weights {
            let current_weight = current_weights.get(symbol).copied().unwrap_or(Decimal::ZERO);
            let drift = (*target_weight - current_weight).abs();

            total_drift += drift;
            if drift > max_drift {
                max_drift = drift;
            }

            if drift * dec!(100) >= self.config.min_deviation_pct {
                drifted_assets.push(DriftedAsset {
                    symbol: symbol.clone(),
                    current_weight,
                    target_weight: *target_weight,
                    drift,
                    drift_pct: drift * dec!(100),
                });
            }
        }

        let needs_rebalance = !drifted_assets.is_empty();

        DriftReport {
            total_drift,
            max_drift,
            average_drift: total_drift / Decimal::from(target.weights.len().max(1)),
            needs_rebalance,
            drifted_assets,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &RebalancerConfig {
        &self.config
    }
}

/// Asset information for weight calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub symbol: String,
    pub volatility: Decimal,
    pub momentum: Decimal,
    pub current_price: Decimal,
}

/// Drift report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub total_drift: Decimal,
    pub max_drift: Decimal,
    pub average_drift: Decimal,
    pub needs_rebalance: bool,
    pub drifted_assets: Vec<DriftedAsset>,
}

/// Drifted asset detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftedAsset {
    pub symbol: String,
    pub current_weight: Decimal,
    pub target_weight: Decimal,
    pub drift: Decimal,
    pub drift_pct: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_weights() {
        let rebalancer = PortfolioRebalancer::with_defaults();

        let assets = vec![
            AssetInfo {
                symbol: "BTC".to_string(),
                volatility: dec!(0.03),
                momentum: dec!(0.02),
                current_price: dec!(67000),
            },
            AssetInfo {
                symbol: "ETH".to_string(),
                volatility: dec!(0.04),
                momentum: dec!(-0.01),
                current_price: dec!(3500),
            },
            AssetInfo {
                symbol: "SOL".to_string(),
                volatility: dec!(0.05),
                momentum: dec!(0.05),
                current_price: dec!(150),
            },
        ];

        let allocation = rebalancer.calculate_weights(&assets, dec!(0.5));

        assert!(allocation.is_valid());
        // BTC should have higher weight (lower vol)
        assert!(allocation.weights.get("BTC").unwrap() > allocation.weights.get("SOL").unwrap());
    }

    #[test]
    fn test_generate_rebalance_trades() {
        let rebalancer = PortfolioRebalancer::with_defaults();

        let mut target_weights = HashMap::new();
        target_weights.insert("BTC".to_string(), dec!(0.5));
        target_weights.insert("ETH".to_string(), dec!(0.3));
        target_weights.insert("cash".to_string(), dec!(0.2));

        let target = TargetAllocation {
            weights: target_weights,
            reason: "Test".to_string(),
            generated_at: 0,
        };

        let mut positions = HashMap::new();
        positions.insert("BTC".to_string(), dec!(3000)); // 30%
        positions.insert("ETH".to_string(), dec!(4000)); // 40%

        let current = PortfolioState {
            positions,
            total_value: dec!(10000),
            cash: dec!(3000),
        };

        let trades = rebalancer.generate_rebalance_trades(&current, &target);

        // Should have trades for BTC (buy) and ETH (sell)
        assert!(!trades.is_empty());

        let btc_trade = trades.iter().find(|t| t.symbol == "BTC").unwrap();
        assert_eq!(btc_trade.side, Side::Long); // Need to buy more BTC

        let eth_trade = trades.iter().find(|t| t.symbol == "ETH").unwrap();
        assert_eq!(eth_trade.side, Side::Short); // Need to sell ETH
    }

    #[test]
    fn test_correlation_check() {
        let mut rebalancer = PortfolioRebalancer::with_defaults();

        // BTC and ETH highly correlated
        rebalancer.update_correlation("BTC", "ETH", dec!(0.85));
        rebalancer.update_correlation("BTC", "GOLD", dec!(0.2));

        let positions = vec!["BTC".to_string(), "ETH".to_string(), "GOLD".to_string()];
        let check = rebalancer.check_correlation(&positions);

        assert!(!check.passed);
        assert_eq!(check.highly_correlated_pairs.len(), 1);
        assert!(!check.recommendations.is_empty());
    }

    #[test]
    fn test_drift_report() {
        let rebalancer = PortfolioRebalancer::with_defaults();

        let mut target_weights = HashMap::new();
        target_weights.insert("BTC".to_string(), dec!(0.5));
        target_weights.insert("ETH".to_string(), dec!(0.5));

        let target = TargetAllocation {
            weights: target_weights,
            reason: "Test".to_string(),
            generated_at: 0,
        };

        let mut positions = HashMap::new();
        positions.insert("BTC".to_string(), dec!(7000)); // 70%
        positions.insert("ETH".to_string(), dec!(3000)); // 30%

        let current = PortfolioState {
            positions,
            total_value: dec!(10000),
            cash: Decimal::ZERO,
        };

        let drift = rebalancer.calculate_drift(&current, &target);

        assert!(drift.needs_rebalance);
        assert_eq!(drift.drifted_assets.len(), 2);
        assert!(drift.max_drift >= dec!(0.2)); // 20% drift
    }
}
