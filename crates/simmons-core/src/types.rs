//! Core types for the Simmons trading system

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Trading signal strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Signal {
    StrongBuy,
    Buy,
    Hold,
    Sell,
    StrongSell,
}

impl Signal {
    pub fn is_bullish(&self) -> bool {
        matches!(self, Signal::StrongBuy | Signal::Buy)
    }

    pub fn is_bearish(&self) -> bool {
        matches!(self, Signal::StrongSell | Signal::Sell)
    }

    pub fn to_numeric(&self) -> i8 {
        match self {
            Signal::StrongBuy => 2,
            Signal::Buy => 1,
            Signal::Hold => 0,
            Signal::Sell => -1,
            Signal::StrongSell => -2,
        }
    }
}

/// Market regime classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Regime {
    TrendingUp,
    TrendingDown,
    MeanReverting,
    HighVolatility,
    LowVolatility,
    Choppy,
}

impl Default for Regime {
    fn default() -> Self {
        Regime::MeanReverting
    }
}

/// Data source identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Okx,
    Binance,
    Dex,
    OnchainOS,
    Chainlink,
}

/// Trading side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Long,
    Short,
}

impl Side {
    pub fn opposite(&self) -> Self {
        match self {
            Side::Long => Side::Short,
            Side::Short => Side::Long,
        }
    }
}

/// Trading action from Claude brain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Trade,
    Skip,
    ClosePosition,
}

/// Trading mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TradingMode {
    #[default]
    Paper,
    Live,
    Simulation,
}

/// Price tick from exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceTick {
    pub symbol: String,
    pub price: Decimal,
    pub bid: Decimal,
    pub ask: Decimal,
    pub volume_24h: Decimal,
    pub timestamp: i64,
    pub source: Source,
}

impl PriceTick {
    pub fn spread_bps(&self) -> Decimal {
        if self.bid.is_zero() {
            return Decimal::ZERO;
        }
        ((self.ask - self.bid) / self.bid) * Decimal::from(10000)
    }

    pub fn mid_price(&self) -> Decimal {
        (self.bid + self.ask) / Decimal::from(2)
    }
}

/// Order book level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: Decimal,
    pub size: Decimal,
}

/// Order book snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
    pub timestamp: i64,
}

impl OrderBook {
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.first().map(|l| l.price)
    }

    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.first().map(|l| l.price)
    }

    pub fn spread_bps(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) if !bid.is_zero() => {
                Some(((ask - bid) / bid) * Decimal::from(10000))
            }
            _ => None,
        }
    }

    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / Decimal::from(2)),
            _ => None,
        }
    }

    /// Estimate slippage for a given order size
    pub fn estimate_slippage(&self, size: Decimal, is_buy: bool) -> Option<Decimal> {
        let levels = if is_buy { &self.asks } else { &self.bids };
        let mid = self.mid_price()?;

        let mut remaining = size;
        let mut total_cost = Decimal::ZERO;

        for level in levels {
            let fill_size = remaining.min(level.size);
            total_cost += fill_size * level.price;
            remaining -= fill_size;
            if remaining.is_zero() {
                break;
            }
        }

        if remaining > Decimal::ZERO {
            return None; // Not enough liquidity
        }

        let avg_price = total_cost / size;
        Some(((avg_price - mid).abs() / mid) * Decimal::from(10000))
    }
}

/// Strategy signal with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySignal {
    pub strategy: String,
    pub signal: Signal,
    pub confidence: Decimal,
    pub reason: String,
}

/// Arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbOpportunity {
    #[serde(rename = "type")]
    pub arb_type: String,
    pub spread_bps: Decimal,
    pub net_profit_usd: Decimal,
    pub buy_venue: String,
    pub sell_venue: String,
}

/// Market state aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub symbol: String,
    pub price: Decimal,
    pub spread_bps: Decimal,
    pub volatility_1h: Decimal,
    pub regime: Regime,
    pub cex_price: Option<Decimal>,
    pub dex_price: Option<Decimal>,
}

/// Rolling price window for calculations
#[derive(Debug, Clone)]
pub struct PriceWindow {
    pub symbol: String,
    pub prices: VecDeque<(i64, Decimal)>,
    pub max_size: usize,
}

