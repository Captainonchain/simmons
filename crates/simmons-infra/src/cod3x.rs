//! Cod3x Lending Protocol integration
//!
//! Cod3x is a DeFi lending protocol on X Layer allowing users to:
//! - Deposit collateral
//! - Borrow against collateral
//! - Earn yield on deposits
//! - Leverage positions

use anyhow::{anyhow, Result};
use chrono;
use ethers::prelude::*;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Cod3x contract addresses on X Layer
///
/// Note: In production, these should be loaded from environment variables
/// or configuration files to support different networks.
pub mod contracts {
    use ethers::types::Address;
    use std::str::FromStr;
    use std::sync::OnceLock;

    // Use OnceLock for safe lazy initialization that validates at startup
    static LENDING_POOL: OnceLock<Address> = OnceLock::new();
    static PRICE_ORACLE: OnceLock<Address> = OnceLock::new();
    static DATA_PROVIDER: OnceLock<Address> = OnceLock::new();

    fn parse_address(addr: &str) -> Address {
        Address::from_str(addr).expect("Invalid hardcoded address - this is a build error")
    }

    /// Main lending pool
    pub fn lending_pool() -> Address {
        *LENDING_POOL.get_or_init(|| {
            std::env::var("COD3X_LENDING_POOL")
                .ok()
                .and_then(|a| Address::from_str(&a).ok())
                .unwrap_or_else(|| parse_address("0x794a61358D6845594F94dc1DB02A252b5b4814aD"))
        })
    }

    /// Price oracle
    pub fn price_oracle() -> Address {
        *PRICE_ORACLE.get_or_init(|| {
            std::env::var("COD3X_PRICE_ORACLE")
                .ok()
                .and_then(|a| Address::from_str(&a).ok())
                .unwrap_or_else(|| parse_address("0x54586bE62E3c3580375aE3723C145253060Ca0C2"))
        })
    }

    /// Protocol data provider
    pub fn data_provider() -> Address {
        *DATA_PROVIDER.get_or_init(|| {
            std::env::var("COD3X_DATA_PROVIDER")
                .ok()
                .and_then(|a| Address::from_str(&a).ok())
                .unwrap_or_else(|| parse_address("0x69FA688f1Dc47d4B5d8029D5a35FB7a548310654"))
        })
    }
}

/// Asset configuration in the lending pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    pub token: Address,
    pub symbol: String,
    pub decimals: u8,
    pub ltv_bps: u32,           // Loan-to-Value ratio in basis points
    pub liquidation_threshold_bps: u32,
    pub liquidation_bonus_bps: u32,
    pub supply_cap: Decimal,
    pub borrow_cap: Decimal,
    pub is_active: bool,
    pub is_borrowable: bool,
}

/// User position in the lending pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LendingPosition {
    pub user: Address,
    pub collateral: Vec<CollateralPosition>,
    pub debt: Vec<DebtPosition>,
    pub total_collateral_usd: Decimal,
    pub total_debt_usd: Decimal,
    pub available_borrow_usd: Decimal,
    pub health_factor: Decimal,
    pub ltv: Decimal,
}

impl LendingPosition {
    /// Check if position is at risk of liquidation
    pub fn is_at_risk(&self) -> bool {
        self.health_factor < Decimal::new(12, 1) // < 1.2
    }

    /// Check if position is liquidatable
    pub fn is_liquidatable(&self) -> bool {
        self.health_factor < Decimal::ONE
    }

    /// Net equity (collateral - debt)
    pub fn net_equity(&self) -> Decimal {
        self.total_collateral_usd - self.total_debt_usd
    }
}

/// Collateral position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralPosition {
    pub token: Address,
    pub symbol: String,
    pub amount: Decimal,
    pub amount_usd: Decimal,
    pub apy: Decimal,
}

/// Debt position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtPosition {
    pub token: Address,
    pub symbol: String,
    pub amount: Decimal,
    pub amount_usd: Decimal,
    pub apy: Decimal,
    pub is_stable_rate: bool,
}

