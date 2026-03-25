//! Trading state management for MCP server
//!
//! Maintains portfolio, positions, trade history, and signals.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::{
    Action, ArbOpportunity, MarketState, Position, PortfolioSnapshot, Regime, Side,
    StrategySignal, TradeOutcome,
};
use std::collections::VecDeque;
use std::sync::Arc;
use uuid::Uuid;

/// Maximum trade history to keep
const MAX_HISTORY: usize = 100;

/// Trading decision from Claude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeDecision {
    pub action: Action,
    pub symbol: Option<String>,
    pub side: Option<Side>,
    pub size_pct: Option<Decimal>,
    pub confidence: Decimal,
    pub reasoning: String,
    pub stop_loss_pct: Option<Decimal>,
    pub take_profit_pct: Option<Decimal>,
}

/// Result of a trade submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeResult {
    pub success: bool,
    pub trade_id: Option<String>,
    pub message: String,
    pub executed_price: Option<Decimal>,
    pub executed_size: Option<Decimal>,
}

/// Current signals snapshot for Claude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub market_states: Vec<MarketState>,
    pub signals: Vec<StrategySignal>,
    pub arbitrage: Vec<ArbOpportunity>,
    pub regime: Regime,
    pub recent_trades: Vec<TradeRecord>,
}

/// Simplified trade record for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: String,
    pub symbol: String,
    pub side: Side,
    pub entry_price: Decimal,
    pub exit_price: Option<Decimal>,
    pub pnl: Option<Decimal>,
    pub outcome: Option<TradeOutcome>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub reasoning: String,
}

/// Regime state for Claude
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeState {
    pub current: Regime,
    pub confidence: Decimal,
    pub volatility_1h: Decimal,
    pub trend_strength: Decimal,
    pub recommendation: String,
}

/// Shared trading state
pub struct TradingState {
    inner: Arc<RwLock<TradingStateInner>>,
}

struct TradingStateInner {
    /// Starting capital
    capital: Decimal,
    /// Current available balance
    balance: Decimal,
    /// Active positions
    positions: Vec<Position>,
    /// Trade history (recent first)
    history: VecDeque<TradeRecord>,
    /// Current market states
    market_states: Vec<MarketState>,
    /// Current signals
    signals: Vec<StrategySignal>,
    /// Arbitrage opportunities
    arbitrage: Vec<ArbOpportunity>,
    /// Current regime
    regime: Regime,
    /// Realized P&L
    realized_pnl: Decimal,
    /// Max drawdown seen
    max_drawdown: Decimal,
    /// Daily P&L
    daily_pnl: Decimal,
    /// Consecutive losses
    consecutive_losses: u32,
    /// Last update timestamp
    last_update: DateTime<Utc>,
}

impl Clone for TradingState {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl std::fmt::Debug for TradingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TradingState")
            .field("capital", &self.inner.read().capital)
            .field("balance", &self.inner.read().balance)
            .field("positions", &self.inner.read().positions.len())
            .finish()
    }
}

impl Default for TradingState {
    fn default() -> Self {
        Self::new(dec!(1000))
    }
}

impl TradingState {
    /// Create new trading state with initial capital
    pub fn new(capital: Decimal) -> Self {
        Self {
            inner: Arc::new(RwLock::new(TradingStateInner {
                capital,
                balance: capital,
                positions: Vec::new(),
                history: VecDeque::with_capacity(MAX_HISTORY),
                market_states: Vec::new(),
                signals: Vec::new(),
                arbitrage: Vec::new(),
                regime: Regime::MeanReverting,
                realized_pnl: Decimal::ZERO,
                max_drawdown: Decimal::ZERO,
                daily_pnl: Decimal::ZERO,
                consecutive_losses: 0,
                last_update: Utc::now(),
            })),
        }
    }

