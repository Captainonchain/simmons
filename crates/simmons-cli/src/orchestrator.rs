//! Main orchestrator - coordinates all components

use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_alpha::AlphaEngine;
use simmons_brain::{
    BrainArbOpportunity, BrainBridge, BrainDecision, BrainInput, BrainMarketState,
    BrainPortfolio, BrainSignal, BrainTradeOutcome, FeedbackLoop, MarketConditions,
};
use simmons_core::{Action, Config, Regime, Signal, StrategySignal, TradingMode};
use simmons_exec::ExecutionEngine;
use simmons_feeds::{create_integrated_news_feed, MarketAggregator, NewsFeed, NunchiSignals, OkxFeed, XLayerFeed};
use simmons_risk::{Portfolio, RiskGovernor};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep, Instant};
use chrono::{Datelike, Timelike};
use tracing::{debug, error, info, warn};

/// Main trading engine orchestrator
pub struct Engine {
    config: Config,
    feed: OkxFeed,
    aggregator: MarketAggregator,
    alpha: AlphaEngine,
    brain: BrainBridge,
    portfolio: Arc<Portfolio>,
    risk: RiskGovernor,
    exec: ExecutionEngine,
    symbols: Vec<String>,
    feedback: FeedbackLoop,
    /// Nunchi signal aggregator
    nunchi: NunchiSignals,
    /// X Layer DEX feed
    xlayer_feed: XLayerFeed,
    /// News/sentiment feed (shared with background fetcher)
    news_feed: Arc<RwLock<NewsFeed>>,
    /// Background task handles
    _background_tasks: Vec<tokio::task::JoinHandle<()>>,
    /// Last decision reasoning for feedback loop
    last_reasoning: Option<String>,
    /// Last signals used for feedback loop
    last_signals: Vec<String>,
    /// Last market conditions for feedback loop
    last_conditions: Option<MarketConditions>,
}

impl Engine {
    pub async fn new(config: Config) -> Result<Self> {
        let portfolio = Arc::new(Portfolio::new(config.capital_usd));
        let risk = RiskGovernor::new(portfolio.clone(), config.risk.clone());

        // Initialize execution engine
        let mut exec = ExecutionEngine::new(config.mode, config.execution.clone(), portfolio.clone());

        // For live mode, initialize live executor
        if config.mode == simmons_core::TradingMode::Live {
            let use_testnet = std::env::var("XLAYER_TESTNET")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false);

            match exec.with_live_executor(use_testnet) {
                Ok(e) => {
                    exec = e;
                    info!("Live execution initialized (testnet={})", use_testnet);
                }
                Err(e) => {
                    warn!("Live executor initialization failed: {}", e);
                    warn!("Falling back to paper trading mode");
                    // Fall back to paper mode
                    exec = ExecutionEngine::new(
                        simmons_core::TradingMode::Paper,
                        config.execution.clone(),
                        portfolio.clone(),
                    );
                }
            }
        }

        let brain = BrainBridge::new(
            &config.brain.data_dir,
            config.brain.timeout_secs,
            config.brain.auto_invoke,
        );
        brain.init()?;

        let feed = OkxFeed::new(&config.feeds.okx_ws_url);
        let aggregator = MarketAggregator::new(config.feeds.price_window_size);
        let alpha = AlphaEngine::default();

        let symbols = config.symbols.clone();
        let feedback = FeedbackLoop::with_defaults();

        // Initialize Nunchi signal aggregator
        let nunchi = NunchiSignals::with_defaults();

        // Initialize X Layer feed with token addresses
        let xlayer_feed = XLayerFeed::with_defaults();