/// Pool state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub total_liquidity: Decimal,
    pub total_borrows: Decimal,
    pub utilization_rate: Decimal,
    pub supply_apy: Decimal,
    pub borrow_apy: Decimal,
    pub stable_borrow_apy: Decimal,
}

/// Liquidation risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiquidationRisk {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl LiquidationRisk {
    pub fn from_health_factor(hf: Decimal) -> Self {
        if hf >= Decimal::new(2, 0) {
            LiquidationRisk::Safe
        } else if hf >= Decimal::new(15, 1) {
            LiquidationRisk::Low
        } else if hf >= Decimal::new(12, 1) {
            LiquidationRisk::Medium
        } else if hf >= Decimal::ONE {
            LiquidationRisk::High
        } else {
            LiquidationRisk::Critical
        }
    }
}

/// Cod3x lending protocol client
pub struct Cod3xClient {
    provider: Arc<Provider<Http>>,
    lending_pool: Address,
    oracle: Address,
    data_provider: Address,
}

impl Cod3xClient {
    /// Create client with default addresses
    pub fn new(provider: Arc<Provider<Http>>) -> Self {
        Self {
            provider,
            lending_pool: contracts::lending_pool(),
            oracle: contracts::price_oracle(),
            data_provider: contracts::data_provider(),
        }
    }

    /// Create from RPC URL (validates URL and enforces HTTPS for mainnet)
    pub fn from_rpc(rpc_url: &str) -> Result<Self> {
        // Validate RPC URL format
        let url = url::Url::parse(rpc_url)
            .map_err(|e| anyhow!("Invalid RPC URL: {}", e))?;

        // Enforce HTTPS for non-localhost URLs (security requirement)
        if !url.host_str().map_or(false, |h| h == "localhost" || h == "127.0.0.1")
            && url.scheme() != "https"
        {
            return Err(anyhow!("RPC URL must use HTTPS for non-localhost connections"));
        }

        let provider = Provider::<Http>::try_from(rpc_url)?;
        Ok(Self::new(Arc::new(provider)))
    }

    /// Get provider reference
    pub fn provider(&self) -> Arc<Provider<Http>> {
        self.provider.clone()
    }

    /// Get asset price from oracle with staleness check
    ///
    /// # Arguments
    /// * `asset` - Token address to get price for
    /// * `max_staleness_secs` - Maximum allowed price age in seconds (default: 3600 = 1 hour)
    pub async fn get_asset_price(&self, asset: Address) -> Result<Decimal> {
        self.get_asset_price_with_staleness(asset, 3600).await
    }

    /// Get asset price with configurable staleness threshold
    pub async fn get_asset_price_with_staleness(
        &self,
        asset: Address,
        max_staleness_secs: u64,
    ) -> Result<Decimal> {
        // latestRoundData() selector for Chainlink-style oracles
        // Returns: (roundId, answer, startedAt, updatedAt, answeredInRound)
        let selector = ethers::utils::id("latestRoundData()");

        let tx = TransactionRequest::new()
            .to(self.oracle)
            .data(selector[..4].to_vec());

        let result = self.provider.call(&tx.into(), None).await;

        // Try Chainlink-style first (with timestamp)
        if let Ok(data) = result {
            if data.len() >= 160 {
                // Parse Chainlink response
                let price = U256::from_big_endian(&data[32..64]);
                let updated_at = U256::from_big_endian(&data[96..128]);

                // Check staleness
                let now = chrono::Utc::now().timestamp() as u64;
                let price_age = now.saturating_sub(updated_at.as_u64());

                if price_age > max_staleness_secs {
                    warn!(
                        "Oracle price for {:?} is stale: {} seconds old (max: {})",
                        asset, price_age, max_staleness_secs
                    );
                    return Err(anyhow!(
                        "Oracle price is stale: {} seconds old (max allowed: {})",
                        price_age,
                        max_staleness_secs
                    ));
                }

                // Validate price is within reasonable bounds
                let price_decimal = decimal_from_u256(price, 8);
                if price_decimal.is_zero() {
                    return Err(anyhow!("Oracle returned zero price"));
                }

                return Ok(price_decimal);
            }
        }

        // Fallback to simple getAssetPrice (Aave-style)
        let selector = ethers::utils::id("getAssetPrice(address)");
        let data = ethers::abi::encode(&[ethers::abi::Token::Address(asset)]);

        let mut call_data = selector[..4].to_vec();
        call_data.extend_from_slice(&data);

        let tx = TransactionRequest::new()
            .to(self.oracle)
            .data(call_data);

        let result = self.provider.call(&tx.into(), None).await?;

        if result.len() >= 32 {
            let price = U256::from_big_endian(&result[..32]);
            let price_decimal = decimal_from_u256(price, 8);

            // Validate price
            if price_decimal.is_zero() {
                return Err(anyhow!("Oracle returned zero price"));
            }

            // Warning: No staleness check available for simple oracle
            warn!("Using oracle without staleness check for {:?}", asset);
            Ok(price_decimal)
        } else {
            Err(anyhow!("Invalid oracle response"))
        }
    }

