//! Risk governor - enforces trading limits

use crate::kelly::KellyCriterion;
use crate::portfolio::Portfolio;
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_core::{config::RiskConfig, error::RiskError, Order, OrderType, Side};
use std::sync::Arc;

/// Risk governor that enforces trading limits
pub struct RiskGovernor {
    /// Portfolio reference
    portfolio: Arc<Portfolio>,
    /// Risk configuration
    config: RiskConfig,
    /// Kelly calculator
    kelly: KellyCriterion,
    /// Trading halted flag
    halted: RwLock<bool>,
    /// Daily loss tracker
    daily_loss: RwLock<Decimal>,
}

impl RiskGovernor {
    pub fn new(portfolio: Arc<Portfolio>, config: RiskConfig) -> Self {
        let kelly = KellyCriterion {
            fraction: config.kelly_fraction,
            max_position: config.max_position_pct,
            min_position: dec!(0.01),
        };

        Self {
            portfolio,
            config,
            kelly,
            halted: RwLock::new(false),
            daily_loss: RwLock::new(Decimal::ZERO),
        }
    }

    /// Check if trading is allowed
    pub fn can_trade(&self) -> Result<(), RiskError> {
        // Check halt
        if *self.halted.read() {
            return Err(RiskError::TradingHalted);
        }

        // Check drawdown
        let drawdown = self.portfolio.drawdown();
        if drawdown > self.config.max_drawdown {
            return Err(RiskError::MaxDrawdownExceeded {
                current: drawdown * dec!(100),
                limit: self.config.max_drawdown * dec!(100),
            });
        }

        // Check daily loss
        let daily_loss = *self.daily_loss.read();
        if daily_loss > self.config.daily_loss_limit {
            return Err(RiskError::DailyLossLimit { loss: daily_loss });
        }

        // Check position count
        if self.portfolio.position_count() >= self.config.max_positions {
            return Err(RiskError::MaxPositionExceeded {
                size: Decimal::from(self.portfolio.position_count()),
                limit: Decimal::from(self.config.max_positions),
            });
        }

        Ok(())
    }

    /// Calculate position size using Kelly criterion
    pub fn calculate_position_size(
        &self,
        confidence: Decimal,
        risk_reward: Option<(Decimal, Decimal)>,
    ) -> Decimal {
        let capital = self.portfolio.total_equity();

        // Use historical data if available
        let (avg_win, avg_loss) = self.portfolio.avg_win_loss();
        let win_rate = self.portfolio.win_rate();

        let base_size = if win_rate > Decimal::ZERO && avg_loss > Decimal::ZERO {
            // Use Kelly with historical data
            self.kelly.from_history(
                (win_rate * dec!(100)).to_string().parse().unwrap_or(50),
                ((dec!(1) - win_rate) * dec!(100)).to_string().parse().unwrap_or(50),
                avg_win,
                avg_loss,
            )
        } else if let Some((win, loss)) = risk_reward {
            // Use Kelly with provided risk/reward
            self.kelly.calculate(confidence, win, loss)
        } else {
            // Default to minimum
            self.config.max_position_pct / dec!(3)
        };

        // Adjust by confidence
        let adjusted = self.kelly.with_confidence(base_size, confidence);

        // Convert to dollar amount
        capital * adjusted
    }

