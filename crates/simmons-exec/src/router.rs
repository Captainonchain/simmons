//! Smart Order Router
//!
//! Routes orders to the best execution venue (OKX CEX vs X Layer DEX)
//! considering price, slippage, fees, and latency.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::{Order, OrderBook, Side};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Execution venue
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Venue {
    Okx,
    XLayerDex,
    Cod3x,
    Primary,
}

impl Venue {
    pub fn name(&self) -> &'static str {
        match self {
            Venue::Okx => "OKX CEX",
            Venue::XLayerDex => "X Layer DEX",
            Venue::Cod3x => "Cod3x",
            Venue::Primary => "Primary",
        }
    }
}

/// Venue quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueQuote {
    pub venue: Venue,
    pub price: Decimal,
    pub available_liquidity: Decimal,
    pub fee_bps: Decimal,
    pub estimated_slippage_bps: Decimal,
    pub latency_ms: u64,
    pub timestamp: i64,
}

impl VenueQuote {
    /// Total cost including fees and slippage
    pub fn total_cost_bps(&self) -> Decimal {
        self.fee_bps + self.estimated_slippage_bps
    }

    /// Effective price after costs
    pub fn effective_price(&self, is_buy: bool) -> Decimal {
        let cost_multiplier = self.total_cost_bps() / dec!(10000);
        if is_buy {
            self.price * (Decimal::ONE + cost_multiplier)
        } else {
            self.price * (Decimal::ONE - cost_multiplier)
        }
    }
}

/// Router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Threshold above which to split orders (USD)
    pub split_threshold_usd: Decimal,
    /// Maximum slippage allowed (bps)
    pub max_slippage_bps: Decimal,
    /// Prefer CEX for orders above this size (USD)
    pub cex_preference_threshold_usd: Decimal,
    /// Maximum venues to split across
    pub max_split_venues: usize,
    /// Venue-specific fee rates
    pub venue_fees: HashMap<Venue, Decimal>,
    /// Venue latencies (ms)
    pub venue_latencies: HashMap<Venue, u64>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        let mut fees = HashMap::new();
        fees.insert(Venue::Okx, dec!(10)); // 10 bps
        fees.insert(Venue::XLayerDex, dec!(30)); // 30 bps
        fees.insert(Venue::Cod3x, dec!(5)); // 5 bps (lending)

        let mut latencies = HashMap::new();
        latencies.insert(Venue::Okx, 50);
        latencies.insert(Venue::XLayerDex, 2000);
        latencies.insert(Venue::Cod3x, 2000);

        Self {
            split_threshold_usd: dec!(10000),
            max_slippage_bps: dec!(50),
            cex_preference_threshold_usd: dec!(50000),
            max_split_venues: 3,
            venue_fees: fees,
            venue_latencies: latencies,
        }
    }
}

/// Smart order router
pub struct SmartOrderRouter {
    config: RouterConfig,
    venue_quotes: HashMap<(String, Venue), VenueQuote>,
}

