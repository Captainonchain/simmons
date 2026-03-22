//! Cod3x Lending Executor
//!
//! Executes lending operations on Cod3x protocol:
//! - Deposit/withdraw collateral
//! - Borrow/repay loans
//! - Monitor health factor
//! - Execute liquidations

use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Lending operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LendingOperation {
    Deposit,
    Withdraw,
    Borrow,
    Repay,
    Liquidate,
}

/// Interest rate mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterestRateMode {
    Stable,
    Variable,
}

impl InterestRateMode {
    pub fn to_u8(&self) -> u8 {
        match self {
            InterestRateMode::Stable => 1,
            InterestRateMode::Variable => 2,
        }
    }
}

/// Lending execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LendingRequest {
    pub operation: LendingOperation,
    pub asset: String,
    pub amount: Decimal,
    pub interest_rate_mode: Option<InterestRateMode>,
    pub on_behalf_of: Option<String>,
    pub max_slippage_bps: Decimal,
    pub deadline_secs: u64,
}

/// Lending execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LendingResult {
    pub request: LendingRequest,
    pub success: bool,
    pub tx_hash: Option<String>,
    pub actual_amount: Decimal,
    pub gas_used: u64,
    pub gas_price_gwei: Decimal,
    pub execution_time_ms: u64,
    pub new_health_factor: Option<Decimal>,
    pub error: Option<String>,
}

/// Liquidation target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationTarget {
    pub user: String,
    pub collateral_asset: String,
    pub debt_asset: String,
    pub health_factor: Decimal,
    pub max_liquidatable_debt: Decimal,
    pub liquidation_bonus_bps: u32,
    pub estimated_profit_usd: Decimal,
}

/// Cod3x lending executor
pub struct Cod3xExecutor {
    config: Cod3xExecutorConfig,
    pending_operations: Vec<PendingOperation>,
}

/// Executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cod3xExecutorConfig {
    /// Minimum health factor to maintain
    pub min_health_factor: Decimal,
    /// Target health factor after operations
    pub target_health_factor: Decimal,
    /// Maximum gas price (gwei) to accept
    pub max_gas_gwei: Decimal,
    /// Minimum profit for liquidations (USD)
    pub min_liquidation_profit_usd: Decimal,
    /// Enable auto-repay on low health factor
    pub auto_repay_enabled: bool,
    /// Default interest rate mode
    pub default_rate_mode: InterestRateMode,
}

impl Default for Cod3xExecutorConfig {
    fn default() -> Self {
        Self {
            min_health_factor: dec!(1.2),
            target_health_factor: dec!(1.5),
            max_gas_gwei: dec!(50),
            min_liquidation_profit_usd: dec!(10),
            auto_repay_enabled: true,
            default_rate_mode: InterestRateMode::Variable,
        }
    }
}

/// Pending operation tracking
#[derive(Debug, Clone)]
struct PendingOperation {
    request: LendingRequest,
    submitted_at: Instant,
    tx_hash: Option<String>,
}

