//! DEX Execution module using OnchainOS
//!
//! Provides security scanning and swap execution via onchainos MCP.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported chains for DEX trading
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Solana,
    Ethereum,
    Base,
    Bsc,
    Arbitrum,
    Polygon,
    Avalanche,
    Optimism,
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chain::Solana => write!(f, "solana"),
            Chain::Ethereum => write!(f, "ethereum"),
            Chain::Base => write!(f, "base"),
            Chain::Bsc => write!(f, "bsc"),
            Chain::Arbitrum => write!(f, "arbitrum"),
            Chain::Polygon => write!(f, "polygon"),
            Chain::Avalanche => write!(f, "avalanche"),
            Chain::Optimism => write!(f, "optimism"),
        }
    }
}

impl Chain {
    /// Get the native token address for this chain
    pub fn native_token(&self) -> &'static str {
        match self {
            Chain::Solana => "So11111111111111111111111111111111111111112",
            _ => "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        }
    }

    /// Get common stablecoin address for this chain
    pub fn usdc_address(&self) -> &'static str {
        match self {
            Chain::Solana => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            Chain::Ethereum => "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
            Chain::Base => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            Chain::Bsc => "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",
            Chain::Arbitrum => "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
            Chain::Polygon => "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359",
            Chain::Avalanche => "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E",
            Chain::Optimism => "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
        }
    }
}

/// Token security scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResult {
    pub token_address: String,
    pub chain: String,
    pub is_safe: bool,
    pub risk_level: SecurityRiskLevel,
    pub checks: SecurityChecks,
    pub warnings: Vec<String>,
    pub block_reasons: Vec<String>,
}

/// Security risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityRiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

/// Individual security checks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityChecks {
    pub is_honeypot: bool,
    pub buy_tax_pct: Option<Decimal>,
    pub sell_tax_pct: Option<Decimal>,
    pub is_open_source: bool,
    pub can_take_back_ownership: bool,
    pub can_change_balance: bool,
    pub is_mintable: bool,
    pub has_blacklist: bool,
    pub has_whitelist: bool,
    pub is_proxy: bool,
    pub liquidity_locked: bool,
    pub top_holder_pct: Option<Decimal>,
}

impl SecurityChecks {
    /// Check if this token should be blocked from trading
    pub fn should_block(&self) -> Option<String> {
        if self.is_honeypot {
            return Some("Honeypot detected".to_string());
        }
        if let Some(buy_tax) = self.buy_tax_pct {
            if buy_tax > Decimal::from(20) {
                return Some(format!("Buy tax too high: {}%", buy_tax));
            }
        }
        if let Some(sell_tax) = self.sell_tax_pct {
            if sell_tax > Decimal::from(20) {
                return Some(format!("Sell tax too high: {}%", sell_tax));
            }
        }
        if self.can_take_back_ownership {
            return Some("Owner can reclaim ownership".to_string());
        }
        if self.can_change_balance {
            return Some("Contract can modify balances".to_string());
        }
        None
    }

    /// Get warnings for this token (not blocking but concerning)
    pub fn get_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if let Some(buy_tax) = self.buy_tax_pct {
            if buy_tax > Decimal::from(5) && buy_tax <= Decimal::from(20) {
                warnings.push(format!("Moderate buy tax: {}%", buy_tax));
            }
        }
        if let Some(sell_tax) = self.sell_tax_pct {
            if sell_tax > Decimal::from(5) && sell_tax <= Decimal::from(20) {
                warnings.push(format!("Moderate sell tax: {}%", sell_tax));
            }
        }
        if !self.is_open_source {
            warnings.push("Contract is not open source".to_string());
        }
        if self.is_mintable {
            warnings.push("Token is mintable".to_string());
        }
        if self.has_blacklist {
            warnings.push("Contract has blacklist function".to_string());
        }
        if !self.liquidity_locked {
            warnings.push("Liquidity not locked".to_string());
        }
        if let Some(top_pct) = self.top_holder_pct {
            if top_pct > Decimal::from(50) {
                warnings.push(format!("High holder concentration: {}%", top_pct));
            }
        }

        warnings
    }
}

/// Swap quote result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub chain: String,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: String,
    pub to_amount_min: String,
    pub slippage_pct: Decimal,
    pub price_impact_pct: Option<Decimal>,
    pub gas_estimate: Option<String>,
    pub route: Vec<String>,
    pub expires_at: Option<i64>,
}