    /// Create with sample data for testing
    pub fn with_sample_data(capital: Decimal) -> Self {
        use simmons_core::Signal;

        let state = Self::new(capital);

        // Add sample market state
        let market_states = vec![
            MarketState {
                symbol: "BTC-USDT".to_string(),
                price: dec!(67000),
                spread_bps: dec!(2.5),
                volatility_1h: dec!(0.023),
                regime: Regime::TrendingUp,
                cex_price: Some(dec!(67000)),
                dex_price: Some(dec!(66950)),
            },
            MarketState {
                symbol: "ETH-USDT".to_string(),
                price: dec!(3500),
                spread_bps: dec!(3.0),
                volatility_1h: dec!(0.028),
                regime: Regime::MeanReverting,
                cex_price: Some(dec!(3500)),
                dex_price: Some(dec!(3495)),
            },
        ];

        // Add sample signals
        let signals = vec![
            StrategySignal {
                strategy: "momentum".to_string(),
                signal: Signal::Buy,
                confidence: dec!(0.78),
                reason: "RSI oversold at 32, positive momentum divergence".to_string(),
            },
            StrategySignal {
                strategy: "mean_reversion".to_string(),
                signal: Signal::StrongBuy,
                confidence: dec!(0.82),
                reason: "Z-score at -2.1, price 2 std below mean".to_string(),
            },
            StrategySignal {
                strategy: "regime".to_string(),
                signal: Signal::Buy,
                confidence: dec!(0.70),
                reason: "Trending up regime detected, momentum favorable".to_string(),
            },
        ];

        // Add sample arbitrage opportunity
        let arbitrage = vec![ArbOpportunity {
            arb_type: "cedefi".to_string(),
            spread_bps: dec!(45),
            net_profit_usd: dec!(4.50),
            buy_venue: "dex".to_string(),
            sell_venue: "cex".to_string(),
        }];

        state.update_signals(market_states, signals, arbitrage, Regime::TrendingUp);
        state
    }

    /// Update signals from alpha engine
    pub fn update_signals(
        &self,
        market_states: Vec<MarketState>,
        signals: Vec<StrategySignal>,
        arbitrage: Vec<ArbOpportunity>,
        regime: Regime,
    ) {
        let mut state = self.inner.write();
        state.market_states = market_states;
        state.signals = signals;
        state.arbitrage = arbitrage;
        state.regime = regime;
        state.last_update = Utc::now();
    }

    /// Get current signals snapshot
    pub fn get_signals(&self) -> SignalsSnapshot {
        let state = self.inner.read();
        SignalsSnapshot {
            timestamp: state.last_update,
            market_states: state.market_states.clone(),
            signals: state.signals.clone(),
            arbitrage: state.arbitrage.clone(),
            regime: state.regime,
            recent_trades: state.history.iter().take(5).cloned().collect(),
        }
    }

