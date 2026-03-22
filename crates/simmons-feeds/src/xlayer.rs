//! X Layer Data Feed
//!
//! Real-time price and event data from X Layer DEXes.

use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// X Layer feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XLayerFeedConfig {
    pub rpc_url: String,
    pub poll_interval_ms: u64,
    pub tracked_pools: Vec<TrackedPool>,
    pub price_cache_ttl_ms: u64,
}

impl Default for XLayerFeedConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://rpc.xlayer.tech".to_string(),
            poll_interval_ms: 2000,
            tracked_pools: vec![],
            price_cache_ttl_ms: 5000,
        }
    }
}

/// Pool to track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedPool {
    pub address: String,
    pub token0_symbol: String,
    pub token1_symbol: String,
    pub dex_name: String,
}

/// X Layer price feed
pub struct XLayerFeed {
    config: XLayerFeedConfig,
    price_cache: Arc<RwLock<HashMap<String, CachedPrice>>>,
    pool_states: Arc<RwLock<HashMap<String, PoolState>>>,
}

/// Cached price entry
#[derive(Debug, Clone)]
struct CachedPrice {
    price: Decimal,
    timestamp: i64,
    source: String,
}

/// Pool state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub address: String,
    pub token0_reserve: Decimal,
    pub token1_reserve: Decimal,
    pub price_0_in_1: Decimal,
    pub price_1_in_0: Decimal,
    pub liquidity_usd: Decimal,
    pub volume_24h_usd: Decimal,
    pub fee_tier_bps: u32,
    pub last_update: i64,
}

impl PoolState {
    /// Calculate price impact for a given trade size
    pub fn estimate_price_impact(&self, size: Decimal, is_buy_token0: bool) -> Decimal {
        let (reserve_in, reserve_out) = if is_buy_token0 {
            (self.token1_reserve, self.token0_reserve)
        } else {
            (self.token0_reserve, self.token1_reserve)
        };

        if reserve_in.is_zero() || reserve_out.is_zero() {
            return dec!(100); // 100% impact = no liquidity
        }

        // Constant product: new_price = reserve_out / (reserve_in + size)
        // Impact = (old_price - new_price) / old_price
        let old_price = reserve_out / reserve_in;
        let new_out = (reserve_out * reserve_in) / (reserve_in + size);
        let new_price = new_out / (reserve_in + size);

        ((old_price - new_price).abs() / old_price) * dec!(100)
    }
}

/// DEX price tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexTick {
    pub symbol: String,
    pub price: Decimal,
    pub bid: Decimal,
    pub ask: Decimal,
    pub liquidity: Decimal,
    pub timestamp: i64,
    pub pool_address: String,
    pub dex_name: String,
}

/// Swap event from DEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
    pub pool_address: String,
    pub sender: String,
    pub recipient: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: Decimal,
    pub amount_out: Decimal,
    pub price: Decimal,
    pub tx_hash: String,
    pub block_number: u64,
    pub timestamp: i64,
}

