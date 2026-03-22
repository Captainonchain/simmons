//! MEV Protection Shield
//!
//! Protects transactions from MEV attacks including:
//! - Sandwich attacks
//! - Front-running
//! - Back-running
//!
//! Uses private mempools, transaction batching, and timing strategies.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::Order;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// MEV protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevShieldConfig {
    /// Enable private transaction pools
    pub private_tx_enabled: bool,
    /// Enable decoy transactions
    pub decoys_enabled: bool,
    /// Maximum priority fee (gwei)
    pub max_priority_fee: Decimal,
    /// Minimum delay between related transactions (ms)
    pub min_tx_delay_ms: u64,
    /// Enable transaction batching
    pub batch_enabled: bool,
    /// Private pool endpoints
    pub private_pools: Vec<PrivatePoolConfig>,
}

impl Default for MevShieldConfig {
    fn default() -> Self {
        Self {
            private_tx_enabled: true,
            decoys_enabled: false,
            max_priority_fee: dec!(2),
            min_tx_delay_ms: 100,
            batch_enabled: true,
            private_pools: vec![
                PrivatePoolConfig {
                    name: "flashbots".to_string(),
                    endpoint: "https://relay.flashbots.net".to_string(),
                    chain_ids: vec![1],
                    enabled: true,
                },
                PrivatePoolConfig {
                    name: "xlayer_private".to_string(),
                    endpoint: "https://private.xlayer.tech".to_string(),
                    chain_ids: vec![196],
                    enabled: true,
                },
            ],
        }
    }
}

/// Private pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivatePoolConfig {
    pub name: String,
    pub endpoint: String,
    pub chain_ids: Vec<u64>,
    pub enabled: bool,
}

/// MEV protection shield
pub struct MevShield {
    config: MevShieldConfig,
    pending_batch: Vec<ProtectedOrder>,
    mempool_monitor: MempoolMonitor,
}