impl PriceWindow {
    pub fn new(symbol: String, max_size: usize) -> Self {
        Self {
            symbol,
            prices: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, timestamp: i64, price: Decimal) {
        if self.prices.len() >= self.max_size {
            self.prices.pop_front();
        }
        self.prices.push_back((timestamp, price));
    }

    pub fn len(&self) -> usize {
        self.prices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }

    pub fn latest_price(&self) -> Option<Decimal> {
        self.prices.back().map(|(_, p)| *p)
    }

    pub fn prices_only(&self) -> Vec<Decimal> {
        self.prices.iter().map(|(_, p)| *p).collect()
    }
}

/// Position in portfolio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    pub symbol: String,
    pub side: Side,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub opened_at: DateTime<Utc>,
    pub unrealized_pnl: Decimal,
}

impl Position {
    pub fn update_pnl(&mut self, current_price: Decimal) {
        self.current_price = current_price;
        let price_diff = current_price - self.entry_price;
        self.unrealized_pnl = match self.side {
            Side::Long => price_diff * self.size,
            Side::Short => -price_diff * self.size,
        };
    }

    pub fn pnl_percent(&self) -> Decimal {
        if self.entry_price.is_zero() {
            return Decimal::ZERO;
        }
        let price_diff = self.current_price - self.entry_price;
        let pct = (price_diff / self.entry_price) * Decimal::from(100);
        match self.side {
            Side::Long => pct,
            Side::Short => -pct,
        }
    }

    pub fn should_stop_loss(&self) -> bool {
        self.stop_loss.map_or(false, |sl| match self.side {
            Side::Long => self.current_price <= sl,
            Side::Short => self.current_price >= sl,
        })
    }

    pub fn should_take_profit(&self) -> bool {
        self.take_profit.map_or(false, |tp| match self.side {
            Side::Long => self.current_price >= tp,
            Side::Short => self.current_price <= tp,
        })
    }
}

/// Completed trade record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub symbol: String,
    pub side: Side,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub exit_price: Decimal,
    pub pnl: Decimal,
    pub outcome: TradeOutcome,
    pub reason: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: DateTime<Utc>,
}

/// Trade outcome for learning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeOutcome {
    Win,
    Loss,
    Breakeven,
}

/// Portfolio snapshot for brain input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub capital: Decimal,
    pub positions: Vec<Position>,
    pub drawdown: Decimal,
    pub risk_level: String,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
}

/// Order to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub symbol: String,
    pub side: Side,
    pub size: Decimal,
    pub order_type: OrderType,
    pub limit_price: Option<Decimal>,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub order_id: String,
    pub symbol: String,
    pub side: Side,
    pub size: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
    pub timestamp: DateTime<Utc>,
    pub venue: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_price_tick_spread() {
        let tick = PriceTick {
            symbol: "BTC-USDT".to_string(),
            price: dec!(67000),
            bid: dec!(66990),
            ask: dec!(67010),
            volume_24h: dec!(1000000),
            timestamp: 0,
            source: Source::Okx,
        };
        // Spread = (67010 - 66990) / 66990 * 10000 ≈ 2.98 bps
        assert!(tick.spread_bps() > dec!(2) && tick.spread_bps() < dec!(3.5));
    }

    #[test]
    fn test_position_pnl() {
        let mut pos = Position {
            id: "1".to_string(),
            symbol: "BTC-USDT".to_string(),
            side: Side::Long,
            size: dec!(0.01),
            entry_price: dec!(67000),
            current_price: dec!(67000),
            stop_loss: Some(dec!(66000)),
            take_profit: Some(dec!(70000)),
            opened_at: Utc::now(),
            unrealized_pnl: Decimal::ZERO,
        };

        pos.update_pnl(dec!(68000));
        assert_eq!(pos.unrealized_pnl, dec!(10)); // 0.01 * 1000 = 10
        assert!(!pos.should_stop_loss());
        assert!(!pos.should_take_profit());

        pos.update_pnl(dec!(70500));
        assert!(pos.should_take_profit());
    }

    #[test]
    fn test_price_window() {
        let mut window = PriceWindow::new("BTC".to_string(), 5);
        for i in 1..=7 {
            window.push(i, Decimal::from(i * 100));
        }
        assert_eq!(window.len(), 5);
        assert_eq!(window.latest_price(), Some(dec!(700)));
        assert_eq!(window.prices_only(), vec![dec!(300), dec!(400), dec!(500), dec!(600), dec!(700)]);
    }
}