impl SmartOrderRouter {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            venue_quotes: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(RouterConfig::default())
    }

    /// Legacy constructor for compatibility
    pub fn legacy(split_threshold: Decimal, max_slippage_bps: Decimal) -> LegacyRouter {
        LegacyRouter {
            split_threshold,
            max_slippage_bps,
        }
    }

    /// Update venue quote
    pub fn update_quote(&mut self, symbol: &str, quote: VenueQuote) {
        self.venue_quotes
            .insert((symbol.to_string(), quote.venue.clone()), quote);
    }

    /// Route order to best venue(s)
    pub fn route(&self, order: &Order) -> RoutedOrder {
        let order_value = order.size * order.limit_price.unwrap_or(Decimal::ZERO);
        let is_buy = matches!(order.side, Side::Long);

        // Get available quotes for this symbol
        let quotes: Vec<&VenueQuote> = self
            .venue_quotes
            .iter()
            .filter(|((sym, _), _)| sym == &order.symbol)
            .map(|(_, q)| q)
            .collect();

        if quotes.is_empty() {
            // No quotes available, use primary venue
            return RoutedOrder {
                original_order: order.clone(),
                routes: vec![OrderRoute {
                    venue: Venue::Primary,
                    size: order.size,
                    expected_price: order.limit_price.unwrap_or_default(),
                    fee_bps: dec!(15),
                    estimated_slippage_bps: dec!(10),
                    priority: 1,
                }],
                total_cost_bps: dec!(25),
                estimated_fill_time_ms: 100,
                split: false,
            };
        }

        // Sort quotes by effective price
        let mut sorted_quotes: Vec<_> = quotes.clone();
        sorted_quotes.sort_by(|a, b| {
            let eff_a = a.effective_price(is_buy);
            let eff_b = b.effective_price(is_buy);
            if is_buy {
                eff_a.cmp(&eff_b) // Lower is better for buys
            } else {
                eff_b.cmp(&eff_a) // Higher is better for sells
            }
        });

        // Check if we should split
        let should_split = order_value > self.config.split_threshold_usd;

        if !should_split {
            // Single venue execution
            let best = sorted_quotes[0];
            return RoutedOrder {
                original_order: order.clone(),
                routes: vec![OrderRoute {
                    venue: best.venue.clone(),
                    size: order.size,
                    expected_price: best.price,
                    fee_bps: best.fee_bps,
                    estimated_slippage_bps: best.estimated_slippage_bps,
                    priority: 1,
                }],
                total_cost_bps: best.total_cost_bps(),
                estimated_fill_time_ms: best.latency_ms,
                split: false,
            };
        }

        // Split across multiple venues
        let mut routes = Vec::new();
        let mut remaining = order.size;
        let mut total_cost_bps = Decimal::ZERO;
        let mut max_latency = 0u64;

        for (i, quote) in sorted_quotes
            .iter()
            .take(self.config.max_split_venues)
            .enumerate()
        {
            // Calculate how much to fill at this venue
            let venue_capacity = quote.available_liquidity / quote.price;
            let fill_size = remaining.min(venue_capacity);

            if fill_size > Decimal::ZERO {
                let weight = fill_size / order.size;
                total_cost_bps += quote.total_cost_bps() * weight;
                max_latency = max_latency.max(quote.latency_ms);

                routes.push(OrderRoute {
                    venue: quote.venue.clone(),
                    size: fill_size,
                    expected_price: quote.price,
                    fee_bps: quote.fee_bps,
                    estimated_slippage_bps: quote.estimated_slippage_bps,
                    priority: (i + 1) as u8,
                });

                remaining -= fill_size;
                if remaining <= Decimal::ZERO {
                    break;
                }
            }
        }

        // If we couldn't fill everything, add remainder to best venue
        if remaining > Decimal::ZERO {
            if let Some(route) = routes.first_mut() {
                route.size += remaining;
            }
        }

        let split = routes.len() > 1;
        RoutedOrder {
            original_order: order.clone(),
            routes,
            total_cost_bps,
            estimated_fill_time_ms: max_latency,
            split,
        }
    }

    /// Estimate execution cost
    pub fn estimate_cost(&self, order: &Order) -> ExecutionCostEstimate {
        let routed = self.route(order);
        let order_value = order.size * order.limit_price.unwrap_or(Decimal::ZERO);

        let fee_cost = order_value * routed.total_cost_bps / dec!(10000);

        ExecutionCostEstimate {
            order_value,
            total_cost_bps: routed.total_cost_bps,
            total_cost_usd: fee_cost,
            venues_used: routed.routes.len(),
            estimated_fill_time_ms: routed.estimated_fill_time_ms,
            split_order: routed.split,
        }
    }

    /// Check if execution is feasible
    pub fn is_feasible(&self, order: &Order) -> FeasibilityCheck {
        let routed = self.route(order);

        // Check slippage
        let max_slippage = routed
            .routes
            .iter()
            .map(|r| r.estimated_slippage_bps)
            .max()
            .unwrap_or(Decimal::ZERO);

        let slippage_ok = max_slippage <= self.config.max_slippage_bps;

        // Check liquidity
        let total_available: Decimal = self
            .venue_quotes
            .iter()
            .filter(|((sym, _), _)| sym == &order.symbol)
            .map(|(_, q)| q.available_liquidity)
            .sum();

        let order_value = order.size * order.limit_price.unwrap_or(Decimal::ZERO);
        let liquidity_ok = total_available >= order_value * dec!(0.5); // At least 50% coverage

        let feasible = slippage_ok && liquidity_ok;

        let reason = if feasible {
            None
        } else if !slippage_ok {
            Some(format!(
                "Slippage {} bps exceeds max {} bps",
                max_slippage, self.config.max_slippage_bps
            ))
        } else {
            Some(format!(
                "Insufficient liquidity: {} / {} USD",
                total_available, order_value
            ))
        };

        FeasibilityCheck {
            feasible,
            reason,
            max_slippage_bps: max_slippage,
            available_liquidity: total_available,
        }
    }

    /// Get best venue for a symbol
    pub fn best_venue(&self, symbol: &str, is_buy: bool) -> Option<Venue> {
        self.venue_quotes
            .iter()
            .filter(|((sym, _), _)| sym == symbol)
            .min_by(|(_, a), (_, b)| {
                let eff_a = a.effective_price(is_buy);
                let eff_b = b.effective_price(is_buy);
                if is_buy {
                    eff_a.cmp(&eff_b)
                } else {
                    eff_b.cmp(&eff_a)
                }
            })
            .map(|(_, q)| q.venue.clone())
    }

    /// Compare venues for an order
    pub fn compare_venues(&self, order: &Order) -> Vec<VenueComparison> {
        let is_buy = matches!(order.side, Side::Long);

        let mut comparisons: Vec<VenueComparison> = self
            .venue_quotes
            .iter()
            .filter(|((sym, _), _)| sym == &order.symbol)
            .map(|(_, quote)| {
                let effective = quote.effective_price(is_buy);
                let order_value = order.size * quote.price;
                let cost_usd = order_value * quote.total_cost_bps() / dec!(10000);

                VenueComparison {
                    venue: quote.venue.clone(),
                    raw_price: quote.price,
                    effective_price: effective,
                    fee_bps: quote.fee_bps,
                    slippage_bps: quote.estimated_slippage_bps,
                    total_cost_bps: quote.total_cost_bps(),
                    cost_usd,
                    latency_ms: quote.latency_ms,
                    liquidity: quote.available_liquidity,
                }
            })
            .collect();

        // Sort by effective price
        comparisons.sort_by(|a, b| {
            if is_buy {
                a.effective_price.cmp(&b.effective_price)
            } else {
                b.effective_price.cmp(&a.effective_price)
            }
        });

        comparisons
    }
}

