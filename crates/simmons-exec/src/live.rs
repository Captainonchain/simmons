//! Live Execution Engine
//!
//! Handles actual trade execution on OKX CEX and X Layer DEX.

use crate::okx_api::{OkxApiClient, OkxApiConfig, OrderSide as OkxSide};
use crate::signer::{SignerConfig, TxSigner};
use anyhow::{anyhow, Result};
use ethers::prelude::*;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_core::{ExecutionResult, Order, OrderType, Side, Trade, TradeOutcome};
use simmons_risk::Portfolio;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

/// Execution venue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Venue {
    /// OKX centralized exchange
    OkxCex,
    /// X Layer DEX (via OnchainOS)
    XLayerDex,
    /// Cod3x lending protocol
    Cod3xLending,
}

impl Venue {
    pub fn from_symbol(symbol: &str) -> Self {
        // CEX pairs typically have -USDT suffix
        if symbol.contains("-USDT") || symbol.contains("-USDC") {
            Venue::OkxCex
        } else {
            // On-chain tokens go to DEX
            Venue::XLayerDex
        }
    }
}

/// Live executor configuration
#[derive(Debug, Clone)]
pub struct LiveExecutorConfig {
    /// Enable OKX CEX trading
    pub okx_enabled: bool,
    /// Enable X Layer DEX trading
    pub dex_enabled: bool,
    /// Maximum slippage in basis points
    pub max_slippage_bps: u32,
    /// Default timeout for order fills (seconds)
    pub order_timeout_secs: u64,
    /// Minimum order size in USD
    pub min_order_usd: Decimal,
    /// Maximum order size in USD
    pub max_order_usd: Decimal,
    /// Number of confirmations to wait for DEX trades
    pub dex_confirmations: usize,
}

impl Default for LiveExecutorConfig {
    fn default() -> Self {
        Self {
            okx_enabled: true,
            dex_enabled: true,
            max_slippage_bps: 100, // 1%
            order_timeout_secs: 60,
            min_order_usd: dec!(1),
            max_order_usd: dec!(10000),
            dex_confirmations: 2,
        }
    }
}

/// Live execution engine
pub struct LiveExecutor {
    config: LiveExecutorConfig,
    okx_client: Option<OkxApiClient>,
    signer: Option<TxSigner>,
    portfolio: Arc<Portfolio>,
    /// Pending orders by symbol
    pending_orders: parking_lot::RwLock<HashMap<String, PendingOrder>>,
}

#[derive(Debug, Clone)]
struct PendingOrder {
    order_id: String,
    venue: Venue,
    submitted_at: Instant,
}

