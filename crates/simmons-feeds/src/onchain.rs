//! OnchainOS CLI wrapper for DEX data

use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
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

/// Smart money signal from OnchainOS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartMoneySignal {
    /// Signal type (smart_money, whale, kol)
    pub signal_type: String,
    /// Action (buy, sell, transfer)
    pub action: String,
    /// Token symbol
    pub token_symbol: Option<String>,
    /// Token address
    pub token_address: Option<String>,
    /// USD amount
    pub amount_usd: Option<Decimal>,
    /// Wallet address
    pub wallet_address: Option<String>,
    /// Chain
    pub chain: String,
    /// Timestamp
    pub timestamp: i64,
}

/// Whale activity summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhaleActivity {
    /// Token symbol
    pub token: String,
    /// Chain
    pub chain: String,
    /// Total buy volume USD in period
    pub buy_volume_usd: Decimal,
    /// Total sell volume USD in period
    pub sell_volume_usd: Decimal,
    /// Net flow (buy - sell)
    pub net_flow_usd: Decimal,
    /// Number of unique whale wallets
    pub unique_wallets: u32,
    /// Individual signals
    pub signals: Vec<SmartMoneySignal>,
    /// Sentiment (-1 to +1)
    pub sentiment: Decimal,
}

/// Meme token from OnchainOS trenches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemeToken {
    /// Token symbol
    pub symbol: String,
    /// Token address
    pub address: String,
    /// Chain
    pub chain: String,
    /// Market cap USD
    pub market_cap_usd: Option<Decimal>,
    /// Liquidity USD
    pub liquidity_usd: Option<Decimal>,
    /// 24h volume
    pub volume_24h: Option<Decimal>,
    /// Age in hours
    pub age_hours: Option<u32>,
    /// Creator/dev address
    pub dev_address: Option<String>,
    /// Dev reputation score (0-100)
    pub dev_reputation: Option<u8>,
    /// Risk score (0-100)
    pub risk_score: u8,
}

impl OnchainFeed {
    /// Get smart money signals from OnchainOS
    pub async fn get_smart_money_signals(&self, chain: &str, limit: usize) -> Result<Vec<SmartMoneySignal>> {
        let output = Command::new(&self.cli_path)
            .args([
                "signal",
                "--chain", chain,
                "--limit", &limit.to_string(),
                "--format", "json",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Return empty vec on error rather than failing
            debug!("OnchainOS signal error: {}", stderr);
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: SignalListResponse = serde_json::from_str(&stdout)
            .unwrap_or(SignalListResponse { signals: Vec::new() });

        Ok(response.signals.into_iter().map(|s| SmartMoneySignal {
            signal_type: s.signal_type,
            action: s.action,
            token_symbol: s.token_symbol,
            token_address: s.token_address,
            amount_usd: s.amount_usd.and_then(|a| Decimal::from_str(&a).ok()),
            wallet_address: s.wallet_address,
            chain: chain.to_string(),
            timestamp: s.timestamp.unwrap_or(0),
        }).collect())
    }

    /// Get whale activity for a specific token
    pub async fn get_whale_activity(&self, chain: &str, token: &str, hours: u32) -> Result<WhaleActivity> {
        let signals = self.get_smart_money_signals(chain, 100).await?;

        let cutoff = chrono::Utc::now().timestamp() - (hours as i64 * 3600);

        // Filter to relevant token and time window
        let filtered: Vec<SmartMoneySignal> = signals
            .into_iter()
            .filter(|s| {
                s.timestamp > cutoff &&
                (s.token_symbol.as_ref().map_or(false, |sym| sym.eq_ignore_ascii_case(token)) ||
                 s.token_address.as_ref().map_or(false, |addr| addr.eq_ignore_ascii_case(token)))
            })
            .collect();

        let mut buy_volume = Decimal::ZERO;
        let mut sell_volume = Decimal::ZERO;
        let mut unique_wallets = std::collections::HashSet::new();

        for signal in &filtered {
            if let Some(addr) = &signal.wallet_address {
                unique_wallets.insert(addr.clone());
            }
            if let Some(amount) = signal.amount_usd {
                match signal.action.as_str() {
                    "buy" => buy_volume += amount,
                    "sell" => sell_volume += amount,
                    _ => {}
                }
            }
        }

        let net_flow = buy_volume - sell_volume;
        let total = buy_volume + sell_volume;
        let sentiment = if total.is_zero() {
            Decimal::ZERO
        } else {
            (net_flow / total).min(Decimal::ONE).max(dec!(-1))
        };

        Ok(WhaleActivity {
            token: token.to_string(),
            chain: chain.to_string(),
            buy_volume_usd: buy_volume,
            sell_volume_usd: sell_volume,
            net_flow_usd: net_flow,
            unique_wallets: unique_wallets.len() as u32,
            signals: filtered,
            sentiment,
        })
    }

    /// Comprehensive security scan using OnchainOS
    pub async fn scan_security(&self, chain: &str, token: &str) -> Result<SecurityResult> {
        // Use the existing check_security method
        self.check_security(chain, token).await
    }

    /// Get new meme tokens from OnchainOS trenches
    pub async fn get_meme_tokens(&self, chain: &str, limit: usize) -> Result<Vec<MemeToken>> {
        let output = Command::new(&self.cli_path)
            .args([
                "trenches",
                "--chain", chain,
                "--limit", &limit.to_string(),
                "--format", "json",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("OnchainOS trenches error: {}", stderr);
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: TrenchesResponse = serde_json::from_str(&stdout)
            .unwrap_or(TrenchesResponse { tokens: Vec::new() });

        Ok(response.tokens.into_iter().map(|t| MemeToken {
            symbol: t.symbol,
            address: t.address,
            chain: chain.to_string(),
            market_cap_usd: t.market_cap.and_then(|m| Decimal::from_str(&m).ok()),
            liquidity_usd: t.liquidity.and_then(|l| Decimal::from_str(&l).ok()),
            volume_24h: t.volume_24h.and_then(|v| Decimal::from_str(&v).ok()),
            age_hours: t.age_hours,
            dev_address: t.dev_address,
            dev_reputation: t.dev_reputation,
            risk_score: t.risk_score.unwrap_or(50),
        }).collect())
    }

    /// Get multiple token prices in batch
    pub async fn get_prices_batch(&self, chain: &str, tokens: &[&str]) -> Result<Vec<OnchainPrice>> {
        let mut prices = Vec::new();
        for token in tokens {
            match self.get_token_price(chain, token).await {
                Ok(price) => prices.push(price),
                Err(e) => debug!("Failed to get price for {}: {}", token, e),
            }
        }
        Ok(prices)
    }
}

// Response types for JSON parsing
#[derive(Debug, Deserialize)]
struct SignalListResponse {
    signals: Vec<SignalItem>,
}

#[derive(Debug, Deserialize)]
struct SignalItem {
    signal_type: String,
    action: String,
    token_symbol: Option<String>,
    token_address: Option<String>,
    amount_usd: Option<String>,
    wallet_address: Option<String>,
    timestamp: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TrenchesResponse {
    tokens: Vec<TrenchesToken>,
}

#[derive(Debug, Deserialize)]
struct TrenchesToken {
    symbol: String,
    address: String,
    market_cap: Option<String>,
    liquidity: Option<String>,
    volume_24h: Option<String>,
    age_hours: Option<u32>,
    dev_address: Option<String>,
    dev_reputation: Option<u8>,
    risk_score: Option<u8>,
}

// Extra types used by new methods
