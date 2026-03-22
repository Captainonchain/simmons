//! CeDeFi Arbitrage Router
//!
//! Routes arbitrage opportunities between OKX CEX and Cod3x DeFi on X Layer.
//! Handles spread detection, route optimization, and execution coordination.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Arbitrage route type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArbRouteType {
    /// Buy on OKX CEX, sell on Cod3x DEX
    CexToDex,
    /// Buy on Cod3x DEX, sell on OKX CEX
    DexToCex,
    /// Flash loan arbitrage (DeFi only)
    FlashLoan,
}

/// Arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeDefiOpportunity {
    pub symbol: String,
    pub route_type: ArbRouteType,
    pub cex_price: Decimal,
    pub dex_price: Decimal,
    pub spread_bps: Decimal,
    pub gross_profit_usd: Decimal,
    pub estimated_costs: ArbCosts,
    pub net_profit_usd: Decimal,
    pub detected_at: i64,
    pub confidence: Decimal,
}

impl CeDefiOpportunity {
    /// Check if opportunity is still profitable after all costs
    pub fn is_profitable(&self) -> bool {
        self.net_profit_usd > Decimal::ZERO
    }

    /// Risk-adjusted return (Sharpe-like)
    pub fn risk_adjusted_return(&self, volatility: Decimal) -> Decimal {
        if volatility.is_zero() {
            return Decimal::ZERO;
        }
        self.net_profit_usd / volatility
    }
}

/// Arbitrage costs breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbCosts {
    pub cex_fee_bps: Decimal,
    pub dex_fee_bps: Decimal,
    pub gas_cost_usd: Decimal,
    pub bridge_fee_usd: Decimal,
    pub slippage_bps: Decimal,
    pub total_bps: Decimal,
    pub total_usd: Decimal,
}

impl ArbCosts {
    pub fn new(
        cex_fee_bps: Decimal,
        dex_fee_bps: Decimal,
        gas_cost_usd: Decimal,
        bridge_fee_usd: Decimal,
        slippage_bps: Decimal,
        trade_size_usd: Decimal,
    ) -> Self {
        let fee_bps_total = cex_fee_bps + dex_fee_bps + slippage_bps;
        let fee_usd = trade_size_usd * fee_bps_total / dec!(10000);
        let total_usd = fee_usd + gas_cost_usd + bridge_fee_usd;
        let total_bps = if trade_size_usd.is_zero() {
            Decimal::ZERO
        } else {
            (total_usd / trade_size_usd) * dec!(10000)
        };

        Self {
            cex_fee_bps,
            dex_fee_bps,
            gas_cost_usd,
            bridge_fee_usd,
            slippage_bps,
            total_bps,
            total_usd,
        }
    }
}

/// Execution plan for an arbitrage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbExecutionPlan {
    pub opportunity: CeDefiOpportunity,
    pub steps: Vec<ArbStep>,
    pub total_time_estimate_secs: u64,
    pub requires_bridge: bool,
    pub capital_required: Decimal,
}

/// Single step in arbitrage execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbStep {
    pub order: u8,
    pub action: ArbAction,
    pub venue: String,
    pub token: String,
    pub amount: Decimal,
    pub expected_price: Decimal,
    pub timeout_secs: u64,
}

/// Arbitrage action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArbAction {
    Buy,
    Sell,
    Bridge,
    Deposit,
    Withdraw,
    Borrow,
    Repay,
}

/// CeDeFi arbitrage router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeDefiArbConfig {
    /// Minimum spread to consider (bps)
    pub min_spread_bps: Decimal,
    /// Maximum position size (USD)
    pub max_position_usd: Decimal,
    /// CEX trading fee (bps)
    pub cex_fee_bps: Decimal,
    /// DEX swap fee (bps)
    pub dex_fee_bps: Decimal,
    /// Estimated gas cost (USD)
    pub gas_estimate_usd: Decimal,
    /// Bridge fee (% of amount)
    pub bridge_fee_pct: Decimal,
    /// Maximum slippage allowed (bps)
    pub max_slippage_bps: Decimal,
    /// Stale price threshold (seconds)
    pub stale_price_secs: u64,
    /// Enable flash loans for DeFi leg
    pub enable_flash_loans: bool,
}

impl Default for CeDefiArbConfig {
    fn default() -> Self {
        Self {
            min_spread_bps: dec!(30),        // 30 bps minimum
            max_position_usd: dec!(10000),   // $10k max
            cex_fee_bps: dec!(10),           // 0.1% OKX fee (VIP tier)
            dex_fee_bps: dec!(30),           // 0.3% Uniswap-style fee
            gas_estimate_usd: dec!(1),       // ~$1 gas on X Layer
            bridge_fee_pct: dec!(0.1),       // 0.1% bridge fee
            max_slippage_bps: dec!(20),      // 20 bps max slippage
            stale_price_secs: 10,            // 10s staleness threshold
            enable_flash_loans: true,
        }
    }
}

