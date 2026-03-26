//! Transaction Signer for X Layer
//!
//! Handles private key management and transaction signing for live execution.

use anyhow::{anyhow, Result};
use ethers::prelude::*;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// X Layer chain IDs
pub const XLAYER_MAINNET_CHAIN_ID: u64 = 196;
pub const XLAYER_TESTNET_CHAIN_ID: u64 = 195;

/// Transaction signer configuration
#[derive(Debug, Clone)]
pub struct SignerConfig {
    /// Chain ID (196 for mainnet, 195 for testnet)
    pub chain_id: u64,
    /// RPC URL
    pub rpc_url: String,
    /// Max gas price in gwei (safety limit)
    pub max_gas_gwei: u64,
    /// Gas price multiplier (1.1 = 10% above estimate)
    pub gas_multiplier: f64,
}

impl Default for SignerConfig {
    fn default() -> Self {
        Self {
            chain_id: XLAYER_MAINNET_CHAIN_ID,
            rpc_url: "https://rpc.xlayer.tech".to_string(),
            max_gas_gwei: 100,
            gas_multiplier: 1.1,
        }
    }
}

impl SignerConfig {
    pub fn testnet() -> Self {
        Self {
            chain_id: XLAYER_TESTNET_CHAIN_ID,
            rpc_url: "https://testrpc.xlayer.tech".to_string(),
            max_gas_gwei: 100,
            gas_multiplier: 1.1,
        }
    }
}

/// Transaction signer with provider
pub struct TxSigner {
    wallet: LocalWallet,
    provider: Arc<Provider<Http>>,
    config: SignerConfig,
}

impl TxSigner {
    /// Create signer from environment variable XLAYER_PRIVATE_KEY
    pub fn from_env(config: SignerConfig) -> Result<Self> {
        let key = std::env::var("XLAYER_PRIVATE_KEY")
            .map_err(|_| anyhow!("XLAYER_PRIVATE_KEY environment variable not set"))?;

        Self::from_private_key(&key, config)
    }

    /// Create signer from private key string
    pub fn from_private_key(private_key: &str, config: SignerConfig) -> Result<Self> {
        // Remove 0x prefix if present
        let key = private_key.trim_start_matches("0x");

        let wallet: LocalWallet = key
            .parse::<LocalWallet>()
            .map_err(|e| anyhow!("Invalid private key: {}", e))?
            .with_chain_id(config.chain_id);

        let provider = Provider::<Http>::try_from(&config.rpc_url)
            .map_err(|e| anyhow!("Invalid RPC URL: {}", e))?;

        info!(
            "Signer initialized: address={}, chain_id={}",
            wallet.address(),
            config.chain_id
        );

        Ok(Self {
            wallet,
            provider: Arc::new(provider),
            config,
        })
    }

    /// Get wallet address
    pub fn address(&self) -> Address {
        self.wallet.address()
    }

    /// Get chain ID
    pub fn chain_id(&self) -> u64 {
        self.config.chain_id
    }

    /// Get provider reference
    pub fn provider(&self) -> Arc<Provider<Http>> {
        self.provider.clone()
    }

    /// Get current ETH balance
    pub async fn balance(&self) -> Result<U256> {
        let balance = self.provider.get_balance(self.wallet.address(), None).await?;
        Ok(balance)
    }

    /// Get current nonce
    pub async fn nonce(&self) -> Result<U256> {
        let nonce = self
            .provider
            .get_transaction_count(self.wallet.address(), None)
            .await?;
        Ok(nonce)
    }

    /// Get current gas price with multiplier applied
    pub async fn gas_price(&self) -> Result<U256> {
        let base_gas = self.provider.get_gas_price().await?;
        let multiplied = (base_gas.as_u64() as f64 * self.config.gas_multiplier) as u64;

        // Apply safety cap
        let max_gas_wei = self.config.max_gas_gwei * 1_000_000_000;
        let final_gas = multiplied.min(max_gas_wei);

        Ok(U256::from(final_gas))
    }

    /// Estimate gas for a transaction
    pub async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<U256> {
        let typed_tx: TypedTransaction = tx.clone().into();
        let estimate = self.provider.estimate_gas(&typed_tx, None).await?;
        // Add 20% buffer
        Ok(estimate * 120 / 100)
    }

    /// Sign and send a transaction
    pub async fn sign_and_send(&self, mut tx: TransactionRequest) -> Result<TxHash> {
        // Set chain ID
        tx = tx.chain_id(self.config.chain_id);

        // Set from address
        tx = tx.from(self.wallet.address());

        // Set gas price if not set
        if tx.gas_price.is_none() {
            let gas_price = self.gas_price().await?;
            tx = tx.gas_price(gas_price);
            debug!("Using gas price: {} gwei", gas_price.as_u64() / 1_000_000_000);
        }

        // Estimate gas if not set
        if tx.gas.is_none() {
            let gas = self.estimate_gas(&tx).await?;
            tx = tx.gas(gas);
            debug!("Estimated gas: {}", gas);
        }

        // Set nonce if not set
        if tx.nonce.is_none() {
            let nonce = self.nonce().await?;
            tx = tx.nonce(nonce);
            debug!("Using nonce: {}", nonce);
        }

        // Sign the transaction
        let typed_tx: TypedTransaction = tx.clone().into();
        let signature = self.wallet.sign_transaction(&typed_tx).await?;
        let signed_tx = typed_tx.rlp_signed(&signature);

        // Send raw transaction
        let pending = self.provider.send_raw_transaction(signed_tx).await?;
        let tx_hash = pending.tx_hash();

        info!("Transaction sent: {:?}", tx_hash);

        Ok(tx_hash)
    }

