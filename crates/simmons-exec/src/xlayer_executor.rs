//! X Layer Mainnet Executor
//!
//! Executes real swaps on X Layer using OKX DEX Aggregator API.
//! Requires wallet private key for transaction signing.

use anyhow::{anyhow, Result};
use ethers::{
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{transaction::eip2718::TypedTransaction, Address, Bytes, U256},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use simmons_infra::onchainos::{chains, OnchainOSClient};
use std::sync::Arc;
use tracing::{error, info, warn};

/// X Layer chain configuration
pub const XLAYER_CHAIN_ID: u64 = 196;
pub const XLAYER_RPC_URL: &str = "https://rpc.xlayer.tech";

/// Token addresses on X Layer
pub mod xlayer_tokens {
    pub const OKB: &str = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"; // Native
    pub const USDT: &str = "0x1E4a5963aBFD975d8c9021ce480b42188849D41d";
    pub const USDC: &str = "0x74b7F16337b8972027F6196A17a631aC6dE26d22";
    pub const WETH: &str = "0x5A77f1443D16ee5761d310e38b62f77f726bC71c";
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub tx_hash: Option<String>,
    pub from_token: String,
    pub to_token: String,
    pub amount_in: String,
    pub amount_out: String,
    pub gas_used: Option<String>,
    pub error: Option<String>,
}

/// X Layer executor for mainnet trading
pub struct XLayerExecutor {
    onchainos: Arc<OnchainOSClient>,
    provider: Provider<Http>,
    wallet: LocalWallet,
    wallet_address: Address,
}

impl XLayerExecutor {
    /// Create new executor from environment variables
    pub async fn from_env() -> Result<Self> {
        let onchainos = Arc::new(OnchainOSClient::from_env()?);

        let provider = Provider::<Http>::try_from(XLAYER_RPC_URL)?;

        let private_key = std::env::var("XLAYER_PRIVATE_KEY")
            .map_err(|_| anyhow!("XLAYER_PRIVATE_KEY not set - required for mainnet execution"))?;

        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?
            .with_chain_id(XLAYER_CHAIN_ID);

        let wallet_address = wallet.address();

        info!("[XLayerExecutor] Initialized for address: {:?}", wallet_address);

        Ok(Self {
            onchainos,
            provider,
            wallet,
            wallet_address,
        })
    }

    /// Create executor with explicit private key
    pub async fn new(onchainos: Arc<OnchainOSClient>, private_key: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(XLAYER_RPC_URL)?;

        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?
            .with_chain_id(XLAYER_CHAIN_ID);

        let wallet_address = wallet.address();

        info!("[XLayerExecutor] Initialized for address: {:?}", wallet_address);

        Ok(Self {
            onchainos,
            provider,
            wallet,
            wallet_address,
        })
    }

    /// Get wallet address
    pub fn wallet_address(&self) -> Address {
        self.wallet_address
    }

    /// Get wallet balance (native OKB)
    pub async fn get_balance(&self) -> Result<U256> {
        let balance = self.provider.get_balance(self.wallet_address, None).await?;
        Ok(balance)
    }

    /// Get token balance
    pub async fn get_token_balance(&self, token_address: &str) -> Result<Decimal> {
        let balances = self.onchainos.get_balances(
            chains::XLAYER,
            &format!("{:?}", self.wallet_address),
        ).await?;

        for balance in balances {
            if balance.token_address.to_lowercase() == token_address.to_lowercase() {
                return Ok(balance.balance.parse().unwrap_or_default());
            }
        }

        Ok(Decimal::ZERO)
    }

    /// Execute a swap on X Layer
    pub async fn execute_swap(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        slippage: &str,
    ) -> Result<ExecutionResult> {
        info!(
            "[XLayerExecutor] Executing swap: {} {} -> {}",
            amount, from_token, to_token
        );

        // Step 1: Get swap transaction data from OKX DEX Aggregator
        let prepared = self.onchainos.prepare_swap(
            chains::XLAYER,
            from_token,
            to_token,
            amount,
            &format!("{:?}", self.wallet_address),
            slippage,
        ).await?;

        info!(
            "[XLayerExecutor] Swap prepared: {} -> {} (min out: {})",
            prepared.from_amount, prepared.to_amount, prepared.min_out
        );

        // Step 2: Build the transaction
        let to_address: Address = prepared.to.parse()?;
        let value = U256::from_dec_str(&prepared.value).unwrap_or(U256::zero());
        let gas_limit = U256::from_dec_str(&prepared.gas_limit).unwrap_or(U256::from(300000));
        let gas_price = U256::from_dec_str(&prepared.gas_price).unwrap_or(U256::from(1000000000)); // 1 gwei default

        let data: Bytes = prepared.data.parse()?;

        let nonce = self.provider.get_transaction_count(self.wallet_address, None).await?;

        let tx = TypedTransaction::Legacy(TransactionRequest {
            from: Some(self.wallet_address),
            to: Some(to_address.into()),
            value: Some(value),
            gas: Some(gas_limit),
            gas_price: Some(gas_price),
            data: Some(data),
            nonce: Some(nonce),
            chain_id: Some(XLAYER_CHAIN_ID.into()),
        });

        // Step 3: Sign the transaction
        let signature = self.wallet.sign_transaction(&tx).await?;
        let signed_tx = tx.rlp_signed(&signature);

        // Step 4: Broadcast the transaction
        info!("[XLayerExecutor] Broadcasting transaction...");

        let pending_tx = self.provider.send_raw_transaction(signed_tx).await?;
        let tx_hash = pending_tx.tx_hash();

        info!("[XLayerExecutor] Transaction sent: {:?}", tx_hash);

        // Step 5: Wait for confirmation
        let receipt = pending_tx.await?;

        match receipt {
            Some(r) => {
                let success = r.status.map(|s| s == U64::from(1)).unwrap_or(false);
                let gas_used = r.gas_used.map(|g| g.to_string());

                if success {
                    info!(
                        "[XLayerExecutor] Swap successful! Tx: {:?}, Gas: {:?}",
                        tx_hash, gas_used
                    );
                } else {
                    error!("[XLayerExecutor] Swap failed! Tx: {:?}", tx_hash);
                }

                Ok(ExecutionResult {
                    success,
                    tx_hash: Some(format!("{:?}", tx_hash)),
                    from_token: from_token.to_string(),
                    to_token: to_token.to_string(),
                    amount_in: prepared.from_amount,
                    amount_out: prepared.to_amount,
                    gas_used,
                    error: if success { None } else { Some("Transaction reverted".to_string()) },
                })
            }
            None => {
                Err(anyhow!("Transaction not confirmed"))
            }
        }
    }

    /// Get quote without executing (for preview)
    pub async fn get_quote(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
    ) -> Result<SwapQuote> {
        let quote = self.onchainos.get_quote(&simmons_infra::onchainos::QuoteRequest {
            chain_id: chains::XLAYER.to_string(),
            from_token: from_token.to_string(),
            to_token: to_token.to_string(),
            amount: amount.to_string(),
            slippage: Some("1.0".to_string()),
        }).await?;

        let price_impact = quote.price_impact().to_string();
        Ok(SwapQuote {
            from_amount: quote.from_token_amount,
            to_amount: quote.to_token_amount,
            price_impact,
            gas_estimate: quote.estimate_gas_fee,
        })
    }
}

/// Swap quote for preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub from_amount: String,
    pub to_amount: String,
    pub price_impact: String,
    pub gas_estimate: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xlayer_tokens() {
        assert_eq!(xlayer_tokens::OKB, "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    }
}
