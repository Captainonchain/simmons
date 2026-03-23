//! Web dashboard server - Full 5-Layer Architecture

use anyhow::Result;
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

    // Build router
    let app = Router::new()
        .route("/api/status", get(full_status_handler))
        .route("/api/feeds/prices", get(prices_handler))
        .route("/api/ai/signals", get(signals_handler))
        .route("/api/ai/regime", get(regime_handler))
        .route("/api/risk/portfolio", get(portfolio_handler))
        .route("/api/risk/positions", get(positions_handler))
        .route("/api/brain/decide", post(decide_handler))
        .route("/api/brain/state", get(brain_state_handler))
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

    // Build Decision & Risk Layer
    let snapshot = state.portfolio.snapshot();
    let initial = state.config.capital_usd;
    let pnl = snapshot.realized_pnl + snapshot.unrealized_pnl;
    let pnl_pct = if !initial.is_zero() {
        (pnl / initial * dec!(100)).to_string().parse().unwrap_or(0.0)
    } else {
        0.0
    };

    let positions: Vec<PositionData> = state.portfolio.positions().into_iter().map(|p| {
        let pnl_pct = p.pnl_percent().to_string().parse().unwrap_or(0.0);
        PositionData {
            symbol: p.symbol,
            side: format!("{:?}", p.side),
            size: p.size.to_string().parse().unwrap_or(0.0),
            entry_price: p.entry_price.to_string().parse().unwrap_or(0.0),
            current_price: p.current_price.to_string().parse().unwrap_or(0.0),
            pnl: p.unrealized_pnl.to_string().parse().unwrap_or(0.0),
            pnl_pct,
            stop_loss: None,
            take_profit: None,
        }
    }).collect();

    let decision_risk = DecisionRiskLayer {
        portfolio: PortfolioData {
            capital: snapshot.capital.to_string().parse().unwrap_or(0.0),
            equity: state.portfolio.total_equity().to_string().parse().unwrap_or(0.0),
            pnl: pnl.to_string().parse().unwrap_or(0.0),
            pnl_pct,
            drawdown: (snapshot.drawdown * dec!(100)).to_string().parse().unwrap_or(0.0),
            max_drawdown: 5.0,
            sharpe_ratio: 1.5,
            win_rate: (state.portfolio.win_rate() * dec!(100)).to_string().parse().unwrap_or(0.0),
            total_trades: state.brain.load_state().map(|s| s.total_trades).unwrap_or(0),
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
        recent_executions: vec![],
    };

    let infrastructure = InfrastructureLayer {
        xlayer: XLayerData { connected: false, block_number: 0, gas_price_gwei: 0.01, tps: 0.0 },
        bridge: BridgeData { l1_balance: 0.0, l2_balance: 0.0, pending_deposits: 0, pending_withdrawals: 0, avg_bridge_time_mins: 15 },
        dex_pools: vec![],
        cod3x: Cod3xData { connected: false, total_deposited: 0.0, total_borrowed: 0.0, health_factor: 0.0, available_to_borrow: 0.0, liquidation_risk: "N/A".to_string() },
    };

    let feedback = FeedbackData {
        learning_enabled: true,
        trades_recorded: 0,
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
    let positions: Vec<_> = state.portfolio.positions().into_iter().map(|p| {
        serde_json::json!({
            "symbol": p.symbol,
            "side": format!("{:?}", p.side),
            "size": p.size.to_string(),
            "entry_price": p.entry_price.to_string(),
            "current_price": p.current_price.to_string(),
            "pnl": p.unrealized_pnl.to_string()
        })
    }).collect();
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
        action: req.action,
        symbol: req.symbol,
        side: req.side,
        size_pct: req.size_pct.map(|s| Decimal::try_from(s).unwrap_or_default()),
        confidence: dec!(0.8),
        reasoning: req.reasoning,
        stop_loss_pct: Some(dec!(0.03)),
        take_profit_pct: Some(dec!(0.08)),
    };

    let path = std::path::Path::new(&state.config.brain.data_dir).join("decision.json");
    match serde_json::to_string_pretty(&decision) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response();
            }
            Json(serde_json::json!({"status": "ok"})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
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