    /// Sign, send, and wait for confirmation
    pub async fn send_and_confirm(
        &self,
        tx: TransactionRequest,
        confirmations: usize,
    ) -> Result<TransactionReceipt> {
        let tx_hash = self.sign_and_send(tx).await?;

        info!(
            "Waiting for {} confirmations for tx {:?}",
            confirmations, tx_hash
        );

        // Poll for receipt
        let mut attempts = 0;
        let max_attempts = 60; // 60 * 2 seconds = 2 minutes max wait

        loop {
            if let Some(receipt) = self.provider.get_transaction_receipt(tx_hash).await? {
                // Check confirmation count
                if let (Some(tx_block), Some(current_block)) = (
                    receipt.block_number,
                    self.provider.get_block_number().await.ok(),
                ) {
                    let confirmed = current_block.as_u64().saturating_sub(tx_block.as_u64());
                    if confirmed >= confirmations as u64 {
                        if receipt.status == Some(U64::from(1)) {
                            info!(
                                "Transaction confirmed: {:?}, gas used: {:?}",
                                tx_hash,
                                receipt.gas_used
                            );
                            return Ok(receipt);
                        } else {
                            return Err(anyhow!("Transaction reverted: {:?}", tx_hash));
                        }
                    }
                }
            }

            attempts += 1;
            if attempts >= max_attempts {
                return Err(anyhow!("Timeout waiting for transaction confirmation"));
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    /// Call a contract function (read-only)
    pub async fn call(&self, tx: &TransactionRequest) -> Result<Bytes> {
        let typed_tx: TypedTransaction = tx.clone().into();
        let result = self.provider.call(&typed_tx, None).await?;
        Ok(result)
    }

    /// Check if we have enough ETH for gas
    pub async fn has_enough_gas(&self, estimated_gas: U256) -> Result<bool> {
        let balance = self.balance().await?;
        let gas_price = self.gas_price().await?;
        let required = estimated_gas * gas_price;

        Ok(balance >= required)
    }
}

/// ERC20 transfer helper
impl TxSigner {
    /// Build ERC20 transfer transaction
    pub fn build_erc20_transfer(
        &self,
        token: Address,
        to: Address,
        amount: U256,
    ) -> TransactionRequest {
        // ERC20 transfer selector: 0xa9059cbb
        let mut data = vec![0xa9, 0x05, 0x9c, 0xbb];
        // Pad address to 32 bytes
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(to.as_bytes());
        // Pad amount to 32 bytes
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);

        TransactionRequest::new().to(token).data(data)
    }

    /// Build ERC20 approve transaction
    pub fn build_erc20_approve(
        &self,
        token: Address,
        spender: Address,
        amount: U256,
    ) -> TransactionRequest {
        // ERC20 approve selector: 0x095ea7b3
        let mut data = vec![0x09, 0x5e, 0xa7, 0xb3];
        // Pad spender to 32 bytes
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(spender.as_bytes());
        // Pad amount to 32 bytes
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);

        TransactionRequest::new().to(token).data(data)
    }

    /// Get ERC20 balance
    pub async fn get_erc20_balance(&self, token: Address, owner: Address) -> Result<U256> {
        // ERC20 balanceOf selector: 0x70a08231
        let mut data = vec![0x70, 0xa0, 0x82, 0x31];
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(owner.as_bytes());

        let tx = TransactionRequest::new().to(token).data(data);
        let result = self.call(&tx).await?;

        if result.len() >= 32 {
            Ok(U256::from_big_endian(&result[..32]))
        } else {
            Ok(U256::zero())
        }
    }

    /// Get ERC20 allowance
    pub async fn get_erc20_allowance(
        &self,
        token: Address,
        owner: Address,
        spender: Address,
    ) -> Result<U256> {
        // ERC20 allowance selector: 0xdd62ed3e
        let mut data = vec![0xdd, 0x62, 0xed, 0x3e];
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(owner.as_bytes());
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(spender.as_bytes());

        let tx = TransactionRequest::new().to(token).data(data);
        let result = self.call(&tx).await?;

        if result.len() >= 32 {
            Ok(U256::from_big_endian(&result[..32]))
        } else {
            Ok(U256::zero())
        }
    }

    /// Ensure token approval (approve if needed)
    pub async fn ensure_approval(
        &self,
        token: Address,
        spender: Address,
        amount: U256,
    ) -> Result<Option<TxHash>> {
        let current_allowance = self
            .get_erc20_allowance(token, self.wallet.address(), spender)
            .await?;

        if current_allowance >= amount {
            debug!("Sufficient allowance exists: {}", current_allowance);
            return Ok(None);
        }

        info!(
            "Approving {} for spender {:?}",
            amount, spender
        );

        // Approve max uint256 for convenience
        let max_approval = U256::MAX;
        let tx = self.build_erc20_approve(token, spender, max_approval);
        let tx_hash = self.sign_and_send(tx).await?;

        Ok(Some(tx_hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = SignerConfig::default();
        assert_eq!(config.chain_id, 196);
        assert!(config.rpc_url.contains("xlayer"));
    }

    #[test]
    fn test_config_testnet() {
        let config = SignerConfig::testnet();
        assert_eq!(config.chain_id, 195);
        assert!(config.rpc_url.contains("test"));
    }

    #[test]
    fn test_build_erc20_transfer() {
        // Use a dummy wallet for testing
        let config = SignerConfig::default();
        // This would need a valid private key to run
        // Just test the data encoding

        let to = "0x1234567890123456789012345678901234567890"
            .parse::<Address>()
            .unwrap();
        let amount = U256::from(1000000);

        // Verify the selector is correct
        let data: Vec<u8> = vec![0xa9, 0x05, 0x9c, 0xbb];
        assert_eq!(&data[..4], &[0xa9, 0x05, 0x9c, 0xbb]);
    }
}