impl Cod3xExecutor {
    pub fn new(config: Cod3xExecutorConfig) -> Self {
        Self {
            config,
            pending_operations: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(Cod3xExecutorConfig::default())
    }

    /// Validate a lending request before execution
    pub fn validate_request(
        &self,
        request: &LendingRequest,
        current_health_factor: Decimal,
        available_balance: Decimal,
    ) -> Result<()> {
        // Check amount
        if request.amount <= Decimal::ZERO {
            return Err(anyhow!("Amount must be positive"));
        }

        // Check deadline
        if request.deadline_secs == 0 {
            return Err(anyhow!("Deadline must be set"));
        }

        // Operation-specific validation
        match request.operation {
            LendingOperation::Deposit => {
                if request.amount > available_balance {
                    return Err(anyhow!(
                        "Insufficient balance: {} > {}",
                        request.amount,
                        available_balance
                    ));
                }
            }
            LendingOperation::Withdraw => {
                // Check if withdrawal would drop health factor too low
                // This is a simplified check - real implementation would calculate impact
                if current_health_factor < self.config.min_health_factor + dec!(0.2) {
                    return Err(anyhow!(
                        "Health factor too low for withdrawal: {}",
                        current_health_factor
                    ));
                }
            }
            LendingOperation::Borrow => {
                // Check if borrow would drop health factor too low
                if current_health_factor < self.config.target_health_factor {
                    return Err(anyhow!(
                        "Health factor would drop below target: {} < {}",
                        current_health_factor,
                        self.config.target_health_factor
                    ));
                }
            }
            LendingOperation::Repay => {
                // Repay always allowed if you have the funds
                if request.amount > available_balance {
                    return Err(anyhow!("Insufficient balance for repayment"));
                }
            }
            LendingOperation::Liquidate => {
                // Liquidation validation is separate
            }
        }

        Ok(())
    }

    /// Calculate safe borrow amount given health factor targets
    pub fn calculate_safe_borrow(
        &self,
        current_collateral_usd: Decimal,
        current_debt_usd: Decimal,
        collateral_ltv: Decimal,
        target_hf: Option<Decimal>,
    ) -> Decimal {
        let target = target_hf.unwrap_or(self.config.target_health_factor);

        if current_collateral_usd.is_zero() {
            return Decimal::ZERO;
        }

        // health_factor = (collateral * ltv) / debt
        // target_hf = (collateral * ltv) / (debt + new_borrow)
        // new_borrow = (collateral * ltv / target_hf) - debt

        let max_total_debt = (current_collateral_usd * collateral_ltv) / target;
        let safe_borrow = max_total_debt - current_debt_usd;

        safe_borrow.max(Decimal::ZERO)
    }

    /// Calculate repayment needed to restore health factor
    pub fn calculate_required_repayment(
        &self,
        current_collateral_usd: Decimal,
        current_debt_usd: Decimal,
        collateral_ltv: Decimal,
        current_hf: Decimal,
        target_hf: Option<Decimal>,
    ) -> Decimal {
        let target = target_hf.unwrap_or(self.config.target_health_factor);

        if current_hf >= target {
            return Decimal::ZERO;
        }

        // target_hf = (collateral * ltv) / (debt - repayment)
        // repayment = debt - (collateral * ltv / target_hf)

        let max_debt_for_target = (current_collateral_usd * collateral_ltv) / target;
        let required_repayment = current_debt_usd - max_debt_for_target;

        required_repayment.max(Decimal::ZERO)
    }

    /// Build execution plan for a lending strategy
    pub fn build_strategy_plan(
        &self,
        strategy: LendingStrategy,
        capital: Decimal,
        current_position: Option<&PositionSnapshot>,
    ) -> Vec<LendingRequest> {
        let mut requests = Vec::new();

        match strategy {
            LendingStrategy::DepositOnly { asset } => {
                requests.push(LendingRequest {
                    operation: LendingOperation::Deposit,
                    asset,
                    amount: capital,
                    interest_rate_mode: None,
                    on_behalf_of: None,
                    max_slippage_bps: dec!(50),
                    deadline_secs: 300,
                });
            }
            LendingStrategy::LeveragedLong {
                collateral_asset,
                borrow_asset,
                target_leverage,
            } => {
                // Step 1: Deposit collateral
                requests.push(LendingRequest {
                    operation: LendingOperation::Deposit,
                    asset: collateral_asset.clone(),
                    amount: capital,
                    interest_rate_mode: None,
                    on_behalf_of: None,
                    max_slippage_bps: dec!(50),
                    deadline_secs: 300,
                });

                // Step 2: Borrow based on leverage target
                // leverage = total_exposure / capital
                // borrow = capital * (leverage - 1)
                let borrow_amount = capital * (target_leverage - Decimal::ONE);
                requests.push(LendingRequest {
                    operation: LendingOperation::Borrow,
                    asset: borrow_asset,
                    amount: borrow_amount,
                    interest_rate_mode: Some(self.config.default_rate_mode),
                    on_behalf_of: None,
                    max_slippage_bps: dec!(50),
                    deadline_secs: 300,
                });
            }
            LendingStrategy::DeltaNeutral {
                long_asset,
                short_asset,
                size,
            } => {
                // Deposit long asset as collateral
                requests.push(LendingRequest {
                    operation: LendingOperation::Deposit,
                    asset: long_asset,
                    amount: size,
                    interest_rate_mode: None,
                    on_behalf_of: None,
                    max_slippage_bps: dec!(50),
                    deadline_secs: 300,
                });

                // Borrow short asset (would sell externally)
                requests.push(LendingRequest {
                    operation: LendingOperation::Borrow,
                    asset: short_asset,
                    amount: size * dec!(0.7), // Conservative 70% LTV
                    interest_rate_mode: Some(InterestRateMode::Variable),
                    on_behalf_of: None,
                    max_slippage_bps: dec!(50),
                    deadline_secs: 300,
                });
            }
            LendingStrategy::Unwind => {
                if let Some(pos) = current_position {
                    // Step 1: Repay all debt
                    for debt in &pos.debts {
                        requests.push(LendingRequest {
                            operation: LendingOperation::Repay,
                            asset: debt.asset.clone(),
                            amount: debt.amount,
                            interest_rate_mode: Some(debt.rate_mode),
                            on_behalf_of: None,
                            max_slippage_bps: dec!(100),
                            deadline_secs: 300,
                        });
                    }

                    // Step 2: Withdraw all collateral
                    for collateral in &pos.collaterals {
                        requests.push(LendingRequest {
                            operation: LendingOperation::Withdraw,
                            asset: collateral.asset.clone(),
                            amount: collateral.amount,
                            interest_rate_mode: None,
                            on_behalf_of: None,
                            max_slippage_bps: dec!(100),
                            deadline_secs: 300,
                        });
                    }
                }
            }
        }

        requests
    }

    /// Find profitable liquidation targets
    pub fn find_liquidation_targets(
        &self,
        unhealthy_positions: &[UnhealthyPosition],
        min_profit_usd: Option<Decimal>,
    ) -> Vec<LiquidationTarget> {
        let min_profit = min_profit_usd.unwrap_or(self.config.min_liquidation_profit_usd);

        unhealthy_positions
            .iter()
            .filter_map(|pos| {
                if pos.health_factor >= Decimal::ONE {
                    return None;
                }

                // Calculate potential profit
                // Liquidator can liquidate up to 50% of debt
                let max_liquidatable = pos.total_debt_usd * dec!(0.5);

                // Profit = debt_covered * bonus_pct
                let bonus_pct = Decimal::from(pos.liquidation_bonus_bps) / dec!(10000);
                let gross_profit = max_liquidatable * bonus_pct;

                // Estimate gas cost (~$5 on X Layer)
                let gas_cost = dec!(5);
                let net_profit = gross_profit - gas_cost;

                if net_profit < min_profit {
                    return None;
                }

                Some(LiquidationTarget {
                    user: pos.user.clone(),
                    collateral_asset: pos.collateral_asset.clone(),
                    debt_asset: pos.debt_asset.clone(),
                    health_factor: pos.health_factor,
                    max_liquidatable_debt: max_liquidatable,
                    liquidation_bonus_bps: pos.liquidation_bonus_bps,
                    estimated_profit_usd: net_profit,
                })
            })
            .collect()
    }

    /// Build liquidation request
    pub fn build_liquidation_request(
        &self,
        target: &LiquidationTarget,
        amount_to_liquidate: Decimal,
    ) -> LendingRequest {
        LendingRequest {
            operation: LendingOperation::Liquidate,
            asset: target.debt_asset.clone(),
            amount: amount_to_liquidate,
            interest_rate_mode: None,
            on_behalf_of: Some(target.user.clone()),
            max_slippage_bps: dec!(100), // Higher tolerance for liquidations
            deadline_secs: 60,
        }
    }

    /// Check if auto-repay should be triggered
    pub fn should_auto_repay(&self, health_factor: Decimal) -> bool {
        self.config.auto_repay_enabled && health_factor < self.config.min_health_factor
    }

    /// Get executor configuration
    pub fn config(&self) -> &Cod3xExecutorConfig {
        &self.config
    }
}

/// Lending strategy types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LendingStrategy {
    /// Simple deposit for yield
    DepositOnly { asset: String },
    /// Leveraged long position
    LeveragedLong {
        collateral_asset: String,
        borrow_asset: String,
        target_leverage: Decimal,
    },
    /// Delta neutral (long asset, short via borrow)
    DeltaNeutral {
        long_asset: String,
        short_asset: String,
        size: Decimal,
    },
    /// Close all positions
    Unwind,
}

