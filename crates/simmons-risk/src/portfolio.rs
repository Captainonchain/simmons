//! Portfolio state management

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_core::{
    Order, OrderType, Position, PortfolioSnapshot, Side, Trade, TradeOutcome,
};
use std::collections::HashMap;

/// Portfolio state manager
pub struct Portfolio {
    /// Available capital (not in positions)
    capital: RwLock<Decimal>,
    /// Initial capital (for drawdown calculation)
    initial_capital: Decimal,
    /// Peak capital (for drawdown calculation)
    peak_capital: RwLock<Decimal>,
    /// Open positions
    positions: RwLock<HashMap<String, Position>>,
    /// Trade history
    trades: RwLock<Vec<Trade>>,
    /// Realized P&L
    realized_pnl: RwLock<Decimal>,
    /// Next position ID
    next_id: RwLock<u64>,
}

impl Portfolio {
    pub fn new(initial_capital: Decimal) -> Self {
        Self {
            capital: RwLock::new(initial_capital),
            initial_capital,
            peak_capital: RwLock::new(initial_capital),
            positions: RwLock::new(HashMap::new()),
            trades: RwLock::new(Vec::new()),
            realized_pnl: RwLock::new(Decimal::ZERO),
            next_id: RwLock::new(1),
        }
    }

    /// Get available capital
    pub fn capital(&self) -> Decimal {
        *self.capital.read()
    }

    /// Get total equity (capital + unrealized P&L)
    pub fn total_equity(&self) -> Decimal {
        let capital = *self.capital.read();
        let unrealized: Decimal = self
            .positions
            .read()
            .values()
            .map(|p| p.unrealized_pnl)
            .sum();
        capital + unrealized
    }

    /// Calculate current drawdown percentage
    pub fn drawdown(&self) -> Decimal {
        let peak = *self.peak_capital.read();
        let current = self.total_equity();

        if peak.is_zero() {
            return Decimal::ZERO;
        }

        ((peak - current) / peak).max(Decimal::ZERO)
    }

    /// Get open positions
    pub fn positions(&self) -> Vec<Position> {
        self.positions.read().values().cloned().collect()
    }

    /// Get position by symbol
    pub fn get_position(&self, symbol: &str) -> Option<Position> {
        self.positions.read().get(symbol).cloned()
    }

    /// Check if position exists for symbol
    pub fn has_position(&self, symbol: &str) -> bool {
        self.positions.read().contains_key(symbol)
    }

    /// Number of open positions
    pub fn position_count(&self) -> usize {
        self.positions.read().len()
    }

    /// Open a new position
    pub fn open_position(
        &self,
        symbol: &str,
        side: Side,
        size: Decimal,
        entry_price: Decimal,
        stop_loss: Option<Decimal>,
        take_profit: Option<Decimal>,
    ) -> Result<Position, String> {
        let cost = size * entry_price;

        // Check capital
        let mut capital = self.capital.write();
        if *capital < cost {
            return Err(format!(
                "Insufficient capital: need ${}, have ${}",
                cost, *capital
            ));
        }

        // Generate ID
        let mut next_id = self.next_id.write();
        let id = format!("pos-{}", *next_id);
        *next_id += 1;

        // Deduct capital
        *capital -= cost;

        // Create position
        let position = Position {
            id: id.clone(),
            symbol: symbol.to_string(),
            side,
            size,
            entry_price,
            current_price: entry_price,
            stop_loss,
            take_profit,
            opened_at: Utc::now(),
            unrealized_pnl: Decimal::ZERO,
        };

        self.positions.write().insert(symbol.to_string(), position.clone());

        Ok(position)
    }

    /// Close a position
    pub fn close_position(
        &self,
        symbol: &str,
        exit_price: Decimal,
        reason: &str,
    ) -> Result<Trade, String> {
        let position = self
            .positions
            .write()
            .remove(symbol)
            .ok_or_else(|| format!("Position not found: {}", symbol))?;

        // Calculate P&L
        let price_diff = exit_price - position.entry_price;
        let pnl = match position.side {
            Side::Long => price_diff * position.size,
            Side::Short => -price_diff * position.size,
        };

        // Return capital + P&L
        let returned = position.size * position.entry_price + pnl;
        *self.capital.write() += returned;

        // Update realized P&L
        *self.realized_pnl.write() += pnl;

        // Update peak
        let equity = self.total_equity();
        let mut peak = self.peak_capital.write();
        if equity > *peak {
            *peak = equity;
        }

        // Determine outcome
        let outcome = if pnl > Decimal::ZERO {
            TradeOutcome::Win
        } else if pnl < Decimal::ZERO {
            TradeOutcome::Loss
        } else {
            TradeOutcome::Breakeven
        };

        // Create trade record
        let trade = Trade {
            id: position.id,
            symbol: position.symbol,
            side: position.side,
            size: position.size,
            entry_price: position.entry_price,
            exit_price,
            pnl,
            outcome,
            reason: reason.to_string(),
            opened_at: position.opened_at,
            closed_at: Utc::now(),
        };

        self.trades.write().push(trade.clone());

        Ok(trade)
    }

    /// Update position with current price
    pub fn update_position(&self, symbol: &str, current_price: Decimal) {
        if let Some(mut pos) = self.positions.write().get_mut(symbol) {
            pos.update_pnl(current_price);
        }
    }

