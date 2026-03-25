//! Simmons CLI - Main entry point

use anyhow::Result;
use clap::{Parser, Subcommand};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simmons_alpha::AlphaEngine;
use simmons_brain::BrainBridge;
use simmons_core::{Config, TradingMode};
use simmons_feeds::{MarketAggregator, OkxFeed};
use simmons_mcp::TradingState;
use simmons_risk::Portfolio;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod orchestrator;
mod web;

use orchestrator::Engine;

#[derive(Parser)]
#[command(name = "simmons")]
#[command(about = "Simmons - Max ROI DeFi Trading System")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config/settings.toml")]
    config: String,

    /// Trading mode
    #[arg(short, long, default_value = "paper")]
    mode: String,

    /// Initial capital in USD
    #[arg(long, default_value = "100")]
    capital: f64,

    /// Symbols to trade (comma-separated) - if not specified, uses config file
    #[arg(long)]
    symbols: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the trading engine
    Run {
        /// Duration to run in seconds (0 = indefinite)
        #[arg(short, long, default_value = "0")]
        duration: u64,
    },
    /// Run a backtest simulation
    Sim {
        /// Simulation duration in seconds
        #[arg(short, long, default_value = "300")]
        duration: u64,
    },
    /// Show current signals without trading
    Signals,
    /// Show portfolio status
    Status,
    /// Start web dashboard
    Dashboard {
        /// Port to run on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    /// Run as MCP server (for Claude integration)
    Mcp,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // For MCP mode, skip logging to stderr (it interferes with the protocol)
    let is_mcp = matches!(cli.command, Some(Commands::Mcp));

    if !is_mcp {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive("simmons=info".parse()?))
            .init();
    }

    // Load config
    let mut config = Config::load_or_default(&cli.config);

    // Override with CLI args
    config.capital_usd = Decimal::try_from(cli.capital)?;
    config.mode = match cli.mode.as_str() {
        "live" => TradingMode::Live,
        "sim" | "simulation" => TradingMode::Simulation,
        _ => TradingMode::Paper,
    };
    // Only override symbols if explicitly provided via CLI
    if let Some(symbols) = cli.symbols {
        config.symbols = symbols
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
    }

    if !is_mcp {
        info!("╔═══════════════════════════════════════════════════════════╗");
        info!("║                      SIMMONS v0.1.0                       ║");
        info!("║              Max ROI DeFi Trading System                  ║");
        info!("╚═══════════════════════════════════════════════════════════╝");
        info!("");
        info!("Mode: {:?}", config.mode);
        info!("Capital: ${}", config.capital_usd);
        info!("Symbols: {:?}", config.symbols);
        info!("");
    }

    match cli.command {
        Some(Commands::Run { duration }) => {
            run_engine(config, duration).await?;
        }
        Some(Commands::Sim { duration }) => {
            config.mode = TradingMode::Simulation;
            run_simulation(config, duration).await?;
        }
        Some(Commands::Signals) => {
            show_signals(config).await?;
        }
        Some(Commands::Status) => {
            show_status(config).await?;
        }
        Some(Commands::Dashboard { port }) => {
            web::start_server(config, port).await?;
        }
        Some(Commands::Mcp) => {
            run_mcp_server(config).await?;
        }
        None => {
            // Default: start dashboard
            info!("Starting dashboard on http://localhost:3000");
            web::start_server(config, 3000).await?;
        }
    }

    Ok(())
}

async fn run_engine(config: Config, duration: u64) -> Result<()> {
    let symbols: Vec<String> = config.symbols.clone();
    let mut engine = Engine::new(config).await?;

    let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
    engine.run(&symbol_refs, duration).await
}

