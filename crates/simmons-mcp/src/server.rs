//! Simmons MCP Server implementation
//!
//! Exposes trading engine functionality via Model Context Protocol.

use crate::memory::{
    AgentType, Learning, LearningCategory, MemorySystem, AgentPrediction, TradeReflection,
};
use crate::state::{TradeDecision, TradingState};
use crate::tools::{
    AddLearningParams, AddReflectionParams, CheckCircuitBreakerParams, GetHistoryParams,
    GetMemoryParams, GetPortfolioParams, GetRegimeParams, GetSignalsParams, RecordOutcomeParams,
    SubmitTradeParams,
};
use chrono::Utc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{
        CallToolResult, Content, ErrorCode, ErrorData as McpError, Implementation,
        ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router, ServerHandler,
};
use rust_decimal::Decimal;
use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use uuid::Uuid;

/// Simmons MCP Server
///
/// Exposes trading engine functionality to Claude via MCP protocol.
#[derive(Debug, Clone)]
pub struct SimmonsServer {
    /// Trading state (shared with engine)
    state: TradingState,
    /// Memory system for learning
    memory: MemorySystem,
    /// Tool router for MCP
    tool_router: ToolRouter<SimmonsServer>,
}

#[tool_router]
impl SimmonsServer {
    /// Create a new Simmons MCP server
    pub fn new(state: TradingState) -> Self {
        Self {
            state,
            memory: MemorySystem::default(),
            tool_router: Self::tool_router(),
        }
    }

    /// Create with custom memory path
    pub fn with_memory(state: TradingState, memory_path: &str) -> Self {
        Self {
            state,
            memory: MemorySystem::new(memory_path),
            tool_router: Self::tool_router(),
        }
    }

    /// Create with default state
    pub fn with_default_state() -> Self {
        Self::new(TradingState::default())
    }

    /// Get a clone of the trading state
    pub fn state(&self) -> TradingState {
        self.state.clone()
    }

    /// Get memory system reference
    pub fn memory(&self) -> &MemorySystem {
        &self.memory
    }

    // =========================================================================
    // MCP Tools
    // =========================================================================

