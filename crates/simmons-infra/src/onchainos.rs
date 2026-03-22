//! OKX OnchainOS Integration
//!
//! OnchainOS is OKX's Web3 infrastructure for the AI era.
//! - 60+ networks, 500+ DEXs aggregated
//! - Market data, swap execution, risk detection
//! - x402 gas-free payments
//!
//! API Docs: https://web3.okx.com/onchainos/dev-docs

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::{Client, Method};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// OnchainOS API base URL
pub const ONCHAINOS_BASE_URL: &str = "https://www.okx.com";

/// Supported chain IDs
pub mod chains {
    pub const ETHEREUM: &str = "1";
    pub const BSC: &str = "56";
    pub const POLYGON: &str = "137";
    pub const ARBITRUM: &str = "42161";
    pub const OPTIMISM: &str = "10";
    pub const AVALANCHE: &str = "43114";
    pub const BASE: &str = "8453";
    pub const XLAYER: &str = "196";
    pub const SOLANA: &str = "501";
}

/// Native token addresses
pub mod native_tokens {
    /// ETH on EVM chains
    pub const EVM_NATIVE: &str = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    /// SOL on Solana
    pub const SOLANA_NATIVE: &str = "11111111111111111111111111111111";
}

/// OnchainOS client configuration
#[derive(Debug, Clone)]
pub struct OnchainOSConfig {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
    pub base_url: String,
}

impl OnchainOSConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api_key: std::env::var("OKX_API_KEY")
                .map_err(|_| anyhow!("OKX_API_KEY not set"))?,
            secret_key: std::env::var("OKX_SECRET_KEY")
                .map_err(|_| anyhow!("OKX_SECRET_KEY not set"))?,
            passphrase: std::env::var("OKX_PASSPHRASE")
                .map_err(|_| anyhow!("OKX_PASSPHRASE not set"))?,
            base_url: std::env::var("OKX_BASE_URL")
                .unwrap_or_else(|_| ONCHAINOS_BASE_URL.to_string()),
        })
    }
}

/// OnchainOS client for DEX aggregation and market data
pub struct OnchainOSClient {
    config: OnchainOSConfig,
    http: Client,
}

impl OnchainOSClient {
    pub fn new(config: OnchainOSConfig) -> Self {
        Self {
            config,
            http: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(OnchainOSConfig::from_env()?))
    }

    /// Generate HMAC SHA256 signature for request
    fn sign(&self, timestamp: &str, method: &str, path: &str, body: &str) -> String {
        let prehash = format!("{}{}{}{}", timestamp, method, path, body);

        let mut mac = Hmac::<Sha256>::new_from_slice(self.config.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(prehash.as_bytes());

        BASE64.encode(mac.finalize().into_bytes())
    }

    /// Make authenticated request to OnchainOS API
    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: Method,
        path: &str,
        query: Option<&HashMap<String, String>>,
        body: Option<&impl Serialize>,
    ) -> Result<T> {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let body_str = match body {
            Some(b) => serde_json::to_string(b)?,
            None => String::new(),
        };

        // Build full path with query params for signature
        let full_path = if let Some(q) = query {
            let qs: String = q.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}?{}", path, qs)
        } else {
            path.to_string()
        };

        let signature = self.sign(&timestamp, method.as_str(), &full_path, &body_str);

        let mut req = self.http
            .request(method.clone(), format!("{}{}", self.config.base_url, full_path))
            .header("OK-ACCESS-KEY", &self.config.api_key)
            .header("OK-ACCESS-TIMESTAMP", &timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.config.passphrase)
            .header("OK-ACCESS-SIGN", &signature)
            .header("Content-Type", "application/json");

        if !body_str.is_empty() {
            req = req.body(body_str);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(anyhow!("OnchainOS API error {}: {}", status, text));
        }

        debug!("OnchainOS response: {}", text);

        let response: OnchainOSResponse<T> = serde_json::from_str(&text)?;

        if response.code != "0" {
            return Err(anyhow!("OnchainOS error {}: {}", response.code, response.msg));
        }