impl MevShield {
    pub fn new(config: MevShieldConfig) -> Self {
        Self {
            config,
            pending_batch: Vec::new(),
            mempool_monitor: MempoolMonitor::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(MevShieldConfig::default())
    }

    /// Legacy constructor for compatibility
    pub fn legacy(private_tx: bool) -> LegacyMevShield {
        LegacyMevShield {
            private_tx,
            decoys: false,
            max_priority_fee: dec!(2),
        }
    }

    /// Analyze order for MEV risk
    pub fn analyze_risk(&self, order: &Order, context: Option<&MevContext>) -> MevRiskAnalysis {
        // Size-based risk
        let size_risk = if order.size > dec!(1.0) {
            MevRisk::High
        } else if order.size > dec!(0.1) {
            MevRisk::Medium
        } else {
            MevRisk::Low
        };

        // Order type risk
        let type_risk = match order.order_type {
            simmons_core::OrderType::Market => MevRisk::High,
            simmons_core::OrderType::Limit => MevRisk::Low,
        };

        // Mempool congestion risk
        let mempool_risk = if let Some(ctx) = context {
            if ctx.mempool_tx_count > 10000 {
                MevRisk::High
            } else if ctx.mempool_tx_count > 5000 {
                MevRisk::Medium
            } else {
                MevRisk::Low
            }
        } else {
            MevRisk::Medium
        };

        // Detect sandwich attack potential
        let sandwich_risk = self.detect_sandwich_potential(order, context);

        // Overall risk (worst of all factors)
        let overall = [size_risk, type_risk, mempool_risk, sandwich_risk]
            .into_iter()
            .max_by_key(|r| match r {
                MevRisk::High => 3,
                MevRisk::Medium => 2,
                MevRisk::Low => 1,
            })
            .unwrap_or(MevRisk::Low);

        // Generate recommendations
        let mut recommendations = Vec::new();
        if matches!(overall, MevRisk::High | MevRisk::Medium) {
            if self.config.private_tx_enabled {
                recommendations.push("Use private transaction pool".to_string());
            }
            if order.order_type == simmons_core::OrderType::Market {
                recommendations.push("Consider limit order instead of market".to_string());
            }
            if size_risk == MevRisk::High {
                recommendations.push("Split into smaller orders".to_string());
            }
            if self.config.batch_enabled {
                recommendations.push("Batch with other transactions".to_string());
            }
        }

        // Estimate potential MEV loss
        let estimated_mev_loss = self.estimate_mev_loss(order, &overall);

        MevRiskAnalysis {
            size_risk,
            type_risk,
            mempool_risk,
            sandwich_risk,
            overall,
            recommendations,
            estimated_mev_loss,
        }
    }

    /// Detect potential for sandwich attack
    fn detect_sandwich_potential(
        &self,
        order: &Order,
        context: Option<&MevContext>,
    ) -> MevRisk {
        // Check for indicators of sandwich vulnerability
        let mut risk_score = 0;

        // Large market orders are prime targets
        if order.order_type == simmons_core::OrderType::Market && order.size > dec!(0.5) {
            risk_score += 3;
        }

        // High gas price environment means active searchers
        if let Some(ctx) = context {
            if ctx.gas_price > dec!(50) {
                risk_score += 2;
            }
            if ctx.recent_sandwiches > 0 {
                risk_score += 2;
            }
        }

        // No slippage protection
        if order.limit_price.is_none() {
            risk_score += 1;
        }

        match risk_score {
            0..=2 => MevRisk::Low,
            3..=4 => MevRisk::Medium,
            _ => MevRisk::High,
        }
    }

    /// Estimate potential MEV loss
    fn estimate_mev_loss(&self, order: &Order, risk: &MevRisk) -> Decimal {
        let order_value = order.size * order.limit_price.unwrap_or(dec!(1000));

        // Estimated MEV extraction percentage by risk level
        let mev_pct = match risk {
            MevRisk::High => dec!(0.5),    // 0.5%
            MevRisk::Medium => dec!(0.2),  // 0.2%
            MevRisk::Low => dec!(0.05),    // 0.05%
        };

        order_value * mev_pct / dec!(100)
    }

    /// Wrap order with MEV protection
    pub fn protect(&self, order: &Order) -> ProtectedOrder {
        let analysis = self.analyze_risk(order, None);

        let protection = self.select_protection(&analysis);

        ProtectedOrder {
            order: order.clone(),
            use_private_pool: protection.use_private_pool,
            private_pool_name: protection.pool_name,
            priority_fee: protection.priority_fee,
            max_slippage_bps: protection.max_slippage_bps,
            use_batching: protection.use_batching,
            decoy_count: protection.decoy_count,
            analysis,
        }
    }

    /// Select protection strategy based on risk
    fn select_protection(&self, analysis: &MevRiskAnalysis) -> ProtectionStrategy {
        match analysis.overall {
            MevRisk::High => ProtectionStrategy {
                use_private_pool: self.config.private_tx_enabled,
                pool_name: self.select_private_pool(),
                priority_fee: self.config.max_priority_fee,
                max_slippage_bps: dec!(50),
                use_batching: self.config.batch_enabled,
                decoy_count: if self.config.decoys_enabled { 2 } else { 0 },
            },
            MevRisk::Medium => ProtectionStrategy {
                use_private_pool: self.config.private_tx_enabled,
                pool_name: self.select_private_pool(),
                priority_fee: self.config.max_priority_fee * dec!(0.7),
                max_slippage_bps: dec!(100),
                use_batching: false,
                decoy_count: 0,
            },
            MevRisk::Low => ProtectionStrategy {
                use_private_pool: false,
                pool_name: None,
                priority_fee: self.config.max_priority_fee * dec!(0.5),
                max_slippage_bps: dec!(200),
                use_batching: false,
                decoy_count: 0,
            },
        }
    }

    /// Select appropriate private pool
    fn select_private_pool(&self) -> Option<String> {
        self.config
            .private_pools
            .iter()
            .find(|p| p.enabled)
            .map(|p| p.name.clone())
    }

    /// Create a transaction bundle for atomic execution
    pub fn create_bundle(&self, orders: Vec<&Order>) -> TransactionBundle {
        let protected: Vec<ProtectedOrder> = orders
            .iter()
            .map(|o| self.protect(o))
            .collect();

        let total_value: Decimal = protected
            .iter()
            .map(|p| p.order.size * p.order.limit_price.unwrap_or(Decimal::ZERO))
            .sum();

        let overall_risk = protected
            .iter()
            .map(|p| &p.analysis.overall)
            .max_by_key(|r| match r {
                MevRisk::High => 3,
                MevRisk::Medium => 2,
                MevRisk::Low => 1,
            })
            .cloned()
            .unwrap_or(MevRisk::Low);

        TransactionBundle {
            transactions: protected,
            total_value,
            overall_risk,
            use_private_pool: self.config.private_tx_enabled,
            pool_name: self.select_private_pool(),
            atomic: true,
        }
    }

    /// Submit to private mempool
    pub async fn submit_private(&self, bundle: &TransactionBundle) -> Result<PrivateSubmission, String> {
        let pool_name = bundle.pool_name.as_ref()
            .ok_or("No private pool configured")?;

        let pool = self.config.private_pools
            .iter()
            .find(|p| &p.name == pool_name)
            .ok_or("Private pool not found")?;

        // In real implementation, this would make HTTP request to the relay
        // For now, return a mock submission result

        info!(
            "Submitting bundle of {} txs to {} private pool",
            bundle.transactions.len(),
            pool_name
        );

        Ok(PrivateSubmission {
            bundle_hash: format!("0x{}", "a".repeat(64)),
            pool_name: pool_name.clone(),
            submitted_at: chrono::Utc::now().timestamp(),
            estimated_inclusion_block: 0,
            status: SubmissionStatus::Pending,
        })
    }

    /// Monitor mempool for MEV activity
    pub fn update_mempool_context(&mut self, context: MevContext) {
        self.mempool_monitor.update(context);
    }
}

/// Protection strategy selection
struct ProtectionStrategy {
    use_private_pool: bool,
    pool_name: Option<String>,
    priority_fee: Decimal,
    max_slippage_bps: Decimal,
    use_batching: bool,
    decoy_count: u8,
}

/// MEV risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MevRisk {
    Low,
    Medium,
    High,
}