/// CeDeFi arbitrage router
pub struct CeDefiArbRouter {
    config: CeDefiArbConfig,
    active_opportunities: Vec<CeDefiOpportunity>,
    last_scan_at: Option<Instant>,
}

impl CeDefiArbRouter {
    pub fn new(config: CeDefiArbConfig) -> Self {
        Self {
            config,
            active_opportunities: Vec::new(),
            last_scan_at: None,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(CeDefiArbConfig::default())
    }

    /// Find arbitrage opportunities between CEX and DEX prices
    pub fn find_opportunities(
        &mut self,
        symbol: &str,
        cex_price: Decimal,
        dex_price: Decimal,
        cex_timestamp: i64,
        dex_timestamp: i64,
        trade_size_usd: Decimal,
    ) -> Vec<CeDefiOpportunity> {
        self.last_scan_at = Some(Instant::now());
        let mut opportunities = Vec::new();

        // Validate prices
        if cex_price.is_zero() || dex_price.is_zero() {
            return opportunities;
        }

        // Check for staleness
        let now = chrono::Utc::now().timestamp();
        let cex_age = (now - cex_timestamp).unsigned_abs();
        let dex_age = (now - dex_timestamp).unsigned_abs();

        if cex_age > self.config.stale_price_secs || dex_age > self.config.stale_price_secs {
            debug!("Skipping stale prices: CEX {}s, DEX {}s", cex_age, dex_age);
            return opportunities;
        }

        // Calculate spread
        let spread_bps = ((dex_price - cex_price).abs() / cex_price) * dec!(10000);

        if spread_bps < self.config.min_spread_bps {
            return opportunities;
        }

        // Cap trade size
        let effective_size = trade_size_usd.min(self.config.max_position_usd);

        // Determine route direction
        let route_type = if cex_price < dex_price {
            ArbRouteType::CexToDex
        } else {
            ArbRouteType::DexToCex
        };

        // Calculate costs
        let needs_bridge = false; // Assume capital already on both sides
        let bridge_fee = if needs_bridge {
            effective_size * self.config.bridge_fee_pct / dec!(100)
        } else {
            Decimal::ZERO
        };

        let costs = ArbCosts::new(
            self.config.cex_fee_bps,
            self.config.dex_fee_bps,
            self.config.gas_estimate_usd,
            bridge_fee,
            self.config.max_slippage_bps / dec!(2), // Use half max as estimate
            effective_size,
        );

        // Calculate profit
        let gross_profit = effective_size * spread_bps / dec!(10000);
        let net_profit = gross_profit - costs.total_usd;

        // Confidence based on spread magnitude and price freshness
        let spread_confidence = (spread_bps / dec!(100)).min(Decimal::ONE);
        let freshness_confidence = Decimal::ONE
            - Decimal::from(cex_age.max(dex_age) as i64) / Decimal::from(self.config.stale_price_secs as i64);
        let confidence = (spread_confidence + freshness_confidence) / dec!(2);

        let opp = CeDefiOpportunity {
            symbol: symbol.to_string(),
            route_type,
            cex_price,
            dex_price,
            spread_bps,
            gross_profit_usd: gross_profit,
            estimated_costs: costs,
            net_profit_usd: net_profit,
            detected_at: now,
            confidence: confidence.max(Decimal::ZERO),
        };

        if opp.is_profitable() {
            opportunities.push(opp);
        }

        // Update active opportunities
        self.active_opportunities = opportunities.clone();

        opportunities
    }

    /// Create execution plan for an opportunity
    pub fn create_execution_plan(&self, opp: &CeDefiOpportunity) -> ArbExecutionPlan {
        let capital_required = opp.net_profit_usd / opp.spread_bps * dec!(10000);

        let steps = match opp.route_type {
            ArbRouteType::CexToDex => vec![
                // Buy on CEX
                ArbStep {
                    order: 1,
                    action: ArbAction::Buy,
                    venue: "okx".to_string(),
                    token: opp.symbol.clone(),
                    amount: capital_required,
                    expected_price: opp.cex_price,
                    timeout_secs: 30,
                },
                // Sell on DEX
                ArbStep {
                    order: 2,
                    action: ArbAction::Sell,
                    venue: "cod3x_dex".to_string(),
                    token: opp.symbol.clone(),
                    amount: capital_required,
                    expected_price: opp.dex_price,
                    timeout_secs: 60,
                },
            ],
            ArbRouteType::DexToCex => vec![
                // Buy on DEX
                ArbStep {
                    order: 1,
                    action: ArbAction::Buy,
                    venue: "cod3x_dex".to_string(),
                    token: opp.symbol.clone(),
                    amount: capital_required,
                    expected_price: opp.dex_price,
                    timeout_secs: 60,
                },
                // Sell on CEX
                ArbStep {
                    order: 2,
                    action: ArbAction::Sell,
                    venue: "okx".to_string(),
                    token: opp.symbol.clone(),
                    amount: capital_required,
                    expected_price: opp.cex_price,
                    timeout_secs: 30,
                },
            ],
            ArbRouteType::FlashLoan => {
                // Flash loan arb - all on-chain
                vec![
                    ArbStep {
                        order: 1,
                        action: ArbAction::Borrow,
                        venue: "cod3x_lending".to_string(),
                        token: "USDT".to_string(),
                        amount: capital_required,
                        expected_price: Decimal::ONE,
                        timeout_secs: 60,
                    },
                    ArbStep {
                        order: 2,
                        action: ArbAction::Buy,
                        venue: "cod3x_dex".to_string(),
                        token: opp.symbol.clone(),
                        amount: capital_required,
                        expected_price: opp.dex_price,
                        timeout_secs: 60,
                    },
                    ArbStep {
                        order: 3,
                        action: ArbAction::Sell,
                        venue: "external_dex".to_string(),
                        token: opp.symbol.clone(),
                        amount: capital_required,
                        expected_price: opp.cex_price, // Higher price
                        timeout_secs: 60,
                    },
                    ArbStep {
                        order: 4,
                        action: ArbAction::Repay,
                        venue: "cod3x_lending".to_string(),
                        token: "USDT".to_string(),
                        amount: capital_required,
                        expected_price: Decimal::ONE,
                        timeout_secs: 60,
                    },
                ]
            }
        };

        let total_time = steps.iter().map(|s| s.timeout_secs).sum();

        ArbExecutionPlan {
            opportunity: opp.clone(),
            steps,
            total_time_estimate_secs: total_time,
            requires_bridge: false,
            capital_required,
        }
    }

    /// Check if a Cod3x lending position can be used for leverage
    pub fn find_leveraged_arb(
        &self,
        symbol: &str,
        cex_price: Decimal,
        dex_price: Decimal,
        available_collateral_usd: Decimal,
        max_ltv: Decimal,
    ) -> Option<LeveragedArbOpportunity> {
        let spread_bps = ((dex_price - cex_price).abs() / cex_price) * dec!(10000);

        if spread_bps < self.config.min_spread_bps {
            return None;
        }

        // Calculate max leverage
        let max_borrow = available_collateral_usd * max_ltv;
        let total_position = available_collateral_usd + max_borrow;
        let leverage = total_position / available_collateral_usd;

        // Borrow costs (assume 5% APR, pro-rated to execution time ~1 min)
        let borrow_cost_usd = max_borrow * dec!(0.05) / dec!(525600); // Per minute

        // Calculate leveraged profit
        let gross_profit = total_position * spread_bps / dec!(10000);
        let total_costs = self.config.gas_estimate_usd * dec!(2) + borrow_cost_usd;
        let net_profit = gross_profit - total_costs;

        if net_profit <= Decimal::ZERO {
            return None;
        }

        Some(LeveragedArbOpportunity {
            symbol: symbol.to_string(),
            collateral_usd: available_collateral_usd,
            borrowed_usd: max_borrow,
            total_position_usd: total_position,
            leverage,
            spread_bps,
            gross_profit_usd: gross_profit,
            net_profit_usd: net_profit,
            roi_pct: (net_profit / available_collateral_usd) * dec!(100),
        })
    }

    /// Get active opportunities
    pub fn active_opportunities(&self) -> &[CeDefiOpportunity] {
        &self.active_opportunities
    }

    /// Get best opportunity by net profit
    pub fn best_opportunity(&self) -> Option<&CeDefiOpportunity> {
        self.active_opportunities
            .iter()
            .max_by(|a, b| a.net_profit_usd.cmp(&b.net_profit_usd))
    }

    /// Clear stale opportunities
    pub fn clear_stale(&mut self, max_age_secs: i64) {
        let now = chrono::Utc::now().timestamp();
        self.active_opportunities
            .retain(|opp| now - opp.detected_at < max_age_secs);
    }
}

/// Leveraged arbitrage opportunity using Cod3x lending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeveragedArbOpportunity {
    pub symbol: String,
    pub collateral_usd: Decimal,
    pub borrowed_usd: Decimal,
    pub total_position_usd: Decimal,
    pub leverage: Decimal,
    pub spread_bps: Decimal,
    pub gross_profit_usd: Decimal,
    pub net_profit_usd: Decimal,
    pub roi_pct: Decimal,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbExecutionResult {
    pub success: bool,
    pub steps_completed: usize,
    pub total_steps: usize,
    pub actual_profit_usd: Decimal,
    pub execution_time_secs: u64,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_cex_to_dex_opportunity() {
        let mut router = CeDefiArbRouter::with_defaults();
        let now = chrono::Utc::now().timestamp();

        // CEX cheaper than DEX - buy CEX, sell DEX
        let opps = router.find_opportunities(
            "ETH-USDT",
            dec!(3000),  // CEX price
            dec!(3030),  // DEX price - 100 bps higher
            now,
            now,
            dec!(1000),  // $1000 trade
        );

        assert!(!opps.is_empty());
        let opp = &opps[0];
        assert_eq!(opp.route_type, ArbRouteType::CexToDex);
        assert!(opp.spread_bps > dec!(90)); // ~100 bps
        assert!(opp.is_profitable());
    }

    #[test]
    fn test_find_dex_to_cex_opportunity() {
        let mut router = CeDefiArbRouter::with_defaults();
        let now = chrono::Utc::now().timestamp();

        // DEX cheaper than CEX - buy DEX, sell CEX
        let opps = router.find_opportunities(
            "ETH-USDT",
            dec!(3030),  // CEX price
            dec!(3000),  // DEX price - 100 bps lower
            now,
            now,
            dec!(1000),
        );

        assert!(!opps.is_empty());
        assert_eq!(opps[0].route_type, ArbRouteType::DexToCex);
    }

    #[test]
    fn test_no_opportunity_small_spread() {
        let mut router = CeDefiArbRouter::with_defaults();
        let now = chrono::Utc::now().timestamp();

        // Only 10 bps spread - below minimum
        let opps = router.find_opportunities(
            "ETH-USDT",
            dec!(3000),
            dec!(3003),  // Only 10 bps
            now,
            now,
            dec!(1000),
        );

        assert!(opps.is_empty());
    }

    #[test]
    fn test_stale_prices_rejected() {
        let mut router = CeDefiArbRouter::with_defaults();
        let now = chrono::Utc::now().timestamp();

        // Old CEX price
        let opps = router.find_opportunities(
            "ETH-USDT",
            dec!(3000),
            dec!(3050),
            now - 60, // 60 seconds old
            now,
            dec!(1000),
        );

        assert!(opps.is_empty());
    }

    #[test]
    fn test_execution_plan() {
        let router = CeDefiArbRouter::with_defaults();

        let opp = CeDefiOpportunity {
            symbol: "ETH-USDT".to_string(),
            route_type: ArbRouteType::CexToDex,
            cex_price: dec!(3000),
            dex_price: dec!(3050),
            spread_bps: dec!(166),
            gross_profit_usd: dec!(16.6),
            estimated_costs: ArbCosts::new(
                dec!(10), dec!(30), dec!(1), Decimal::ZERO, dec!(10), dec!(1000),
            ),
            net_profit_usd: dec!(10),
            detected_at: 0,
            confidence: dec!(0.8),
        };

        let plan = router.create_execution_plan(&opp);
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].venue, "okx");
        assert_eq!(plan.steps[1].venue, "cod3x_dex");
    }

    #[test]
    fn test_leveraged_arb() {
        let router = CeDefiArbRouter::with_defaults();

        let leveraged = router.find_leveraged_arb(
            "ETH-USDT",
            dec!(3000),
            dec!(3050),  // ~166 bps spread
            dec!(1000),  // $1000 collateral
            dec!(0.75),  // 75% LTV
        );

        assert!(leveraged.is_some());
        let lev = leveraged.unwrap();
        assert!(lev.leverage > Decimal::ONE);
        assert!(lev.net_profit_usd > Decimal::ZERO);
    }

    #[test]
    fn test_arb_costs() {
        let costs = ArbCosts::new(
            dec!(10),     // 10 bps CEX fee
            dec!(30),     // 30 bps DEX fee
            dec!(1),      // $1 gas
            dec!(5),      // $5 bridge
            dec!(10),     // 10 bps slippage
            dec!(1000),   // $1000 trade
        );

        // Fee bps: 10 + 30 + 10 = 50 bps = $5
        // Fixed costs: $1 + $5 = $6
        // Total: $11
        assert!(costs.total_usd > dec!(10));
        assert!(costs.total_usd < dec!(12));
    }
}