/// Swap execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    pub success: bool,
    pub tx_hash: Option<String>,
    pub from_amount: String,
    pub to_amount: String,
    pub error: Option<String>,
    pub gas_used: Option<String>,
    pub block_number: Option<u64>,
}

/// DEX execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    /// Default slippage tolerance
    pub default_slippage_pct: Decimal,
    /// Maximum allowed slippage
    pub max_slippage_pct: Decimal,
    /// Require security scan before trade
    pub require_security_scan: bool,
    /// Block trades on any security warning
    pub strict_security: bool,
    /// Maximum price impact allowed
    pub max_price_impact_pct: Decimal,
}

impl Default for DexConfig {
    fn default() -> Self {
        Self {
            default_slippage_pct: Decimal::from_str_exact("0.5").unwrap(),
            max_slippage_pct: Decimal::from_str_exact("3.0").unwrap(),
            require_security_scan: true,
            strict_security: false,
            max_price_impact_pct: Decimal::from_str_exact("5.0").unwrap(),
        }
    }
}

/// Pre-trade check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreTradeCheck {
    pub passed: bool,
    pub security_scan: Option<SecurityScanResult>,
    pub quote: Option<SwapQuote>,
    pub block_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub recommended_slippage: Decimal,
}

impl PreTradeCheck {
    /// Create a failed pre-trade check
    pub fn failed(reasons: Vec<String>) -> Self {
        Self {
            passed: false,
            security_scan: None,
            quote: None,
            block_reasons: reasons,
            warnings: Vec::new(),
            recommended_slippage: Decimal::from_str_exact("0.5").unwrap(),
        }
    }

    /// Create a passed pre-trade check
    pub fn passed(security: SecurityScanResult, quote: SwapQuote, warnings: Vec<String>) -> Self {
        let recommended_slippage = if let Some(impact) = quote.price_impact_pct {
            // Recommend slippage slightly higher than price impact
            (impact + Decimal::from_str_exact("0.5").unwrap()).min(Decimal::from(3))
        } else {
            Decimal::from_str_exact("0.5").unwrap()
        };

        Self {
            passed: true,
            security_scan: Some(security),
            quote: Some(quote),
            block_reasons: Vec::new(),
            warnings,
            recommended_slippage,
        }
    }
}

/// OnchainOS integration commands
///
/// These map to the onchainos CLI commands that would be called via MCP
pub struct OnchainOSCommands;

impl OnchainOSCommands {
    /// Generate security scan command
    pub fn security_token_scan(chain: &str, token: &str) -> String {
        format!(
            r#"{{"method": "security_token_scan", "params": {{"chain": "{}", "tokens": ["{}"]}}}}"#,
            chain, token
        )
    }

    /// Generate swap quote command
    pub fn swap_quote(
        chain: &str,
        from_token: &str,
        to_token: &str,
        amount: &str,
        slippage: &str,
    ) -> String {
        format!(
            r#"{{"method": "swap_quote", "params": {{"chain": "{}", "from_token": "{}", "to_token": "{}", "amount": "{}", "slippage": "{}"}}}}"#,
            chain, from_token, to_token, amount, slippage
        )
    }

    /// Generate swap execute command
    pub fn swap_execute(
        chain: &str,
        from_token: &str,
        to_token: &str,
        amount: &str,
        slippage: &str,
    ) -> String {
        format!(
            r#"{{"method": "swap_swap", "params": {{"chain": "{}", "from_token": "{}", "to_token": "{}", "amount": "{}", "slippage": "{}"}}}}"#,
            chain, from_token, to_token, amount, slippage
        )
    }

    /// Generate smart money signal command
    pub fn signal_list(chain: &str, limit: u32) -> String {
        format!(
            r#"{{"method": "signal_list", "params": {{"chain": "{}", "limit": {}}}}}"#,
            chain, limit
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_security_checks_block() {
        let mut checks = SecurityChecks::default();
        assert!(checks.should_block().is_none());

        checks.is_honeypot = true;
        assert!(checks.should_block().is_some());

        checks.is_honeypot = false;
        checks.buy_tax_pct = Some(dec!(25));
        assert!(checks.should_block().is_some());
    }

    #[test]
    fn test_security_checks_warnings() {
        let mut checks = SecurityChecks::default();
        checks.buy_tax_pct = Some(dec!(10));
        checks.is_mintable = true;

        let warnings = checks.get_warnings();
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_chain_addresses() {
        assert_eq!(Chain::Solana.native_token(), "So11111111111111111111111111111111111111112");
        assert_eq!(Chain::Ethereum.native_token(), "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    }
}