/// Routed order with venue allocations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedOrder {
    pub original_order: Order,
    pub routes: Vec<OrderRoute>,
    pub total_cost_bps: Decimal,
    pub estimated_fill_time_ms: u64,
    pub split: bool,
}

/// Single route in a routed order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRoute {
    pub venue: Venue,
    pub size: Decimal,
    pub expected_price: Decimal,
    pub fee_bps: Decimal,
    pub estimated_slippage_bps: Decimal,
    pub priority: u8,
}

/// Execution cost estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCostEstimate {
    pub order_value: Decimal,
    pub total_cost_bps: Decimal,
    pub total_cost_usd: Decimal,
    pub venues_used: usize,
    pub estimated_fill_time_ms: u64,
    pub split_order: bool,
}

/// Feasibility check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeasibilityCheck {
    pub feasible: bool,
    pub reason: Option<String>,
    pub max_slippage_bps: Decimal,
    pub available_liquidity: Decimal,
}

/// Venue comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueComparison {
    pub venue: Venue,
    pub raw_price: Decimal,
    pub effective_price: Decimal,
    pub fee_bps: Decimal,
    pub slippage_bps: Decimal,
    pub total_cost_bps: Decimal,
    pub cost_usd: Decimal,
    pub latency_ms: u64,
    pub liquidity: Decimal,
}

// =============================================================================
// Legacy Router for backward compatibility
// =============================================================================

/// Legacy router (maintains old API)
pub struct LegacyRouter {
    pub split_threshold: Decimal,
    pub max_slippage_bps: Decimal,
}

impl Default for LegacyRouter {
    fn default() -> Self {
        Self {
            split_threshold: dec!(1000),
            max_slippage_bps: dec!(50),
        }
    }
}

