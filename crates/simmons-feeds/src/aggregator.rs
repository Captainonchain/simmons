//! Market data aggregator

use dashmap::DashMap;
use parking_lot::RwLock;
use rust_decimal::Decimal;
use simmons_core::{MarketState, OrderBook, PriceTick, PriceWindow, Regime};
use std::sync::Arc;
use std::collections::VecDeque;
use tracing::{debug, info};

/// Aggregates market data from multiple sources
pub struct MarketAggregator {
    /// Latest ticks by symbol
    ticks: DashMap<String, PriceTick>,
    /// Latest order books by symbol
    books: DashMap<String, OrderBook>,
    /// Price windows for calculations
    windows: DashMap<String, PriceWindow>,
    /// DEX prices (from OnchainOS)
    dex_prices: DashMap<String, Decimal>,
    /// Window size
    window_size: usize,
}

impl MarketAggregator {
    pub fn new(window_size: usize) -> Self {
        Self {
            ticks: DashMap::new(),
            books: DashMap::new(),
            windows: DashMap::new(),
            dex_prices: DashMap::new(),
            window_size,
        }
    }

    /// Update with a new price tick
    pub fn update_tick(&self, tick: PriceTick) {
        let symbol = tick.symbol.clone();
        let price = tick.price;
        let timestamp = tick.timestamp;

        // Update tick
        self.ticks.insert(symbol.clone(), tick);

        // Update window
        self.windows
            .entry(symbol.clone())
            .or_insert_with(|| PriceWindow::new(symbol, self.window_size))
            .push(timestamp, price);
    }

    /// Update with order book
    pub fn update_book(&self, book: OrderBook) {
        self.books.insert(book.symbol.clone(), book);
    }

    /// Update DEX price
    pub fn update_dex_price(&self, symbol: &str, price: Decimal) {
        self.dex_prices.insert(symbol.to_string(), price);
    }

    /// Get latest tick for symbol
    pub fn get_tick(&self, symbol: &str) -> Option<PriceTick> {
        self.ticks.get(symbol).map(|r| r.clone())
    }

    /// Get latest order book for symbol
    pub fn get_book(&self, symbol: &str) -> Option<OrderBook> {
        self.books.get(symbol).map(|r| r.clone())
    }

    /// Get price history for symbol
    pub fn get_prices(&self, symbol: &str) -> Option<Vec<Decimal>> {
        self.windows.get(symbol).map(|w| w.prices_only())
    }

    /// Get aggregated market state
    pub fn get_market_state(&self, symbol: &str) -> Option<MarketState> {
        let tick = self.ticks.get(symbol)?;
        let prices = self.get_prices(symbol)?;

        let volatility = self.calculate_volatility(&prices);
        let regime = self.detect_regime(&prices, volatility);

        Some(MarketState {
            symbol: symbol.to_string(),
            price: tick.price,
            spread_bps: tick.spread_bps(),
            volatility_1h: volatility,
            regime,
            cex_price: Some(tick.price),
            dex_price: self.dex_prices.get(symbol).map(|r| *r),
        })
    }

    /// Calculate volatility (standard deviation of returns)
    fn calculate_volatility(&self, prices: &[Decimal]) -> Decimal {
        if prices.len() < 2 {
            return Decimal::ZERO;
        }

        // Calculate returns
        let returns: Vec<Decimal> = prices
            .windows(2)
            .filter_map(|w| {
                if w[0].is_zero() {
                    None
                } else {
                    Some((w[1] - w[0]) / w[0])
                }
            })
            .collect();

        if returns.is_empty() {
            return Decimal::ZERO;
        }

        // Mean
        let n = Decimal::from(returns.len());
        let sum: Decimal = returns.iter().sum();
        let mean = sum / n;

        // Variance
        let variance: Decimal = returns
            .iter()
            .map(|r| {
                let diff = *r - mean;
                diff * diff
            })
            .sum::<Decimal>()
            / n;

        // Standard deviation (approximate sqrt)
        self.decimal_sqrt(variance)
    }

    /// Approximate square root for Decimal
    fn decimal_sqrt(&self, x: Decimal) -> Decimal {
        if x.is_zero() || x.is_sign_negative() {
            return Decimal::ZERO;
        }

        // Newton-Raphson method
        let mut guess = x / Decimal::from(2);
        for _ in 0..10 {
            if guess.is_zero() {
                break;
            }
            guess = (guess + x / guess) / Decimal::from(2);
        }
        guess
    }

    /// Detect market regime
    fn detect_regime(&self, prices: &[Decimal], volatility: Decimal) -> Regime {
        if prices.len() < 10 {
            return Regime::MeanReverting;
        }

        let len = prices.len();
        let recent = &prices[len.saturating_sub(10)..];

        // Calculate trend
        let first = recent.first().copied().unwrap_or_default();
        let last = recent.last().copied().unwrap_or_default();

        if first.is_zero() {
            return Regime::MeanReverting;
        }

        let change_pct = (last - first) / first * Decimal::from(100);

        // High volatility check
        if volatility > Decimal::new(3, 2) {
            return Regime::HighVolatility;
        }

        // Low volatility check
        if volatility < Decimal::new(5, 3) {
            return Regime::LowVolatility;
        }

        // Trend detection
        if change_pct > Decimal::from(1) {
            Regime::TrendingUp
        } else if change_pct < Decimal::from(-1) {
            Regime::TrendingDown
        } else {
            // Check for choppy market (many direction changes)
            let direction_changes = recent
                .windows(2)
                .map(|w| (w[1] - w[0]).is_sign_positive())
                .collect::<Vec<_>>()
                .windows(2)
                .filter(|w| w[0] != w[1])
                .count();

            if direction_changes > 5 {
                Regime::Choppy
            } else {
                Regime::MeanReverting
            }
        }
    }

    /// Get CeDeFi spread (CEX vs DEX price difference)
    pub fn get_cedefi_spread(&self, symbol: &str) -> Option<Decimal> {
        let cex_price = self.ticks.get(symbol)?.price;
        let dex_price = *self.dex_prices.get(symbol)?;

        if cex_price.is_zero() {
            return None;
        }

        // Spread in basis points
        Some(((dex_price - cex_price).abs() / cex_price) * Decimal::from(10000))
    }

    /// Get all tracked symbols
    pub fn symbols(&self) -> Vec<String> {
        self.ticks.iter().map(|r| r.key().clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use simmons_core::Source;

    #[test]
    fn test_aggregator_update() {
        let agg = MarketAggregator::new(100);

        let tick = PriceTick {
            symbol: "BTC-USDT".to_string(),
            price: dec!(67000),
            bid: dec!(66990),
            ask: dec!(67010),
            volume_24h: dec!(1000000),
            timestamp: 1234567890,
            source: Source::Okx,
        };

        agg.update_tick(tick.clone());

        let retrieved = agg.get_tick("BTC-USDT").unwrap();
        assert_eq!(retrieved.price, dec!(67000));
    }

    #[test]
    fn test_volatility_calculation() {
        let agg = MarketAggregator::new(100);

        // Simulate price updates
        for i in 0..20 {
            let price = dec!(67000) + Decimal::from(i * 10);
            let tick = PriceTick {
                symbol: "BTC-USDT".to_string(),
                price,
                bid: price - dec!(10),
                ask: price + dec!(10),
                volume_24h: dec!(1000000),
                timestamp: 1234567890 + i,
                source: Source::Okx,
            };
            agg.update_tick(tick);
        }

        let state = agg.get_market_state("BTC-USDT").unwrap();
        assert!(state.volatility_1h < dec!(0.01)); // Low volatility for linear price movement
    }
}
