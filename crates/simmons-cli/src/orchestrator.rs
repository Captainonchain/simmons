//! Main orchestrator - coordinates all components

use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_alpha::AlphaEngine;
use simmons_brain::{
    BrainArbOpportunity, BrainBridge, BrainDecision, BrainInput, BrainMarketState,
    BrainPortfolio, BrainSignal, BrainTradeOutcome,
};
use simmons_core::{Action, Config};
use simmons_exec::ExecutionEngine;
use simmons_feeds::{MarketAggregator, OkxFeed};
use simmons_risk::{Portfolio, RiskGovernor};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep, Instant};
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
}

impl Engine {
    pub async fn new(config: Config) -> Result<Self> {
        let portfolio = Arc::new(Portfolio::new(config.capital_usd));
        let risk = RiskGovernor::new(portfolio.clone(), config.risk.clone());
        let exec = ExecutionEngine::new(config.mode, config.execution.clone(), portfolio.clone());

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

        info!("Starting trading loop...");
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

            // Update market data
            while let Ok(tick) = tick_rx.try_recv() {
                self.aggregator.update_tick(tick);
            }

            // Wait for update interval
            update_timer.tick().await;

            // Generate signals for each symbol
            for symbol in &self.symbols.clone() {
                if let Some(prices) = self.aggregator.get_prices(symbol) {
                    if prices.len() < 20 {
                        continue; // Need more data
                    }

                    // Generate signals
                    let signals = self.alpha.generate_signals(symbol, &prices);
                    let regime = self.alpha.detect_regime(&prices);
                    let (combined, confidence) = self.alpha.combine_signals(&signals);

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

                    // Write signals for Claude
                    if let Err(e) = self.brain.write_signals(&input) {
                        error!("Failed to write signals: {}", e);
                    }
                }
            }

            // Check for position exits (stop loss / take profit)
            self.check_exits().await?;

            // Check for Claude decision
            if let Some(decision) = self.brain.read_decision()? {
                self.handle_decision(decision).await?;
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

    async fn check_exits(&self) -> Result<()> {
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
            if let Err(e) = self.exec.close_position(&symbol, price, &reason).await {
                error!("Failed to close position: {}", e);
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

                            // Update state
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
        info!("╚═══════════════════════════════════════╝");
    }
}
