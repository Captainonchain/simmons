//! MCP tool parameter definitions
//!
//! Defines the JSON schema for all tools exposed by the Simmons MCP server.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::Deserialize;
use simmons_core::{Action, Side, TradeOutcome};

/// Parameters for get_signals tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSignalsParams {
    /// Optional symbol filter (e.g., "BTC-USDT")
    #[schemars(description = "Filter signals to a specific symbol (optional)")]
    pub symbol: Option<String>,
}

/// Parameters for get_portfolio tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPortfolioParams {
    /// Include position details
    #[schemars(description = "Include full position details (default: true)")]
    pub include_positions: Option<bool>,
}

/// Parameters for get_history tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetHistoryParams {
    /// Maximum number of trades to return
    #[schemars(description = "Maximum number of trades to return (default: 10, max: 50)")]
    pub limit: Option<usize>,

    /// Filter by outcome
    #[schemars(description = "Filter by trade outcome: win, loss, or breakeven")]
    pub outcome: Option<String>,
}

/// Parameters for submit_trade tool
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubmitTradeParams {
    /// Trading action
    #[schemars(description = "Action to take: trade, skip, or close_position")]
    pub action: String,

    /// Symbol to trade
    #[schemars(description = "Trading pair symbol (e.g., 'BTC-USDT')")]
    pub symbol: Option<String>,

    /// Trade side
    #[schemars(description = "Trade direction: long or short")]
    pub side: Option<String>,

    /// Position size as percentage of capital
    #[schemars(description = "Position size as decimal (0.10 = 10% of capital)")]
    pub size_pct: Option<f64>,

    /// Confidence level
    #[schemars(description = "Confidence in the trade (0.0 to 1.0)")]
    pub confidence: f64,

    /// Reasoning for the decision
    #[schemars(description = "Explanation for why this decision was made")]
    pub reasoning: String,

    /// Stop loss percentage
    #[schemars(description = "Stop loss as decimal (0.02 = 2% below entry)")]
    pub stop_loss_pct: Option<f64>,

    /// Take profit percentage
    #[schemars(description = "Take profit as decimal (0.05 = 5% above entry)")]
    pub take_profit_pct: Option<f64>,
}

impl SubmitTradeParams {
    /// Parse action string to Action enum
    pub fn parse_action(&self) -> Option<Action> {
        match self.action.to_lowercase().as_str() {
            "trade" | "buy" | "sell" => Some(Action::Trade),
            "skip" | "hold" | "wait" => Some(Action::Skip),
            "close" | "close_position" | "exit" => Some(Action::ClosePosition),
            _ => None,
        }
    }

    /// Parse side string to Side enum
    pub fn parse_side(&self) -> Option<Side> {
        self.side.as_ref().and_then(|s| match s.to_lowercase().as_str() {
            "long" | "buy" => Some(Side::Long),
            "short" | "sell" => Some(Side::Short),
            _ => None,
        })
    }

    /// Convert size_pct f64 to Decimal
    pub fn size_as_decimal(&self) -> Option<Decimal> {
        self.size_pct.and_then(|v| Decimal::try_from(v).ok())
    }

    /// Convert confidence f64 to Decimal
    pub fn confidence_as_decimal(&self) -> Decimal {
        Decimal::try_from(self.confidence).unwrap_or(rust_decimal_macros::dec!(0.5))
    }

    /// Convert stop_loss_pct f64 to Decimal
    pub fn stop_loss_as_decimal(&self) -> Option<Decimal> {
        self.stop_loss_pct.and_then(|v| Decimal::try_from(v).ok())
    }

    /// Convert take_profit_pct f64 to Decimal
    pub fn take_profit_as_decimal(&self) -> Option<Decimal> {
        self.take_profit_pct.and_then(|v| Decimal::try_from(v).ok())
    }
}

/// Parameters for record_outcome tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecordOutcomeParams {
    /// Trade ID to record outcome for
    #[schemars(description = "The trade ID to record outcome for")]
    pub trade_id: String,

    /// Trade outcome
    #[schemars(description = "Trade outcome: win, loss, or breakeven")]
    pub outcome: String,

    /// Reflection on the trade
    #[schemars(description = "Reflection on what worked or didn't work")]
    pub reflection: String,
}

impl RecordOutcomeParams {
    /// Parse outcome string to TradeOutcome enum
    pub fn parse_outcome(&self) -> Option<TradeOutcome> {
        match self.outcome.to_lowercase().as_str() {
            "win" | "profit" | "positive" => Some(TradeOutcome::Win),
            "loss" | "lose" | "negative" => Some(TradeOutcome::Loss),
            "breakeven" | "flat" | "neutral" => Some(TradeOutcome::Breakeven),
            _ => None,
        }
    }
}

/// Parameters for get_regime tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRegimeParams {
    /// Symbol to check regime for
    #[schemars(description = "Symbol to check regime for (optional, uses primary by default)")]
    pub symbol: Option<String>,
}

/// Parameters for get_memory tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMemoryParams {
    /// Filter by agent type
    #[schemars(description = "Filter learnings by agent type (e.g., 'technical_analyst')")]
    pub agent: Option<String>,

    /// Filter by category
    #[schemars(description = "Filter by category: winning_pattern, mistake, market_insight, risk_lesson")]
    pub category: Option<String>,
}

/// Parameters for add_reflection tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddReflectionParams {
    /// Trade ID
    #[schemars(description = "The trade ID this reflection is for")]
    pub trade_id: String,

    /// Symbol traded
    #[schemars(description = "Symbol that was traded")]
    pub symbol: String,

    /// Trade side
    #[schemars(description = "Trade side: long or short")]
    pub side: String,

    /// Entry price
    #[schemars(description = "Entry price")]
    pub entry_price: f64,

    /// Exit price
    #[schemars(description = "Exit price")]
    pub exit_price: f64,

    /// P&L amount
    #[schemars(description = "Profit/loss in USD")]
    pub pnl: f64,

    /// Outcome
    #[schemars(description = "Trade outcome: win, loss, or breakeven")]
    pub outcome: String,

    /// Duration in minutes
    #[schemars(description = "How long the trade was held in minutes")]
    pub duration_minutes: i64,

    /// Agent predictions (JSON object)
    #[schemars(description = "JSON object with agent predictions: {agent_name: {recommendation, confidence, was_correct, key_reason}}")]
    pub agent_predictions: Option<String>,

    /// What worked
    #[schemars(description = "List of things that worked well")]
    pub what_worked: Vec<String>,

    /// What failed
    #[schemars(description = "List of things that failed or could improve")]
    pub what_failed: Vec<String>,

    /// Lessons learned
    #[schemars(description = "Key lessons from this trade")]
    pub lessons: Vec<String>,
}

/// Parameters for add_learning tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddLearningParams {
    /// Agent type
    #[schemars(description = "Agent that learned this: technical_analyst, sentiment_analyst, etc.")]
    pub agent: String,

    /// Category
    #[schemars(description = "Category: winning_pattern, mistake, market_insight, risk_lesson, signal_reliability, timing_insight")]
    pub category: String,

    /// Description
    #[schemars(description = "Description of the learning")]
    pub description: String,

    /// Context
    #[schemars(description = "Context where this applies (e.g., symbol, market condition)")]
    pub context: String,

    /// Related trade ID
    #[schemars(description = "Trade ID this learning came from (optional)")]
    pub trade_id: Option<String>,
}

/// Parameters for check_circuit_breaker tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckCircuitBreakerParams {
    /// Include detailed status
    #[schemars(description = "Include detailed circuit breaker status")]
    pub detailed: Option<bool>,
}