impl LiveExecutor {
    /// Create new live executor
    pub fn new(config: LiveExecutorConfig, portfolio: Arc<Portfolio>) -> Self {
        Self {
            config,
            okx_client: None,
            signer: None,
            portfolio,
            pending_orders: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Initialize OKX client from environment
    pub fn with_okx(mut self) -> Result<Self> {
        if self.config.okx_enabled {
            let okx_config = OkxApiConfig::from_env()?;
            self.okx_client = Some(OkxApiClient::new(okx_config));
            info!("OKX API client initialized");
        }
        Ok(self)
    }

    /// Initialize X Layer signer from environment
    pub fn with_xlayer_signer(mut self, testnet: bool) -> Result<Self> {
        if self.config.dex_enabled {
            let signer_config = if testnet {
                SignerConfig::testnet()
            } else {
                SignerConfig::default()
            };
            self.signer = Some(TxSigner::from_env(signer_config)?);
            info!("X Layer signer initialized");
        }
        Ok(self)
    }

    /// Check if executor is ready for live trading
    pub fn is_ready(&self) -> bool {
        (self.config.okx_enabled && self.okx_client.is_some())
            || (self.config.dex_enabled && self.signer.is_some())
    }

    /// Get available venues
    pub fn available_venues(&self) -> Vec<Venue> {
        let mut venues = Vec::new();
        if self.okx_client.is_some() {
            venues.push(Venue::OkxCex);
        }
        if self.signer.is_some() {
            venues.push(Venue::XLayerDex);
            venues.push(Venue::Cod3xLending);
        }
        venues
    }

    /// Execute an order
    pub async fn execute(
        &self,
        order: &Order,
        market_price: Decimal,
    ) -> Result<ExecutionResult> {
        let start = Instant::now();

        // Validate order
        self.validate_order(order, market_price)?;

        // Determine venue
        let venue = Venue::from_symbol(&order.symbol);

        info!(
            "Live execution: {:?} {} {} @ {} via {:?}",
            order.side, order.size, order.symbol, market_price, venue
        );

        // Execute based on venue
        let result = match venue {
            Venue::OkxCex => self.execute_okx(order, market_price).await,
            Venue::XLayerDex => self.execute_dex(order, market_price).await,
            Venue::Cod3xLending => {
                Err(anyhow!("Use Cod3xExecutor for lending operations"))
            }
        };

        match &result {
            Ok(exec) => {
                info!(
                    "Execution complete: filled {} @ ${} ({}ms)",
                    exec.size,
                    exec.price,
                    start.elapsed().as_millis()
                );
            }
            Err(e) => {
                error!("Execution failed: {}", e);
            }
        }

        result
    }

    /// Validate order before execution
    fn validate_order(&self, order: &Order, market_price: Decimal) -> Result<()> {
        // Check size
        if order.size <= Decimal::ZERO {
            return Err(anyhow!("Order size must be positive"));
        }

        // Check notional value
        let notional = order.size * market_price;
        if notional < self.config.min_order_usd {
            return Err(anyhow!(
                "Order too small: ${} < ${}",
                notional,
                self.config.min_order_usd
            ));
        }
        if notional > self.config.max_order_usd {
            return Err(anyhow!(
                "Order too large: ${} > ${}",
                notional,
                self.config.max_order_usd
            ));
        }

        // Check limit price if set
        if let Some(limit_price) = order.limit_price {
            let deviation = ((limit_price - market_price) / market_price).abs();
            let max_deviation = Decimal::from(self.config.max_slippage_bps) / dec!(10000);
            if deviation > max_deviation * dec!(2) {
                warn!(
                    "Limit price deviates significantly from market: {} vs {}",
                    limit_price, market_price
                );
            }
        }

        Ok(())
    }

    /// Execute on OKX CEX
    async fn execute_okx(
        &self,
        order: &Order,
        market_price: Decimal,
    ) -> Result<ExecutionResult> {
        let client = self
            .okx_client
            .as_ref()
            .ok_or_else(|| anyhow!("OKX client not initialized"))?;

        let side: OkxSide = order.side.into();

        // Place order
        let response = match order.order_type {
            OrderType::Market => {
                client
                    .place_market_order(&order.symbol, side, order.size)
                    .await?
            }
            OrderType::Limit => {
                let price = order
                    .limit_price
                    .ok_or_else(|| anyhow!("Limit order requires price"))?;
                client
                    .place_limit_order(&order.symbol, side, order.size, price)
                    .await?
            }
        };

        // Track pending order
        {
            let mut pending = self.pending_orders.write();
            pending.insert(
                order.symbol.clone(),
                PendingOrder {
                    order_id: response.ord_id.clone(),
                    venue: Venue::OkxCex,
                    submitted_at: Instant::now(),
                },
            );
        }

        // Wait for fill
        let filled = client
            .wait_for_fill(&order.symbol, &response.ord_id, self.config.order_timeout_secs)
            .await?;

        // Remove from pending
        {
            let mut pending = self.pending_orders.write();
            pending.remove(&order.symbol);
        }

        // Update portfolio
        self.portfolio.open_position(
            &order.symbol,
            order.side,
            filled.fill_size(),
            filled.fill_price(),
            order.stop_loss,
            order.take_profit,
        );

        Ok(ExecutionResult {
            order_id: response.ord_id,
            symbol: order.symbol.clone(),
            side: order.side,
            size: filled.fill_size(),
            price: filled.fill_price(),
            fee: filled.fee().abs(),
            timestamp: chrono::Utc::now(),
            venue: "okx".to_string(),
        })
    }

    /// Execute on X Layer DEX via OnchainOS
    async fn execute_dex(
        &self,
        order: &Order,
        market_price: Decimal,
    ) -> Result<ExecutionResult> {
        let signer = self
            .signer
            .as_ref()
            .ok_or_else(|| anyhow!("X Layer signer not initialized"))?;

        // Parse symbol to get token addresses
        // Symbol format: TOKEN-USDT or direct address
        let (from_token, to_token) = self.parse_dex_pair(&order.symbol, order.side)?;

        // Calculate amounts
        let amount_in = if order.side == Side::Long {
            // Buying: spending USDT to get token
            order.size * market_price
        } else {
            // Selling: spending token to get USDT
            order.size
        };

        // Use OnchainOS for swap execution
        let onchain = simmons_feeds::OnchainFeed::new();

        // Calculate slippage
        let slippage = Decimal::from(self.config.max_slippage_bps) / dec!(10000);

        // Execute swap
        let swap_result = onchain
            .execute_swap("xlayer", &from_token, &to_token, amount_in, slippage)
            .await?;

        info!(
            "DEX swap executed: {} -> {} (tx: {})",
            swap_result.from_amount, swap_result.to_amount, swap_result.tx_hash
        );

        // Calculate fill details
        let filled_size = if order.side == Side::Long {
            swap_result.to_amount
        } else {
            swap_result.from_amount
        };

        let avg_price = if order.side == Side::Long {
            swap_result.from_amount / swap_result.to_amount
        } else {
            swap_result.to_amount / swap_result.from_amount
        };

        // Estimate gas fee in USD (approximate)
        let gas_fee = dec!(0.10); // ~$0.10 on X Layer

        // Update portfolio
        self.portfolio.open_position(
            &order.symbol,
            order.side,
            filled_size,
            avg_price,
            order.stop_loss,
            order.take_profit,
        );

        Ok(ExecutionResult {
            order_id: swap_result.tx_hash,
            symbol: order.symbol.clone(),
            side: order.side,
            size: filled_size,
            price: avg_price,
            fee: gas_fee,
            timestamp: chrono::Utc::now(),
            venue: "xlayer_dex".to_string(),
        })
    }

    /// Parse trading pair to token addresses
    fn parse_dex_pair(&self, symbol: &str, side: Side) -> Result<(String, String)> {
        // Common X Layer tokens (symbol -> address)
        let tokens: HashMap<&str, &str> = [
            ("WETH", "0x5A77f1443D16ee5761d310e38b62f77f726bC71c"),
            ("ETH", "0x5A77f1443D16ee5761d310e38b62f77f726bC71c"), // ETH = WETH
            ("USDT", "0x1E4a5963aBFD975d8c9021ce480b42188849D41d"),
            ("USDC", "0x74b7F16337b8972027F6196A17a631aC6dE26d22"),
            ("OKB", "0x75231F58b43240C9718Dd58B4967c5114342a86c"),
        ]
        .into_iter()
        .collect();

        // Parse symbol (e.g., "WETH-USDT" or "ETH-USDT")
        let parts: Vec<&str> = symbol.split('-').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid symbol format: {}", symbol));
        }

        let base = parts[0];
        let quote = parts[1];

        // Get base address (either from known tokens or assume it's already an address)
        let base_addr = if base.starts_with("0x") {
            base.to_string()
        } else {
            tokens
                .get(base)
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("Unknown token: {}", base))?
        };

        // Get quote address
        let quote_addr = tokens
            .get(quote)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Unknown quote token: {}", quote))?;

        // Direction depends on side
        if side == Side::Long {
            // Buy base with quote
            Ok((quote_addr, base_addr))
        } else {
            // Sell base for quote
            Ok((base_addr, quote_addr))
        }
    }

