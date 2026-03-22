//! Unified execution engine

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
            portfolio,
        }
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
                // Live execution would use OnchainOS or exchange API
                warn!("Live trading not implemented, falling back to paper");
                Err(anyhow::anyhow!("Live trading not implemented"))
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
                warn!("Live trading not implemented");
                Err(anyhow::anyhow!("Live trading not implemented"))
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
}