async fn run_simulation(config: Config, duration: u64) -> Result<()> {
    info!("Running simulation for {} seconds...", duration);

    let portfolio = Arc::new(Portfolio::new(config.capital_usd));
    let alpha = AlphaEngine::default();

    // Generate synthetic price data
    let mut prices: Vec<Decimal> = vec![dec!(67000)];
    let mut rng_state: u64 = 12345;

    for _ in 0..100 {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let change = ((rng_state % 200) as i64 - 100) as i32;
        let last = *prices.last().unwrap();
        prices.push(last + Decimal::from(change));
    }

    // Generate signals
    let signals = alpha.generate_signals("BTC-USDT", &prices);
    let (combined_signal, confidence) = alpha.combine_signals(&signals);
    let regime = alpha.detect_regime(&prices);

    info!("Simulated price: ${}", prices.last().unwrap());
    info!("Regime: {:?}", regime);
    info!("Combined signal: {:?} ({:.1}% confidence)", combined_signal, confidence * dec!(100));

    for sig in &signals {
        info!(
            "  {} {:?} ({:.1}%): {}",
            sig.strategy,
            sig.signal,
            sig.confidence * dec!(100),
            sig.reason
        );
    }

    // Simulate trading
    if combined_signal.is_bullish() && confidence > dec!(0.6) {
        info!("Would open LONG position");
    } else if combined_signal.is_bearish() && confidence > dec!(0.6) {
        info!("Would open SHORT position");
    } else {
        info!("No trade - insufficient signal strength");
    }

    info!("\nSimulation complete.");
    info!("Final capital: ${}", portfolio.total_equity());

    Ok(())
}

async fn show_signals(config: Config) -> Result<()> {
    info!("Fetching current signals...\n");

    // Connect to feed
    let feed = OkxFeed::new(&config.feeds.okx_ws_url);
    let symbols: Vec<&str> = config.symbols.iter().map(|s| s.as_str()).collect();

    feed.connect(&symbols).await?;

    let aggregator = MarketAggregator::new(config.feeds.price_window_size);
    let alpha = AlphaEngine::default();

    // Collect some data
    let mut tick_rx = feed.tick_receiver();
    let mut count = 0;

    while count < 20 {
        if let Ok(tick) = tick_rx.recv().await {
            aggregator.update_tick(tick);
            count += 1;
        }
    }

    // Generate signals
    for symbol in &config.symbols {
        if let Some(prices) = aggregator.get_prices(symbol) {
            let signals = alpha.generate_signals(symbol, &prices);
            let (combined, confidence) = alpha.combine_signals(&signals);
            let regime = alpha.detect_regime(&prices);

            info!("═══ {} ═══", symbol);
            info!("Price: ${}", prices.last().unwrap_or(&Decimal::ZERO));
            info!("Regime: {:?}", regime);
            info!("Combined: {:?} ({:.1}%)", combined, confidence * dec!(100));

            for sig in &signals {
                info!(
                    "  • {} {:?} ({:.1}%): {}",
                    sig.strategy,
                    sig.signal,
                    sig.confidence * dec!(100),
                    sig.reason
                );
            }
            info!("");
        }
    }

    feed.stop();
    Ok(())
}

async fn show_status(config: Config) -> Result<()> {
    let brain = BrainBridge::new(&config.brain.data_dir, config.brain.timeout_secs, false);

    // Load state
    let state = brain.load_state()?;

    info!("═══ Portfolio Status ═══\n");
    info!("Total Trades: {}", state.total_trades);
    info!("Wins: {} | Losses: {}", state.wins, state.losses);
    info!("Win Rate: {:.1}%", state.win_rate() * dec!(100));
    info!("Total P&L: ${}", state.total_pnl);

    if let Some(last) = &state.last_decision {
        info!("\nLast Decision:");
        info!("  Action: {}", last.action);
        info!("  Confidence: {:.1}%", last.confidence * dec!(100));
        info!("  Reasoning: {}", last.reasoning);
    }

    Ok(())
}

async fn run_mcp_server(config: Config) -> Result<()> {
    // For MCP mode, we don't log to stderr as it interferes with the protocol
    // The MCP server handles its own logging

    // Create trading state with sample data for testing
    // In production, this would be updated by the alpha engine
    let state = TradingState::with_sample_data(config.capital_usd);

    // Run the MCP server (blocks until client disconnects)
    simmons_mcp::server::run_server(state).await
}
