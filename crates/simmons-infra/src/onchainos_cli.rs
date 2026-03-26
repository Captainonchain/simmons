//! OnchainOS CLI Wrapper
//!
//! Wraps the onchainos CLI at ~/.local/bin/onchainos for:
//! - Market data (prices, klines, portfolio)
//! - Token info (search, info, price-info)
//! - Swap execution (quote, swap, approve)
//! - Signals (smart money, whale, KOL)

use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// OnchainOS CLI wrapper
pub struct OnchainOSCli {
    cli_path: String,
    default_chain: String,
}

impl Default for OnchainOSCli {
    fn default() -> Self {
        Self::new()
    }
}

impl OnchainOSCli {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/sandeep".to_string());
        Self {
            cli_path: format!("{}/.local/bin/onchainos", home),
            default_chain: "xlayer".to_string(),
        }
    }

    pub fn with_chain(mut self, chain: &str) -> Self {
        self.default_chain = chain.to_string();
        self
    }

    /// Execute CLI command and parse JSON output
    async fn run_command(&self, args: &[&str]) -> Result<String> {
        debug!("onchainos {}", args.join(" "));

        let output = Command::new(&self.cli_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute onchainos CLI: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("onchainos error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    /// Parse JSON output from CLI
    fn parse_json<T: for<'de> Deserialize<'de>>(&self, output: &str) -> Result<T> {
        // CLI output may have non-JSON prefix, find the JSON part
        let json_start = output.find('{').or_else(|| output.find('['));
        let json_str = match json_start {
            Some(idx) => &output[idx..],
            None => output,
        };

        // First try to parse directly
        if let Ok(result) = serde_json::from_str::<T>(json_str) {
            return Ok(result);
        }

        // Try to parse as wrapper object { "ok": true, "data": ... }
        if let Ok(wrapper) = serde_json::from_str::<CliResponse<T>>(json_str) {
            if wrapper.ok {
                return Ok(wrapper.data);
            } else {
                return Err(anyhow!("CLI returned error response"));
            }
        }

        // Try to parse as array wrapper { "ok": true, "data": [...] } where we want first element
        if let Ok(wrapper) = serde_json::from_str::<CliArrayResponse<T>>(json_str) {
            if wrapper.ok {
                return wrapper
                    .data
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("Empty data array in response"));
            }
        }

        Err(anyhow!("Failed to parse JSON: {}", json_str))
    }

    // ============ Market Data ============

    /// Get token price by address
    pub async fn get_price(&self, token_address: &str, chain: Option<&str>) -> Result<TokenPrice> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["market", "price", "--chain", chain, "--address", token_address])
            .await?;
        self.parse_json(&output)
    }

    /// Get index price (aggregated from multiple sources)
    pub async fn get_index_price(&self, token_address: &str, chain: Option<&str>) -> Result<IndexPrice> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["market", "index", "--chain", chain, "--address", token_address])
            .await?;
        self.parse_json(&output)
    }

    /// Get kline/candlestick data
    pub async fn get_kline(
        &self,
        token_address: &str,
        period: &str, // 1m, 5m, 15m, 1h, 4h, 1d
        limit: u32,
        chain: Option<&str>,
    ) -> Result<Vec<Candle>> {
        let chain = chain.unwrap_or(&self.default_chain);
        let limit_str = limit.to_string();
        let output = self
            .run_command(&[
                "market",
                "kline",
                "--chain",
                chain,
                "--address",
                token_address,
                "--period",
                period,
                "--limit",
                &limit_str,
            ])
            .await?;
        self.parse_json(&output)
    }

    // ============ Token Info ============

    /// Search for tokens
    pub async fn search_tokens(&self, query: &str, chain: Option<&str>) -> Result<Vec<TokenSearchResult>> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["token", "search", "--chain", chain, "--query", query])
            .await?;
        self.parse_json(&output)
    }

    /// Get token info
    pub async fn get_token_info(&self, token_address: &str, chain: Option<&str>) -> Result<TokenInfo> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["token", "info", "--chain", chain, "--address", token_address])
            .await?;
        self.parse_json(&output)
    }

    /// Get detailed price info with market cap, liquidity, volume
    pub async fn get_price_info(&self, token_address: &str, chain: Option<&str>) -> Result<TokenPriceInfo> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["token", "price-info", "--chain", chain, "--address", token_address])
            .await?;
        self.parse_json(&output)
    }

    /// Get trending tokens
    pub async fn get_trending_tokens(&self, chain: Option<&str>) -> Result<Vec<TrendingToken>> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["token", "trending", "--chain", chain])
            .await?;
        self.parse_json(&output)
    }

    // ============ Swap ============

    /// Get swap quote
    pub async fn get_swap_quote(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        chain: Option<&str>,
    ) -> Result<SwapQuote> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&[
                "swap",
                "quote",
                "--chain",
                chain,
                "--from",
                from_token,
                "--to",
                to_token,
                "--amount",
                amount,
            ])
            .await?;
        self.parse_json(&output)
    }

    /// Get swap transaction data
    pub async fn get_swap_tx(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        slippage: &str,
        chain: Option<&str>,
    ) -> Result<SwapTx> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&[
                "swap",
                "swap",
                "--chain",
                chain,
                "--from",
                from_token,
                "--to",
                to_token,
                "--amount",
                amount,
                "--slippage",
                slippage,
            ])
            .await?;
        self.parse_json(&output)
    }

    /// Get ERC-20 approval transaction
    pub async fn get_approve_tx(
        &self,
        token: &str,
        spender: &str,
        amount: &str,
        chain: Option<&str>,
    ) -> Result<ApproveTx> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&[
                "swap",
                "approve",
                "--chain",
                chain,
                "--token",
                token,
                "--spender",
                spender,
                "--amount",
                amount,
            ])
            .await?;
        self.parse_json(&output)
    }

    /// Get supported chains for DEX
    pub async fn get_supported_chains(&self) -> Result<Vec<ChainInfo>> {
        let output = self.run_command(&["swap", "chains"]).await?;
        self.parse_json(&output)
    }

    /// Get liquidity sources on a chain
    pub async fn get_liquidity_sources(&self, chain: Option<&str>) -> Result<Vec<LiquiditySource>> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["swap", "liquidity", "--chain", chain])
            .await?;
        self.parse_json(&output)
    }

    // ============ Signals (Smart Money / Whale / KOL) ============

    /// Get smart money / whale / KOL signals
    pub async fn get_signals(&self, chain: Option<&str>) -> Result<Vec<Signal>> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["signal", "list", "--chain", chain])
            .await?;
        self.parse_json(&output)
    }

    /// Get supported chains for signals
    pub async fn get_signal_chains(&self) -> Result<Vec<String>> {
        let output = self.run_command(&["signal", "chains"]).await?;
        self.parse_json(&output)
    }

    // ============ Portfolio ============

    /// Get portfolio overview for a wallet
    pub async fn get_portfolio_overview(
        &self,
        wallet_address: &str,
        chain: Option<&str>,
    ) -> Result<PortfolioOverview> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&[
                "market",
                "portfolio-overview",
                "--chain",
                chain,
                "--address",
                wallet_address,
            ])
            .await?;
        self.parse_json(&output)
    }

    /// Get wallet balances
    pub async fn get_balances(&self, wallet_address: &str, chain: Option<&str>) -> Result<Vec<TokenBalance>> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["portfolio", "--chain", chain, "--address", wallet_address])
            .await?;
        self.parse_json(&output)
    }

    // ============ Wallet Operations ============

    /// Check wallet status
    pub async fn wallet_status(&self) -> Result<WalletStatus> {
        let output = self.run_command(&["wallet", "status"]).await?;
        self.parse_json(&output)
    }

    /// Get wallet balance
    pub async fn wallet_balance(&self, chain: Option<&str>) -> Result<WalletBalance> {
        let chain = chain.unwrap_or(&self.default_chain);
        let output = self
            .run_command(&["wallet", "balance", "--chain", chain])
            .await?;
        self.parse_json(&output)
    }
}

