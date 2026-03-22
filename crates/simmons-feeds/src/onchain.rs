//! OnchainOS CLI wrapper for DEX data

use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use simmons_core::{PriceTick, Source};
use std::process::Command;
use std::str::FromStr;
use tracing::{debug, error, info};

/// OnchainOS feed wrapper
pub struct OnchainFeed {
    cli_path: String,
}

impl OnchainFeed {
    pub fn new() -> Self {
        Self {
            cli_path: shellexpand::tilde("~/.local/bin/onchainos").to_string(),
        }
    }

    pub fn with_cli_path(cli_path: &str) -> Self {
        Self {
            cli_path: cli_path.to_string(),
        }
    }

    /// Get token price from DEX
    pub async fn get_token_price(&self, chain: &str, token: &str) -> Result<OnchainPrice> {
        let output = Command::new(&self.cli_path)
            .args(["token", "--chain", chain, "--address", token, "--format", "json"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("OnchainOS error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: OnchainTokenResponse = serde_json::from_str(&stdout)?;

        Ok(OnchainPrice {
            symbol: response.symbol,
            price: Decimal::from_str(&response.price).unwrap_or_default(),
            liquidity: Decimal::from_str(&response.liquidity.unwrap_or_default()).ok(),
            volume_24h: Decimal::from_str(&response.volume_24h.unwrap_or_default()).ok(),
            chain: chain.to_string(),
        })
    }

    /// Get market overview
    pub async fn get_market_overview(&self, chain: &str) -> Result<Vec<OnchainPrice>> {
        let output = Command::new(&self.cli_path)
            .args(["market", "--chain", chain, "--format", "json"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("OnchainOS market error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: OnchainMarketResponse = serde_json::from_str(&stdout)?;

        Ok(response
            .tokens
            .into_iter()
            .map(|t| OnchainPrice {
                symbol: t.symbol,
                price: Decimal::from_str(&t.price).unwrap_or_default(),
                liquidity: Decimal::from_str(&t.liquidity.unwrap_or_default()).ok(),
                volume_24h: Decimal::from_str(&t.volume_24h.unwrap_or_default()).ok(),
                chain: chain.to_string(),
            })
            .collect())
    }

    /// Check security for a token
    pub async fn check_security(&self, chain: &str, token: &str) -> Result<SecurityResult> {
        let output = Command::new(&self.cli_path)
            .args(["security", "--chain", chain, "--address", token, "--format", "json"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("OnchainOS security error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: SecurityResponse = serde_json::from_str(&stdout)?;

        Ok(SecurityResult {
            is_honeypot: response.is_honeypot.unwrap_or(false),
            buy_tax: response.buy_tax.and_then(|s| Decimal::from_str(&s).ok()),
            sell_tax: response.sell_tax.and_then(|s| Decimal::from_str(&s).ok()),
            is_mintable: response.is_mintable.unwrap_or(false),
            has_proxy: response.has_proxy.unwrap_or(false),
            risk_score: response.risk_score.unwrap_or(0),
        })
    }

    /// Execute a swap (for live trading)
    pub async fn execute_swap(
        &self,
        chain: &str,
        from_token: &str,
        to_token: &str,
        amount: Decimal,
        slippage: Decimal,
    ) -> Result<SwapResult> {
        let output = Command::new(&self.cli_path)
            .args([
                "swap",
                "--chain", chain,
                "--from", from_token,
                "--to", to_token,
                "--amount", &amount.to_string(),
                "--slippage", &slippage.to_string(),
                "--format", "json",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("OnchainOS swap error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: SwapResponse = serde_json::from_str(&stdout)?;

        Ok(SwapResult {
            tx_hash: response.tx_hash,
            from_amount: Decimal::from_str(&response.from_amount).unwrap_or_default(),
            to_amount: Decimal::from_str(&response.to_amount).unwrap_or_default(),
            price: Decimal::from_str(&response.price).unwrap_or_default(),
            gas_used: response.gas_used,
        })
    }
}

impl Default for OnchainFeed {
    fn default() -> Self {
        Self::new()
    }
}

/// Onchain price data
#[derive(Debug, Clone)]
pub struct OnchainPrice {
    pub symbol: String,
    pub price: Decimal,
    pub liquidity: Option<Decimal>,
    pub volume_24h: Option<Decimal>,
    pub chain: String,
}

/// Security check result
#[derive(Debug, Clone)]
pub struct SecurityResult {
    pub is_honeypot: bool,
    pub buy_tax: Option<Decimal>,
    pub sell_tax: Option<Decimal>,
    pub is_mintable: bool,
    pub has_proxy: bool,
    pub risk_score: u8,
}

impl SecurityResult {
    pub fn is_safe(&self) -> bool {
        !self.is_honeypot
            && self.buy_tax.map_or(true, |t| t < Decimal::from(20))
            && self.sell_tax.map_or(true, |t| t < Decimal::from(20))
            && self.risk_score < 50
    }
}

/// Swap execution result
#[derive(Debug, Clone)]
pub struct SwapResult {
    pub tx_hash: String,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub price: Decimal,
    pub gas_used: Option<String>,
}

// Response types for JSON parsing
#[derive(Debug, Deserialize)]
struct OnchainTokenResponse {
    symbol: String,
    price: String,
    liquidity: Option<String>,
    volume_24h: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OnchainMarketResponse {
    tokens: Vec<OnchainTokenResponse>,
}

#[derive(Debug, Deserialize)]
struct SecurityResponse {
    is_honeypot: Option<bool>,
    buy_tax: Option<String>,
    sell_tax: Option<String>,
    is_mintable: Option<bool>,
    has_proxy: Option<bool>,
    risk_score: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct SwapResponse {
    tx_hash: String,
    from_amount: String,
    to_amount: String,
    price: String,
    gas_used: Option<String>,
}

// Add shellexpand to Cargo.toml dependencies