        // X Layer token addresses for price tracking
        let xlayer_tokens = vec![
            ("ETH".to_string(), "0x5a77f1443d16ee5761d310e38b62f77f726bc71c".to_string()), // WETH
            ("USDT".to_string(), "0x1E4a5963aBFD975d8c9021ce480b42188849D41d".to_string()),
            ("USDC".to_string(), "0x74b7f16337b8972027f6196a17a631ac6de26d22".to_string()),
            ("BTC".to_string(), "0xea034fb02eb1808c2cc3adbc15f447b93cbe08e1".to_string()), // WBTC
            ("OKB".to_string(), "0xdf54b6c6195ea4d948d03bfd818d365cf175cfc2".to_string()),
        ];
        let xlayer_handle = xlayer_feed.start_polling(xlayer_tokens);

        // Initialize integrated news feed with OnchainOS signals
        let (news_feed, news_handle) = create_integrated_news_feed(vec![
            "xlayer".to_string(),
            "ethereum".to_string(),
        ]);

        // Track background tasks
        let mut background_tasks = Vec::new();
        background_tasks.push(xlayer_handle);
        background_tasks.push(news_handle);

        info!("Initialized data feeds:");
        info!("  - OKX WebSocket: {}", config.feeds.okx_ws_url);
        info!("  - X Layer DEX feed: enabled");
        info!("  - News/Signals feed: enabled (xlayer, ethereum)");
        info!("  - Nunchi aggregator: enabled");

