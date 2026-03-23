//! Paper trading execution

use chrono::Utc;
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_core::{ExecutionResult, Order, Side};
use simmons_risk::Portfolio;
use std::sync::Arc;

/// Paper trading executor
pub struct PaperTrader {
    /// Portfolio reference
    portfolio: Arc<Portfolio>,
    /// Simulated slippage in basis points
    slippage_bps: Decimal,
    /// Simulated fee in basis points
    fee_bps: Decimal,
    /// Next order ID
    next_id: RwLock<u64>,
}

impl PaperTrader {
    pub fn new(portfolio: Arc<Portfolio>) -> Self {
        Self {
            portfolio,
            slippage_bps: dec!(5),  // 5 bps slippage
            fee_bps: dec!(10),      // 10 bps fee
            next_id: RwLock::new(1),
        }
    }

    pub fn with_fees(portfolio: Arc<Portfolio>, slippage_bps: Decimal, fee_bps: Decimal) -> Self {
        Self {
            portfolio,
            slippage_bps,
            fee_bps,
            next_id: RwLock::new(1),
        }
    }

    /// Execute order in paper trading mode
    pub fn execute(&self, order: &Order, market_price: Decimal) -> Result<ExecutionResult, String> {
        // Calculate fill price - only apply slippage to market orders
        let fill_price = match order.order_type {
            simmons_core::OrderType::Limit => {
                // Limit orders fill at limit price or better (no slippage)
                order.limit_price.unwrap_or(market_price)
            }
            simmons_core::OrderType::Market => {
                // Market orders get slippage
                let slippage_mult = match order.side {
                    Side::Long => dec!(1) + self.slippage_bps / dec!(10000),
                    Side::Short => dec!(1) - self.slippage_bps / dec!(10000),
                };
                market_price * slippage_mult
            }
        };

        // Calculate fee
        let fee = order.size * fill_price * self.fee_bps / dec!(10000);

        // Open position in portfolio
        self.portfolio.open_position(
            &order.symbol,
            order.side,
            order.size,
            fill_price,
            order.stop_loss,
            order.take_profit,
        )?;

        // Generate order ID
        let mut next_id = self.next_id.write();
        let order_id = format!("paper-{}", *next_id);
        *next_id += 1;

        Ok(ExecutionResult {
            order_id,
            symbol: order.symbol.clone(),
            side: order.side,
            size: order.size,
            price: fill_price,
            fee,
            timestamp: Utc::now(),
            venue: "paper".to_string(),
        })
    }

    /// Close a position in paper trading
    pub fn close(
        &self,
        symbol: &str,
        market_price: Decimal,
        reason: &str,
    ) -> Result<simmons_core::Trade, String> {
        // Get position for slippage direction
        let position = self
            .portfolio
            .get_position(symbol)
            .ok_or_else(|| format!("Position not found: {}", symbol))?;

        // Apply slippage (unfavorable direction when closing)
        let slippage_mult = match position.side {
            Side::Long => dec!(1) - self.slippage_bps / dec!(10000), // Closing long = selling
            Side::Short => dec!(1) + self.slippage_bps / dec!(10000), // Closing short = buying
        };
        let exit_price = market_price * slippage_mult;

        self.portfolio.close_position(symbol, exit_price, reason)
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
    use simmons_core::OrderType;

    #[test]
    fn test_paper_execute() {
        let portfolio = Arc::new(Portfolio::new(dec!(10000)));
        let trader = PaperTrader::new(portfolio.clone());

        let order = Order {
            symbol: "BTC-USDT".to_string(),
            side: Side::Long,
            size: dec!(0.1),
            order_type: OrderType::Market,
            limit_price: None,
            stop_loss: Some(dec!(65000)),
            take_profit: Some(dec!(70000)),
        };

        let result = trader.execute(&order, dec!(67000)).unwrap();

        assert_eq!(result.symbol, "BTC-USDT");
        assert!(result.price > dec!(67000)); // Slippage for buy
        assert!(result.fee > Decimal::ZERO);

        // Position should be open
        assert!(portfolio.has_position("BTC-USDT"));
    }

    #[test]
    fn test_paper_close() {
        let portfolio = Arc::new(Portfolio::new(dec!(10000)));
        let trader = PaperTrader::new(portfolio.clone());

        // Open position
        let order = Order {
            symbol: "BTC-USDT".to_string(),
            side: Side::Long,
            size: dec!(0.1),
            order_type: OrderType::Market,
            limit_price: None,
            stop_loss: None,
            take_profit: None,
        };
        trader.execute(&order, dec!(67000)).unwrap();

        // Close with profit
        let trade = trader.close("BTC-USDT", dec!(68000), "take_profit").unwrap();

        assert!(trade.pnl > Decimal::ZERO);
        assert!(!portfolio.has_position("BTC-USDT"));
    }
}
