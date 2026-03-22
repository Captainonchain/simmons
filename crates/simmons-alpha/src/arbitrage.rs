//! Arbitrage detection

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_core::ArbOpportunity;

/// Arbitrage detector
pub struct ArbitrageEngine {
    /// Minimum spread in basis points to consider
    pub min_spread_bps: Decimal,
    /// Estimated transaction cost in basis points
    pub tx_cost_bps: Decimal,
    /// Slippage estimate in basis points
    pub slippage_bps: Decimal,
}

impl Default for ArbitrageEngine {
    fn default() -> Self {
        Self {
            min_spread_bps: dec!(20),   // 20 bps minimum
            tx_cost_bps: dec!(10),      // 10 bps tx cost
            slippage_bps: dec!(5),      // 5 bps slippage
        }
    }
}

impl ArbitrageEngine {
    pub fn new(min_spread_bps: Decimal) -> Self {
        Self {
            min_spread_bps,
            ..Default::default()
        }
    }

    /// Check for CeDeFi arbitrage (CEX vs DEX)
    pub fn check_cedefi_arb(
        &self,
        symbol: &str,
        cex_price: Decimal,
        dex_price: Decimal,
        capital: Decimal,
    ) -> Option<ArbOpportunity> {
        if cex_price.is_zero() || dex_price.is_zero() {
            return None;
        }

        let spread_bps = ((dex_price - cex_price).abs() / cex_price) * dec!(10000);

        // Check if spread exceeds costs
        let total_cost_bps = self.tx_cost_bps + self.slippage_bps;
        let net_spread_bps = spread_bps - total_cost_bps;

        if net_spread_bps < self.min_spread_bps {
            return None;
        }

        // Calculate profit
        let net_profit = capital * net_spread_bps / dec!(10000);

        let (buy_venue, sell_venue) = if cex_price < dex_price {
            ("cex".to_string(), "dex".to_string())
        } else {
            ("dex".to_string(), "cex".to_string())
        };

        Some(ArbOpportunity {
            arb_type: "cedefi".to_string(),
            spread_bps,
            net_profit_usd: net_profit,
            buy_venue,
            sell_venue,
        })
    }

    /// Check for cross-exchange arbitrage
    pub fn check_cross_exchange_arb(
        &self,
        symbol: &str,
        exchange_a_price: Decimal,
        exchange_a_name: &str,
        exchange_b_price: Decimal,
        exchange_b_name: &str,
        capital: Decimal,
    ) -> Option<ArbOpportunity> {
        if exchange_a_price.is_zero() || exchange_b_price.is_zero() {
            return None;
        }

        let mid_price = (exchange_a_price + exchange_b_price) / dec!(2);
        let spread_bps = ((exchange_a_price - exchange_b_price).abs() / mid_price) * dec!(10000);

        // Cross-exchange has higher costs (two tx fees)
        let total_cost_bps = self.tx_cost_bps * dec!(2) + self.slippage_bps * dec!(2);
        let net_spread_bps = spread_bps - total_cost_bps;

        if net_spread_bps < self.min_spread_bps {
            return None;
        }

        let net_profit = capital * net_spread_bps / dec!(10000);

        let (buy_venue, sell_venue) = if exchange_a_price < exchange_b_price {
            (exchange_a_name.to_string(), exchange_b_name.to_string())
        } else {
            (exchange_b_name.to_string(), exchange_a_name.to_string())
        };

        Some(ArbOpportunity {
            arb_type: "cross_exchange".to_string(),
            spread_bps,
            net_profit_usd: net_profit,
            buy_venue,
            sell_venue,
        })
    }

    /// Check for triangular arbitrage
    pub fn check_triangular_arb(
        &self,
        base: &str,
        quote: &str,
        intermediate: &str,
        rate_base_quote: Decimal,
        rate_base_intermediate: Decimal,
        rate_intermediate_quote: Decimal,
        capital: Decimal,
    ) -> Option<ArbOpportunity> {
        // Path 1: base -> intermediate -> quote
        let implied_rate = rate_base_intermediate * rate_intermediate_quote;

        // Check for discrepancy
        let arb_pct = ((implied_rate - rate_base_quote).abs() / rate_base_quote) * dec!(100);

        // Higher costs for triangular (three legs)
        let total_cost_pct = (self.tx_cost_bps + self.slippage_bps) * dec!(3) / dec!(100);

        if arb_pct < total_cost_pct + dec!(0.1) {
            return None;
        }

        let net_profit = capital * (arb_pct - total_cost_pct) / dec!(100);
        let spread_bps = arb_pct * dec!(100);

        Some(ArbOpportunity {
            arb_type: "triangular".to_string(),
            spread_bps,
            net_profit_usd: net_profit,
            buy_venue: format!("{}->{}->{}", base, intermediate, quote),
            sell_venue: format!("{}->{}", base, quote),
        })
    }

    /// Estimate execution profitability with slippage model
    pub fn estimate_execution_profit(
        &self,
        spread_bps: Decimal,
        order_size: Decimal,
        liquidity: Decimal,
    ) -> Decimal {
        // Slippage increases with order size relative to liquidity
        let size_impact = if liquidity.is_zero() {
            dec!(50) // High slippage if no liquidity info
        } else {
            (order_size / liquidity) * dec!(100) // Linear impact model
        };

        let total_slippage = self.slippage_bps + size_impact;
        let net_spread = spread_bps - self.tx_cost_bps - total_slippage;

        if net_spread.is_sign_positive() {
            order_size * net_spread / dec!(10000)
        } else {
            Decimal::ZERO
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cedefi_arb_profitable() {
        let engine = ArbitrageEngine::new(dec!(10));

        // 100 bps spread to overcome costs
        let arb = engine.check_cedefi_arb(
            "BTC-USDT",
            dec!(67000), // CEX
            dec!(67670), // DEX - ~100 bps higher (1% = $670)
            dec!(1000),  // $1000 capital
        );

        assert!(arb.is_some());
        let arb = arb.unwrap();
        assert_eq!(arb.buy_venue, "cex");
        assert_eq!(arb.sell_venue, "dex");
        assert!(arb.net_profit_usd > Decimal::ZERO);
    }

    #[test]
    fn test_cedefi_arb_too_small() {
        let engine = ArbitrageEngine::new(dec!(30)); // Higher minimum

        let arb = engine.check_cedefi_arb(
            "BTC-USDT",
            dec!(67000), // CEX
            dec!(67010), // DEX - only ~1.5 bps higher
            dec!(1000),
        );

        assert!(arb.is_none()); // Too small after costs
    }

    #[test]
    fn test_execution_profit_estimate() {
        let engine = ArbitrageEngine::default();

        // Small order relative to liquidity
        let profit = engine.estimate_execution_profit(
            dec!(50),      // 50 bps spread
            dec!(100),     // $100 order
            dec!(100000),  // $100k liquidity
        );
        assert!(profit > Decimal::ZERO);

        // Large order relative to liquidity (high slippage)
        let profit = engine.estimate_execution_profit(
            dec!(50),     // 50 bps spread
            dec!(10000),  // $10k order
            dec!(10000),  // $10k liquidity
        );
        assert!(profit <= Decimal::ZERO); // Slippage eats the profit
    }
}
