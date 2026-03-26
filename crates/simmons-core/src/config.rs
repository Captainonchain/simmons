//! Configuration management for Simmons

use crate::types::TradingMode;
use crate::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mode: TradingMode,
    pub capital_usd: Decimal,
    pub symbols: Vec<String>,
    pub risk: RiskConfig,
    pub feeds: FeedsConfig,
    pub alpha: AlphaConfig,
    pub execution: ExecutionConfig,
    pub brain: BrainConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: TradingMode::Paper,
            capital_usd: Decimal::from(100),
            symbols: vec!["BTC-USDT".to_string(), "ETH-USDT".to_string()],
            risk: RiskConfig::default(),
            feeds: FeedsConfig::default(),
            alpha: AlphaConfig::default(),
            execution: ExecutionConfig::default(),
            brain: BrainConfig::default(),
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        Self::load(path).unwrap_or_default()
    }
}

/// Risk management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Maximum position size as fraction of capital (0.15 = 15%)
    pub max_position_pct: Decimal,
    /// Maximum drawdown before halting (0.20 = 20%)
    pub max_drawdown: Decimal,
    /// Kelly criterion fraction (0.25 = quarter Kelly)
    pub kelly_fraction: Decimal,
    /// Maximum daily loss in USD
    pub daily_loss_limit: Decimal,
    /// Default stop loss percentage (0.03 = 3%)
    pub default_stop_loss_pct: Decimal,
    /// Default take profit percentage (0.08 = 8%)
    pub default_take_profit_pct: Decimal,
    /// Maximum concurrent positions
    pub max_positions: usize,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_pct: Decimal::new(15, 2),    // 0.15
            max_drawdown: Decimal::new(20, 2),        // 0.20
            kelly_fraction: Decimal::new(25, 2),      // 0.25
            daily_loss_limit: Decimal::from(50),      // $50
            default_stop_loss_pct: Decimal::new(3, 2), // 0.03
            default_take_profit_pct: Decimal::new(8, 2), // 0.08
            max_positions: 5,
        }
    }
}

/// Feeds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedsConfig {
    /// OKX WebSocket URL
    pub okx_ws_url: String,
    /// Price window size for calculations
    pub price_window_size: usize,
    /// Update interval in milliseconds
    pub update_interval_ms: u64,
    /// Enable X Layer DEX feed
    #[serde(default = "default_true")]
    pub xlayer_enabled: bool,
    /// X Layer poll interval in milliseconds
    #[serde(default = "default_xlayer_poll")]
    pub xlayer_poll_interval_ms: u64,
    /// Enable news/sentiment feed
    #[serde(default = "default_true")]
    pub news_enabled: bool,
    /// Chains to track for signals
    #[serde(default = "default_signal_chains")]
    pub signal_chains: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_xlayer_poll() -> u64 {
    2000
}

fn default_signal_chains() -> Vec<String> {
    vec!["xlayer".to_string(), "ethereum".to_string()]
}

impl Default for FeedsConfig {
    fn default() -> Self {
        Self {
            okx_ws_url: "wss://ws.okx.com:8443/ws/v5/public".to_string(),
            price_window_size: 100,
            update_interval_ms: 100,
            xlayer_enabled: true,
            xlayer_poll_interval_ms: 2000,
            news_enabled: true,
            signal_chains: default_signal_chains(),
        }
    }
}

/// Alpha/signal generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaConfig {
    /// Momentum lookback period
    pub momentum_lookback: usize,
    /// RSI period
    pub rsi_period: usize,
    /// RSI overbought threshold
    pub rsi_overbought: Decimal,
    /// RSI oversold threshold
    pub rsi_oversold: Decimal,
    /// Mean reversion z-score threshold
    pub zscore_threshold: Decimal,
    /// Minimum confidence for signals
    pub min_confidence: Decimal,
}

impl Default for AlphaConfig {
    fn default() -> Self {
        Self {
            momentum_lookback: 14,
            rsi_period: 14,
            rsi_overbought: Decimal::from(70),
            rsi_oversold: Decimal::from(30),
            zscore_threshold: Decimal::new(2, 0),
            min_confidence: Decimal::new(60, 2), // 0.60
        }
    }
}

/// Execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum slippage in basis points
    pub max_slippage_bps: Decimal,
    /// Split threshold in USD (orders above this are split)
    pub split_threshold_usd: Decimal,
    /// MEV protection enabled
    pub mev_protection: bool,
    /// Preferred venues in order
    pub preferred_venues: Vec<String>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_slippage_bps: Decimal::from(50),
            split_threshold_usd: Decimal::from(1000),
            mev_protection: true,
            preferred_venues: vec!["okx".to_string()],
        }
    }
}

/// Brain (Claude) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainConfig {
    /// Data directory for signal/decision files
    pub data_dir: String,
    /// Timeout waiting for Claude decision in seconds
    pub timeout_secs: u64,
    /// Auto-invoke Claude (vs waiting for user)
    pub auto_invoke: bool,
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            data_dir: "data".to_string(),
            timeout_secs: 60,
            auto_invoke: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.mode, TradingMode::Paper);
        assert_eq!(config.capital_usd, dec!(100));
        assert_eq!(config.risk.max_position_pct, dec!(0.15));
    }
}