    /// Close a position
    pub async fn close_position(
        &self,
        symbol: &str,
        market_price: Decimal,
        reason: &str,
    ) -> Result<Trade> {
        let position = self
            .portfolio
            .get_position(symbol)
            .ok_or_else(|| anyhow!("No position to close: {}", symbol))?;

        info!(
            "Closing position: {} {} @ {} ({})",
            position.size, symbol, market_price, reason
        );

        // Create closing order (opposite side)
        let closing_side = match position.side {
            Side::Long => Side::Short,
            Side::Short => Side::Long,
        };

        let order = Order {
            symbol: symbol.to_string(),
            side: closing_side,
            size: position.size,
            order_type: OrderType::Market,
            limit_price: None,
            stop_loss: None,
            take_profit: None,
        };

        // Execute closing order
        let result = self.execute(&order, market_price).await?;

        // Record trade
        let pnl = if position.side == Side::Long {
            (result.price - position.entry_price) * position.size
        } else {
            (position.entry_price - result.price) * position.size
        };

        // Determine outcome
        let outcome = if pnl > Decimal::ZERO {
            TradeOutcome::Win
        } else if pnl < Decimal::ZERO {
            TradeOutcome::Loss
        } else {
            TradeOutcome::Breakeven
        };

        let trade = Trade {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            side: position.side,
            size: position.size,
            entry_price: position.entry_price,
            exit_price: result.price,
            pnl,
            outcome,
            reason: reason.to_string(),
            opened_at: position.opened_at,
            closed_at: chrono::Utc::now(),
        };

        // Close portfolio position (ignore result as we already have our trade)
        let _ = self.portfolio.close_position(symbol, result.price, reason);

        Ok(trade)
    }