    /// Get portfolio snapshot
    pub fn get_portfolio(&self) -> PortfolioSnapshot {
        let state = self.inner.read();

        // Calculate unrealized P&L
        let unrealized_pnl: Decimal = state.positions.iter().map(|p| p.unrealized_pnl).sum();

        // Calculate current drawdown
        let current_equity = state.balance + unrealized_pnl;
        let drawdown = if state.capital > Decimal::ZERO {
            ((state.capital - current_equity) / state.capital).max(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };

        // Determine risk level
        let risk_level = if drawdown > dec!(0.15) {
            "critical".to_string()
        } else if drawdown > dec!(0.10) || state.consecutive_losses >= 3 {
            "elevated".to_string()
        } else {
            "normal".to_string()
        };

        PortfolioSnapshot {
            capital: state.capital,
            positions: state.positions.clone(),
            drawdown,
            risk_level,
            realized_pnl: state.realized_pnl,
            unrealized_pnl,
        }
    }

    /// Get trade history
    pub fn get_history(&self, limit: usize) -> Vec<TradeRecord> {
        let state = self.inner.read();
        state.history.iter().take(limit).cloned().collect()
    }

    /// Get current regime state
    pub fn get_regime(&self) -> RegimeState {
        let state = self.inner.read();

        let (confidence, trend_strength, recommendation) = match state.regime {
            Regime::TrendingUp => (
                dec!(0.75),
                dec!(0.8),
                "Favor long positions, momentum strategies".to_string(),
            ),
            Regime::TrendingDown => (
                dec!(0.75),
                dec!(-0.8),
                "Favor short positions or stay flat".to_string(),
            ),
            Regime::MeanReverting => (
                dec!(0.6),
                dec!(0.0),
                "Mean reversion strategies, range trading".to_string(),
            ),
            Regime::HighVolatility => (
                dec!(0.5),
                dec!(0.0),
                "Reduce position sizes, wider stops".to_string(),
            ),
            Regime::LowVolatility => (
                dec!(0.5),
                dec!(0.0),
                "Watch for breakouts, consider vol expansion".to_string(),
            ),
            Regime::Choppy => (
                dec!(0.3),
                dec!(0.0),
                "Avoid trading, signals unreliable".to_string(),
            ),
        };

        // Get volatility from market states
        let volatility = state
            .market_states
            .first()
            .map(|m| m.volatility_1h)
            .unwrap_or(dec!(0.02));

        RegimeState {
            current: state.regime,
            confidence,
            volatility_1h: volatility,
            trend_strength,
            recommendation,
        }
    }

    /// Submit a trade decision
    pub fn submit_trade(&self, decision: TradeDecision) -> TradeResult {
        let mut state = self.inner.write();

        match decision.action {
            Action::Skip => TradeResult {
                success: true,
                trade_id: None,
                message: "Trade skipped as requested".to_string(),
                executed_price: None,
                executed_size: None,
            },

            Action::ClosePosition => {
                // Find and close position
                let symbol = decision.symbol.unwrap_or_default();
                if let Some(pos_idx) = state.positions.iter().position(|p| p.symbol == symbol) {
                    let pos = state.positions.remove(pos_idx);
                    let pnl = pos.unrealized_pnl;

                    // Update state
                    state.balance += pos.size * pos.entry_price + pnl;
                    state.realized_pnl += pnl;

                    // Update consecutive losses
                    if pnl < Decimal::ZERO {
                        state.consecutive_losses += 1;
                    } else {
                        state.consecutive_losses = 0;
                    }

                    // Record in history
                    let record = TradeRecord {
                        id: pos.id.clone(),
                        symbol: pos.symbol.clone(),
                        side: pos.side,
                        entry_price: pos.entry_price,
                        exit_price: Some(pos.current_price),
                        pnl: Some(pnl),
                        outcome: Some(if pnl > Decimal::ZERO {
                            TradeOutcome::Win
                        } else if pnl < Decimal::ZERO {
                            TradeOutcome::Loss
                        } else {
                            TradeOutcome::Breakeven
                        }),
                        opened_at: pos.opened_at,
                        closed_at: Some(Utc::now()),
                        reasoning: decision.reasoning,
                    };

                    if state.history.len() >= MAX_HISTORY {
                        state.history.pop_back();
                    }
                    state.history.push_front(record);

                    TradeResult {
                        success: true,
                        trade_id: Some(pos.id),
                        message: format!("Position closed, P&L: {:.2}", pnl),
                        executed_price: Some(pos.current_price),
                        executed_size: Some(pos.size),
                    }
                } else {
                    TradeResult {
                        success: false,
                        trade_id: None,
                        message: format!("No position found for {}", symbol),
                        executed_price: None,
                        executed_size: None,
                    }
                }
            }

            Action::Trade => {
                let symbol = decision.symbol.unwrap_or_else(|| "BTC-USDT".to_string());
                let side = decision.side.unwrap_or(Side::Long);
                let size_pct = decision.size_pct.unwrap_or(dec!(0.1));

                // Get current price from market states
                let current_price = state
                    .market_states
                    .iter()
                    .find(|m| m.symbol == symbol)
                    .map(|m| m.price)
                    .unwrap_or(dec!(50000)); // Default for BTC

                // Calculate position size
                let size_usd = state.balance * size_pct;
                let size = size_usd / current_price;

                // Check if we have enough balance
                if size_usd > state.balance {
                    return TradeResult {
                        success: false,
                        trade_id: None,
                        message: "Insufficient balance".to_string(),
                        executed_price: None,
                        executed_size: None,
                    };
                }

                // Calculate stop loss and take profit prices
                let stop_loss = decision.stop_loss_pct.map(|pct| match side {
                    Side::Long => current_price * (Decimal::ONE - pct),
                    Side::Short => current_price * (Decimal::ONE + pct),
                });

                let take_profit = decision.take_profit_pct.map(|pct| match side {
                    Side::Long => current_price * (Decimal::ONE + pct),
                    Side::Short => current_price * (Decimal::ONE - pct),
                });

                // Create position
                let trade_id = Uuid::new_v4().to_string();
                let position = Position {
                    id: trade_id.clone(),
                    symbol: symbol.clone(),
                    side,
                    size,
                    entry_price: current_price,
                    current_price,
                    stop_loss,
                    take_profit,
                    opened_at: Utc::now(),
                    unrealized_pnl: Decimal::ZERO,
                };

                // Update state
                state.balance -= size_usd;
                state.positions.push(position);

                // Record in history (open position)
                let record = TradeRecord {
                    id: trade_id.clone(),
                    symbol: symbol.clone(),
                    side,
                    entry_price: current_price,
                    exit_price: None,
                    pnl: None,
                    outcome: None,
                    opened_at: Utc::now(),
                    closed_at: None,
                    reasoning: decision.reasoning,
                };

                if state.history.len() >= MAX_HISTORY {
                    state.history.pop_back();
                }
                state.history.push_front(record);

                TradeResult {
                    success: true,
                    trade_id: Some(trade_id),
                    message: format!(
                        "Opened {} {} position at {:.2}",
                        side_str(side),
                        symbol,
                        current_price
                    ),
                    executed_price: Some(current_price),
                    executed_size: Some(size),
                }
            }
        }
    }

    /// Record outcome for a closed trade (for learning)
    pub fn record_outcome(&self, trade_id: &str, outcome: TradeOutcome, reflection: &str) -> bool {
        let mut state = self.inner.write();

        if let Some(record) = state.history.iter_mut().find(|r| r.id == trade_id) {
            record.outcome = Some(outcome);
            // Append reflection to reasoning
            record.reasoning = format!("{}\n\nReflection: {}", record.reasoning, reflection);
            true
        } else {
            false
        }
    }

    /// Update position prices (called by engine)
    pub fn update_prices(&self, symbol: &str, price: Decimal) {
        let mut state = self.inner.write();
        for pos in &mut state.positions {
            if pos.symbol == symbol {
                pos.update_pnl(price);
            }
        }
    }

    /// Check circuit breakers
    pub fn check_circuit_breaker(&self) -> Option<String> {
        let state = self.inner.read();

        // Check consecutive losses
        if state.consecutive_losses >= 3 {
            return Some(format!(
                "Circuit breaker: {} consecutive losses",
                state.consecutive_losses
            ));
        }

        // Check drawdown
        let current_equity: Decimal = state.balance
            + state
                .positions
                .iter()
                .map(|p| p.unrealized_pnl)
                .sum::<Decimal>();
        let drawdown = (state.capital - current_equity) / state.capital;

        if drawdown > dec!(0.20) {
            return Some(format!(
                "Circuit breaker: {:.1}% drawdown exceeded 20% limit",
                drawdown * dec!(100)
            ));
        }

        None
    }

    /// Get detailed circuit breaker status
    pub fn get_circuit_breaker_status(&self) -> CircuitBreakerStatus {
        let state = self.inner.read();

        // Calculate current equity and drawdown
        let unrealized_pnl: Decimal = state.positions.iter().map(|p| p.unrealized_pnl).sum();
        let current_equity = state.balance + unrealized_pnl;
        let drawdown = if state.capital > Decimal::ZERO {
            ((state.capital - current_equity) / state.capital).max(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };

        // Determine risk level
        let risk_level = if drawdown > dec!(0.15) || state.consecutive_losses >= 3 {
            RiskLevel::Critical
        } else if drawdown > dec!(0.10) || state.consecutive_losses >= 2 {
            RiskLevel::Elevated
        } else {
            RiskLevel::Normal
        };

        // Calculate position size modifier
        let position_size_modifier = match risk_level {
            RiskLevel::Critical => dec!(0.0),  // No trading
            RiskLevel::Elevated => dec!(0.5),  // Half size
            RiskLevel::Normal => dec!(1.0),    // Full size
        };

        // Check if any breaker is triggered
        let breaker_triggered = state.consecutive_losses >= 3 || drawdown > dec!(0.20);

        // Build reason if triggered
        let trigger_reason = if state.consecutive_losses >= 3 {
            Some(format!("{} consecutive losses", state.consecutive_losses))
        } else if drawdown > dec!(0.20) {
            Some(format!("{:.1}% drawdown exceeds 20% limit", drawdown * dec!(100)))
        } else {
            None
        };

        CircuitBreakerStatus {
            triggered: breaker_triggered,
            reason: trigger_reason,
            risk_level,
            current_drawdown: drawdown,
            max_drawdown_limit: dec!(0.20),
            consecutive_losses: state.consecutive_losses,
            max_consecutive_losses: 3,
            position_size_modifier,
            can_trade: !breaker_triggered && risk_level != RiskLevel::Critical,
            recommendations: self.get_risk_recommendations(&risk_level, drawdown, state.consecutive_losses),
        }
    }

    /// Get risk recommendations based on current state
    fn get_risk_recommendations(&self, risk_level: &RiskLevel, drawdown: Decimal, consecutive_losses: u32) -> Vec<String> {
        let mut recommendations = Vec::new();

        match risk_level {
            RiskLevel::Critical => {
                recommendations.push("STOP TRADING - Circuit breaker active".to_string());
                recommendations.push("Review recent losses for patterns".to_string());
                recommendations.push("Wait for market conditions to improve".to_string());
            }
            RiskLevel::Elevated => {
                recommendations.push("Reduce position sizes by 50%".to_string());
                recommendations.push("Only take high-conviction setups (>75%)".to_string());
                recommendations.push("Tighten stop losses".to_string());
            }
            RiskLevel::Normal => {
                recommendations.push("Normal trading permitted".to_string());
            }
        }

        if drawdown > dec!(0.10) {
            recommendations.push(format!(
                "Drawdown at {:.1}% - consider defensive positioning",
                drawdown * dec!(100)
            ));
        }

        if consecutive_losses >= 2 {
            recommendations.push(format!(
                "{} losses in a row - verify strategy alignment",
                consecutive_losses
            ));
        }

        recommendations
    }

    /// Get consecutive losses count
    pub fn get_consecutive_losses(&self) -> u32 {
        self.inner.read().consecutive_losses
    }

    /// Reset consecutive losses (after a win)
    pub fn reset_consecutive_losses(&self) {
        self.inner.write().consecutive_losses = 0;
    }
}

/// Risk level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Normal,
    Elevated,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Normal => write!(f, "normal"),
            RiskLevel::Elevated => write!(f, "elevated"),
            RiskLevel::Critical => write!(f, "critical"),
        }
    }
}