        Ok(Self {
            config,
            feed,
            aggregator,
            alpha,
            brain,
            portfolio,
            risk,
            exec,
            symbols,
            feedback,
            nunchi,
            xlayer_feed,
            news_feed,
            _background_tasks: background_tasks,
            last_reasoning: None,
            last_signals: Vec::new(),
            last_conditions: None,
        })
    }

    pub fn symbols(&self) -> &[String] {
        &self.symbols
    }

    /// Main trading loop
    pub async fn run(&mut self, symbols: &[&str], duration_secs: u64) -> Result<()> {
        // Connect to feeds
        info!("Connecting to data feeds...");
        self.feed.connect(symbols).await?;
        info!("✓ Connected to OKX WebSocket");

        let mut tick_rx = self.feed.tick_receiver();
        let update_interval = Duration::from_millis(self.config.feeds.update_interval_ms);
        let mut update_timer = interval(update_interval);

        let start = Instant::now();
        let run_duration = if duration_secs > 0 {
            Some(Duration::from_secs(duration_secs))
        } else {
            None
        };

        let auto_invoke = self.config.brain.auto_invoke;
        info!("Starting trading loop (auto_invoke={})...", auto_invoke);
        info!("Signals file: {:?}", self.brain.signals_path());
        info!("Decision file: {:?}", self.brain.decision_path());
        info!("");

        loop {
            // Check duration
            if let Some(dur) = run_duration {
                if start.elapsed() > dur {
                    info!("Duration reached, stopping...");
                    break;
                }
            }

            // Update market data from OKX WebSocket
            while let Ok(tick) = tick_rx.try_recv() {
                self.aggregator.update_tick(tick);
            }

            // Update DEX prices from X Layer for CeDeFi arbitrage detection
            for symbol in &self.symbols.clone() {
                if let Some(dex_price) = self.get_xlayer_price(symbol).await {
                    self.aggregator.update_dex_price(symbol, dex_price);
                }
            }

            // Wait for update interval
            update_timer.tick().await;

            // Generate signals for each symbol
            for symbol in &self.symbols.clone() {
                if let Some(prices) = self.aggregator.get_prices(symbol) {
                    if prices.len() < 20 {
                        continue; // Need more data
                    }

                    // Generate signals from alpha engine
                    let mut signals = self.alpha.generate_signals(symbol, &prices);
                    let regime = self.alpha.detect_regime(&prices);

                    // Add sentiment signal from news feed
                    if let Some(sentiment_signal) = self.get_sentiment_signal(symbol).await {
                        signals.push(sentiment_signal);
                    }

                    // Use Nunchi to aggregate signals
                    self.nunchi.set_regime(regime);
                    let nunchi_score = self.nunchi.aggregate(&signals);
                    let trade_decision = self.nunchi.should_trade(&nunchi_score, dec!(0.3));

                    // Use Nunchi combined signal and confidence
                    let combined = match nunchi_score.recommendation {
                        simmons_feeds::nunchi::NunchiRecommendation::StrongBuy => Signal::StrongBuy,
                        simmons_feeds::nunchi::NunchiRecommendation::Buy => Signal::Buy,
                        simmons_feeds::nunchi::NunchiRecommendation::Sell => Signal::Sell,
                        simmons_feeds::nunchi::NunchiRecommendation::StrongSell => Signal::StrongSell,
                        _ => Signal::Hold,
                    };
                    let confidence = nunchi_score.confidence;

                    let current_price = *prices.last().unwrap_or(&Decimal::ZERO);

                    // Display current state
                    self.display_state(symbol, current_price, regime, &combined, confidence);

                    // Check for arbitrage
                    let arb_opps = self.alpha.check_arbitrage(
                        symbol,
                        current_price,
                        self.aggregator
                            .get_market_state(symbol)
                            .and_then(|s| s.dex_price),
                        self.portfolio.capital(),
                    );

                    // Build brain input
                    let input = self.build_brain_input(symbol, &prices, &signals, &arb_opps);

                    // Store market conditions for feedback loop
                    self.last_conditions = Some(MarketConditions {
                        regime: format!("{:?}", regime).to_lowercase(),
                        volatility: input.market_state.volatility_1h,
                        spread_bps: input.market_state.spread_bps,
                        volume_relative: Decimal::ONE, // TODO: calculate relative volume
                        time_of_day: chrono::Utc::now().hour() as u8,
                        day_of_week: chrono::Utc::now().weekday().num_days_from_monday() as u8,
                    });

                    // Store signals used
                    self.last_signals = signals.iter().map(|s| s.strategy.clone()).collect();

                    // AUTONOMOUS MODE: Request decision directly from Claude
                    if auto_invoke {
                        match self.brain.request_decision(&input).await {
                            Ok(Some(decision)) => {
                                self.handle_decision(decision).await?;
                            }
                            Ok(None) => {
                                debug!("No decision returned (auto_invoke may be disabled)");
                            }
                            Err(e) => {
                                error!("Failed to get Claude decision: {}", e);
                            }
                        }
                    } else {
                        // INTERACTIVE MODE: Write signals, check for decision later
                        if let Err(e) = self.brain.write_signals(&input) {
                            error!("Failed to write signals: {}", e);
                        }
                    }
                }
            }

            // Check for position exits (stop loss / take profit)
            self.check_exits().await?;

            // INTERACTIVE MODE: Check for Claude decision (manual invocation)
            if !auto_invoke {
                if let Some(decision) = self.brain.read_decision()? {
                    self.handle_decision(decision).await?;
                }
            }

            // Generate feedback report if due
            if self.feedback.should_report() {
                let report = self.feedback.performance_report();
                info!("");
                info!("╔═══ LEARNING REPORT ═══╗");
                info!("║ Win Rate: {:.1}%", report.win_rate * dec!(100));
                info!("║ Total PnL: ${:.2}", report.total_pnl);
                info!("║ Trades: {}", report.total_trades);
                for rec in &report.recommendations {
                    info!("║ → {}", rec);
                }
                info!("╚════════════════════════╝");
                self.feedback.mark_reported();
            }

            // Small delay to prevent CPU spinning
            sleep(Duration::from_millis(10)).await;
        }

        self.feed.stop();
        self.show_summary();

        Ok(())
    }

    fn display_state(
        &self,
        symbol: &str,
        price: Decimal,
        regime: simmons_core::Regime,
        signal: &simmons_core::Signal,
        confidence: Decimal,
    ) {
        let regime_str = format!("{:?}", regime);
        let signal_str = format!("{:?}", signal);

        info!(
            "[{}] ${:.2} | {} | {} ({:.0}%) | Equity: ${:.2}",
            symbol,
            price,
            regime_str,
            signal_str,
            confidence * dec!(100),
            self.portfolio.total_equity()
        );
    }

    fn build_brain_input(
        &self,
        symbol: &str,
        prices: &[Decimal],
        signals: &[simmons_core::StrategySignal],
        arb_opps: &[simmons_core::ArbOpportunity],
    ) -> BrainInput {
        let current_price = *prices.last().unwrap_or(&Decimal::ZERO);
        let regime = self.alpha.detect_regime(prices);

        // Calculate volatility
        let volatility = if prices.len() > 10 {
            let returns: Vec<Decimal> = prices
                .windows(2)
                .filter_map(|w| {
                    if w[0].is_zero() {
                        None
                    } else {
                        Some((w[1] - w[0]) / w[0])
                    }
                })
                .collect();

            if returns.is_empty() {
                Decimal::ZERO
            } else {
                let mean: Decimal = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
                let variance: Decimal = returns
                    .iter()
                    .map(|r| (*r - mean) * (*r - mean))
                    .sum::<Decimal>()
                    / Decimal::from(returns.len());

                // Approximate sqrt
                let mut guess = variance / dec!(2);
                for _ in 0..10 {
                    if guess.is_zero() {
                        break;
                    }
                    guess = (guess + variance / guess) / dec!(2);
                }
                guess
            }
        } else {
            Decimal::ZERO
        };

        // Convert signals
        let brain_signals: Vec<BrainSignal> = signals.iter().map(|s| BrainSignal {
            strategy: s.strategy.clone(),
            signal: format!("{:?}", s.signal).to_uppercase(),
            confidence: s.confidence,
            reason: s.reason.clone(),
        }).collect();

        // Convert arbitrage
        let brain_arb: Vec<BrainArbOpportunity> = arb_opps.iter().map(|a| BrainArbOpportunity {
            arb_type: a.arb_type.clone(),
            spread_bps: a.spread_bps,
            net_profit_usd: a.net_profit_usd,
        }).collect();

        // Get recent trades
        let recent_trades: Vec<BrainTradeOutcome> = self
            .portfolio
            .recent_trades(5)
            .into_iter()
            .map(|t| BrainTradeOutcome {
                symbol: t.symbol,
                pnl: t.pnl,
                outcome: format!("{:?}", t.outcome).to_lowercase(),
                reason: t.reason,
            })
            .collect();

        BrainInput {
            timestamp: chrono::Utc::now().timestamp(),
            symbol: symbol.to_string(),
            market_state: BrainMarketState {
                price: current_price,
                spread_bps: self
                    .aggregator
                    .get_tick(symbol)
                    .map(|t| t.spread_bps())
                    .unwrap_or_default(),
                volatility_1h: volatility,
                regime: format!("{:?}", regime).to_lowercase(),
            },
            signals: brain_signals,
            arbitrage: brain_arb,
            portfolio: BrainPortfolio {
                capital: self.portfolio.capital(),
                positions: self
                    .portfolio
                    .positions()
                    .into_iter()
                    .map(|p| simmons_brain::BrainPosition {
                        symbol: p.symbol,
                        side: format!("{:?}", p.side).to_lowercase(),
                        size: p.size,
                        entry_price: p.entry_price,
                        unrealized_pnl: p.unrealized_pnl,
                    })
                    .collect(),
                drawdown: self.portfolio.drawdown(),
                risk_level: if self.portfolio.drawdown() > dec!(0.1) {
                    "elevated".to_string()
                } else {
                    "normal".to_string()
                },
            },
            recent_trades,
        }
    }

    async fn check_exits(&mut self) -> Result<()> {
        // Update positions with current prices
        let mut prices = HashMap::new();
        for symbol in &self.symbols {
            if let Some(tick) = self.aggregator.get_tick(symbol) {
                prices.insert(symbol.clone(), tick.price);
            }
        }
        self.portfolio.update_positions(&prices);

        // Check for exits
        let exits = self.portfolio.check_exits();
        for (symbol, price, reason) in exits {
            info!("Triggering {} for {} @ ${}", reason, symbol, price);
            match self.exec.close_position(&symbol, price, &reason).await {
                Ok(trade) => {
                    info!("✓ Exit filled: P&L ${:.2}", trade.pnl);

                    // FEEDBACK LOOP: Record trade outcome for learning
                    let reasoning = format!("Auto exit: {}", reason);
                    let signals = self.last_signals.clone();
                    let conditions = self.last_conditions.clone().unwrap_or_default();

                    self.feedback.on_trade_complete(
                        trade.clone(),
                        &reasoning,
                        signals,
                        vec![],
                        conditions,
                    );

                    // Update brain state
                    if let Ok(mut state) = self.brain.load_state() {
                        state.record_trade(trade.outcome, trade.pnl);
                        let _ = self.brain.save_state(&state);
                    }
                }
                Err(e) => {
                    error!("Failed to close position: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_decision(&mut self, decision: BrainDecision) -> Result<()> {
        info!("");
        info!("╔═══ Claude Decision ═══╗");
        info!("║ Action: {}", decision.action);
        if let Some(ref symbol) = decision.symbol {
            info!("║ Symbol: {}", symbol);
        }
        if let Some(ref side) = decision.side {
            info!("║ Side: {}", side);
        }
        if let Some(size) = decision.size_pct {
            info!("║ Size: {:.1}%", size * dec!(100));
        }
        info!("║ Confidence: {:.0}%", decision.confidence * dec!(100));
        info!("║ Reasoning: {}", decision.reasoning);
        info!("╚════════════════════════╝");
        info!("");

        // Store reasoning for feedback loop
        self.last_reasoning = Some(decision.reasoning.clone());

        match decision.action() {
            Action::Trade => {
                let symbol = decision.symbol.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Trade decision missing symbol")
                })?;

                let side = decision.side().ok_or_else(|| {
                    anyhow::anyhow!("Trade decision missing side")
                })?;

                // Get current price
                let current_price = self
                    .aggregator
                    .get_tick(symbol)
                    .map(|t| t.price)
                    .ok_or_else(|| anyhow::anyhow!("No price for {}", symbol))?;

                // Check risk
                if let Err(e) = self.risk.can_trade() {
                    warn!("Risk check failed: {}", e);
                    return Ok(());
                }

                // Size order
                let order = self.risk.size_order(
                    symbol,
                    side,
                    current_price,
                    decision.confidence,
                    decision.stop_loss_pct,
                    decision.take_profit_pct,
                )?;

                info!(
                    "Executing: {:?} {} {} @ ${:.2}",
                    order.side, order.size, order.symbol, current_price
                );

                // Execute
                match self.exec.execute(order, current_price).await {
                    Ok(result) => {
                        info!("✓ Filled @ ${:.2} (fee: ${:.4})", result.price, result.fee);

                        // Update brain state
                        let mut state = self.brain.load_state()?;
                        state.last_decision = Some(decision);
                        self.brain.save_state(&state)?;
                    }
                    Err(e) => {
                        error!("Execution failed: {}", e);
                    }
                }
            }
            Action::ClosePosition => {
                if let Some(symbol) = &decision.symbol {
                    let current_price = self
                        .aggregator
                        .get_tick(symbol)
                        .map(|t| t.price)
                        .unwrap_or_default();

                    match self.exec.close_position(symbol, current_price, "claude_decision").await {
                        Ok(trade) => {
                            info!("✓ Closed position: P&L ${:.2}", trade.pnl);

                            // FEEDBACK LOOP: Record trade outcome for learning
                            let reasoning = self.last_reasoning.clone().unwrap_or_default();
                            let signals = self.last_signals.clone();
                            let conditions = self.last_conditions.clone().unwrap_or_default();

                            self.feedback.on_trade_complete(
                                trade.clone(),
                                &reasoning,
                                signals,
                                vec![], // patterns (could extract from signals)
                                conditions,
                            );

                            // Update brain state
                            let mut state = self.brain.load_state()?;
                            state.record_trade(trade.outcome, trade.pnl);
                            state.last_decision = Some(decision);
                            self.brain.save_state(&state)?;
                        }
                        Err(e) => {
                            error!("Failed to close position: {}", e);
                        }
                    }
                }
            }
            Action::Skip => {
                debug!("Skipping trade: {}", decision.reasoning);
            }
        }

        Ok(())
    }

    fn show_summary(&self) {
        info!("");
        info!("╔═══════════════════════════════════════╗");
        info!("║           SESSION SUMMARY              ║");
        info!("╠═══════════════════════════════════════╣");
        info!("║ Final Equity: ${:.2}", self.portfolio.total_equity());
        info!("║ Realized P&L: ${:.2}", self.portfolio.snapshot().realized_pnl);
        info!("║ Win Rate: {:.1}%", self.portfolio.win_rate() * dec!(100));
        info!("║ Max Drawdown: {:.1}%", self.portfolio.drawdown() * dec!(100));
        info!("╠═══════════════════════════════════════╣");
        info!("║          LEARNING INSIGHTS             ║");
        info!("╠═══════════════════════════════════════╣");

        let insights = self.feedback.get_insights();
        info!("║ Period Trades: {}", insights.period_trades);
        info!("║ Period Win Rate: {:.1}%", insights.period_win_rate * dec!(100));
        info!("║ Period PnL: ${:.2}", insights.period_pnl);

        // Show best strategies
        if !insights.best_strategies.is_empty() {
            info!("║ Best Strategies:");
            for strat in insights.best_strategies.iter().take(3) {
                info!("║   • {} ({:.1}% WR)", strat.name, strat.win_rate * dec!(100));
            }
        }

        // Show recommendations
        if !insights.recommendations.is_empty() {
            info!("║ Recommendations:");
            for rec in &insights.recommendations {
                info!("║   → {}", rec);
            }
        }

        // Strategy health
        let health = self.feedback.evaluate_strategy_health();
        info!("║ Strategy Health: {:?}", health.overall);
        if !health.critical.is_empty() {
            for entry in &health.critical {
                warn!("║   ⚠ {} needs attention ({:.1}% WR)", entry.name, entry.win_rate * dec!(100));
            }
        }

        info!("╚═══════════════════════════════════════╝");
    }

    /// Get feedback loop for external access
    pub fn feedback(&self) -> &FeedbackLoop {
        &self.feedback
    }

    /// Get mutable feedback loop
    pub fn feedback_mut(&mut self) -> &mut FeedbackLoop {
        &mut self.feedback
    }

    /// Get sentiment signal from news feed
    async fn get_sentiment_signal(&self, symbol: &str) -> Option<StrategySignal> {
        let news_feed = self.news_feed.read().await;
        let snapshot = news_feed.aggregate();

        // Only use sentiment if we have enough data
        if snapshot.confidence < dec!(0.3) {
            return None;
        }

        // Extract keyword (e.g., "BTC" from "BTC-USDT")
        let keyword = symbol.split('-').next().unwrap_or(symbol).to_lowercase();

        // Try keyword-specific sentiment first
        let score = news_feed
            .get_keyword_sentiment(&keyword)
            .unwrap_or(snapshot.overall_score);

        let signal = if score > dec!(0.5) {
            Signal::StrongBuy
        } else if score > dec!(0.2) {
            Signal::Buy
        } else if score < dec!(-0.5) {
            Signal::StrongSell
        } else if score < dec!(-0.2) {
            Signal::Sell
        } else {
            Signal::Hold
        };

        Some(StrategySignal {
            strategy: "sentiment".to_string(),
            signal,
            confidence: snapshot.confidence,
            reason: format!(
                "Sentiment: {:?} (score: {:.2}, {} samples)",
                snapshot.overall_level, score, snapshot.sample_size
            ),
        })
    }

    /// Get X Layer DEX price for a symbol
    async fn get_xlayer_price(&self, symbol: &str) -> Option<Decimal> {
        // Extract base token (e.g., "ETH" from "ETH-USDT")
        let base = symbol.split('-').next().unwrap_or(symbol);
        self.xlayer_feed.get_price(base).await
    }
}