    /// Update all positions
    pub fn update_positions(&self, prices: &HashMap<String, Decimal>) {
        let mut positions = self.positions.write();
        for (symbol, price) in prices {
            if let Some(pos) = positions.get_mut(symbol) {
                pos.update_pnl(*price);
            }
        }
    }

    /// Check for stop losses and take profits
    pub fn check_exits(&self) -> Vec<(String, Decimal, String)> {
        let positions = self.positions.read();
        let mut exits = Vec::new();

        for pos in positions.values() {
            if pos.should_stop_loss() {
                exits.push((
                    pos.symbol.clone(),
                    pos.current_price,
                    "stop_loss".to_string(),
                ));
            } else if pos.should_take_profit() {
                exits.push((
                    pos.symbol.clone(),
                    pos.current_price,
                    "take_profit".to_string(),
                ));
            }
        }

        exits
    }

    /// Get recent trades
    pub fn recent_trades(&self, n: usize) -> Vec<Trade> {
        let trades = self.trades.read();
        trades.iter().rev().take(n).cloned().collect()
    }

    /// Get win rate
    pub fn win_rate(&self) -> Decimal {
        let trades = self.trades.read();
        if trades.is_empty() {
            return Decimal::ZERO;
        }

        let wins = trades
            .iter()
            .filter(|t| t.outcome == TradeOutcome::Win)
            .count();

        Decimal::from(wins) / Decimal::from(trades.len())
    }

    /// Get average win and loss amounts
    pub fn avg_win_loss(&self) -> (Decimal, Decimal) {
        let trades = self.trades.read();

        let wins: Vec<Decimal> = trades
            .iter()
            .filter(|t| t.outcome == TradeOutcome::Win)
            .map(|t| t.pnl)
            .collect();

        let losses: Vec<Decimal> = trades
            .iter()
            .filter(|t| t.outcome == TradeOutcome::Loss)
            .map(|t| t.pnl.abs())
            .collect();

        let avg_win = if wins.is_empty() {
            Decimal::ZERO
        } else {
            wins.iter().sum::<Decimal>() / Decimal::from(wins.len())
        };

        let avg_loss = if losses.is_empty() {
            Decimal::ZERO
        } else {
            losses.iter().sum::<Decimal>() / Decimal::from(losses.len())
        };

        (avg_win, avg_loss)
    }

    /// Get portfolio snapshot for brain input
    pub fn snapshot(&self) -> PortfolioSnapshot {
        PortfolioSnapshot {
            capital: *self.capital.read(),
            positions: self.positions(),
            drawdown: self.drawdown(),
            risk_level: self.risk_level(),
            realized_pnl: *self.realized_pnl.read(),
            unrealized_pnl: self
                .positions
                .read()
                .values()
                .map(|p| p.unrealized_pnl)
                .sum(),
        }
    }

    /// Get current risk level
    fn risk_level(&self) -> String {
        let dd = self.drawdown();

        if dd > dec!(0.15) {
            "high".to_string()
        } else if dd > dec!(0.10) {
            "elevated".to_string()
        } else if dd > dec!(0.05) {
            "moderate".to_string()
        } else {
            "normal".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_open_close() {
        let portfolio = Portfolio::new(dec!(1000));

        // Open position
        let pos = portfolio
            .open_position(
                "BTC-USDT",
                Side::Long,
                dec!(0.01),
                dec!(67000),
                Some(dec!(65000)),
                Some(dec!(70000)),
            )
            .unwrap();

        assert_eq!(pos.symbol, "BTC-USDT");
        assert_eq!(portfolio.capital(), dec!(330)); // 1000 - (0.01 * 67000) = 330

        // Close with profit
        let trade = portfolio
            .close_position("BTC-USDT", dec!(68000), "take_profit")
            .unwrap();

        assert_eq!(trade.pnl, dec!(10)); // 0.01 * 1000 = 10
        assert_eq!(trade.outcome, TradeOutcome::Win);
        assert_eq!(portfolio.capital(), dec!(1010)); // 1000 + 10
    }

    #[test]
    fn test_drawdown() {
        let portfolio = Portfolio::new(dec!(1000));

        // Initial drawdown should be 0
        assert_eq!(portfolio.drawdown(), Decimal::ZERO);

        // Open and close with loss
        portfolio
            .open_position("BTC-USDT", Side::Long, dec!(0.01), dec!(67000), None, None)
            .unwrap();
        portfolio.close_position("BTC-USDT", dec!(66000), "stop_loss").unwrap();

        // Should have drawdown
        assert!(portfolio.drawdown() > Decimal::ZERO);
    }

    #[test]
    fn test_stop_loss_check() {
        let portfolio = Portfolio::new(dec!(10000));

        portfolio
            .open_position(
                "BTC-USDT",
                Side::Long,
                dec!(0.01),
                dec!(67000),
                Some(dec!(65000)),
                None,
            )
            .unwrap();

        // Price above stop - no exit
        portfolio.update_position("BTC-USDT", dec!(66000));
        assert!(portfolio.check_exits().is_empty());

        // Price below stop - should exit
        portfolio.update_position("BTC-USDT", dec!(64000));
        let exits = portfolio.check_exits();
        assert_eq!(exits.len(), 1);
        assert_eq!(exits[0].2, "stop_loss");
    }
}