    /// Size an order with risk constraints
    pub fn size_order(
        &self,
        symbol: &str,
        side: Side,
        entry_price: Decimal,
        confidence: Decimal,
        stop_loss_pct: Option<Decimal>,
        take_profit_pct: Option<Decimal>,
    ) -> Result<Order, RiskError> {
        // Check if we can trade
        self.can_trade()?;

        // Calculate position size
        let sl_pct = stop_loss_pct.unwrap_or(self.config.default_stop_loss_pct);
        let tp_pct = take_profit_pct.unwrap_or(self.config.default_take_profit_pct);

        // Risk/reward ratio
        let potential_loss = entry_price * sl_pct;
        let potential_win = entry_price * tp_pct;

        let position_usd = self.calculate_position_size(confidence, Some((potential_win, potential_loss)));

        // Calculate size in asset terms
        let size = position_usd / entry_price;

        // Calculate stop/take levels
        let stop_loss = match side {
            Side::Long => Some(entry_price * (dec!(1) - sl_pct)),
            Side::Short => Some(entry_price * (dec!(1) + sl_pct)),
        };

        let take_profit = match side {
            Side::Long => Some(entry_price * (dec!(1) + tp_pct)),
            Side::Short => Some(entry_price * (dec!(1) - tp_pct)),
        };

        Ok(Order {
            symbol: symbol.to_string(),
            side,
            size,
            order_type: OrderType::Market,
            limit_price: None,
            stop_loss,
            take_profit,
        })
    }

    /// Record a loss for daily tracking
    pub fn record_loss(&self, amount: Decimal) {
        *self.daily_loss.write() += amount;
    }

    /// Reset daily loss tracker
    pub fn reset_daily(&self) {
        *self.daily_loss.write() = Decimal::ZERO;
    }

    /// Halt trading
    pub fn halt(&self) {
        *self.halted.write() = true;
    }

    /// Resume trading
    pub fn resume(&self) {
        *self.halted.write() = false;
    }

    /// Check if halted
    pub fn is_halted(&self) -> bool {
        *self.halted.read()
    }

    /// Get risk metrics
    pub fn metrics(&self) -> RiskMetrics {
        RiskMetrics {
            drawdown: self.portfolio.drawdown(),
            max_drawdown: self.config.max_drawdown,
            daily_loss: *self.daily_loss.read(),
            daily_limit: self.config.daily_loss_limit,
            position_count: self.portfolio.position_count(),
            max_positions: self.config.max_positions,
            win_rate: self.portfolio.win_rate(),
            halted: *self.halted.read(),
        }
    }
}

/// Risk metrics summary
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub drawdown: Decimal,
    pub max_drawdown: Decimal,
    pub daily_loss: Decimal,
    pub daily_limit: Decimal,
    pub position_count: usize,
    pub max_positions: usize,
    pub win_rate: Decimal,
    pub halted: bool,
}

impl RiskMetrics {
    pub fn is_healthy(&self) -> bool {
        !self.halted
            && self.drawdown < self.max_drawdown * dec!(0.8)
            && self.daily_loss < self.daily_limit * dec!(0.8)
            && self.position_count < self.max_positions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simmons_core::config::RiskConfig;
    use std::sync::Arc;

    fn make_governor() -> RiskGovernor {
        let portfolio = Arc::new(Portfolio::new(dec!(1000)));
        let config = RiskConfig::default();
        RiskGovernor::new(portfolio, config)
    }

    #[test]
    fn test_can_trade_normal() {
        let gov = make_governor();
        assert!(gov.can_trade().is_ok());
    }

    #[test]
    fn test_can_trade_halted() {
        let gov = make_governor();
        gov.halt();
        assert!(matches!(gov.can_trade(), Err(RiskError::TradingHalted)));
    }

    #[test]
    fn test_position_sizing() {
        let gov = make_governor();

        // High confidence should give larger size
        let high_conf = gov.calculate_position_size(dec!(0.9), Some((dec!(100), dec!(50))));
        let low_conf = gov.calculate_position_size(dec!(0.5), Some((dec!(100), dec!(50))));

        assert!(high_conf > low_conf);
    }

    #[test]
    fn test_daily_loss_tracking() {
        let portfolio = Arc::new(Portfolio::new(dec!(1000)));
        let mut config = RiskConfig::default();
        config.daily_loss_limit = dec!(50);

        let gov = RiskGovernor::new(portfolio, config);

        gov.record_loss(dec!(30));
        assert!(gov.can_trade().is_ok());

        gov.record_loss(dec!(25));
        assert!(matches!(gov.can_trade(), Err(RiskError::DailyLossLimit { .. })));

        gov.reset_daily();
        assert!(gov.can_trade().is_ok());
    }
}