impl XLayerFeed {
    pub fn new(config: XLayerFeedConfig) -> Self {
        Self {
            config,
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            pool_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(XLayerFeedConfig::default())
    }

    /// Add a pool to track
    pub async fn add_pool(&mut self, pool: TrackedPool) {
        self.config.tracked_pools.push(pool);
    }

    /// Get current price for a symbol
    pub async fn get_price(&self, symbol: &str) -> Option<Decimal> {
        let cache = self.price_cache.read().await;
        let now = chrono::Utc::now().timestamp_millis();

        cache.get(symbol).and_then(|cached| {
            if now - cached.timestamp < self.config.price_cache_ttl_ms as i64 {
                Some(cached.price)
            } else {
                None
            }
        })
    }

    /// Get pool state
    pub async fn get_pool_state(&self, pool_address: &str) -> Option<PoolState> {
        let states = self.pool_states.read().await;
        states.get(pool_address).cloned()
    }

    /// Update pool state (called by poller)
    pub async fn update_pool_state(&self, state: PoolState) {
        let mut states = self.pool_states.write().await;
        states.insert(state.address.clone(), state);
    }

    /// Update price cache
    pub async fn update_price(&self, symbol: &str, price: Decimal, source: &str) {
        let mut cache = self.price_cache.write().await;
        cache.insert(
            symbol.to_string(),
            CachedPrice {
                price,
                timestamp: chrono::Utc::now().timestamp_millis(),
                source: source.to_string(),
            },
        );
    }

    /// Get all current prices
    pub async fn get_all_prices(&self) -> HashMap<String, Decimal> {
        let cache = self.price_cache.read().await;
        let now = chrono::Utc::now().timestamp_millis();

        cache
            .iter()
            .filter(|(_, v)| now - v.timestamp < self.config.price_cache_ttl_ms as i64)
            .map(|(k, v)| (k.clone(), v.price))
            .collect()
    }

    /// Generate DexTick from pool state
    pub fn pool_to_tick(&self, pool: &TrackedPool, state: &PoolState) -> DexTick {
        let symbol = format!("{}-{}", pool.token0_symbol, pool.token1_symbol);

        // Estimate bid/ask from liquidity (wider spread for low liquidity)
        let spread_bps = if state.liquidity_usd > dec!(1_000_000) {
            dec!(10) // 10 bps for high liquidity
        } else if state.liquidity_usd > dec!(100_000) {
            dec!(30) // 30 bps for medium liquidity
        } else {
            dec!(100) // 100 bps for low liquidity
        };

        let half_spread = state.price_0_in_1 * spread_bps / dec!(20000);

        DexTick {
            symbol,
            price: state.price_0_in_1,
            bid: state.price_0_in_1 - half_spread,
            ask: state.price_0_in_1 + half_spread,
            liquidity: state.liquidity_usd,
            timestamp: state.last_update,
            pool_address: pool.address.clone(),
            dex_name: pool.dex_name.clone(),
        }
    }

    /// Get best price across all tracked pools
    pub async fn get_best_price(&self, token0: &str, token1: &str, is_buy: bool) -> Option<DexTick> {
        let states = self.pool_states.read().await;

        let mut best: Option<DexTick> = None;

        for pool in &self.config.tracked_pools {
            if (pool.token0_symbol == token0 && pool.token1_symbol == token1)
                || (pool.token0_symbol == token1 && pool.token1_symbol == token0)
            {
                if let Some(state) = states.get(&pool.address) {
                    let tick = self.pool_to_tick(pool, state);

                    let is_better = best.as_ref().map_or(true, |b| {
                        if is_buy {
                            tick.ask < b.ask
                        } else {
                            tick.bid > b.bid
                        }
                    });

                    if is_better {
                        best = Some(tick);
                    }
                }
            }
        }

        best
    }

    /// Calculate arbitrage opportunity between pools
    pub fn find_pool_arbitrage(
        &self,
        pool_a: &PoolState,
        pool_b: &PoolState,
    ) -> Option<PoolArbitrage> {
        let spread_bps = ((pool_a.price_0_in_1 - pool_b.price_0_in_1).abs()
            / pool_a.price_0_in_1.min(pool_b.price_0_in_1))
            * dec!(10000);

        // Minimum 20 bps spread after fees (2 * 30 bps)
        if spread_bps < dec!(80) {
            return None;
        }

        let (buy_pool, sell_pool) = if pool_a.price_0_in_1 < pool_b.price_0_in_1 {
            (pool_a, pool_b)
        } else {
            (pool_b, pool_a)
        };

        // Estimate optimal size based on liquidity
        let min_liquidity = buy_pool.liquidity_usd.min(sell_pool.liquidity_usd);
        let optimal_size = min_liquidity * dec!(0.01); // 1% of smaller pool

        Some(PoolArbitrage {
            buy_pool: buy_pool.address.clone(),
            sell_pool: sell_pool.address.clone(),
            spread_bps,
            buy_price: buy_pool.price_0_in_1,
            sell_price: sell_pool.price_0_in_1,
            optimal_size_usd: optimal_size,
            estimated_profit_bps: spread_bps - dec!(60), // Minus 2 swaps
        })
    }
}

/// Pool arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolArbitrage {
    pub buy_pool: String,
    pub sell_pool: String,
    pub spread_bps: Decimal,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
    pub optimal_size_usd: Decimal,
    pub estimated_profit_bps: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_state_price_impact() {
        let state = PoolState {
            address: "0x...".to_string(),
            token0_reserve: dec!(100),      // 100 ETH
            token1_reserve: dec!(300000),   // 300k USDT
            price_0_in_1: dec!(3000),
            price_1_in_0: dec!(0.000333),
            liquidity_usd: dec!(600000),
            volume_24h_usd: dec!(50000),
            fee_tier_bps: 30,
            last_update: 0,
        };

        // Small trade (0.1 ETH) should have low impact
        let small_impact = state.estimate_price_impact(dec!(0.1), false);
        assert!(small_impact < dec!(1), "Small impact was: {}", small_impact);

        // Large trade (50 ETH = 50% of reserves) should have higher impact
        let large_impact = state.estimate_price_impact(dec!(50), false);
        assert!(large_impact > dec!(10), "Large impact was: {}", large_impact);
    }

    #[test]
    fn test_pool_to_tick() {
        let feed = XLayerFeed::with_defaults();

        let pool = TrackedPool {
            address: "0x123".to_string(),
            token0_symbol: "ETH".to_string(),
            token1_symbol: "USDT".to_string(),
            dex_name: "xlayer_dex".to_string(),
        };

        let state = PoolState {
            address: "0x123".to_string(),
            token0_reserve: dec!(100),
            token1_reserve: dec!(300000),
            price_0_in_1: dec!(3000),
            price_1_in_0: dec!(0.000333),
            liquidity_usd: dec!(600000),
            volume_24h_usd: dec!(50000),
            fee_tier_bps: 30,
            last_update: 0,
        };

        let tick = feed.pool_to_tick(&pool, &state);
        assert_eq!(tick.symbol, "ETH-USDT");
        assert_eq!(tick.price, dec!(3000));
        assert!(tick.bid < tick.price);
        assert!(tick.ask > tick.price);
    }

    #[test]
    fn test_find_pool_arbitrage() {
        let feed = XLayerFeed::with_defaults();

        let pool_a = PoolState {
            address: "0xA".to_string(),
            token0_reserve: dec!(100),
            token1_reserve: dec!(300000),
            price_0_in_1: dec!(3000),
            price_1_in_0: dec!(0.000333),
            liquidity_usd: dec!(600000),
            volume_24h_usd: dec!(50000),
            fee_tier_bps: 30,
            last_update: 0,
        };

        let pool_b = PoolState {
            address: "0xB".to_string(),
            token0_reserve: dec!(100),
            token1_reserve: dec!(303000), // 1% higher
            price_0_in_1: dec!(3030),
            price_1_in_0: dec!(0.00033),
            liquidity_usd: dec!(606000),
            volume_24h_usd: dec!(40000),
            fee_tier_bps: 30,
            last_update: 0,
        };

        let arb = feed.find_pool_arbitrage(&pool_a, &pool_b);
        assert!(arb.is_some());

        let arb = arb.unwrap();
        assert_eq!(arb.buy_pool, "0xA");
        assert_eq!(arb.sell_pool, "0xB");
        assert!(arb.spread_bps > dec!(90));
    }
}