    /// Get user account data
    pub async fn get_user_account_data(&self, user: Address) -> Result<AccountData> {
        // getUserAccountData(address)
        let selector = ethers::utils::id("getUserAccountData(address)");
        let data = ethers::abi::encode(&[ethers::abi::Token::Address(user)]);

        let mut call_data = selector[..4].to_vec();
        call_data.extend_from_slice(&data);

        let tx = TransactionRequest::new()
            .to(self.lending_pool)
            .data(call_data);

        let result = self.provider.call(&tx.into(), None).await?;

        if result.len() < 192 {
            return Err(anyhow!("Invalid account data response"));
        }

        // Parse 6 uint256 values
        let total_collateral_eth = U256::from_big_endian(&result[0..32]);
        let total_debt_eth = U256::from_big_endian(&result[32..64]);
        let available_borrow_eth = U256::from_big_endian(&result[64..96]);
        let current_liquidation_threshold = U256::from_big_endian(&result[96..128]);
        let ltv = U256::from_big_endian(&result[128..160]);
        let health_factor = U256::from_big_endian(&result[160..192]);

        Ok(AccountData {
            total_collateral_eth: decimal_from_u256(total_collateral_eth, 18),
            total_debt_eth: decimal_from_u256(total_debt_eth, 18),
            available_borrow_eth: decimal_from_u256(available_borrow_eth, 18),
            current_liquidation_threshold: decimal_from_u256(current_liquidation_threshold, 4),
            ltv: decimal_from_u256(ltv, 4),
            health_factor: decimal_from_u256(health_factor, 18),
        })
    }

    /// Get reserve data for an asset
    pub async fn get_reserve_data(&self, asset: Address) -> Result<ReserveData> {
        // getReserveData(address)
        let selector = ethers::utils::id("getReserveData(address)");
        let data = ethers::abi::encode(&[ethers::abi::Token::Address(asset)]);

        let mut call_data = selector[..4].to_vec();
        call_data.extend_from_slice(&data);

        let tx = TransactionRequest::new()
            .to(self.lending_pool)
            .data(call_data);

        let result = self.provider.call(&tx.into(), None).await?;

        // Simplified parsing - actual struct is more complex
        if result.len() < 256 {
            return Err(anyhow!("Invalid reserve data response"));
        }

        let liquidity_rate = U256::from_big_endian(&result[32..64]);
        let stable_borrow_rate = U256::from_big_endian(&result[64..96]);
        let variable_borrow_rate = U256::from_big_endian(&result[96..128]);
        let liquidity_index = U256::from_big_endian(&result[128..160]);
        let variable_borrow_index = U256::from_big_endian(&result[160..192]);

        // Convert RAY (27 decimals) to percentage APY
        let ray_to_pct = |ray: U256| -> Decimal {
            let d = decimal_from_u256(ray, 27);
            d * Decimal::from(100)
        };

        Ok(ReserveData {
            asset,
            liquidity_rate: ray_to_pct(liquidity_rate),
            stable_borrow_rate: ray_to_pct(stable_borrow_rate),
            variable_borrow_rate: ray_to_pct(variable_borrow_rate),
            liquidity_index: decimal_from_u256(liquidity_index, 27),
            variable_borrow_index: decimal_from_u256(variable_borrow_index, 27),
        })
    }

