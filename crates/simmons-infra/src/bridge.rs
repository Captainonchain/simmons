//! OKX Bridge - L1 ↔ X Layer bridging
//!
//! Handles deposits from Ethereum L1 to X Layer and withdrawals back.
//! The bridge is based on Polygon CDK's native bridge architecture.

use anyhow::{anyhow, Result};
use ethers::prelude::*;
use ethers::utils::format_units;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Bridge contract addresses
pub mod contracts {
    use ethers::types::Address;
    use std::str::FromStr;

    /// L1 (Ethereum) bridge contract
    pub fn l1_bridge() -> Address {
        Address::from_str("0x2a3DD3EB832aF982ec71669E178424b10Dca2EDe").unwrap()
    }

    /// L2 (X Layer) bridge contract
    pub fn l2_bridge() -> Address {
        Address::from_str("0x2a3DD3EB832aF982ec71669E178424b10Dca2EDe").unwrap()
    }

    /// Native ETH on L1 representation
    pub fn eth_token() -> Address {
        Address::zero() // Native ETH uses zero address
    }
}

/// Bridge operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BridgeDirection {
    /// L1 (Ethereum) → L2 (X Layer)
    Deposit,
    /// L2 (X Layer) → L1 (Ethereum)
    Withdraw,
}

/// Bridge transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeStatus {
    Pending,
    L1Confirmed,
    L2Ready,
    Claimable,
    Completed,
    Failed,
}

/// OKX Bridge client for L1 ↔ X Layer transfers
pub struct OkxBridge {
    l1_provider: Arc<Provider<Http>>,
    l2_provider: Arc<Provider<Http>>,
    l1_bridge_address: Address,
    l2_bridge_address: Address,
}

impl OkxBridge {
    /// Create bridge client with default addresses
    pub fn new(l1_rpc: &str, l2_rpc: &str) -> Result<Self> {
        let l1_provider = Provider::<Http>::try_from(l1_rpc)?;
        let l2_provider = Provider::<Http>::try_from(l2_rpc)?;

        Ok(Self {
            l1_provider: Arc::new(l1_provider),
            l2_provider: Arc::new(l2_provider),
            l1_bridge_address: contracts::l1_bridge(),
            l2_bridge_address: contracts::l2_bridge(),
        })
    }

    /// Create with custom bridge addresses
    pub fn with_addresses(
        l1_rpc: &str,
        l2_rpc: &str,
        l1_bridge: Address,
        l2_bridge: Address,
    ) -> Result<Self> {
        let l1_provider = Provider::<Http>::try_from(l1_rpc)?;
        let l2_provider = Provider::<Http>::try_from(l2_rpc)?;

        Ok(Self {
            l1_provider: Arc::new(l1_provider),
            l2_provider: Arc::new(l2_provider),
            l1_bridge_address: l1_bridge,
            l2_bridge_address: l2_bridge,
        })
    }

    /// Estimate bridge operation
    pub async fn estimate(&self, direction: BridgeDirection, amount: Decimal) -> Result<BridgeEstimate> {
        // Bridge fees are typically 0.1% with minimum ~$1
        let fee_pct = Decimal::new(1, 3); // 0.1%
        let fee = (amount * fee_pct).max(Decimal::ONE);

        // Estimated times based on direction
        let (estimated_time_secs, confirmations_needed) = match direction {
            BridgeDirection::Deposit => (300, 12), // ~5 min, 12 L1 confirmations
            BridgeDirection::Withdraw => (3600, 64), // ~1 hour, ZK proof generation
        };

        // Estimate gas costs
        let gas_price = match direction {
            BridgeDirection::Deposit => {
                let price = self.l1_provider.get_gas_price().await?;
                Decimal::from_str(&format_units(price, "gwei")?)?
            }
            BridgeDirection::Withdraw => {
                let price = self.l2_provider.get_gas_price().await?;
                Decimal::from_str(&format_units(price, "gwei")?)?
            }
        };

        // Approximate gas units for bridge operations
        let gas_units = Decimal::from(150_000);
        let gas_cost_gwei = gas_units * gas_price;
        let gas_cost_eth = gas_cost_gwei / Decimal::from(1_000_000_000);

        Ok(BridgeEstimate {
            direction,
            amount,
            fee,
            fee_pct,
            gas_cost_eth,
            net_amount: amount - fee,
            estimated_time_secs,
            confirmations_needed,
        })
    }