    /// Get current market signals and trading opportunities
    #[tool(description = "Get current market signals, regime, and trading opportunities. Returns signals from momentum, mean reversion, and regime detection strategies, plus any arbitrage opportunities.")]
    async fn get_signals(
        &self,
        Parameters(params): Parameters<GetSignalsParams>,
    ) -> Result<CallToolResult, McpError> {
        let signals = self.state.get_signals();

        // Filter by symbol if provided
        let filtered_signals = if let Some(symbol) = params.symbol {
            let mut s = signals.clone();
            s.market_states.retain(|m| m.symbol == symbol);
            s
        } else {
            signals
        };

        let json = serde_json::to_string_pretty(&filtered_signals).map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to serialize signals: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get current portfolio state including positions and risk metrics
    #[tool(description = "Get current portfolio state including capital, positions, drawdown, and risk level. Use this to assess current exposure and available capital before making trade decisions.")]
    async fn get_portfolio(
        &self,
        Parameters(params): Parameters<GetPortfolioParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut portfolio = self.state.get_portfolio();

        // Optionally hide position details
        if params.include_positions == Some(false) {
            portfolio.positions.clear();
        }

        let json = serde_json::to_string_pretty(&portfolio).map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to serialize portfolio: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get trade history for learning and pattern analysis
    #[tool(description = "Get recent trade history including entry/exit prices, P&L, and outcomes. Use this to learn from past decisions and identify patterns.")]
    async fn get_history(
        &self,
        Parameters(params): Parameters<GetHistoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(10).min(50);
        let mut history = self.state.get_history(limit);

        // Filter by outcome if provided
        if let Some(outcome_str) = params.outcome {
            let outcome_filter = match outcome_str.to_lowercase().as_str() {
                "win" => Some(simmons_core::TradeOutcome::Win),
                "loss" => Some(simmons_core::TradeOutcome::Loss),
                "breakeven" => Some(simmons_core::TradeOutcome::Breakeven),
                _ => None,
            };

            if let Some(filter) = outcome_filter {
                history.retain(|t| t.outcome == Some(filter));
            }
        }

        let json = serde_json::to_string_pretty(&history).map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to serialize history: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Submit a trade decision for execution
    #[tool(description = "Submit a trading decision (trade, skip, or close_position). For trade action, specify symbol, side (long/short), size_pct (e.g., 0.10 for 10%), and optionally stop_loss_pct and take_profit_pct. Always provide confidence and reasoning.")]
    async fn submit_trade(
        &self,
        Parameters(params): Parameters<SubmitTradeParams>,
    ) -> Result<CallToolResult, McpError> {
        // Check circuit breakers first
        if let Some(reason) = self.state.check_circuit_breaker() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "{{\"success\": false, \"message\": \"{}\"}}",
                reason
            ))]));
        }

        // Parse action
        let action = params.parse_action().ok_or_else(|| McpError {
            code: ErrorCode(-32602),
            message: Cow::from(format!(
                "Invalid action '{}'. Use: trade, skip, or close_position",
                params.action
            )),
            data: None,
        })?;

        // Build trade decision
        let decision = TradeDecision {
            action,
            symbol: params.symbol.clone(),
            side: params.parse_side(),
            size_pct: params.size_as_decimal(),
            confidence: params.confidence_as_decimal(),
            reasoning: params.reasoning.clone(),
            stop_loss_pct: params.stop_loss_as_decimal(),
            take_profit_pct: params.take_profit_as_decimal(),
        };

        // Execute trade
        let result = self.state.submit_trade(decision);

        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to serialize result: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Record trade outcome for learning
    #[tool(description = "Record the outcome of a closed trade for learning purposes. Provide trade_id, outcome (win/loss/breakeven), and a reflection on what worked or didn't.")]
    async fn record_outcome(
        &self,
        Parameters(params): Parameters<RecordOutcomeParams>,
    ) -> Result<CallToolResult, McpError> {
        let outcome = params.parse_outcome().ok_or_else(|| McpError {
            code: ErrorCode(-32602),
            message: Cow::from(format!(
                "Invalid outcome '{}'. Use: win, loss, or breakeven",
                params.outcome
            )),
            data: None,
        })?;

        let success = self
            .state
            .record_outcome(&params.trade_id, outcome, &params.reflection);

        let response = if success {
            format!(
                "{{\"success\": true, \"message\": \"Recorded {} outcome for trade {}\"}}",
                params.outcome, params.trade_id
            )
        } else {
            format!(
                "{{\"success\": false, \"message\": \"Trade {} not found\"}}",
                params.trade_id
            )
        };

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Get current market regime
    #[tool(description = "Get current market regime classification (trending_up, trending_down, mean_reverting, high_volatility, low_volatility, choppy) with confidence and trading recommendation.")]
    async fn get_regime(
        &self,
        Parameters(_params): Parameters<GetRegimeParams>,
    ) -> Result<CallToolResult, McpError> {
        let regime = self.state.get_regime();

        let json = serde_json::to_string_pretty(&regime).map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to serialize regime: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // =========================================================================
    // Memory & Learning Tools
    // =========================================================================

    /// Get memory and learnings
    #[tool(description = "Get agent memory including learnings, mistakes to avoid, winning patterns, and agent accuracy stats. Use this to inform trading decisions based on past experience.")]
    async fn get_memory(
        &self,
        Parameters(params): Parameters<GetMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let snapshot = self.memory.get_memory_snapshot();

        // If filtering by agent, get specific learnings
        let response = if let Some(agent_str) = params.agent {
            let agent = parse_agent_type(&agent_str).ok_or_else(|| McpError {
                code: ErrorCode(-32602),
                message: Cow::from(format!("Invalid agent type: {}", agent_str)),
                data: None,
            })?;

            let learnings = self.memory.get_agent_learnings(agent);
            serde_json::to_string_pretty(&learnings)
        } else {
            serde_json::to_string_pretty(&snapshot)
        };

        let json = response.map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to serialize memory: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Add a learning from trade experience
    #[tool(description = "Record a learning from trading experience. Categories: winning_pattern, mistake, market_insight, risk_lesson, signal_reliability, timing_insight.")]
    async fn add_learning(
        &self,
        Parameters(params): Parameters<AddLearningParams>,
    ) -> Result<CallToolResult, McpError> {
        let agent = parse_agent_type(&params.agent).ok_or_else(|| McpError {
            code: ErrorCode(-32602),
            message: Cow::from(format!("Invalid agent type: {}", params.agent)),
            data: None,
        })?;

        let category = parse_learning_category(&params.category).ok_or_else(|| McpError {
            code: ErrorCode(-32602),
            message: Cow::from(format!("Invalid category: {}", params.category)),
            data: None,
        })?;

        let learning = Learning {
            id: Uuid::new_v4().to_string(),
            agent,
            category,
            description: params.description,
            context: params.context,
            trade_id: params.trade_id,
            outcome: None,
            confidence_delta: None,
            created_at: Utc::now(),
            times_applied: 0,
            success_rate: None,
        };

        self.memory.add_learning(learning);

        // Save to disk
        if let Err(e) = self.memory.save() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "{{\"success\": true, \"warning\": \"Learning added but failed to save: {}\"}}",
                e
            ))]));
        }

        Ok(CallToolResult::success(vec![Content::text(
            "{\"success\": true, \"message\": \"Learning recorded and saved\"}",
        )]))
    }

    /// Add a full trade reflection
    #[tool(description = "Record a complete trade reflection including agent predictions, what worked, what failed, and lessons learned. This updates agent accuracy stats.")]
    async fn add_reflection(
        &self,
        Parameters(params): Parameters<AddReflectionParams>,
    ) -> Result<CallToolResult, McpError> {
        // Parse agent predictions if provided
        let agent_predictions: HashMap<String, AgentPrediction> =
            if let Some(json_str) = params.agent_predictions {
                serde_json::from_str(&json_str).unwrap_or_default()
            } else {
                HashMap::new()
            };

        let pnl_pct = if params.entry_price > 0.0 {
            Decimal::try_from((params.exit_price - params.entry_price) / params.entry_price * 100.0)
                .unwrap_or_default()
        } else {
            Decimal::ZERO
        };

        let reflection = TradeReflection {
            trade_id: params.trade_id,
            symbol: params.symbol,
            side: params.side,
            entry_price: Decimal::try_from(params.entry_price).unwrap_or_default(),
            exit_price: Decimal::try_from(params.exit_price).unwrap_or_default(),
            pnl: Decimal::try_from(params.pnl).unwrap_or_default(),
            pnl_pct,
            outcome: params.outcome,
            duration_minutes: params.duration_minutes,
            agent_predictions,
            what_worked: params.what_worked,
            what_failed: params.what_failed,
            lessons: params.lessons,
            created_at: Utc::now(),
        };

        self.memory.add_reflection(reflection);

        // Save to disk
        if let Err(e) = self.memory.save() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "{{\"success\": true, \"warning\": \"Reflection added but failed to save: {}\"}}",
                e
            ))]));
        }

        Ok(CallToolResult::success(vec![Content::text(
            "{\"success\": true, \"message\": \"Trade reflection recorded, agent stats updated\"}",
        )]))
    }

    /// Check circuit breaker status
    #[tool(description = "Check if any circuit breakers are triggered (max drawdown, consecutive losses). Returns current risk status and any active blocks.")]
    async fn check_circuit_breaker(
        &self,
        Parameters(params): Parameters<CheckCircuitBreakerParams>,
    ) -> Result<CallToolResult, McpError> {
        let status = self.state.get_circuit_breaker_status();

        let response = if params.detailed.unwrap_or(false) {
            // Full detailed response
            serde_json::to_value(&status).unwrap_or_default()
        } else {
            // Simple response
            serde_json::json!({
                "triggered": status.triggered,
                "reason": status.reason,
                "can_trade": status.can_trade,
                "risk_level": status.risk_level,
                "position_size_modifier": status.position_size_modifier
            })
        };

        let json = serde_json::to_string_pretty(&response).map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to serialize: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

/// Parse agent type from string
fn parse_agent_type(s: &str) -> Option<AgentType> {
    match s.to_lowercase().replace('-', "_").as_str() {
        "technical_analyst" | "technical" => Some(AgentType::TechnicalAnalyst),
        "fundamental_analyst" | "fundamental" => Some(AgentType::FundamentalAnalyst),
        "sentiment_analyst" | "sentiment" => Some(AgentType::SentimentAnalyst),
        "onchain_analyst" | "onchain" => Some(AgentType::OnchainAnalyst),
        "bull_researcher" | "bull" => Some(AgentType::BullResearcher),
        "bear_researcher" | "bear" => Some(AgentType::BearResearcher),
        "research_manager" | "research" => Some(AgentType::ResearchManager),
        "aggressive_risk" | "aggressive" => Some(AgentType::AggressiveRisk),
        "conservative_risk" | "conservative" => Some(AgentType::ConservativeRisk),
        "neutral_risk" | "neutral" => Some(AgentType::NeutralRisk),
        "orchestrator" => Some(AgentType::Orchestrator),
        _ => None,
    }
}

/// Parse learning category from string
fn parse_learning_category(s: &str) -> Option<LearningCategory> {
    match s.to_lowercase().replace('-', "_").as_str() {
        "winning_pattern" | "pattern" | "win" => Some(LearningCategory::WinningPattern),
        "mistake" | "error" | "avoid" => Some(LearningCategory::Mistake),
        "market_insight" | "market" | "insight" => Some(LearningCategory::MarketInsight),
        "risk_lesson" | "risk" => Some(LearningCategory::RiskLesson),
        "signal_reliability" | "signal" => Some(LearningCategory::SignalReliability),
        "timing_insight" | "timing" => Some(LearningCategory::TimingInsight),
        _ => None,
    }
}

#[tool_handler]
impl ServerHandler for SimmonsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "simmons".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                r#"Simmons Trading Engine MCP Server v2.0

Available tools:
- get_signals: Get current market signals and trading opportunities
- get_portfolio: Get portfolio state, positions, and risk metrics
- get_history: Get recent trade history for learning
- submit_trade: Submit a trade decision (trade/skip/close_position)
- record_outcome: Record trade outcome for learning
- get_regime: Get current market regime classification
- get_memory: Get agent learnings, mistakes, and accuracy stats
- add_learning: Record a new learning (pattern, mistake, insight)
- add_reflection: Record full trade reflection with agent predictions
- check_circuit_breaker: Check if trading is blocked by risk limits

Trading Decision Flow:
1. Call get_memory to review past learnings
2. Call get_signals to see current opportunities
3. Call get_portfolio to check capital and risk
4. Call check_circuit_breaker before trading
5. Call get_regime to understand market conditions
6. Make decision and call submit_trade
7. After trade closes, call add_reflection for learning

Risk Rules:
- Max position size: 15% of capital
- Max drawdown: 20% (circuit breaker)
- Max consecutive losses: 3 (circuit breaker)
- Risk elevated: reduce position 50%
- Risk critical: no new trades
- Always provide stop_loss_pct and take_profit_pct"#
                    .to_string(),
            ),
        }
    }
}

/// Run the MCP server on stdio
pub async fn run_server(state: TradingState) -> anyhow::Result<()> {
    use rmcp::{transport::stdio, ServiceExt};

    tracing::info!("Starting Simmons MCP server...");

    let server = SimmonsServer::new(state);
    let service = server.serve(stdio()).await?;

    tracing::info!("Simmons MCP server running");
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let state = TradingState::default();
        let server = SimmonsServer::new(state);
        let info = server.get_info();
        assert_eq!(info.server_info.name, "simmons");
    }
}