        response.data.ok_or_else(|| anyhow!("No data in response"))
    }

    // ============ DEX Aggregator APIs ============

    /// Get swap quote from DEX aggregator
    pub async fn get_quote(&self, request: &QuoteRequest) -> Result<QuoteResponse> {
        let mut params = HashMap::new();
        params.insert("chainId".to_string(), request.chain_id.clone());
        params.insert("fromTokenAddress".to_string(), request.from_token.clone());
        params.insert("toTokenAddress".to_string(), request.to_token.clone());
        params.insert("amount".to_string(), request.amount.clone());

        if let Some(slippage) = &request.slippage {
            params.insert("slippage".to_string(), slippage.clone());
        }

        self.request(Method::GET, "/api/v6/dex/aggregator/quote", Some(&params), None::<&()>).await
    }

    /// Get swap transaction data
    pub async fn get_swap(&self, request: &SwapRequest) -> Result<SwapResponse> {
        let mut params = HashMap::new();
        params.insert("chainId".to_string(), request.chain_id.clone());
        params.insert("fromTokenAddress".to_string(), request.from_token.clone());
        params.insert("toTokenAddress".to_string(), request.to_token.clone());
        params.insert("amount".to_string(), request.amount.clone());
        params.insert("userWalletAddress".to_string(), request.user_address.clone());
        params.insert("slippage".to_string(), request.slippage.clone());

        self.request(Method::GET, "/api/v6/dex/aggregator/swap", Some(&params), None::<&()>).await
    }

    /// Get supported tokens for a chain
    pub async fn get_supported_tokens(&self, chain_id: &str) -> Result<Vec<TokenInfo>> {
        let mut params = HashMap::new();
        params.insert("chainId".to_string(), chain_id.to_string());

        self.request(Method::GET, "/api/v6/dex/aggregator/all-tokens", Some(&params), None::<&()>).await
    }

    /// Get liquidity sources for a chain
    pub async fn get_liquidity_sources(&self, chain_id: &str) -> Result<Vec<LiquiditySource>> {
        let mut params = HashMap::new();
        params.insert("chainId".to_string(), chain_id.to_string());

        self.request(Method::GET, "/api/v6/dex/aggregator/get-liquidity", Some(&params), None::<&()>).await
    }

    // ============ Market APIs ============

    /// Get current token price
    pub async fn get_price(&self, chain_id: &str, token_address: &str) -> Result<TokenPrice> {
        let mut params = HashMap::new();
        params.insert("chainId".to_string(), chain_id.to_string());
        params.insert("tokenAddress".to_string(), token_address.to_string());

        self.request(Method::GET, "/api/v6/dex/index/current-price", Some(&params), None::<&()>).await
    }

    /// Get token market data
    pub async fn get_token_info(&self, chain_id: &str, token_address: &str) -> Result<TokenMarketData> {
        let mut params = HashMap::new();
        params.insert("chainId".to_string(), chain_id.to_string());
        params.insert("tokenContractAddress".to_string(), token_address.to_string());

        self.request(Method::GET, "/api/v6/dex/market/token", Some(&params), None::<&()>).await
    }

    /// Get candlestick data
    pub async fn get_candles(
        &self,
        chain_id: &str,
        token_address: &str,
        period: &str, // 1m, 5m, 15m, 1h, 4h, 1d
        limit: u32,
    ) -> Result<Vec<Candle>> {
        let mut params = HashMap::new();
        params.insert("chainId".to_string(), chain_id.to_string());
        params.insert("tokenContractAddress".to_string(), token_address.to_string());
        params.insert("period".to_string(), period.to_string());
        params.insert("limit".to_string(), limit.to_string());

        self.request(Method::GET, "/api/v6/dex/market/candles", Some(&params), None::<&()>).await
    }

    // ============ Wallet/Balance APIs ============

    /// Get token balances for an address
    pub async fn get_balances(&self, chain_id: &str, address: &str) -> Result<Vec<TokenBalance>> {
        let mut params = HashMap::new();
        params.insert("chainId".to_string(), chain_id.to_string());
        params.insert("address".to_string(), address.to_string());

        self.request(Method::GET, "/api/v6/dex/balance/token-balances", Some(&params), None::<&()>).await
    }

    /// Get transaction history
    pub async fn get_transactions(
        &self,
        chain_id: &str,
        address: &str,
        limit: u32,
    ) -> Result<Vec<Transaction>> {
        let mut params = HashMap::new();
        params.insert("chainId".to_string(), chain_id.to_string());
        params.insert("address".to_string(), address.to_string());
        params.insert("limit".to_string(), limit.to_string());

        self.request(Method::GET, "/api/v6/dex/transaction/transactions-by-address", Some(&params), None::<&()>).await
    }

    // ============ Helper Methods ============

    /// Find best swap route across chains
    pub async fn find_best_route(
        &self,
        from_chain: &str,
        to_chain: &str,
        from_token: &str,
        to_token: &str,
        amount: &str,
    ) -> Result<BestRoute> {
        // If same chain, just get quote
        if from_chain == to_chain {
            let quote = self.get_quote(&QuoteRequest {
                chain_id: from_chain.to_string(),
                from_token: from_token.to_string(),
                to_token: to_token.to_string(),
                amount: amount.to_string(),
                slippage: Some("0.5".to_string()),
            }).await?;

            return Ok(BestRoute {
                from_chain: from_chain.to_string(),
                to_chain: to_chain.to_string(),
                route_type: RouteType::SingleChain,
                estimated_output: quote.to_token_amount.clone(),
                price_impact: quote.price_impact.clone(),
                gas_estimate: quote.estimate_gas_fee.clone(),
                steps: vec![RouteStep::Swap {
                    dex: quote.dex_router_list.first()
                        .map(|d| d.router.clone())
                        .unwrap_or_default(),
                    from_token: from_token.to_string(),
                    to_token: to_token.to_string(),
                    amount_in: amount.to_string(),
                    amount_out: quote.to_token_amount,
                }],
            });
        }

        // Cross-chain would require bridge integration
        Err(anyhow!("Cross-chain routing not yet implemented"))
    }

    /// Execute a swap (returns tx data to sign)
    pub async fn prepare_swap(
        &self,
        chain_id: &str,
        from_token: &str,
        to_token: &str,
        amount: &str,
        user_address: &str,
        slippage: &str,
    ) -> Result<PreparedSwap> {
        let swap = self.get_swap(&SwapRequest {
            chain_id: chain_id.to_string(),
            from_token: from_token.to_string(),
            to_token: to_token.to_string(),
            amount: amount.to_string(),
            user_address: user_address.to_string(),
            slippage: slippage.to_string(),
        }).await?;

        Ok(PreparedSwap {
            to: swap.tx.to,
            data: swap.tx.data,
            value: swap.tx.value,
            gas_limit: swap.tx.gas,
            gas_price: swap.tx.gas_price,
            from_amount: swap.from_token_amount,
            to_amount: swap.to_token_amount,
            min_out: swap.min_out_amount,
        })
    }
}

