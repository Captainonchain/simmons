//! X Layer ZK-EVM integration
//!
//! X Layer is OKX's L2 built on Polygon CDK (ZK-EVM).
//! Chain ID: 196 (mainnet), 195 (testnet)

use anyhow::{anyhow, Result};
use ethers::prelude::*;
use ethers::utils::format_units;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// X Layer chain configuration
pub const XLAYER_MAINNET_CHAIN_ID: u64 = 196;
pub const XLAYER_TESTNET_CHAIN_ID: u64 = 195;
pub const XLAYER_MAINNET_RPC: &str = "https://rpc.xlayer.tech";
pub const XLAYER_TESTNET_RPC: &str = "https://testrpc.xlayer.tech";

/// Common token addresses on X Layer
/// Supports override via environment variables for different networks
pub mod tokens {
    use ethers::types::Address;
    use std::str::FromStr;
    use std::sync::OnceLock;

    static WETH: OnceLock<Address> = OnceLock::new();
    static USDT: OnceLock<Address> = OnceLock::new();
    static USDC: OnceLock<Address> = OnceLock::new();
    static OKB: OnceLock<Address> = OnceLock::new();

    fn parse_address(addr: &str, name: &str) -> Address {
        Address::from_str(addr)
            .unwrap_or_else(|_| panic!("Invalid {} address - build error", name))
    }

    pub fn weth() -> Address {
        *WETH.get_or_init(|| {
            std::env::var("XLAYER_WETH")
                .ok()
                .and_then(|a| Address::from_str(&a).ok())
                .unwrap_or_else(|| parse_address("0x5A77f1443D16ee5761d310e38b62f77f726bC71c", "WETH"))
        })
    }

    pub fn usdt() -> Address {
        *USDT.get_or_init(|| {
            std::env::var("XLAYER_USDT")
                .ok()
                .and_then(|a| Address::from_str(&a).ok())
                .unwrap_or_else(|| parse_address("0x1E4a5963aBFD975d8c9021ce480b42188849D41d", "USDT"))
        })
    }

    pub fn usdc() -> Address {
        *USDC.get_or_init(|| {
            std::env::var("XLAYER_USDC")
                .ok()
                .and_then(|a| Address::from_str(&a).ok())
                .unwrap_or_else(|| parse_address("0x74b7F16337b8972027F6196A17a631aC6dE26d22", "USDC"))
        })
    }

    pub fn okb() -> Address {
        *OKB.get_or_init(|| {
            std::env::var("XLAYER_OKB")
                .ok()
                .and_then(|a| Address::from_str(&a).ok())
                .unwrap_or_else(|| parse_address("0x75231F58b43240C9718Dd58B4967c5114342a86c", "OKB"))
        })
    }
}

/// X Layer client with full provider capabilities
pub struct XLayerClient {
    provider: Arc<Provider<Http>>,
    chain_id: u64,
    block_time_secs: u64,
}

impl XLayerClient {
    /// Create a new X Layer client for mainnet
    pub fn mainnet() -> Result<Self> {
        Self::new(XLAYER_MAINNET_RPC, XLAYER_MAINNET_CHAIN_ID)
    }

    /// Create a new X Layer client for testnet
    pub fn testnet() -> Result<Self> {
        Self::new(XLAYER_TESTNET_RPC, XLAYER_TESTNET_CHAIN_ID)
    }