/// MEV risk analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevRiskAnalysis {
    pub size_risk: MevRisk,
    pub type_risk: MevRisk,
    pub mempool_risk: MevRisk,
    pub sandwich_risk: MevRisk,
    pub overall: MevRisk,
    pub recommendations: Vec<String>,
    pub estimated_mev_loss: Decimal,
}

/// Order wrapped with MEV protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedOrder {
    pub order: Order,
    pub use_private_pool: bool,
    pub private_pool_name: Option<String>,
    pub priority_fee: Decimal,
    pub max_slippage_bps: Decimal,
    pub use_batching: bool,
    pub decoy_count: u8,
    pub analysis: MevRiskAnalysis,
}

/// Transaction bundle for atomic execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionBundle {
    pub transactions: Vec<ProtectedOrder>,
    pub total_value: Decimal,
    pub overall_risk: MevRisk,
    pub use_private_pool: bool,
    pub pool_name: Option<String>,
    pub atomic: bool,
}

/// Context for MEV analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MevContext {
    pub mempool_tx_count: usize,
    pub gas_price: Decimal,
    pub recent_sandwiches: u32,
    pub block_utilization: Decimal,
    pub searcher_activity: SearcherActivity,
}

/// Searcher activity levels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearcherActivity {
    #[default]
    Low,
    Normal,
    High,
    Aggressive,
}

/// Private submission result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateSubmission {
    pub bundle_hash: String,
    pub pool_name: String,
    pub submitted_at: i64,
    pub estimated_inclusion_block: u64,
    pub status: SubmissionStatus,
}

/// Submission status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    Pending,
    Included,
    Failed,
    Expired,
}

/// Mempool monitor for MEV detection
struct MempoolMonitor {
    recent_contexts: Vec<MevContext>,
    sandwich_detections: u32,
}

impl MempoolMonitor {
    fn new() -> Self {
        Self {
            recent_contexts: Vec::new(),
            sandwich_detections: 0,
        }
    }

    fn update(&mut self, context: MevContext) {
        self.recent_contexts.push(context);
        if self.recent_contexts.len() > 100 {
            self.recent_contexts.remove(0);
        }
    }
}

// Legacy support for existing code
pub struct LegacyMevShield {
    pub private_tx: bool,
    pub decoys: bool,
    pub max_priority_fee: Decimal,
}

impl Default for LegacyMevShield {
    fn default() -> Self {
        Self {
            private_tx: true,
            decoys: false,
            max_priority_fee: dec!(2),
        }
    }
}

impl LegacyMevShield {
    pub fn new(private_tx: bool) -> Self {
        Self {
            private_tx,
            ..Default::default()
        }
    }

    pub fn analyze_risk(&self, order: &Order) -> LegacyMevRiskAnalysis {
        let size_risk = if order.size > dec!(1.0) {
            MevRisk::High
        } else if order.size > dec!(0.1) {
            MevRisk::Medium
        } else {
            MevRisk::Low
        };

        let type_risk = match order.order_type {
            simmons_core::OrderType::Market => MevRisk::High,
            simmons_core::OrderType::Limit => MevRisk::Low,
        };

        let overall = match (&size_risk, &type_risk) {
            (MevRisk::High, _) | (_, MevRisk::High) => MevRisk::High,
            (MevRisk::Medium, _) | (_, MevRisk::Medium) => MevRisk::Medium,
            _ => MevRisk::Low,
        };

        let recommendations = match overall {
            MevRisk::High => vec![
                "Use private transaction pool".to_string(),
                "Consider splitting into smaller orders".to_string(),
                "Use limit order instead of market".to_string(),
            ],
            MevRisk::Medium => vec![
                "Consider private transaction pool".to_string(),
                "Set tight slippage tolerance".to_string(),
            ],
            MevRisk::Low => vec![],
        };

        LegacyMevRiskAnalysis {
            size_risk,
            type_risk,
            overall,
            recommendations,
        }
    }