// ============ Request/Response Types ============

#[derive(Debug, Deserialize)]
struct OnchainOSResponse<T> {
    code: String,
    msg: String,
    data: Option<T>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuoteRequest {
    pub chain_id: String,
    pub from_token: String,
    pub to_token: String,
    pub amount: String,
    pub slippage: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub from_token_amount: String,
    pub to_token_amount: String,
    pub price_impact: String,
    pub estimate_gas_fee: String,
    pub dex_router_list: Vec<DexRouter>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DexRouter {
    pub router: String,
    pub router_percent: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapRequest {
    pub chain_id: String,
    pub from_token: String,
    pub to_token: String,
    pub amount: String,
    pub user_address: String,
    pub slippage: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub from_token_amount: String,
    pub to_token_amount: String,
    pub min_out_amount: String,
    pub tx: SwapTx,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapTx {
    pub to: String,
    pub data: String,
    pub value: String,
    pub gas: String,
    pub gas_price: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub token_address: String,
    pub token_symbol: String,
    pub token_name: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquiditySource {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPrice {
    pub price: String,
    pub time: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenMarketData {
    pub token_symbol: String,
    pub token_name: String,
    pub price: String,
    pub price_change_24h: String,
    pub volume_24h: String,
    pub market_cap: String,
    pub holders: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candle {
    pub time: u64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalance {
    pub token_address: String,
    pub token_symbol: String,
    pub balance: String,
    pub balance_usd: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub tx_hash: String,
    pub block_number: u64,
    pub time: u64,
    pub from: String,
    pub to: String,
    pub value: String,
    pub token_address: Option<String>,
    pub method: String,
}

// ============ Helper Types ============

#[derive(Debug, Clone)]
pub struct BestRoute {
    pub from_chain: String,
    pub to_chain: String,
    pub route_type: RouteType,
    pub estimated_output: String,
    pub price_impact: String,
    pub gas_estimate: String,
    pub steps: Vec<RouteStep>,
}

#[derive(Debug, Clone)]
pub enum RouteType {
    SingleChain,
    CrossChain,
}

#[derive(Debug, Clone)]
pub enum RouteStep {
    Swap {
        dex: String,
        from_token: String,
        to_token: String,
        amount_in: String,
        amount_out: String,
    },
    Bridge {
        from_chain: String,
        to_chain: String,
        token: String,
        amount: String,
    },
}

#[derive(Debug, Clone)]
pub struct PreparedSwap {
    pub to: String,
    pub data: String,
    pub value: String,
    pub gas_limit: String,
    pub gas_price: String,
    pub from_amount: String,
    pub to_amount: String,
    pub min_out: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_constants() {
        assert_eq!(chains::XLAYER, "196");
        assert_eq!(chains::ETHEREUM, "1");
        assert_eq!(chains::SOLANA, "501");
    }

    #[test]
    fn test_native_tokens() {
        assert_eq!(native_tokens::EVM_NATIVE, "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
        assert_eq!(native_tokens::SOLANA_NATIVE, "11111111111111111111111111111111");
    }

    #[test]
    fn test_signature_generation() {
        let config = OnchainOSConfig {
            api_key: "test-key".to_string(),
            secret_key: "test-secret".to_string(),
            passphrase: "test-pass".to_string(),
            base_url: ONCHAINOS_BASE_URL.to_string(),
        };
        let client = OnchainOSClient::new(config);

        let sig = client.sign(
            "2024-01-01T00:00:00.000Z",
            "GET",
            "/api/v6/dex/aggregator/quote",
            "",
        );

        // Signature should be base64 encoded
        assert!(sig.len() > 20);
        assert!(BASE64.decode(&sig).is_ok());
    }
}