    /// Create deposit transaction
    pub fn create_deposit_tx(
        &self,
        asset: Address,
        amount: U256,
        on_behalf_of: Address,
        referral_code: u16,
    ) -> Result<TransactionRequest> {
        // deposit(address,uint256,address,uint16)
        let selector = ethers::utils::id("deposit(address,uint256,address,uint16)");

        let data = ethers::abi::encode(&[
            ethers::abi::Token::Address(asset),
            ethers::abi::Token::Uint(amount),
            ethers::abi::Token::Address(on_behalf_of),
            ethers::abi::Token::Uint(referral_code.into()),
        ]);

        let mut tx_data = selector[..4].to_vec();
        tx_data.extend_from_slice(&data);

        Ok(TransactionRequest::new()
            .to(self.lending_pool)
            .data(tx_data))
    }

    /// Create withdraw transaction
    pub fn create_withdraw_tx(
        &self,
        asset: Address,
        amount: U256,
        to: Address,
    ) -> Result<TransactionRequest> {
        // withdraw(address,uint256,address)
        let selector = ethers::utils::id("withdraw(address,uint256,address)");

        let data = ethers::abi::encode(&[
            ethers::abi::Token::Address(asset),
            ethers::abi::Token::Uint(amount),
            ethers::abi::Token::Address(to),
        ]);

        let mut tx_data = selector[..4].to_vec();
        tx_data.extend_from_slice(&data);

        Ok(TransactionRequest::new()
            .to(self.lending_pool)
            .data(tx_data))
    }

    /// Create borrow transaction
    pub fn create_borrow_tx(
        &self,
        asset: Address,
        amount: U256,
        interest_rate_mode: u8, // 1 = stable, 2 = variable
        referral_code: u16,
        on_behalf_of: Address,
    ) -> Result<TransactionRequest> {
        // borrow(address,uint256,uint256,uint16,address)
        let selector = ethers::utils::id("borrow(address,uint256,uint256,uint16,address)");

        let data = ethers::abi::encode(&[
            ethers::abi::Token::Address(asset),
            ethers::abi::Token::Uint(amount),
            ethers::abi::Token::Uint(interest_rate_mode.into()),
            ethers::abi::Token::Uint(referral_code.into()),
            ethers::abi::Token::Address(on_behalf_of),
        ]);

        let mut tx_data = selector[..4].to_vec();
        tx_data.extend_from_slice(&data);

        Ok(TransactionRequest::new()
            .to(self.lending_pool)
            .data(tx_data))
    }

    /// Create repay transaction
    pub fn create_repay_tx(
        &self,
        asset: Address,
        amount: U256,
        interest_rate_mode: u8,
        on_behalf_of: Address,
    ) -> Result<TransactionRequest> {
        // repay(address,uint256,uint256,address)
        let selector = ethers::utils::id("repay(address,uint256,uint256,address)");

        let data = ethers::abi::encode(&[
            ethers::abi::Token::Address(asset),
            ethers::abi::Token::Uint(amount),
            ethers::abi::Token::Uint(interest_rate_mode.into()),
            ethers::abi::Token::Address(on_behalf_of),
        ]);

        let mut tx_data = selector[..4].to_vec();
        tx_data.extend_from_slice(&data);

        Ok(TransactionRequest::new()
            .to(self.lending_pool)
            .data(tx_data))
    }

    /// Create liquidation transaction
    pub fn create_liquidation_tx(
        &self,
        collateral_asset: Address,
        debt_asset: Address,
        user: Address,
        debt_to_cover: U256,
        receive_a_token: bool,
    ) -> Result<TransactionRequest> {
        // liquidationCall(address,address,address,uint256,bool)
        let selector = ethers::utils::id("liquidationCall(address,address,address,uint256,bool)");

        let data = ethers::abi::encode(&[
            ethers::abi::Token::Address(collateral_asset),
            ethers::abi::Token::Address(debt_asset),
            ethers::abi::Token::Address(user),
            ethers::abi::Token::Uint(debt_to_cover),
            ethers::abi::Token::Bool(receive_a_token),
        ]);

        let mut tx_data = selector[..4].to_vec();
        tx_data.extend_from_slice(&data);

        Ok(TransactionRequest::new()
            .to(self.lending_pool)
            .data(tx_data))
    }

