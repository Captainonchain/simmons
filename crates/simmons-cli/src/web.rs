//! Web dashboard server

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_alpha::AlphaEngine;
use simmons_brain::{BrainBridge, BrainDecision};
use simmons_core::{Config, Signal, StrategySignal};
use simmons_feeds::{MarketAggregator, OkxFeed};
use simmons_risk::Portfolio;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use tracing::{error, info};

/// Shared application state
pub struct AppState {
    pub config: Config,
    pub aggregator: Arc<MarketAggregator>,
    pub alpha: Arc<AlphaEngine>,
    pub portfolio: Arc<Portfolio>,
    pub brain: Arc<BrainBridge>,
    pub tx: broadcast::Sender<DashboardUpdate>,
}

/// Dashboard update message
#[derive(Debug, Clone, Serialize)]
pub struct DashboardUpdate {
    pub timestamp: i64,
    pub symbols: Vec<SymbolData>,
    pub portfolio: PortfolioData,
    pub signals: Vec<SignalData>,
    pub decision: Option<DecisionData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolData {
    pub symbol: String,
    pub price: String,
    pub change_24h: String,
    pub spread_bps: String,
    pub regime: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioData {
    pub capital: String,
    pub equity: String,
    pub pnl: String,
    pub pnl_pct: String,
    pub drawdown: String,
    pub win_rate: String,
    pub total_trades: u32,
    pub positions: Vec<PositionData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionData {
    pub symbol: String,
    pub side: String,
    pub size: String,
    pub entry: String,
    pub current: String,
    pub pnl: String,
    pub pnl_pct: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignalData {
    pub symbol: String,
    pub strategy: String,
    pub signal: String,
    pub confidence: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionData {
    pub action: String,
    pub symbol: String,
    pub side: String,
    pub size_pct: String,
    pub confidence: String,
    pub reasoning: String,
}

/// Start the web server
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

    let (tx, _) = broadcast::channel::<DashboardUpdate>(100);

    let state = Arc::new(AppState {
        config: config.clone(),
        aggregator: aggregator.clone(),
        alpha,
        portfolio: portfolio.clone(),
        brain: brain.clone(),
        tx: tx.clone(),
    });

    // Start OKX feed
    let feed = OkxFeed::new(&config.feeds.okx_ws_url);
    let symbols: Vec<&str> = config.symbols.iter().map(|s| s.as_str()).collect();

    if let Err(e) = feed.connect(&symbols).await {
        error!("Failed to connect to OKX: {}", e);
    } else {
        info!("Connected to OKX WebSocket");

        // Spawn feed handler
        let agg = aggregator.clone();
        let mut rx = feed.tick_receiver();
        tokio::spawn(async move {
            while let Ok(tick) = rx.recv().await {
                agg.update_tick(tick);
            }
        });
    }

    // Spawn update broadcaster
    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            if let Some(update) = build_update(&state_clone).await {
                let _ = state_clone.tx.send(update);
            }
        }
    });

    // Build router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/status", get(status_handler))
        .route("/api/decide", post(decide_handler))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    info!("Starting dashboard at http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn build_update(state: &AppState) -> Option<DashboardUpdate> {
    let mut symbols = Vec::new();
    let mut all_signals = Vec::new();

    for symbol in &state.config.symbols {
        if let Some(tick) = state.aggregator.get_tick(symbol) {
            let prices = state.aggregator.get_prices(symbol).unwrap_or_default();
            let regime = if prices.len() >= 20 {
                format!("{:?}", state.alpha.detect_regime(&prices))
            } else {
                "Loading...".to_string()
            };

            symbols.push(SymbolData {
                symbol: symbol.clone(),
                price: format!("{:.2}", tick.price),
                change_24h: "+0.0%".to_string(), // Would need historical data
                spread_bps: format!("{:.1}", tick.spread_bps()),
                regime,
            });

            // Generate signals
            if prices.len() >= 20 {
                let sigs = state.alpha.generate_signals(symbol, &prices);
                for sig in sigs {
                    all_signals.push(SignalData {
                        symbol: symbol.clone(),
                        strategy: sig.strategy,
                        signal: format!("{:?}", sig.signal),
                        confidence: format!("{:.0}%", sig.confidence * dec!(100)),
                        reason: sig.reason,
                    });
                }
            }
        }
    }

    // Portfolio data
    let snapshot = state.portfolio.snapshot();
    let initial = state.config.capital_usd;
    let pnl = snapshot.realized_pnl + snapshot.unrealized_pnl;
    let pnl_pct = if !initial.is_zero() {
        pnl / initial * dec!(100)
    } else {
        Decimal::ZERO
    };

    let positions: Vec<PositionData> = state
        .portfolio
        .positions()
        .into_iter()
        .map(|p| {
            let pnl_pct = p.pnl_percent();
            PositionData {
                symbol: p.symbol,
                side: format!("{:?}", p.side),
                size: format!("{:.4}", p.size),
                entry: format!("{:.2}", p.entry_price),
                current: format!("{:.2}", p.current_price),
                pnl: format!("{:+.2}", p.unrealized_pnl),
                pnl_pct: format!("{:+.1}%", pnl_pct),
            }
        })
        .collect();

    let portfolio = PortfolioData {
        capital: format!("{:.2}", snapshot.capital),
        equity: format!("{:.2}", state.portfolio.total_equity()),
        pnl: format!("{:+.2}", pnl),
        pnl_pct: format!("{:+.1}%", pnl_pct),
        drawdown: format!("{:.1}%", snapshot.drawdown * dec!(100)),
        win_rate: format!("{:.0}%", state.portfolio.win_rate() * dec!(100)),
        total_trades: state.brain.load_state().map(|s| s.total_trades).unwrap_or(0),
        positions,
    };

    // Check for decision
    let decision = state.brain.peek_decision().ok().flatten().map(|d| DecisionData {
        action: d.action.clone(),
        symbol: d.symbol.unwrap_or_default(),
        side: d.side.unwrap_or_default(),
        size_pct: d.size_pct.map(|s| format!("{:.0}%", s * dec!(100))).unwrap_or_default(),
        confidence: format!("{:.0}%", d.confidence * dec!(100)),
        reasoning: d.reasoning,
    });

    Some(DashboardUpdate {
        timestamp: chrono::Utc::now().timestamp_millis(),
        symbols,
        portfolio,
        signals: all_signals,
        decision,
    })
}

async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../../../frontend/index.html"))
}

async fn status_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match build_update(&state).await {
        Some(update) => Json(update).into_response(),
        None => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
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
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Write error: {}", e)).into_response();
            }
            Json(serde_json::json!({"status": "ok"})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("JSON error: {}", e)).into_response(),
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

    // Send initial state
    if let Some(update) = build_update(&state).await {
        if let Ok(json) = serde_json::to_string(&update) {
            let _ = socket.send(Message::Text(json)).await;
        }
    }

    // Stream updates
    while let Ok(update) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&update) {
            if socket.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    }
}