// ============ Response Types ============

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPrice {
    pub price: String,
    #[serde(alias = "time")]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub chain_index: Option<String>,
    #[serde(default)]
    pub token_contract_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexPrice {
    pub price: String,
    pub sources: Vec<PriceSource>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceSource {
    pub name: String,
    pub price: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candle {
    pub time: u64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSearchResult {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub chain: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub logo_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPriceInfo {
    pub price: String,
    #[serde(alias = "priceChange24H")]
    pub price_change_24h: Option<String>,
    pub market_cap: Option<String>,
    pub liquidity: Option<String>,
    #[serde(alias = "volume24H")]
    pub volume_24h: Option<String>,
    // Additional fields from CLI
    #[serde(default)]
    pub chain_index: Option<String>,
    #[serde(default)]
    pub token_contract_address: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendingToken {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub price: String,
    pub price_change_24h: Option<String>,
    pub trending_score: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapQuote {
    pub from_token_amount: String,
    pub to_token_amount: String,
    pub price_impact: Option<String>,
    pub estimate_gas_fee: Option<String>,
    pub routes: Option<Vec<SwapRoute>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRoute {
    pub dex: String,
    pub percent: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapTx {
    pub to: String,
    pub data: String,
    pub value: String,
    pub gas: Option<String>,
    pub gas_price: Option<String>,
    pub from_amount: String,
    pub to_amount: String,
    pub min_out_amount: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveTx {
    pub to: String,
    pub data: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainInfo {
    pub chain_id: String,
    pub name: String,
    pub short_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquiditySource {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
    pub signal_type: String, // smart_money, whale, kol
    pub address: String,
    pub token_address: Option<String>,
    pub token_symbol: Option<String>,
    pub action: String, // buy, sell
    pub amount_usd: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioOverview {
    pub total_value_usd: String,
    pub realized_pnl: Option<String>,
    pub unrealized_pnl: Option<String>,
    pub win_rate: Option<String>,
    pub total_trades: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBalance {
    pub token_address: String,
    pub token_symbol: String,
    pub balance: String,
    pub balance_usd: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletStatus {
    pub logged_in: bool,
    pub address: Option<String>,
    pub chain: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletBalance {
    pub native_balance: String,
    pub native_balance_usd: Option<String>,
    pub tokens: Vec<TokenBalance>,
}

// ============ Internal Helper Types ============

/// CLI wrapper response for single object
#[derive(Debug, Deserialize)]
struct CliResponse<T> {
    ok: bool,
    data: T,
}

/// CLI wrapper response for array
#[derive(Debug, Deserialize)]
struct CliArrayResponse<T> {
    ok: bool,
    data: Vec<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_path() {
        let cli = OnchainOSCli::new();
        assert!(cli.cli_path.ends_with("onchainos"));
        assert_eq!(cli.default_chain, "xlayer");
    }

    #[test]
    fn test_with_chain() {
        let cli = OnchainOSCli::new().with_chain("ethereum");
        assert_eq!(cli.default_chain, "ethereum");
    }
}