impl LegacyRouter {
    pub fn new(split_threshold: Decimal, max_slippage_bps: Decimal) -> Self {
        Self {
            split_threshold,
            max_slippage_bps,
        }
    }

    pub fn create_execution_plan(&self, order: &Order, book: Option<&OrderBook>) -> ExecutionPlan {
        let order_value = order.size * order.limit_price.unwrap_or(Decimal::ZERO);

        let estimated_slippage = book.and_then(|b| {
            b.estimate_slippage(order.size, matches!(order.side, Side::Long))
        });

        let should_split = order_value > self.split_threshold
            || estimated_slippage.map_or(false, |s| s > self.max_slippage_bps);

        if should_split {
            let num_parts = (order_value / self.split_threshold).ceil();
            let part_size = order.size / num_parts;

            let parts: Vec<OrderPart> = (0..num_parts.to_string().parse().unwrap_or(3))
                .map(|i| OrderPart {
                    size: part_size,
                    delay_ms: i * 100,
                    venue: "primary".to_string(),
                })
                .collect();

            ExecutionPlan {
                original_order: order.clone(),
                parts,
                estimated_slippage,
                split: true,
            }
        } else {
            ExecutionPlan {
                original_order: order.clone(),
                parts: vec![OrderPart {
                    size: order.size,
                    delay_ms: 0,
                    venue: "primary".to_string(),
                }],
                estimated_slippage,
                split: false,
            }
        }
    }

    pub fn estimate_cost(&self, order: &Order, book: Option<&OrderBook>) -> LegacyExecutionCost {
        let mid_price = book.and_then(|b| b.mid_price()).unwrap_or_default();
        let slippage_bps = book
            .and_then(|b| b.estimate_slippage(order.size, matches!(order.side, Side::Long)))
            .unwrap_or(dec!(10));

        let slippage_cost = order.size * mid_price * slippage_bps / dec!(10000);
        let fee_bps = dec!(10);
        let fee_cost = order.size * mid_price * fee_bps / dec!(10000);

        LegacyExecutionCost {
            slippage_bps,
            slippage_usd: slippage_cost,
            fee_bps,
            fee_usd: fee_cost,
            total_bps: slippage_bps + fee_bps,
            total_usd: slippage_cost + fee_cost,
        }
    }

    pub fn is_feasible(&self, order: &Order, book: Option<&OrderBook>) -> LegacyFeasibilityCheck {
        let cost = self.estimate_cost(order, book);

        let feasible = cost.slippage_bps <= self.max_slippage_bps;
        let reason = if feasible {
            None
        } else {
            Some(format!(
                "Slippage {} bps exceeds max {} bps",
                cost.slippage_bps, self.max_slippage_bps
            ))
        };

        LegacyFeasibilityCheck {
            feasible,
            reason,
            cost,
        }
    }
}

/// Legacy execution plan
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub original_order: Order,
    pub parts: Vec<OrderPart>,
    pub estimated_slippage: Option<Decimal>,
    pub split: bool,
}

/// Legacy order part
#[derive(Debug, Clone)]
pub struct OrderPart {
    pub size: Decimal,
    pub delay_ms: u64,
    pub venue: String,
}

/// Legacy execution cost
#[derive(Debug, Clone)]
pub struct LegacyExecutionCost {
    pub slippage_bps: Decimal,
    pub slippage_usd: Decimal,
    pub fee_bps: Decimal,
    pub fee_usd: Decimal,
    pub total_bps: Decimal,
    pub total_usd: Decimal,
}

/// Legacy feasibility check
#[derive(Debug, Clone)]
pub struct LegacyFeasibilityCheck {
    pub feasible: bool,
    pub reason: Option<String>,
    pub cost: LegacyExecutionCost,
}

#[cfg(test)]
mod tests {
    use super::*;
    use simmons_core::OrderType;

    fn test_order() -> Order {
        Order {
            symbol: "BTC-USDT".to_string(),
            side: Side::Long,
            size: dec!(0.1),
            order_type: OrderType::Market,
            limit_price: Some(dec!(67000)),
            stop_loss: None,
            take_profit: None,
        }
    }