    /// Create with custom RPC URL
    pub fn new(rpc_url: &str, chain_id: u64) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        Ok(Self {
            provider: Arc::new(provider),
            chain_id,
            block_time_secs: 2, // X Layer has ~2s blocks
        })
    }

    /// Get provider reference for external use
    pub fn provider(&self) -> Arc<Provider<Http>> {
        self.provider.clone()
    }

    /// Get chain ID
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Get latest block number
    pub async fn get_block_number(&self) -> Result<u64> {
        let block = self.provider.get_block_number().await?;
        Ok(block.as_u64())
    }

    /// Get block by number with full details
    pub async fn get_block(&self, number: Option<u64>) -> Result<Option<XLayerBlock>> {
        let block_id = match number {
            Some(n) => BlockId::Number(BlockNumber::Number(n.into())),
            None => BlockId::Number(BlockNumber::Latest),
        };

        let block = self.provider.get_block(block_id).await?;

        Ok(block.map(|b| XLayerBlock {
            number: b.number.map(|n| n.as_u64()).unwrap_or(0),
            hash: format!("{:?}", b.hash.unwrap_or_default()),
            timestamp: b.timestamp.as_u64(),
            transactions: b.transactions.len(),
            gas_used: Decimal::from(b.gas_used.as_u64()),
            gas_limit: Decimal::from(b.gas_limit.as_u64()),
            base_fee: b
                .base_fee_per_gas
                .map(|f| Decimal::from(f.as_u64()))
                .unwrap_or_default(),
        }))
    }

    /// Get ETH balance for an address
    pub async fn get_balance(&self, address: Address) -> Result<Decimal> {
        let balance = self.provider.get_balance(address, None).await?;
        // Convert from wei to ETH (18 decimals)
        let balance_str = format_units(balance, 18)?;
        let balance_dec = Decimal::from_str(&balance_str)?;
        Ok(balance_dec)
    }

    /// Get ERC20 token balance
    pub async fn get_token_balance(&self, token: Address, owner: Address) -> Result<Decimal> {
        // ERC20 balanceOf selector: 0x70a08231
        let data = ethers::abi::encode(&[ethers::abi::Token::Address(owner)]);
        let mut call_data = vec![0x70, 0xa0, 0x82, 0x31];
        call_data.extend_from_slice(&data);

        let tx = TransactionRequest::new()
            .to(token)
            .data(call_data);

        let result = self.provider.call(&tx.into(), None).await?;

        if result.len() >= 32 {
            let balance = U256::from_big_endian(&result[..32]);
            // Assume 18 decimals (should query decimals() for accuracy)
            let balance_str = format_units(balance, 18)?;
            Ok(Decimal::from_str(&balance_str)?)
        } else {
            Ok(Decimal::ZERO)
        }
    }

    /// Get current gas price in gwei
    pub async fn get_gas_price(&self) -> Result<Decimal> {
        let gas_price = self.provider.get_gas_price().await?;
        let gwei = format_units(gas_price, "gwei")?;
        Ok(Decimal::from_str(&gwei)?)
    }

    /// Estimate gas for a transaction
    pub async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<u64> {
        let typed_tx: ethers::types::transaction::eip2718::TypedTransaction = tx.clone().into();
        let estimate = self.provider.estimate_gas(&typed_tx, None).await?;
        Ok(estimate.as_u64())
    }

    /// Get transaction receipt
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> Result<Option<TransactionReceipt>> {
        Ok(self.provider.get_transaction_receipt(tx_hash).await?)
    }

    /// Wait for transaction confirmation
    pub async fn wait_for_confirmation(
        &self,
        tx_hash: H256,
        confirmations: usize,
    ) -> Result<TransactionReceipt> {
        let pending = PendingTransaction::new(tx_hash, &self.provider);
        let receipt = pending
            .confirmations(confirmations)
            .await?
            .ok_or_else(|| anyhow!("Transaction not found"))?;
        Ok(receipt)
    }

    /// Subscribe to new blocks (returns stream)
    pub async fn subscribe_blocks(&self) -> Result<impl futures_util::Stream<Item = u64>> {
        // For HTTP providers, we poll instead of true subscription
        // Create a polling interval stream
        let provider = self.provider.clone();
        let interval = tokio::time::interval(std::time::Duration::from_secs(self.block_time_secs));

        use futures_util::StreamExt;
        let stream = tokio_stream::wrappers::IntervalStream::new(interval).then(move |_| {
            let p = provider.clone();
            async move {
                p.get_block_number()
                    .await
                    .map(|n| n.as_u64())
                    .unwrap_or(0)
            }
        });

        Ok(stream)
    }

    /// Get logs for contract events
    pub async fn get_logs(&self, filter: &Filter) -> Result<Vec<Log>> {
        Ok(self.provider.get_logs(filter).await?)
    }

    /// Call a contract function (read-only)
    pub async fn call(&self, tx: &TransactionRequest) -> Result<Bytes> {
        Ok(self.provider.call(&tx.clone().into(), None).await?)
    }
}

/// X Layer block summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XLayerBlock {
    pub number: u64,
    pub hash: String,
    pub timestamp: u64,
    pub transactions: usize,
    pub gas_used: Decimal,
    pub gas_limit: Decimal,
    pub base_fee: Decimal,
}

impl XLayerBlock {
    /// Gas utilization percentage
    pub fn gas_utilization(&self) -> Decimal {
        if self.gas_limit.is_zero() {
            return Decimal::ZERO;
        }
        (self.gas_used / self.gas_limit) * Decimal::from(100)
    }
}

/// Network health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealth {
    pub connected: bool,
    pub chain_id: u64,
    pub latest_block: u64,
    pub gas_price_gwei: Decimal,
    pub block_time_secs: u64,
}

impl XLayerClient {
    /// Check network health
    pub async fn health_check(&self) -> NetworkHealth {
        let (block_num, gas_price) = tokio::join!(
            self.get_block_number(),
            self.get_gas_price()
        );

        NetworkHealth {
            connected: block_num.is_ok(),
            chain_id: self.chain_id,
            latest_block: block_num.unwrap_or(0),
            gas_price_gwei: gas_price.unwrap_or_default(),
            block_time_secs: self.block_time_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xlayer_config() {
        assert_eq!(XLAYER_MAINNET_CHAIN_ID, 196);
        assert_eq!(XLAYER_TESTNET_CHAIN_ID, 195);
    }

    #[test]
    fn test_token_addresses() {
        // Verify addresses are valid
        let weth = tokens::weth();
        let usdt = tokens::usdt();
        assert_ne!(weth, Address::zero());
        assert_ne!(usdt, Address::zero());
    }

    #[test]
    fn test_block_gas_utilization() {
        let block = XLayerBlock {
            number: 1,
            hash: "0x...".to_string(),
            timestamp: 0,
            transactions: 10,
            gas_used: Decimal::from(5_000_000),
            gas_limit: Decimal::from(10_000_000),
            base_fee: Decimal::from(1),
        };
        assert_eq!(block.gas_utilization(), Decimal::from(50));
    }
}
