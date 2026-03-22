//! Types for Claude brain communication

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use simmons_core::{
    Action, ArbOpportunity, MarketState, PortfolioSnapshot, Position, Regime, Side,
    StrategySignal, Trade, TradeOutcome,
};

/// Input signals for Claude brain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainInput {
    /// Unix timestamp
    pub timestamp: i64,
    /// Primary symbol being analyzed
    pub symbol: String,
    /// Current market state
    pub market_state: BrainMarketState,
    /// Strategy signals
    pub signals: Vec<BrainSignal>,
    /// Arbitrage opportunities
    pub arbitrage: Vec<BrainArbOpportunity>,
    /// Current portfolio state
    pub portfolio: BrainPortfolio,
    /// Recent trade outcomes for learning
    pub recent_trades: Vec<BrainTradeOutcome>,
}

/// Market state for brain input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainMarketState {
    pub price: Decimal,
    pub spread_bps: Decimal,
    pub volatility_1h: Decimal,
    pub regime: String,
}

impl From<MarketState> for BrainMarketState {
    fn from(state: MarketState) -> Self {
        Self {
            price: state.price,
            spread_bps: state.spread_bps,
            volatility_1h: state.volatility_1h,
            regime: format!("{:?}", state.regime).to_lowercase(),
        }
    }
}

/// Strategy signal for brain input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainSignal {
    pub strategy: String,
    pub signal: String,
    pub confidence: Decimal,
    pub reason: String,
}

impl From<StrategySignal> for BrainSignal {
    fn from(sig: StrategySignal) -> Self {
        Self {
            strategy: sig.strategy,
            signal: format!("{:?}", sig.signal).to_uppercase(),
            confidence: sig.confidence,
            reason: sig.reason,
        }
    }
}

/// Arbitrage opportunity for brain input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainArbOpportunity {
    #[serde(rename = "type")]
    pub arb_type: String,
    pub spread_bps: Decimal,
    pub net_profit_usd: Decimal,
}

impl From<ArbOpportunity> for BrainArbOpportunity {
    fn from(arb: ArbOpportunity) -> Self {
        Self {
            arb_type: arb.arb_type,
            spread_bps: arb.spread_bps,
            net_profit_usd: arb.net_profit_usd,
        }
    }
}

/// Portfolio snapshot for brain input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainPortfolio {
    pub capital: Decimal,
    pub positions: Vec<BrainPosition>,
    pub drawdown: Decimal,
    pub risk_level: String,
}

impl From<PortfolioSnapshot> for BrainPortfolio {
    fn from(snapshot: PortfolioSnapshot) -> Self {
        Self {
            capital: snapshot.capital,
            positions: snapshot.positions.into_iter().map(Into::into).collect(),
            drawdown: snapshot.drawdown,
            risk_level: snapshot.risk_level,
        }
    }
}

/// Position for brain input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainPosition {
    pub symbol: String,
    pub side: String,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub unrealized_pnl: Decimal,
}

impl From<Position> for BrainPosition {
    fn from(pos: Position) -> Self {
        Self {
            symbol: pos.symbol,
            side: format!("{:?}", pos.side).to_lowercase(),
            size: pos.size,
            entry_price: pos.entry_price,
            unrealized_pnl: pos.unrealized_pnl,
        }
    }
}

/// Trade outcome for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainTradeOutcome {
    pub symbol: String,
    pub pnl: Decimal,
    pub outcome: String,
    pub reason: String,
}

impl From<Trade> for BrainTradeOutcome {
    fn from(trade: Trade) -> Self {
        Self {
            symbol: trade.symbol,
            pnl: trade.pnl,
            outcome: format!("{:?}", trade.outcome).to_lowercase(),
            reason: trade.reason,
        }
    }
}

/// Decision from Claude brain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainDecision {
    /// Action to take
    pub action: String,
    /// Symbol to trade (if action is trade)
    pub symbol: Option<String>,
    /// Trade side (if action is trade)
    pub side: Option<String>,
    /// Position size as percentage of capital
    pub size_pct: Option<Decimal>,
    /// Confidence in decision (0-1)
    pub confidence: Decimal,
    /// Reasoning for the decision
    pub reasoning: String,
    /// Stop loss percentage
    pub stop_loss_pct: Option<Decimal>,
    /// Take profit percentage
    pub take_profit_pct: Option<Decimal>,
}

impl BrainDecision {
    pub fn action(&self) -> Action {
        match self.action.to_lowercase().as_str() {
            "trade" => Action::Trade,
            "close" | "close_position" => Action::ClosePosition,
            _ => Action::Skip,
        }
    }

    pub fn side(&self) -> Option<Side> {
        self.side.as_ref().and_then(|s| match s.to_lowercase().as_str() {
            "long" | "buy" => Some(Side::Long),
            "short" | "sell" => Some(Side::Short),
            _ => None,
        })
    }

    /// Create a skip decision
    pub fn skip(reason: &str) -> Self {
        Self {
            action: "skip".to_string(),
            symbol: None,
            side: None,
            size_pct: None,
            confidence: Decimal::ZERO,
            reasoning: reason.to_string(),
            stop_loss_pct: None,
            take_profit_pct: None,
        }
    }
}

/// State file for persistence
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrainState {
    pub total_trades: u32,
    pub wins: u32,
    pub losses: u32,
    pub total_pnl: Decimal,
    pub last_decision: Option<BrainDecision>,
    pub learning_notes: Vec<String>,
}

impl BrainState {
    pub fn win_rate(&self) -> Decimal {
        if self.total_trades == 0 {
            return Decimal::ZERO;
        }
        Decimal::from(self.wins) / Decimal::from(self.total_trades)
    }

    pub fn record_trade(&mut self, outcome: TradeOutcome, pnl: Decimal) {
        self.total_trades += 1;
        self.total_pnl += pnl;
        match outcome {
            TradeOutcome::Win => self.wins += 1,
            TradeOutcome::Loss => self.losses += 1,
            TradeOutcome::Breakeven => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_brain_decision_parse() {
        let json = r#"{
            "action": "trade",
            "symbol": "BTC-USDT",
            "side": "long",
            "size_pct": 0.12,
            "confidence": 0.85,
            "reasoning": "Strong momentum signal",
            "stop_loss_pct": 0.03,
            "take_profit_pct": 0.08
        }"#;

        let decision: BrainDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision.action(), Action::Trade);
        assert_eq!(decision.side(), Some(Side::Long));
        assert_eq!(decision.size_pct, Some(dec!(0.12)));
    }

    #[test]
    fn test_brain_state() {
        let mut state = BrainState::default();
        state.record_trade(TradeOutcome::Win, dec!(10));
        state.record_trade(TradeOutcome::Win, dec!(5));
        state.record_trade(TradeOutcome::Loss, dec!(-8));

        assert_eq!(state.total_trades, 3);
        assert_eq!(state.wins, 2);
        assert_eq!(state.losses, 1);
        assert_eq!(state.total_pnl, dec!(7));
    }
}