    #[test]
    fn test_route_no_quotes() {
        let router = SmartOrderRouter::with_defaults();
        let order = test_order();

        let routed = router.route(&order);
        assert_eq!(routed.routes.len(), 1);
        assert_eq!(routed.routes[0].venue, Venue::Primary);
    }

    #[test]
    fn test_route_with_quotes() {
        let mut router = SmartOrderRouter::with_defaults();

        // Add OKX quote (better price)
        router.update_quote(
            "BTC-USDT",
            VenueQuote {
                venue: Venue::Okx,
                price: dec!(67000),
                available_liquidity: dec!(1000000),
                fee_bps: dec!(10),
                estimated_slippage_bps: dec!(5),
                latency_ms: 50,
                timestamp: 0,
            },
        );

        // Add DEX quote (worse price)
        router.update_quote(
            "BTC-USDT",
            VenueQuote {
                venue: Venue::XLayerDex,
                price: dec!(67100),
                available_liquidity: dec!(500000),
                fee_bps: dec!(30),
                estimated_slippage_bps: dec!(15),
                latency_ms: 2000,
                timestamp: 0,
            },
        );

        let order = test_order();
        let routed = router.route(&order);

        // Should route to OKX (better effective price)
        assert_eq!(routed.routes[0].venue, Venue::Okx);
    }

    #[test]
    fn test_split_large_order() {
        let mut router = SmartOrderRouter::with_defaults();

        router.update_quote(
            "BTC-USDT",
            VenueQuote {
                venue: Venue::Okx,
                price: dec!(67000),
                available_liquidity: dec!(8000),
                fee_bps: dec!(10),
                estimated_slippage_bps: dec!(5),
                latency_ms: 50,
                timestamp: 0,
            },
        );

        router.update_quote(
            "BTC-USDT",
            VenueQuote {
                venue: Venue::XLayerDex,
                price: dec!(67050),
                available_liquidity: dec!(8000),
                fee_bps: dec!(30),
                estimated_slippage_bps: dec!(10),
                latency_ms: 2000,
                timestamp: 0,
            },
        );

        // Large order that should split
        let mut order = test_order();
        order.size = dec!(1.0);
        order.limit_price = Some(dec!(67000));

        let routed = router.route(&order);

        // Should have split across venues
        assert!(routed.split);
        assert!(routed.routes.len() >= 2);
    }

    #[test]
    fn test_compare_venues() {
        let mut router = SmartOrderRouter::with_defaults();

        router.update_quote(
            "BTC-USDT",
            VenueQuote {
                venue: Venue::Okx,
                price: dec!(67000),
                available_liquidity: dec!(1000000),
                fee_bps: dec!(10),
                estimated_slippage_bps: dec!(5),
                latency_ms: 50,
                timestamp: 0,
            },
        );

        router.update_quote(
            "BTC-USDT",
            VenueQuote {
                venue: Venue::XLayerDex,
                price: dec!(66900), // Slightly better raw price
                available_liquidity: dec!(500000),
                fee_bps: dec!(30),
                estimated_slippage_bps: dec!(15),
                latency_ms: 2000,
                timestamp: 0,
            },
        );

        let order = test_order();
        let comparisons = router.compare_venues(&order);

        assert_eq!(comparisons.len(), 2);
        // Should be sorted by effective price
        assert!(comparisons[0].total_cost_bps <= comparisons[1].total_cost_bps);
    }

    // Legacy tests
    #[test]
    fn test_legacy_no_split() {
        let router = LegacyRouter::default();
        // Use a small order that won't exceed split_threshold ($1000)
        let order = Order {
            symbol: "BTC-USDT".to_string(),
            side: Side::Long,
            size: dec!(0.01),  // 0.01 * 67000 = $670 < $1000
            order_type: OrderType::Market,
            limit_price: Some(dec!(67000)),
            stop_loss: None,
            take_profit: None,
        };

        let plan = router.create_execution_plan(&order, None);
        assert!(!plan.split);
        assert_eq!(plan.parts.len(), 1);
    }

    #[test]
    fn test_legacy_split() {
        let router = LegacyRouter::new(dec!(100), dec!(50));

        let mut order = test_order();
        order.size = dec!(1.0);

        let plan = router.create_execution_plan(&order, None);
        assert!(plan.split);
        assert!(plan.parts.len() > 1);
    }
}