/// Current position snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSnapshot {
    pub collaterals: Vec<CollateralItem>,
    pub debts: Vec<DebtItem>,
    pub health_factor: Decimal,
    pub net_apy: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralItem {
    pub asset: String,
    pub amount: Decimal,
    pub value_usd: Decimal,
    pub apy: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtItem {
    pub asset: String,
    pub amount: Decimal,
    pub value_usd: Decimal,
    pub apy: Decimal,
    pub rate_mode: InterestRateMode,
}

/// Unhealthy position for liquidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnhealthyPosition {
    pub user: String,
    pub collateral_asset: String,
    pub debt_asset: String,
    pub total_collateral_usd: Decimal,
    pub total_debt_usd: Decimal,
    pub health_factor: Decimal,
    pub liquidation_bonus_bps: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_safe_borrow() {
        let executor = Cod3xExecutor::with_defaults();

        // $10k collateral, 75% LTV, target 1.5 HF
        // max_debt = 10000 * 0.75 / 1.5 = 5000
        let safe_borrow = executor.calculate_safe_borrow(
            dec!(10000),  // collateral
            dec!(0),      // current debt
            dec!(0.75),   // LTV
            Some(dec!(1.5)),
        );

        assert_eq!(safe_borrow, dec!(5000));
    }

    #[test]
    fn test_calculate_safe_borrow_with_existing_debt() {
        let executor = Cod3xExecutor::with_defaults();

        // $10k collateral, $2k existing debt, 75% LTV, target 1.5 HF
        // max_debt = 10000 * 0.75 / 1.5 = 5000
        // safe_borrow = 5000 - 2000 = 3000
        let safe_borrow = executor.calculate_safe_borrow(
            dec!(10000),
            dec!(2000),
            dec!(0.75),
            Some(dec!(1.5)),
        );

        assert_eq!(safe_borrow, dec!(3000));
    }

    #[test]
    fn test_calculate_required_repayment() {
        let executor = Cod3xExecutor::with_defaults();

        // $10k collateral, $6k debt, 75% LTV, current HF = 1.25, target 1.5
        // max_debt_for_target = 10000 * 0.75 / 1.5 = 5000
        // repayment = 6000 - 5000 = 1000
        let repayment = executor.calculate_required_repayment(
            dec!(10000),
            dec!(6000),
            dec!(0.75),
            dec!(1.25),
            Some(dec!(1.5)),
        );

        assert_eq!(repayment, dec!(1000));
    }

    #[test]
    fn test_no_repayment_needed() {
        let executor = Cod3xExecutor::with_defaults();

        // Already at target HF
        let repayment = executor.calculate_required_repayment(
            dec!(10000),
            dec!(3000),
            dec!(0.75),
            dec!(2.5),
            Some(dec!(1.5)),
        );

        assert_eq!(repayment, Decimal::ZERO);
    }

    #[test]
    fn test_validate_request() {
        let executor = Cod3xExecutor::with_defaults();

        let request = LendingRequest {
            operation: LendingOperation::Deposit,
            asset: "USDT".to_string(),
            amount: dec!(1000),
            interest_rate_mode: None,
            on_behalf_of: None,
            max_slippage_bps: dec!(50),
            deadline_secs: 300,
        };

        // Valid request
        assert!(executor.validate_request(&request, dec!(1.5), dec!(2000)).is_ok());

        // Insufficient balance
        assert!(executor.validate_request(&request, dec!(1.5), dec!(500)).is_err());
    }

    #[test]
    fn test_find_liquidation_targets() {
        let executor = Cod3xExecutor::with_defaults();

        let unhealthy = vec![
            UnhealthyPosition {
                user: "0x123".to_string(),
                collateral_asset: "ETH".to_string(),
                debt_asset: "USDT".to_string(),
                total_collateral_usd: dec!(10000),
                total_debt_usd: dec!(8000),
                health_factor: dec!(0.9),
                liquidation_bonus_bps: 500, // 5%
            },
            UnhealthyPosition {
                user: "0x456".to_string(),
                collateral_asset: "ETH".to_string(),
                debt_asset: "USDT".to_string(),
                total_collateral_usd: dec!(1000),
                total_debt_usd: dec!(900),
                health_factor: dec!(0.95),
                liquidation_bonus_bps: 500,
            },
        ];

        let targets = executor.find_liquidation_targets(&unhealthy, Some(dec!(100)));

        // Only the first position is profitable enough
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].user, "0x123");
    }

    #[test]
    fn test_should_auto_repay() {
        let executor = Cod3xExecutor::with_defaults();

        assert!(!executor.should_auto_repay(dec!(1.5)));
        assert!(executor.should_auto_repay(dec!(1.1)));
    }

    #[test]
    fn test_build_strategy_plan_leveraged() {
        let executor = Cod3xExecutor::with_defaults();

        let strategy = LendingStrategy::LeveragedLong {
            collateral_asset: "ETH".to_string(),
            borrow_asset: "USDT".to_string(),
            target_leverage: dec!(2),
        };

        let requests = executor.build_strategy_plan(strategy, dec!(1000), None);

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].operation, LendingOperation::Deposit);
        assert_eq!(requests[0].amount, dec!(1000));
        assert_eq!(requests[1].operation, LendingOperation::Borrow);
        assert_eq!(requests[1].amount, dec!(1000)); // 2x leverage = borrow 1x
    }
}