    /// Create deposit transaction data (L1 → L2)
    pub fn create_deposit_tx(
        &self,
        token: Address,
        amount: U256,
        recipient: Address,
    ) -> Result<TransactionRequest> {
        // bridgeAsset(uint32 destinationNetwork, address destinationAddress, uint256 amount, address token, bool forceUpdateGlobalExitRoot, bytes permitData)
        let selector = ethers::utils::id("bridgeAsset(uint32,address,uint256,address,bool,bytes)");

        let destination_network: u32 = 1; // X Layer network ID
        let force_update = true;
        let permit_data: Bytes = Bytes::new();

        let data = ethers::abi::encode(&[
            ethers::abi::Token::Uint(destination_network.into()),
            ethers::abi::Token::Address(recipient),
            ethers::abi::Token::Uint(amount),
            ethers::abi::Token::Address(token),
            ethers::abi::Token::Bool(force_update),
            ethers::abi::Token::Bytes(permit_data.to_vec()),
        ]);

        let mut tx_data = selector[..4].to_vec();
        tx_data.extend_from_slice(&data);

        let mut tx = TransactionRequest::new()
            .to(self.l1_bridge_address)
            .data(tx_data);

        // If bridging native ETH, include value
        if token == Address::zero() {
            tx = tx.value(amount);
        }

        Ok(tx)
    }

    /// Create withdraw transaction data (L2 → L1)
    pub fn create_withdraw_tx(
        &self,
        token: Address,
        amount: U256,
        recipient: Address,
    ) -> Result<TransactionRequest> {
        // Same interface on L2 bridge
        let selector = ethers::utils::id("bridgeAsset(uint32,address,uint256,address,bool,bytes)");

        let destination_network: u32 = 0; // Ethereum mainnet
        let force_update = true;
        let permit_data: Bytes = Bytes::new();

        let data = ethers::abi::encode(&[
            ethers::abi::Token::Uint(destination_network.into()),
            ethers::abi::Token::Address(recipient),
            ethers::abi::Token::Uint(amount),
            ethers::abi::Token::Address(token),
            ethers::abi::Token::Bool(force_update),
            ethers::abi::Token::Bytes(permit_data.to_vec()),
        ]);

        let mut tx_data = selector[..4].to_vec();
        tx_data.extend_from_slice(&data);

        let mut tx = TransactionRequest::new()
            .to(self.l2_bridge_address)
            .data(tx_data);

        // If bridging native ETH on L2
        if token == Address::zero() {
            tx = tx.value(amount);
        }

        Ok(tx)
    }