/// Detailed circuit breaker status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStatus {
    pub triggered: bool,
    pub reason: Option<String>,
    pub risk_level: RiskLevel,
    pub current_drawdown: Decimal,
    pub max_drawdown_limit: Decimal,
    pub consecutive_losses: u32,
    pub max_consecutive_losses: u32,
    pub position_size_modifier: Decimal,
    pub can_trade: bool,
    pub recommendations: Vec<String>,
}

fn side_str(side: Side) -> &'static str {
    match side {
        Side::Long => "LONG",
        Side::Short => "SHORT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trading_state() {
        let state = TradingState::new(dec!(1000));

        // Test initial portfolio
        let portfolio = state.get_portfolio();
        assert_eq!(portfolio.capital, dec!(1000));
        assert!(portfolio.positions.is_empty());

        // Submit a trade
        let decision = TradeDecision {
            action: Action::Trade,
            symbol: Some("BTC-USDT".to_string()),
            side: Some(Side::Long),
            size_pct: Some(dec!(0.1)),
            confidence: dec!(0.8),
            reasoning: "Test trade".to_string(),
            stop_loss_pct: Some(dec!(0.02)),
            take_profit_pct: Some(dec!(0.05)),
        };

        let result = state.submit_trade(decision);
        assert!(result.success);
        assert!(result.trade_id.is_some());

        // Check portfolio updated
        let portfolio = state.get_portfolio();
        assert_eq!(portfolio.positions.len(), 1);
    }
}
