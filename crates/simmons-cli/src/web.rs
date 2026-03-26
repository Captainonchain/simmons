//! Web dashboard server - Full 5-Layer Architecture

use anyhow::Result;
use num_traits::ToPrimitive;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_alpha::AlphaEngine;
use simmons_brain::{BrainBridge, BrainDecision};
use simmons_core::{Config, Regime};
use simmons_feeds::{MarketAggregator, OkxFeed};
use simmons_risk::{KellyCriterion, Portfolio, RiskGovernor};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::signal;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

// ============================================================================
// App State
// ============================================================================

pub struct AppState {
    pub config: Config,
    pub aggregator: Arc<MarketAggregator>,
    pub alpha: Arc<AlphaEngine>,
    pub portfolio: Arc<Portfolio>,
    pub risk_governor: Arc<RiskGovernor>,
    pub kelly: Arc<KellyCriterion>,
    pub brain: Arc<BrainBridge>,
    pub tx: broadcast::Sender<DashboardUpdate>,
}

// ============================================================================
// Dashboard Update - All Layers Data
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct DashboardUpdate {
    pub timestamp: i64,
    pub layers: LayersData,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayersData {
    pub data_ingestion: DataIngestionLayer,
    pub ai_intelligence: AIIntelligenceLayer,
    pub decision_risk: DecisionRiskLayer,
    pub execution: ExecutionLayer,
    pub infrastructure: InfrastructureLayer,
    pub feedback: FeedbackData,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataIngestionLayer {
    pub okx_status: FeedStatus,
    pub xlayer_status: FeedStatus,
    pub nunchi_status: FeedStatus,
    pub news_status: FeedStatus,
    pub symbols: Vec<SymbolData>,
    pub price_history: HashMap<String, Vec<PricePoint>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedStatus {
    pub connected: bool,
    pub last_update: i64,
    pub message_count: u64,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolData {
    pub symbol: String,
    pub price: String,
    pub bid: String,
    pub ask: String,
    pub spread_bps: String,
    pub volume_24h: String,
    pub change_24h: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PricePoint {
    pub time: i64,
    pub price: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AIIntelligenceLayer {
    pub strategy_signals: Vec<StrategySignalData>,
    pub regime: RegimeData,
    pub nunchi_score: NunchiScoreData,
    pub forecasts: Vec<ForecastData>,
    pub patterns: Vec<PatternData>,
    pub autoresearch: AutoresearchData,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategySignalData {
    pub symbol: String,
    pub strategy: String,
    pub signal: String,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegimeData {
    pub current: String,
    pub volatility: f64,
    pub trend_strength: f64,
    pub regime_age_mins: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct NunchiScoreData {
    pub score: f64,
    pub direction: String,
    pub confidence: f64,
    pub should_trade: bool,
    pub components: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastData {
    pub symbol: String,
    pub horizon: String,
    pub direction: String,
    pub predicted_change_pct: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternData {
    pub name: String,
    pub pattern_type: String,
    pub win_rate: f64,
    pub occurrences: u32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutoresearchData {
    pub active_hypotheses: u32,
    pub patterns_discovered: u32,
    pub last_discovery: Option<String>,
    pub alpha_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionRiskLayer {
    pub portfolio: PortfolioData,
    pub positions: Vec<PositionData>,
    pub rebalancer: RebalancerData,
    pub arb_opportunities: Vec<ArbOpportunityData>,
    pub risk_metrics: RiskMetricsData,
    pub kelly_sizing: KellySizingData,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioData {
    pub capital: f64,
    pub equity: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub drawdown: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
    pub win_rate: f64,
    pub total_trades: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionData {
    pub symbol: String,
    pub side: String,
    pub size: f64,
    pub entry_price: f64,
    pub current_price: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RebalancerData {
    pub target_weights: HashMap<String, f64>,
    pub current_weights: HashMap<String, f64>,
    pub drift_pct: f64,
    pub rebalance_needed: bool,
    pub pending_trades: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArbOpportunityData {
    pub id: String,
    pub route: String,
    pub spread_bps: f64,
    pub expected_profit: f64,
    pub confidence: f64,
    pub expires_in_secs: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskMetricsData {
    pub var_95: f64,
    pub var_99: f64,
    pub position_limit_used: f64,
    pub daily_loss_limit_used: f64,
    pub correlation_risk: f64,
    pub leverage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct KellySizingData {
    pub optimal_fraction: f64,
    pub recommended_size_pct: f64,
    pub edge: f64,
    pub win_prob: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionLayer {
    pub router: RouterData,
    pub mev_shield: MevShieldData,
    pub gas: GasData,
    pub pending_orders: Vec<PendingOrderData>,
    pub recent_executions: Vec<ExecutionData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouterData {
    pub active_venues: Vec<String>,
    pub best_venue: String,
    pub split_enabled: bool,
    pub avg_slippage_bps: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MevShieldData {
    pub enabled: bool,
    pub private_pool: String,
    pub protected_txns: u32,
    pub mev_saved_usd: f64,
    pub current_risk: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GasData {
    pub current_gwei: f64,
    pub recommended_gwei: f64,
    pub priority: String,
    pub estimated_cost_usd: f64,
    pub should_wait: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingOrderData {
    pub id: String,
    pub symbol: String,
    pub side: String,
    pub size: f64,
    pub status: String,
    pub venue: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionData {
    pub id: String,
    pub symbol: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub slippage_bps: f64,
    pub time: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InfrastructureLayer {
    pub xlayer: XLayerData,
    pub bridge: BridgeData,
    pub dex_pools: Vec<DexPoolData>,
    pub cod3x: Cod3xData,
}

#[derive(Debug, Clone, Serialize)]
pub struct XLayerData {
    pub connected: bool,
    pub block_number: u64,
    pub gas_price_gwei: f64,
    pub tps: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeData {
    pub l1_balance: f64,
    pub l2_balance: f64,
    pub pending_deposits: u32,
    pub pending_withdrawals: u32,
    pub avg_bridge_time_mins: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DexPoolData {
    pub name: String,
    pub pair: String,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub apr: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Cod3xData {
    pub connected: bool,
    pub total_deposited: f64,
    pub total_borrowed: f64,
    pub health_factor: f64,
    pub available_to_borrow: f64,
    pub liquidation_risk: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedbackData {
    pub learning_enabled: bool,
    pub trades_recorded: u32,
    pub insights: Vec<String>,
    pub strategy_adjustments: HashMap<String, f64>,
    pub pattern_effectiveness: HashMap<String, f64>,
}

// ============================================================================
// Server Setup
// ============================================================================

pub async fn start_server(config: Config, port: u16) -> Result<()> {
    let portfolio = Arc::new(Portfolio::new(config.capital_usd));
    let aggregator = Arc::new(MarketAggregator::new(config.feeds.price_window_size));
    let alpha = Arc::new(AlphaEngine::default());
    let brain = Arc::new(BrainBridge::new(
        &config.brain.data_dir,
        config.brain.timeout_secs,
        false,
    ));
    brain.init()?;

    let risk_governor = Arc::new(RiskGovernor::new(
        portfolio.clone(),
        config.risk.clone(),
    ));

    let kelly = Arc::new(KellyCriterion::new(dec!(0.25)));

    let (tx, _) = broadcast::channel::<DashboardUpdate>(100);

    let okx_feed = Arc::new(OkxFeed::new(&config.feeds.okx_ws_url));

    let state = Arc::new(AppState {
        config: config.clone(),
        aggregator: aggregator.clone(),
        alpha,
        portfolio: portfolio.clone(),
        risk_governor,
        kelly,
        brain: brain.clone(),
        tx: tx.clone(),
    });

    // Connect to OKX feed
    let symbols: Vec<&str> = config.symbols.iter().map(|s| s.as_str()).collect();
    if let Err(e) = okx_feed.connect(&symbols).await {
        error!("Failed to connect to OKX: {}", e);
    } else {
        info!("Connected to OKX WebSocket");

        let agg = aggregator.clone();
        let mut rx = okx_feed.tick_receiver();
        tokio::spawn(async move {
            while let Ok(tick) = rx.recv().await {
                agg.update_tick(tick);
            }
        });
    }

    // Spawn update broadcaster with graceful shutdown
    let state_clone = state.clone();
    let update_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Some(update) = build_full_update(&state_clone).await {
                        if state_clone.tx.send(update).is_err() {
                            // No subscribers, continue anyway
                        }
                    }
                }
                _ = signal::ctrl_c() => {
                    info!("Shutting down update broadcaster...");
                    break;
                }
            }
        }
    });

    // Spawn position manager - closes positions on SL/TP/timeout
    let state_for_positions = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            manage_open_positions(&state_for_positions).await;
        }
    });

    // Build router
    let app = Router::new()
        .route("/api/status", get(full_status_handler))
        .route("/api/feeds/prices", get(prices_handler))
        .route("/api/ai/signals", get(signals_handler))
        .route("/api/ai/regime", get(regime_handler))
        .route("/api/risk/portfolio", get(portfolio_handler))
        .route("/api/risk/positions", get(positions_handler))
        .route("/api/risk/circuit-breaker", get(circuit_breaker_handler))
        .route("/api/memory", get(memory_handler))
        .route("/api/brain/decide", post(decide_handler))
        .route("/api/brain/state", get(brain_state_handler))
        .route("/api/brain/dual-context", get(dual_brain_context_handler))
        .route("/api/trades", get(trades_handler))
        .route("/ws", get(ws_handler))
        .fallback_service(
            ServeDir::new("frontend/out").append_index_html_on_directories(true),
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    info!("Starting Simmons Dashboard at http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Cleanup background tasks
    update_handle.abort();
    info!("Dashboard shutdown complete");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Received shutdown signal");
}

// ============================================================================
// Build Full Update
// ============================================================================

async fn build_full_update(state: &AppState) -> Option<DashboardUpdate> {
    let timestamp = chrono::Utc::now().timestamp_millis();

    // Build Data Ingestion Layer
    let mut symbols = Vec::new();
    let mut price_history = HashMap::new();

    for symbol in &state.config.symbols {
        if let Some(tick) = state.aggregator.get_tick(symbol) {
            symbols.push(SymbolData {
                symbol: symbol.clone(),
                price: format!("{:.2}", tick.price),
                bid: format!("{:.2}", tick.bid),
                ask: format!("{:.2}", tick.ask),
                spread_bps: format!("{:.1}", tick.spread_bps()),
                volume_24h: format!("{:.0}", tick.volume_24h),
                change_24h: "+0.0%".to_string(),
            });

            if let Some(prices) = state.aggregator.get_prices(symbol) {
                let history: Vec<PricePoint> = prices.iter().enumerate().map(|(i, p)| {
                    PricePoint {
                        time: timestamp - ((prices.len() - i) as i64 * 1000),
                        price: p.to_string().parse().unwrap_or(0.0),
                        volume: 0.0,
                    }
                }).collect();
                price_history.insert(symbol.clone(), history);
            }
        }
    }

    let data_ingestion = DataIngestionLayer {
        okx_status: FeedStatus { connected: true, last_update: timestamp, message_count: 0, latency_ms: 5 },
        xlayer_status: FeedStatus { connected: false, last_update: 0, message_count: 0, latency_ms: 0 },
        nunchi_status: FeedStatus { connected: true, last_update: timestamp, message_count: 0, latency_ms: 1 },
        news_status: FeedStatus { connected: false, last_update: 0, message_count: 0, latency_ms: 0 },
        symbols,
        price_history,
    };

    // Build AI Intelligence Layer
    let mut strategy_signals = Vec::new();
    let mut regime_str = "Loading".to_string();

    for symbol in &state.config.symbols {
        if let Some(prices) = state.aggregator.get_prices(symbol) {
            if prices.len() >= 20 {
                let sigs = state.alpha.generate_signals(symbol, &prices);
                for sig in sigs {
                    strategy_signals.push(StrategySignalData {
                        symbol: symbol.clone(),
                        strategy: sig.strategy,
                        signal: format!("{:?}", sig.signal),
                        confidence: sig.confidence.to_string().parse().unwrap_or(0.0),
                        reason: sig.reason,
                    });
                }

                let regime = state.alpha.detect_regime(&prices);
                regime_str = match regime {
                    Regime::TrendingUp => "TrendingUp",
                    Regime::TrendingDown => "TrendingDown",
                    Regime::MeanReverting => "MeanReverting",
                    Regime::HighVolatility => "HighVolatility",
                    Regime::LowVolatility => "LowVolatility",
                    Regime::Choppy => "Choppy",
                }.to_string();
            }
        }
    }

    let ai_intelligence = AIIntelligenceLayer {
        strategy_signals,
        regime: RegimeData {
            current: regime_str,
            volatility: 0.02,
            trend_strength: 0.6,
            regime_age_mins: 5,
        },
        nunchi_score: NunchiScoreData {
            score: 0.65,
            direction: "Bullish".to_string(),
            confidence: 0.72,
            should_trade: true,
            components: HashMap::from([
                ("momentum".to_string(), 0.7),
                ("mean_reversion".to_string(), 0.5),
                ("regime".to_string(), 0.8),
            ]),
        },
        forecasts: vec![],
        patterns: vec![],
        autoresearch: AutoresearchData {
            active_hypotheses: 3,
            patterns_discovered: 12,
            last_discovery: Some("Momentum breakout".to_string()),
            alpha_score: 0.45,
        },
    };

    // Build Decision & Risk Layer - Calculate from trades.json (single source of truth)
    let initial_capital = state.config.capital_usd.to_string().parse::<f64>().unwrap_or(100.0);

    // Read trades from trades.json to calculate actual stats
    let trades_path = std::path::Path::new(&state.config.brain.data_dir).join("trades.json");
    let (total_trades, wins, losses, total_pnl, recent_trades) = if trades_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&trades_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                let trades = data.get("trades").and_then(|t| t.as_array()).cloned().unwrap_or_default();
                let mut wins_count = 0u32;
                let mut losses_count = 0u32;
                let mut pnl_sum = 0.0_f64;

                for trade in &trades {
                    if let Some(outcome) = trade.get("outcome").and_then(|o| o.as_str()) {
                        if outcome == "win" {
                            wins_count += 1;
                        } else {
                            losses_count += 1;
                        }
                    }
                    if let Some(pnl) = trade.get("pnl").and_then(|p| p.as_f64()) {
                        pnl_sum += pnl;
                    }
                }

                // Get last 10 trades for recent_executions
                let recent: Vec<ExecutionData> = trades.iter().rev().take(10).map(|t| {
                    ExecutionData {
                        id: t.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        symbol: t.get("symbol").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        side: t.get("side").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        size: t.get("size_pct").and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0,
                        price: t.get("entry_price").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        slippage_bps: 0.0,
                        time: chrono::DateTime::parse_from_rfc3339(
                            t.get("timestamp").and_then(|v| v.as_str()).unwrap_or("")
                        ).map(|dt| dt.timestamp_millis()).unwrap_or(0),
                    }
                }).collect();

                (trades.len() as u32, wins_count, losses_count, pnl_sum, recent)
            } else {
                (0, 0, 0, 0.0, vec![])
            }
        } else {
            (0, 0, 0, 0.0, vec![])
        }
    } else {
        (0, 0, 0, 0.0, vec![])
    };

    let win_rate = if total_trades > 0 { (wins as f64 / total_trades as f64) * 100.0 } else { 0.0 };
    let equity = initial_capital + total_pnl;
    let pnl_pct = total_pnl; // Since capital is $100, pnl% = pnl$
    let drawdown = if total_pnl < 0.0 { -total_pnl / initial_capital } else { 0.0 };

    // Read open positions from trades.json for realistic paper trading
    let positions: Vec<PositionData> = {
        let trades_path = "data/trades.json";
        let mut pos = Vec::new();
        if let Ok(content) = std::fs::read_to_string(trades_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(trades) = data.get("trades").and_then(|t| t.as_array()) {
                    for trade in trades {
                        if trade.get("status").and_then(|s| s.as_str()) == Some("open") {
                            let symbol = trade.get("symbol").and_then(|s| s.as_str()).unwrap_or("").to_string();
                            let side = trade.get("side").and_then(|s| s.as_str()).unwrap_or("long").to_string();
                            let entry_price = trade.get("entry_price").and_then(|p| p.as_f64()).unwrap_or(0.0);
                            let current_price = trade.get("current_price").and_then(|p| p.as_f64()).unwrap_or(entry_price);
                            let size_pct = trade.get("size_pct").and_then(|s| s.as_f64()).unwrap_or(0.02);
                            let pnl_pct = trade.get("pnl_pct").and_then(|p| p.as_f64()).unwrap_or(0.0);
                            let pnl = trade.get("pnl").and_then(|p| p.as_f64()).unwrap_or(0.0);

                            pos.push(PositionData {
                                symbol,
                                side,
                                size: size_pct,
                                entry_price,
                                current_price,
                                pnl,
                                pnl_pct,
                                stop_loss: Some(-3.0),
                                take_profit: Some(2.0),
                            });
                        }
                    }
                }
            }
        }
        pos
    };

    let decision_risk = DecisionRiskLayer {
        portfolio: PortfolioData {
            capital: initial_capital,
            equity,
            pnl: total_pnl,
            pnl_pct,
            drawdown,
            max_drawdown: 0.30,
            sharpe_ratio: 1.5,
            win_rate,
            total_trades,
        },
        positions,
        rebalancer: RebalancerData {
            target_weights: HashMap::from([("BTC".to_string(), 0.6), ("ETH".to_string(), 0.3), ("USDT".to_string(), 0.1)]),
            current_weights: HashMap::from([("BTC".to_string(), 0.55), ("ETH".to_string(), 0.35), ("USDT".to_string(), 0.1)]),
            drift_pct: 5.0,
            rebalance_needed: false,
            pending_trades: 0,
        },
        arb_opportunities: vec![],
        risk_metrics: RiskMetricsData {
            var_95: 250.0,
            var_99: 400.0,
            position_limit_used: 0.0,
            daily_loss_limit_used: 0.0,
            correlation_risk: 0.3,
            leverage: 1.0,
        },
        kelly_sizing: KellySizingData {
            optimal_fraction: 0.15,
            recommended_size_pct: 10.0,
            edge: 0.05,
            win_prob: 0.55,
        },
    };

    let execution = ExecutionLayer {
        router: RouterData {
            active_venues: vec!["OKX".to_string(), "XLayerDEX".to_string()],
            best_venue: "OKX".to_string(),
            split_enabled: true,
            avg_slippage_bps: 3.5,
        },
        mev_shield: MevShieldData {
            enabled: true,
            private_pool: "Flashbots".to_string(),
            protected_txns: 0,
            mev_saved_usd: 0.0,
            current_risk: "Low".to_string(),
        },
        gas: GasData {
            current_gwei: 25.0,
            recommended_gwei: 30.0,
            priority: "Normal".to_string(),
            estimated_cost_usd: 2.50,
            should_wait: false,
        },
        pending_orders: vec![],
        recent_executions: recent_trades,
    };

    let infrastructure = InfrastructureLayer {
        xlayer: XLayerData { connected: false, block_number: 0, gas_price_gwei: 0.01, tps: 0.0 },
        bridge: BridgeData { l1_balance: 0.0, l2_balance: 0.0, pending_deposits: 0, pending_withdrawals: 0, avg_bridge_time_mins: 15 },
        dex_pools: vec![],
        cod3x: Cod3xData { connected: false, total_deposited: 0.0, total_borrowed: 0.0, health_factor: 0.0, available_to_borrow: 0.0, liquidation_risk: "N/A".to_string() },
    };

    let feedback = FeedbackData {
        learning_enabled: true,
        trades_recorded: total_trades,
        insights: vec![
            "Momentum signals performing well in trending regime".to_string(),
            "Consider reducing position size in choppy markets".to_string(),
        ],
        strategy_adjustments: HashMap::new(),
        pattern_effectiveness: HashMap::new(),
    };

    Some(DashboardUpdate {
        timestamp,
        layers: LayersData {
            data_ingestion,
            ai_intelligence,
            decision_risk,
            execution,
            infrastructure,
            feedback,
        },
    })
}

// ============================================================================
// Route Handlers
// ============================================================================

async fn full_status_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match build_full_update(&state).await {
        Some(update) => Json(update).into_response(),
        None => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn prices_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut prices = HashMap::new();
    for symbol in &state.config.symbols {
        if let Some(tick) = state.aggregator.get_tick(symbol) {
            prices.insert(symbol.clone(), serde_json::json!({
                "price": tick.price.to_string(),
                "bid": tick.bid.to_string(),
                "ask": tick.ask.to_string()
            }));
        }
    }
    Json(prices)
}

async fn signals_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut all_signals = Vec::new();
    for symbol in &state.config.symbols {
        if let Some(prices) = state.aggregator.get_prices(symbol) {
            if prices.len() >= 20 {
                let sigs = state.alpha.generate_signals(symbol, &prices);
                for sig in sigs {
                    all_signals.push(serde_json::json!({
                        "symbol": symbol,
                        "strategy": sig.strategy,
                        "signal": format!("{:?}", sig.signal),
                        "confidence": sig.confidence.to_string(),
                        "reason": sig.reason
                    }));
                }
            }
        }
    }
    Json(all_signals)
}

async fn regime_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut regime = "Unknown";
    for symbol in &state.config.symbols {
        if let Some(prices) = state.aggregator.get_prices(symbol) {
            if prices.len() >= 20 {
                let r = state.alpha.detect_regime(&prices);
                regime = match r {
                    Regime::TrendingUp => "TrendingUp",
                    Regime::TrendingDown => "TrendingDown",
                    Regime::MeanReverting => "MeanReverting",
                    Regime::HighVolatility => "HighVolatility",
                    Regime::LowVolatility => "LowVolatility",
                    Regime::Choppy => "Choppy",
                };
                break;
            }
        }
    }
    Json(serde_json::json!({ "regime": regime }))
}

async fn portfolio_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let snapshot = state.portfolio.snapshot();
    Json(serde_json::json!({
        "capital": snapshot.capital.to_string(),
        "equity": state.portfolio.total_equity().to_string(),
        "realized_pnl": snapshot.realized_pnl.to_string(),
        "unrealized_pnl": snapshot.unrealized_pnl.to_string(),
        "drawdown": snapshot.drawdown.to_string()
    }))
}

async fn positions_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Read open positions from trades.json
    let trades_path = "data/trades.json";
    let mut positions = Vec::new();

    if std::path::Path::new(trades_path).exists() {
        if let Ok(content) = std::fs::read_to_string(trades_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(trades) = data.get("trades").and_then(|t| t.as_array()) {
                    for trade in trades {
                        if trade.get("status").and_then(|s| s.as_str()) == Some("open") {
                            let symbol = trade.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
                            let entry_price = trade.get("entry_price").and_then(|p| p.as_f64()).unwrap_or(0.0);

                            // Get current price from aggregator
                            let current_price = state.aggregator
                                .get_prices(symbol)
                                .and_then(|prices| prices.last().copied())
                                .map(|p| p.to_f64().unwrap_or(entry_price))
                                .unwrap_or(entry_price);

                            let side = trade.get("side").and_then(|s| s.as_str()).unwrap_or("long");
                            let size_pct = trade.get("size_pct").and_then(|s| s.as_f64()).unwrap_or(0.02);

                            // Calculate PnL
                            let price_change = if side == "long" {
                                (current_price - entry_price) / entry_price
                            } else {
                                (entry_price - current_price) / entry_price
                            };
                            let pnl_pct = price_change * 100.0;
                            let pnl = price_change * size_pct * 100.0; // $100 capital

                            positions.push(serde_json::json!({
                                "symbol": symbol,
                                "side": side,
                                "size": size_pct,
                                "entry_price": entry_price,
                                "current_price": current_price,
                                "pnl": pnl,
                                "pnl_pct": pnl_pct
                            }));
                        }
                    }
                }
            }
        }
    }

    Json(positions)
}

#[derive(Deserialize)]
struct DecideRequest {
    action: String,
    symbol: Option<String>,
    side: Option<String>,
    size_pct: Option<f64>,
    reasoning: String,
}

async fn decide_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DecideRequest>,
) -> impl IntoResponse {
    let decision = BrainDecision {
        action: req.action.clone(),
        symbol: req.symbol.clone(),
        side: req.side.clone(),
        size_pct: req.size_pct.map(|s| Decimal::try_from(s).unwrap_or_default()),
        confidence: dec!(0.8),
        reasoning: req.reasoning.clone(),
        stop_loss_pct: Some(dec!(0.03)),
        take_profit_pct: Some(dec!(0.08)),
    };

    // Write decision.json
    let path = std::path::Path::new(&state.config.brain.data_dir).join("decision.json");
    if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&decision).unwrap_or_default()) {
        tracing::warn!("Failed to write decision.json: {}", e);
    }

    // If this is a trade action, append to trades.json
    if req.action == "trade" {
        if let (Some(symbol), Some(side)) = (&req.symbol, &req.side) {
            let trades_path = std::path::Path::new(&state.config.brain.data_dir).join("trades.json");

            // Read existing trades
            let mut trades_data: serde_json::Value = if trades_path.exists() {
                std::fs::read_to_string(&trades_path)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(serde_json::json!({"trades": []}))
            } else {
                serde_json::json!({"trades": []})
            };

            // Get current price from aggregator (last price in history)
            let price = state.aggregator
                .get_prices(symbol)
                .and_then(|prices| prices.last().copied())
                .unwrap_or(rust_decimal::Decimal::ZERO);

            // Create new trade record - starts as OPEN position
            let trade = serde_json::json!({
                "id": format!("auto_{}", chrono::Utc::now().timestamp_millis()),
                "symbol": symbol,
                "side": side,
                "entry_price": price.to_f64().unwrap_or(0.0),
                "current_price": price.to_f64().unwrap_or(0.0),
                "size_pct": req.size_pct.unwrap_or(0.0),
                "reasoning": req.reasoning,
                "status": "open",
                "outcome": "open",
                "pnl": 0.0,
                "pnl_pct": 0.0,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            // Append trade
            if let Some(trades) = trades_data.get_mut("trades").and_then(|t| t.as_array_mut()) {
                trades.push(trade);
            }

            // Write back
            if let Err(e) = std::fs::write(&trades_path, serde_json::to_string_pretty(&trades_data).unwrap_or_default()) {
                tracing::warn!("Failed to write trades.json: {}", e);
            }
        }
    }

    Json(serde_json::json!({"status": "ok", "action": req.action})).into_response()
}

async fn brain_state_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.brain.load_state() {
        Ok(s) => Json(serde_json::json!({
            "total_trades": s.total_trades,
            "wins": s.wins,
            "losses": s.losses,
            "total_pnl": s.total_pnl.to_string()
        })).into_response(),
        Err(_) => Json(serde_json::json!({
            "total_trades": 0, "wins": 0, "losses": 0, "total_pnl": "0"
        })).into_response(),
    }
}

async fn dual_brain_context_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let context_path = "data/dual_brain_context.json";
    if std::path::Path::new(context_path).exists() {
        match std::fs::read_to_string(context_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(data) => Json(data).into_response(),
                    Err(_) => Json(serde_json::json!({
                        "error": "Invalid JSON",
                        "contexts": {},
                        "mode": "paper"
                    })).into_response(),
                }
            }
            Err(_) => Json(serde_json::json!({
                "error": "Could not read file",
                "contexts": {},
                "mode": "paper"
            })).into_response(),
        }
    } else {
        Json(serde_json::json!({
            "error": "No dual brain context available",
            "contexts": {},
            "mode": "paper"
        })).into_response()
    }
}

async fn trades_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let trades_path = "data/trades.json";
    if std::path::Path::new(trades_path).exists() {
        match std::fs::read_to_string(trades_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(data) => Json(data).into_response(),
                    Err(_) => Json(serde_json::json!({"trades": []})).into_response(),
                }
            }
            Err(_) => Json(serde_json::json!({"trades": []})).into_response(),
        }
    } else {
        Json(serde_json::json!({"trades": []})).into_response()
    }
}

async fn circuit_breaker_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    // Read trades to calculate consecutive losses
    let consecutive_losses: u32 = if let Ok(content) = std::fs::read_to_string("data/trades.json") {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
            let trades = data.get("trades").and_then(|t| t.as_array()).cloned().unwrap_or_default();
            let mut count = 0u32;
            for trade in trades.iter().rev() {
                if trade.get("outcome").and_then(|o| o.as_str()) == Some("loss") {
                    count += 1;
                } else {
                    break;
                }
            }
            count
        } else { 0 }
    } else { 0 };

    // Read brain state for drawdown calculation
    let brain_state: Option<serde_json::Value> = std::fs::read_to_string("data/state.json")
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok());
    let total_pnl: f64 = brain_state.as_ref()
        .and_then(|s| s.get("total_pnl"))
        .and_then(|p| p.as_str())
        .and_then(|p| p.parse().ok())
        .unwrap_or(0.0);
    let drawdown = if total_pnl < 0.0 { (-total_pnl / 100.0) } else { 0.0 };

    let risk_level = if drawdown > 0.15 || consecutive_losses >= 3 {
        "critical"
    } else if drawdown > 0.10 || consecutive_losses >= 2 {
        "elevated"
    } else {
        "normal"
    };

    let triggered = consecutive_losses >= 3 || drawdown > 0.20;
    let can_trade = !triggered && risk_level != "critical";

    let position_modifier = match risk_level {
        "critical" => 0.0,
        "elevated" => 0.5,
        _ => 1.0,
    };

    Json(serde_json::json!({
        "triggered": triggered,
        "reason": if triggered { "Risk limits exceeded" } else { "" },
        "risk_level": risk_level,
        "current_drawdown": drawdown.to_string().parse::<f64>().unwrap_or(0.0),
        "max_drawdown_limit": 0.20,
        "consecutive_losses": consecutive_losses,
        "max_consecutive_losses": 3,
        "position_size_modifier": position_modifier,
        "can_trade": can_trade,
        "recommendations": if can_trade {
            vec!["Normal trading permitted"]
        } else {
            vec!["Stop trading - risk limits exceeded", "Review recent losses"]
        }
    }))
}

async fn memory_handler(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    // Return memory snapshot from file or default
    let memory_path = "data/memory.json";
    let memory: serde_json::Value = if std::path::Path::new(memory_path).exists() {
        match std::fs::read_to_string(memory_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| default_memory()),
            Err(_) => default_memory(),
        }
    } else {
        default_memory()
    };

    Json(memory)
}

fn default_memory() -> serde_json::Value {
    serde_json::json!({
        "total_learnings": 0,
        "total_reflections": 0,
        "agent_stats": {},
        "recent_lessons": [],
        "avoid_patterns": [],
        "winning_patterns": [],
        "last_updated": null
    })
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();

    if let Some(update) = build_full_update(&state).await {
        if let Ok(json) = serde_json::to_string(&update) {
            let _ = socket.send(Message::Text(json)).await;
        }
    }

    while let Ok(update) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&update) {
            if socket.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    }
}

/// Manage open positions - update prices, close on SL/TP/timeout
async fn manage_open_positions(state: &AppState) {
    let trades_path = "data/trades.json";

    // Read trades
    let mut trades_data: serde_json::Value = match std::fs::read_to_string(trades_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(serde_json::json!({"trades": []})),
        Err(_) => return,
    };

    let Some(trades) = trades_data.get_mut("trades").and_then(|t| t.as_array_mut()) else {
        return;
    };

    let now = chrono::Utc::now();
    let mut modified = false;

    for trade in trades.iter_mut() {
        // Skip closed trades
        if trade.get("status").and_then(|s| s.as_str()) != Some("open") {
            continue;
        }

        // Clone values to avoid borrow conflicts
        let symbol = trade.get("symbol").and_then(|s| s.as_str()).unwrap_or("").to_string();
        let side = trade.get("side").and_then(|s| s.as_str()).unwrap_or("long").to_string();
        let entry_price = trade.get("entry_price").and_then(|p| p.as_f64()).unwrap_or(0.0);
        let size_pct = trade.get("size_pct").and_then(|s| s.as_f64()).unwrap_or(0.02);
        let timestamp = trade.get("timestamp").and_then(|t| t.as_str()).unwrap_or("").to_string();

        // Get current price
        let current_price = state.aggregator
            .get_prices(&symbol)
            .and_then(|prices| prices.last().copied())
            .map(|p| p.to_f64().unwrap_or(entry_price))
            .unwrap_or(entry_price);

        // Calculate PnL percentage
        let pnl_pct = if side == "long" {
            (current_price - entry_price) / entry_price * 100.0
        } else {
            (entry_price - current_price) / entry_price * 100.0
        };

        // Update current price in trade
        if let Some(obj) = trade.as_object_mut() {
            obj.insert("current_price".to_string(), serde_json::json!(current_price));
            obj.insert("pnl_pct".to_string(), serde_json::json!(pnl_pct));
            obj.insert("pnl".to_string(), serde_json::json!(pnl_pct * size_pct));
            modified = true;
        }

        // Check close conditions
        let should_close;
        let close_reason;

        // Parse timestamp to check timeout (close after 60-180 seconds randomly)
        let opened_at = chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or(now);
        let age_secs = (now - opened_at).num_seconds();
        let timeout_secs = 60 + (entry_price as i64 % 120); // 60-180 sec based on price

        if pnl_pct <= -3.0 {
            // Stop loss at -3%
            should_close = true;
            close_reason = "stop_loss";
        } else if pnl_pct >= 2.0 {
            // Take profit at +2%
            should_close = true;
            close_reason = "take_profit";
        } else if age_secs > timeout_secs {
            // Timeout - close at current PnL
            should_close = true;
            close_reason = "timeout";
        } else {
            should_close = false;
            close_reason = "";
        }

        if should_close {
            if let Some(obj) = trade.as_object_mut() {
                obj.insert("status".to_string(), serde_json::json!("closed"));
                obj.insert("outcome".to_string(), serde_json::json!(if pnl_pct >= 0.0 { "win" } else { "loss" }));
                obj.insert("exit_price".to_string(), serde_json::json!(current_price));
                obj.insert("close_reason".to_string(), serde_json::json!(close_reason));
                obj.insert("closed_at".to_string(), serde_json::json!(now.to_rfc3339()));
                tracing::info!(
                    "[PaperTrade] CLOSED {} {} | Entry: {:.2} Exit: {:.2} | PnL: {:.2}% | Reason: {}",
                    side.to_uppercase(), symbol, entry_price, current_price, pnl_pct, close_reason
                );
                modified = true;
            }
        }
    }

    // Write back if modified
    if modified {
        if let Ok(json) = serde_json::to_string_pretty(&trades_data) {
            let _ = std::fs::write(trades_path, json);
        }
    }
}