    /// Calculate liquidation risk
    pub fn calculate_risk(&self, account: &AccountData) -> LiquidationRisk {
        LiquidationRisk::from_health_factor(account.health_factor)
    }

    /// Estimate liquidation profit
    pub fn estimate_liquidation_profit(
        &self,
        debt_to_cover: Decimal,
        collateral_price: Decimal,
        liquidation_bonus_bps: u32,
    ) -> LiquidationEstimate {
        // Liquidator receives collateral worth debt_to_cover * (1 + bonus)
        let bonus_multiplier = Decimal::ONE + Decimal::from(liquidation_bonus_bps) / Decimal::from(10000);
        let collateral_received = debt_to_cover * bonus_multiplier / collateral_price;
        let gross_profit = debt_to_cover * (bonus_multiplier - Decimal::ONE);

        // Estimate gas cost (~500k gas at 1 gwei)
        let estimated_gas_cost = Decimal::new(5, 4); // 0.0005 ETH

        LiquidationEstimate {
            debt_to_cover,
            collateral_received,
            gross_profit,
            estimated_gas_cost,
            net_profit: gross_profit - estimated_gas_cost,
        }
    }
}

/// Account data from getUserAccountData
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub total_collateral_eth: Decimal,
    pub total_debt_eth: Decimal,
    pub available_borrow_eth: Decimal,
    pub current_liquidation_threshold: Decimal,
    pub ltv: Decimal,
    pub health_factor: Decimal,
}

/// Reserve data for an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveData {
    pub asset: Address,
    pub liquidity_rate: Decimal,
    pub stable_borrow_rate: Decimal,
    pub variable_borrow_rate: Decimal,
    pub liquidity_index: Decimal,
    pub variable_borrow_index: Decimal,
}

/// Liquidation profit estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationEstimate {
    pub debt_to_cover: Decimal,
    pub collateral_received: Decimal,
    pub gross_profit: Decimal,
    pub estimated_gas_cost: Decimal,
    pub net_profit: Decimal,
}

/// Helper to convert U256 to Decimal with decimals
fn decimal_from_u256(value: U256, decimals: u32) -> Decimal {
    let divisor = Decimal::from(10u64.pow(decimals.min(18)));
    let value_str = value.to_string();
    Decimal::from_str(&value_str)
        .map(|d| d / divisor)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_liquidation_risk_levels() {
        assert_eq!(LiquidationRisk::from_health_factor(dec!(2.5)), LiquidationRisk::Safe);
        assert_eq!(LiquidationRisk::from_health_factor(dec!(1.6)), LiquidationRisk::Low);
        assert_eq!(LiquidationRisk::from_health_factor(dec!(1.3)), LiquidationRisk::Medium);
        assert_eq!(LiquidationRisk::from_health_factor(dec!(1.05)), LiquidationRisk::High);
        assert_eq!(LiquidationRisk::from_health_factor(dec!(0.9)), LiquidationRisk::Critical);
    }

    #[test]
    fn test_liquidation_estimate() {
        let client = Cod3xClient::from_rpc("http://localhost:8545").unwrap();

        let estimate = client.estimate_liquidation_profit(
            dec!(1000),  // $1000 debt
            dec!(3000),  // ETH at $3000
            500,         // 5% bonus
        );

        // Should receive $1050 worth of ETH = 0.35 ETH
        assert!(estimate.collateral_received > dec!(0.34));
        assert!(estimate.collateral_received < dec!(0.36));
        assert_eq!(estimate.gross_profit, dec!(50));
    }

    #[test]
    fn test_contract_addresses() {
        let pool = contracts::lending_pool();
        let oracle = contracts::price_oracle();
        assert_ne!(pool, Address::zero());
        assert_ne!(oracle, Address::zero());
    }
}
