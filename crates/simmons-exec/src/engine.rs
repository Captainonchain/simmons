//! Unified execution engine
//!
//! Routes orders to paper trading or live execution based on trading mode.

use crate::live::{LiveExecutor, LiveExecutorConfig};
use crate::mev::MevShield;
use crate::paper::PaperTrader;
use crate::router::SmartOrderRouter;
use anyhow::Result;
use rust_decimal::Decimal;
use simmons_core::{config::ExecutionConfig, ExecutionResult, Order, TradingMode};
use simmons_risk::Portfolio;
use std::sync::Arc;
use tracing::{info, warn};

/// Unified execution engine
pub struct ExecutionEngine {
    /// Trading mode
    mode: TradingMode,
    /// Smart order router
    router: SmartOrderRouter,
    /// MEV protection
    mev_shield: MevShield,
    /// Paper trader (for paper mode)
    paper_trader: Option<PaperTrader>,
    /// Live executor (for live mode)
    live_executor: Option<LiveExecutor>,
    /// Portfolio reference
    portfolio: Arc<Portfolio>,
}

impl ExecutionEngine {
    pub fn new(mode: TradingMode, _config: ExecutionConfig, portfolio: Arc<Portfolio>) -> Self {
        use crate::mev::MevShieldConfig;
        use crate::router::RouterConfig;

        let router = SmartOrderRouter::new(RouterConfig::default());
        let mev_shield = MevShield::new(MevShieldConfig::default());

        let paper_trader = match mode {
            TradingMode::Paper | TradingMode::Simulation => {
                Some(PaperTrader::new(portfolio.clone()))
            }
            TradingMode::Live => None,
        };

        Self {
            mode,
            router,
            mev_shield,
            paper_trader,
            live_executor: None,
            portfolio,
        }
    }

    /// Initialize live execution from environment variables
    /// Call this after new() if mode is Live
    pub fn with_live_executor(mut self, testnet: bool) -> Result<Self> {
        if self.mode == TradingMode::Live {
            let config = LiveExecutorConfig::default();
            let executor = LiveExecutor::new(config, self.portfolio.clone())
                .with_okx()?
                .with_xlayer_signer(testnet)?;

            if !executor.is_ready() {
                return Err(anyhow::anyhow!(
                    "Live executor not ready. Check environment variables:\n\
                     - OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE (for CEX)\n\
                     - XLAYER_PRIVATE_KEY (for DEX)"
                ));
            }

            info!(
                "Live executor initialized. Available venues: {:?}",
                executor.available_venues()
            );
            self.live_executor = Some(executor);
        }
        Ok(self)
    }

    /// Check if live trading is ready
    pub fn is_live_ready(&self) -> bool {
        self.live_executor
            .as_ref()
            .map(|e| e.is_ready())
            .unwrap_or(false)
    }

    /// Execute an order
    pub async fn execute(&self, order: Order, market_price: Decimal) -> Result<ExecutionResult> {
        info!(
            "Executing order: {:?} {} {} @ ${}",
            order.side, order.size, order.symbol, market_price
        );

        // Analyze MEV risk
        let protected = self.mev_shield.protect(&order);
        if protected.use_private_pool {
            info!("Using private transaction pool for MEV protection");
        }

        // Route order
        let routed = self.router.route(&order);
        if routed.split {
            info!("Splitting order into {} parts", routed.routes.len());
        }

        match self.mode {
            TradingMode::Paper | TradingMode::Simulation => {
                let trader = self.paper_trader.as_ref().unwrap();
                let result = trader.execute(&order, market_price)
                    .map_err(|e| anyhow::anyhow!(e))?;
                Ok(result)
            }
            TradingMode::Live => {
                // Use live executor if available
                if let Some(executor) = &self.live_executor {
                    executor.execute(&order, market_price).await
                } else {
                    // Fallback to paper if live not configured
                    warn!("Live executor not configured, falling back to paper trading");
                    let trader = PaperTrader::new(self.portfolio.clone());
                    let result = trader.execute(&order, market_price)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    Ok(result)
                }
            }
        }
    }

    /// Close a position
    pub async fn close_position(
        &self,
        symbol: &str,
        market_price: Decimal,
        reason: &str,
    ) -> Result<simmons_core::Trade> {
        info!("Closing position: {} @ ${} ({})", symbol, market_price, reason);

        match self.mode {
            TradingMode::Paper | TradingMode::Simulation => {
                let trader = self.paper_trader.as_ref().unwrap();
                let trade = trader.close(symbol, market_price, reason)
                    .map_err(|e| anyhow::anyhow!(e))?;
                Ok(trade)
            }
            TradingMode::Live => {
                if let Some(executor) = &self.live_executor {
                    executor.close_position(symbol, market_price, reason).await
                } else {
                    warn!("Live executor not configured, using paper close");
                    let trader = PaperTrader::new(self.portfolio.clone());
                    let trade = trader.close(symbol, market_price, reason)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    Ok(trade)
                }
            }
        }
    }

    /// Get trading mode
    pub fn mode(&self) -> TradingMode {
        self.mode
    }

    /// Check if we have an open position
    pub fn has_position(&self, symbol: &str) -> bool {
        self.portfolio.has_position(symbol)
    }

    /// Get current capital
    pub fn capital(&self) -> Decimal {
        self.portfolio.capital()
    }

    /// Get total equity
    pub fn equity(&self) -> Decimal {
        self.portfolio.total_equity()
    }

    /// Get live executor reference (for advanced operations)
    pub fn live_executor(&self) -> Option<&LiveExecutor> {
        self.live_executor.as_ref()
    }

    /// Check if there's a pending order for a symbol
    pub fn has_pending_order(&self, symbol: &str) -> bool {
        self.live_executor
            .as_ref()
            .map(|e| e.has_pending_order(symbol))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use simmons_core::{OrderType, Side};

    #[tokio::test]
    async fn test_paper_execution() {
        let portfolio = Arc::new(Portfolio::new(dec!(10000)));
        let config = ExecutionConfig::default();
        let engine = ExecutionEngine::new(TradingMode::Paper, config, portfolio.clone());

        let order = Order {
            symbol: "BTC-USDT".to_string(),
            side: Side::Long,
            size: dec!(0.1),
            order_type: OrderType::Market,
            limit_price: None,
            stop_loss: Some(dec!(65000)),
            take_profit: Some(dec!(70000)),
        };

        let result = engine.execute(order, dec!(67000)).await.unwrap();
        assert_eq!(result.symbol, "BTC-USDT");
        assert!(engine.has_position("BTC-USDT"));
    }

    #[test]
    fn test_live_mode_needs_executor() {
        let portfolio = Arc::new(Portfolio::new(dec!(10000)));
        let config = ExecutionConfig::default();
        let engine = ExecutionEngine::new(TradingMode::Live, config, portfolio);

        // Without live executor, it should not be ready
        assert!(!engine.is_live_ready());
    }
}