    /// Get current balances
    pub async fn get_balances(&self) -> Result<HashMap<String, Decimal>> {
        let mut balances = HashMap::new();

        // Get OKX balances
        if let Some(client) = &self.okx_client {
            let okx_balances = client.get_balance(None).await?;
            for bal in okx_balances {
                balances.insert(format!("okx_{}", bal.ccy), bal.available());
            }
        }

        // Get X Layer balances
        if let Some(signer) = &self.signer {
            let eth_balance = signer.balance().await?;
            let eth_str = ethers::utils::format_ether(eth_balance);
            let eth_decimal = Decimal::from_str(&eth_str).unwrap_or_default();
            balances.insert("xlayer_ETH".to_string(), eth_decimal);

            // Get USDT balance
            let usdt_addr = "0x1E4a5963aBFD975d8c9021ce480b42188849D41d"
                .parse::<Address>()
                .unwrap();
            let usdt_balance = signer
                .get_erc20_balance(usdt_addr, signer.address())
                .await?;
            let usdt_str = ethers::utils::format_units(usdt_balance, 6)
                .unwrap_or_else(|_| "0".to_string());
            let usdt_decimal = Decimal::from_str(&usdt_str).unwrap_or_default();
            balances.insert("xlayer_USDT".to_string(), usdt_decimal);
        }

        Ok(balances)
    }

    /// Check if we have a pending order for a symbol
    pub fn has_pending_order(&self, symbol: &str) -> bool {
        self.pending_orders.read().contains_key(symbol)
    }

    /// Cancel pending order
    pub async fn cancel_pending(&self, symbol: &str) -> Result<()> {
        let pending = {
            let orders = self.pending_orders.read();
            orders.get(symbol).cloned()
        };

        if let Some(pending) = pending {
            match pending.venue {
                Venue::OkxCex => {
                    if let Some(client) = &self.okx_client {
                        client.cancel_order(symbol, &pending.order_id).await?;
                    }
                }
                _ => {
                    // DEX orders can't be cancelled after submission
                    warn!("Cannot cancel DEX order: {}", pending.order_id);
                }
            }

            let mut orders = self.pending_orders.write();
            orders.remove(symbol);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_venue_from_symbol() {
        assert_eq!(Venue::from_symbol("BTC-USDT"), Venue::OkxCex);
        assert_eq!(Venue::from_symbol("ETH-USDC"), Venue::OkxCex);
        assert_eq!(
            Venue::from_symbol("0x1234-0x5678"),
            Venue::XLayerDex
        );
    }

    #[test]
    fn test_config_defaults() {
        let config = LiveExecutorConfig::default();
        assert_eq!(config.max_slippage_bps, 100);
        assert!(config.okx_enabled);
        assert!(config.dex_enabled);
    }

    #[test]
    fn test_parse_dex_pair() {
        let config = LiveExecutorConfig::default();
        let portfolio = Arc::new(Portfolio::new(dec!(1000)));
        let executor = LiveExecutor::new(config, portfolio);

        // Buy ETH with USDT
        let (from, to) = executor.parse_dex_pair("ETH-USDT", Side::Long).unwrap();
        assert!(from.contains("1E4a")); // USDT address
        assert!(to.contains("5A77")); // WETH address

        // Sell ETH for USDT
        let (from, to) = executor.parse_dex_pair("ETH-USDT", Side::Short).unwrap();
        assert!(from.contains("5A77")); // WETH address
        assert!(to.contains("1E4a")); // USDT address
    }
}