    pub fn protect(&self, order: &Order) -> LegacyProtectedOrder {
        let analysis = self.analyze_risk(order);

        LegacyProtectedOrder {
            order: order.clone(),
            use_private_pool: self.private_tx && matches!(analysis.overall, MevRisk::High | MevRisk::Medium),
            priority_fee: self.max_priority_fee,
            analysis,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LegacyMevRiskAnalysis {
    pub size_risk: MevRisk,
    pub type_risk: MevRisk,
    pub overall: MevRisk,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LegacyProtectedOrder {
    pub order: Order,
    pub use_private_pool: bool,
    pub priority_fee: Decimal,
    pub analysis: LegacyMevRiskAnalysis,
}

#[cfg(test)]
mod tests {
    use super::*;
    use simmons_core::{OrderType, Side};

    fn test_order(size: Decimal, order_type: OrderType) -> Order {
        Order {
            symbol: "BTC-USDT".to_string(),
            side: Side::Long,
            size,
            order_type,
            limit_price: Some(dec!(67000)),
            stop_loss: None,
            take_profit: None,
        }
    }

    #[test]
    fn test_analyze_small_limit_order() {
        let shield = MevShield::with_defaults();
        let order = test_order(dec!(0.01), OrderType::Limit);

        // Provide low-congestion context to avoid Medium default
        let context = MevContext {
            mempool_tx_count: 1000,
            gas_price: dec!(20),
            recent_sandwiches: 0,
            block_utilization: dec!(0.5),
            searcher_activity: SearcherActivity::Low,
        };

        let analysis = shield.analyze_risk(&order, Some(&context));
        assert_eq!(analysis.overall, MevRisk::Low);
        assert!(analysis.recommendations.is_empty());
    }

    #[test]
    fn test_analyze_large_market_order() {
        let shield = MevShield::with_defaults();
        let order = test_order(dec!(2.0), OrderType::Market);

        let analysis = shield.analyze_risk(&order, None);
        assert_eq!(analysis.overall, MevRisk::High);
        assert!(!analysis.recommendations.is_empty());
    }

    #[test]
    fn test_mev_loss_estimate() {
        let shield = MevShield::with_defaults();
        let order = test_order(dec!(1.0), OrderType::Market);

        let analysis = shield.analyze_risk(&order, None);
        assert!(analysis.estimated_mev_loss > Decimal::ZERO);
    }

    #[test]
    fn test_protect_selects_private_pool() {
        let shield = MevShield::with_defaults();
        let order = test_order(dec!(2.0), OrderType::Market);

        let protected = shield.protect(&order);
        assert!(protected.use_private_pool);
        assert!(protected.private_pool_name.is_some());
    }

    #[test]
    fn test_bundle_creation() {
        let shield = MevShield::with_defaults();
        let order1 = test_order(dec!(1.0), OrderType::Market);
        let order2 = test_order(dec!(0.5), OrderType::Limit);

        let bundle = shield.create_bundle(vec![&order1, &order2]);
        assert_eq!(bundle.transactions.len(), 2);
        assert!(bundle.atomic);
    }

    #[test]
    fn test_sandwich_detection() {
        let shield = MevShield::with_defaults();
        let order = test_order(dec!(1.0), OrderType::Market);

        let context = MevContext {
            mempool_tx_count: 15000,
            gas_price: dec!(100),
            recent_sandwiches: 5,
            ..Default::default()
        };

        let analysis = shield.analyze_risk(&order, Some(&context));
        assert_eq!(analysis.sandwich_risk, MevRisk::High);
    }

    // Legacy compatibility tests
    #[test]
    fn test_legacy_mev_analysis() {
        let shield = LegacyMevShield::default();
        let order = test_order(dec!(0.01), OrderType::Limit);

        let analysis = shield.analyze_risk(&order);
        assert_eq!(analysis.overall, MevRisk::Low);
    }

    #[test]
    fn test_legacy_protect() {
        let shield = LegacyMevShield::new(true);
        let order = test_order(dec!(2.0), OrderType::Market);

        let protected = shield.protect(&order);
        assert!(protected.use_private_pool);
    }
}