    /// Check bridge transaction status
    pub async fn check_status(&self, tx_hash: H256, direction: BridgeDirection) -> Result<BridgeTxStatus> {
        let (provider, confirmations_needed) = match direction {
            BridgeDirection::Deposit => (&self.l1_provider, 12u64),
            BridgeDirection::Withdraw => (&self.l2_provider, 1u64),
        };

        let receipt = provider.get_transaction_receipt(tx_hash).await?;

        match receipt {
            None => Ok(BridgeTxStatus {
                tx_hash: format!("{:?}", tx_hash),
                direction,
                status: BridgeStatus::Pending,
                confirmations: 0,
                confirmations_needed,
                claimable: false,
                claim_tx_hash: None,
            }),
            Some(receipt) => {
                let current_block = provider.get_block_number().await?.as_u64();
                let tx_block = receipt.block_number.map(|n| n.as_u64()).unwrap_or(0);
                let confirmations = current_block.saturating_sub(tx_block);

                let status = if receipt.status == Some(0.into()) {
                    BridgeStatus::Failed
                } else if confirmations >= confirmations_needed {
                    match direction {
                        BridgeDirection::Deposit => BridgeStatus::L2Ready,
                        BridgeDirection::Withdraw => BridgeStatus::Claimable,
                    }
                } else {
                    BridgeStatus::L1Confirmed
                };

                Ok(BridgeTxStatus {
                    tx_hash: format!("{:?}", tx_hash),
                    direction,
                    status,
                    confirmations,
                    confirmations_needed,
                    claimable: matches!(status, BridgeStatus::Claimable | BridgeStatus::L2Ready),
                    claim_tx_hash: None,
                })
            }
        }
    }

    /// Get bridge availability and health
    pub async fn health(&self) -> Result<BridgeHealth> {
        let l1_connected = self.l1_provider.get_block_number().await.is_ok();
        let l2_connected = self.l2_provider.get_block_number().await.is_ok();

        let l1_gas = if l1_connected {
            let price = self.l1_provider.get_gas_price().await?;
            Decimal::from_str(&format_units(price, "gwei")?)?
        } else {
            Decimal::ZERO
        };

        let l2_gas = if l2_connected {
            let price = self.l2_provider.get_gas_price().await?;
            Decimal::from_str(&format_units(price, "gwei")?)?
        } else {
            Decimal::ZERO
        };

        Ok(BridgeHealth {
            l1_connected,
            l2_connected,
            bridge_operational: l1_connected && l2_connected,
            l1_gas_gwei: l1_gas,
            l2_gas_gwei: l2_gas,
            estimated_deposit_time_secs: 300,
            estimated_withdraw_time_secs: 3600,
        })
    }
}

/// Bridge operation estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeEstimate {
    pub direction: BridgeDirection,
    pub amount: Decimal,
    pub fee: Decimal,
    pub fee_pct: Decimal,
    pub gas_cost_eth: Decimal,
    pub net_amount: Decimal,
    pub estimated_time_secs: u64,
    pub confirmations_needed: u32,
}

impl BridgeEstimate {
    /// Total cost (fee + gas)
    pub fn total_cost(&self) -> Decimal {
        self.fee + self.gas_cost_eth
    }

    /// Effective amount after all costs
    pub fn effective_amount(&self) -> Decimal {
        self.amount - self.fee - self.gas_cost_eth
    }
}

/// Bridge transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTxStatus {
    pub tx_hash: String,
    pub direction: BridgeDirection,
    pub status: BridgeStatus,
    pub confirmations: u64,
    pub confirmations_needed: u64,
    pub claimable: bool,
    pub claim_tx_hash: Option<String>,
}

/// Bridge health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHealth {
    pub l1_connected: bool,
    pub l2_connected: bool,
    pub bridge_operational: bool,
    pub l1_gas_gwei: Decimal,
    pub l2_gas_gwei: Decimal,
    pub estimated_deposit_time_secs: u64,
    pub estimated_withdraw_time_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bridge_estimate_calculations() {
        let estimate = BridgeEstimate {
            direction: BridgeDirection::Deposit,
            amount: dec!(1000),
            fee: dec!(1),
            fee_pct: dec!(0.001),
            gas_cost_eth: dec!(0.005),
            net_amount: dec!(999),
            estimated_time_secs: 300,
            confirmations_needed: 12,
        };

        assert_eq!(estimate.total_cost(), dec!(1.005));
        assert_eq!(estimate.effective_amount(), dec!(998.995));
    }

    #[test]
    fn test_contract_addresses() {
        let l1 = contracts::l1_bridge();
        let l2 = contracts::l2_bridge();
        assert_ne!(l1, Address::zero());
        assert_ne!(l2, Address::zero());
    }
}
