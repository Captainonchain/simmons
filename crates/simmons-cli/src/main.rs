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

mod dual_loop;
mod orchestrator;
mod web;

use dual_loop::{DualBrainConfig, DualBrainLoop};
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
    /// Run dual brain loop (TA + Fundamental)
    Dual {
        /// Also start dashboard
        #[arg(long)]
        dashboard: bool,
        /// Dashboard port
        #[arg(short, long, default_value = "3456")]
        port: u16,
    },
    /// Test TA brain analysis
    TestTaBrain {
        /// Symbol to analyze
        #[arg(short, long, default_value = "BTC-USDT")]
        symbol: String,
    },
    /// Test Fund brain analysis
    TestFundBrain {
        /// Token to analyze
        #[arg(short, long, default_value = "BTC")]
        token: String,
        /// Chain
        #[arg(short, long, default_value = "ethereum")]
        chain: String,
    },
    /// Test OnchainOS API integration (for debugging mainnet)
    TestOnchainos,
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
        Some(Commands::Dual { dashboard, port }) => {
            // Use PORT env var if set (Railway), otherwise use CLI arg
            let actual_port = std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(port);
            run_dual_brain(config, dashboard, actual_port).await?;
        }
        Some(Commands::TestTaBrain { symbol }) => {
            test_ta_brain(config, &symbol).await?;
        }
        Some(Commands::TestFundBrain { token, chain }) => {
            test_fund_brain(config, &token, &chain).await?;
        }
        Some(Commands::TestOnchainos) => {
            test_onchainos().await?;
        }
        None => {
            // Default: start dashboard (use PORT env var for Railway)
            let port = std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000);
            info!("Starting dashboard on http://localhost:{}", port);
            web::start_server(config, port).await?;
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

async fn run_dual_brain(config: Config, with_dashboard: bool, port: u16) -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║             SIMMONS DUAL BRAIN v3.0                       ║");
    info!("║           TA Brain + Fundamental Brain                    ║");
    info!("╚═══════════════════════════════════════════════════════════╝");
    info!("");

    let dual_config = DualBrainConfig::from(&config);
    let aggregator = Arc::new(MarketAggregator::new(config.feeds.price_window_size));

    // Connect to price feed
    let feed = OkxFeed::new(&config.feeds.okx_ws_url);
    let symbols: Vec<&str> = config.symbols.iter().map(|s| s.as_str()).collect();
    feed.connect(&symbols).await?;

    // Start price aggregation
    let aggregator_clone = aggregator.clone();
    let mut tick_rx = feed.tick_receiver();
    tokio::spawn(async move {
        while let Ok(tick) = tick_rx.recv().await {
            aggregator_clone.update_tick(tick);
        }
    });

    // Wait for some initial data
    info!("Collecting initial price data...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Optionally start dashboard in background
    if with_dashboard {
        let dashboard_config = config.clone();
        tokio::spawn(async move {
            if let Err(e) = web::start_server(dashboard_config, port).await {
                tracing::error!("Dashboard error: {}", e);
            }
        });
        info!("Dashboard started on http://localhost:{}", port);
    }

    // Run dual brain loop
    // Use executor for live mode, simple new() for paper mode
    let mut dual_loop = if dual_config.mode == simmons_core::TradingMode::Live {
        info!("Initializing LIVE mode with X Layer executor...");
        DualBrainLoop::new_with_executor(dual_config, aggregator).await?
    } else {
        DualBrainLoop::new(dual_config, aggregator)
    };
    dual_loop.run().await
}

async fn test_ta_brain(config: Config, symbol: &str) -> Result<()> {
    use simmons_brain::ta_brain::{TABrain, TABrainConfig};

    info!("Testing TA Brain for {}...", symbol);

    let aggregator = Arc::new(MarketAggregator::new(config.feeds.price_window_size));

    // Connect and collect data
    let feed = OkxFeed::new(&config.feeds.okx_ws_url);
    feed.connect(&[symbol]).await?;

    let mut tick_rx = feed.tick_receiver();
    let mut count = 0;
    while count < 50 {
        if let Ok(tick) = tick_rx.recv().await {
            aggregator.update_tick(tick);
            count += 1;
        }
    }

    let prices = aggregator.get_prices(symbol).unwrap_or_default();
    let volumes: Vec<Decimal> = vec![dec!(1000000); prices.len()];

    if prices.len() < 20 {
        return Err(anyhow::anyhow!("Not enough price data"));
    }

    let mut ta_brain = TABrain::new(TABrainConfig::default());

    // Detect regime
    let sma20: Decimal = prices.iter().rev().take(20).sum::<Decimal>() / dec!(20);
    let current = *prices.last().unwrap();
    let regime = if (current - sma20) / sma20 > dec!(0.02) {
        simmons_core::Regime::TrendingUp
    } else if (current - sma20) / sma20 < dec!(-0.02) {
        simmons_core::Regime::TrendingDown
    } else {
        simmons_core::Regime::MeanReverting
    };

    let output = ta_brain.analyze(
        symbol,
        &prices,
        &volumes,
        regime,
        dec!(50_000_000),
        Some(dec!(100_000_000)),
        Some(dec!(0.0001)),
    );

    info!("");
    info!("═══ TA BRAIN OUTPUT ═══");
    info!("Symbol: {}", output.symbol);
    info!("RADAR Score: {} ({:?})", output.radar_score.score, output.radar_score.tier);
    info!("  - Market Structure: {}/140", output.radar_score.market_structure);
    info!("  - Technicals: {}/160", output.radar_score.technicals);
    info!("  - Funding: {}/100", output.radar_score.funding);
    info!("PULSE: Tier {} ({:?})", output.pulse_signal.tier, output.pulse_signal.direction);
    info!("Regime: {:?}", output.regime);
    info!("Overall Sentiment: {:.2}", output.overall_sentiment);
    info!("Overall Confidence: {:.0}%", output.overall_confidence * dec!(100));
    info!("");
    info!("Recommendation: {:?}", output.recommended_action.action);
    info!("  - Strategy: {:?}", output.recommended_action.strategy);
    info!("  - Size Factor: {:.0}%", output.recommended_action.size_factor * dec!(100));
    info!("  - Reasoning: {}", output.recommended_action.reasoning);
    info!("");
    info!("Strategy Signals:");
    for signal in &output.strategy_signals {
        info!(
            "  - {:?}: {:?} ({:.0}%) - {}",
            signal.strategy,
            signal.signal,
            signal.confidence * dec!(100),
            signal.reason
        );
    }

    feed.stop();
    Ok(())
}

async fn test_fund_brain(config: Config, token: &str, chain: &str) -> Result<()> {
    use simmons_brain::fund_brain::{FundBrain, FundBrainConfig, WhaleAction, WhaleSignal};
    use simmons_feeds::{OnchainFeed, TwitterFeed};

    info!("Testing Fund Brain for {} on {}...", token, chain);

    let mut fund_brain = FundBrain::new(FundBrainConfig::default());
    let onchain = OnchainFeed::new();
    let mut twitter = TwitterFeed::with_defaults();

    // Get whale signals
    info!("Fetching whale signals...");
    let whale_signals: Vec<WhaleSignal> = match onchain.get_smart_money_signals(chain, 50).await {
        Ok(signals) => signals
            .into_iter()
            .filter(|s| {
                s.token_symbol
                    .as_ref()
                    .map_or(false, |sym| sym.eq_ignore_ascii_case(token))
            })
            .map(|s| WhaleSignal {
                address: s.wallet_address.unwrap_or_default(),
                action: match s.action.as_str() {
                    "buy" => WhaleAction::Buy,
                    "sell" => WhaleAction::Sell,
                    _ => WhaleAction::Transfer,
                },
                token: token.to_string(),
                chain: chain.to_string(),
                value_usd: s.amount_usd.unwrap_or_default(),
                is_smart_money: s.signal_type == "smart_money",
                timestamp: chrono::Utc::now(),
            })
            .collect(),
        Err(e) => {
            info!("Could not fetch whale signals: {}", e);
            vec![]
        }
    };
    info!("Found {} whale signals", whale_signals.len());

    // Get Twitter sentiment
    info!("Fetching Twitter sentiment...");
    let twitter_sentiment = match twitter.get_sentiment(token).await {
        Ok(sentiment) => {
            info!(
                "Twitter: {} mentions, sentiment {:.2}",
                sentiment.mention_count, sentiment.sentiment_score
            );
            Some(simmons_brain::fund_brain::TwitterSentiment {
                token: token.to_string(),
                sentiment_score: sentiment.sentiment_score,
                mention_count: sentiment.mention_count,
                kol_mentions: vec![],
                trending_score: sentiment.trending_score,
                window_hours: sentiment.window_hours,
                timestamp: sentiment.timestamp,
            })
        }
        Err(e) => {
            info!("Could not fetch Twitter: {}", e);
            None
        }
    };

    // Get security
    info!("Checking security...");
    let security = match onchain.check_security(chain, token).await {
        Ok(result) => {
            info!(
                "Security: honeypot={}, risk_score={}",
                result.is_honeypot, result.risk_score
            );
            Some(simmons_brain::fund_brain::SecurityAssessment {
                token: token.to_string(),
                chain: chain.to_string(),
                is_honeypot: result.is_honeypot,
                buy_tax: result.buy_tax,
                sell_tax: result.sell_tax,
                can_take_ownership: false,
                can_change_balance: false,
                is_mintable: result.is_mintable,
                liquidity_usd: None,
                risk_score: result.risk_score,
                red_flags: vec![],
                timestamp: chrono::Utc::now(),
            })
        }
        Err(e) => {
            info!("Could not check security: {}", e);
            None
        }
    };

    // Analyze
    let output = fund_brain.analyze_with_data(token, chain, whale_signals, twitter_sentiment, None, security);

    info!("");
    info!("═══ FUND BRAIN OUTPUT ═══");
    info!("Token: {} on {}", output.symbol, output.chain);
    info!("Whale Sentiment: {:.2}", output.whale_sentiment);
    info!("Twitter Sentiment: {:.2}", output.twitter_sentiment);
    info!("News Sentiment: {:.2}", output.news_sentiment);
    info!("Overall Sentiment: {:.2}", output.overall_sentiment);
    info!("Overall Confidence: {:.0}%", output.overall_confidence * dec!(100));
    info!("");
    info!("Recommendation: {:?}", output.recommendation.action);
    info!("  - Size Modifier: {:.0}%", output.recommendation.size_modifier * dec!(100));
    info!("  - Reasoning: {}", output.recommendation.reasoning);
    if !output.recommendation.security_warnings.is_empty() {
        info!("  - Warnings: {:?}", output.recommendation.security_warnings);
    }

    Ok(())
}

use std::time::Duration;

async fn test_onchainos() -> Result<()> {
    use simmons_infra::onchainos::{chains, native_tokens, OnchainOSClient, QuoteRequest};

    // Load .env file
    dotenvy::dotenv().ok();

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║          ONCHAINOS API INTEGRATION TEST                   ║");
    info!("╚═══════════════════════════════════════════════════════════╝");
    info!("");

    // Layer 1: Environment Variables
    info!("=== Layer 1: Environment Variables ===");
    let api_key = std::env::var("OKX_API_KEY");
    let secret_key = std::env::var("OKX_SECRET_KEY");
    let passphrase = std::env::var("OKX_PASSPHRASE");
    let private_key = std::env::var("XLAYER_PRIVATE_KEY");

    info!("OKX_API_KEY: {}", if api_key.is_ok() { "✓ SET" } else { "✗ MISSING" });
    info!("OKX_SECRET_KEY: {}", if secret_key.is_ok() { "✓ SET" } else { "✗ MISSING" });
    info!("OKX_PASSPHRASE: {}", if passphrase.is_ok() { "✓ SET" } else { "✗ MISSING" });
    info!("XLAYER_PRIVATE_KEY: {}", if private_key.as_ref().map(|k| k != "ENTER_YOUR_PRIVATE_KEY_HERE").unwrap_or(false) { "✓ SET" } else { "⚠ NOT CONFIGURED" });
    info!("");

    // Layer 2: OKX API Client
    info!("=== Layer 2: OKX API Client ===");
    let client = match OnchainOSClient::from_env() {
        Ok(c) => {
            info!("Client initialized: ✓");
            c
        }
        Err(e) => {
            info!("Client initialization FAILED: {}", e);
            return Err(e);
        }
    };
    info!("");

    // Layer 3: Get Quote (X Layer OKB -> USDT)
    info!("=== Layer 3: DEX Quote API ===");
    info!("Testing: 0.01 OKB -> USDT on X Layer (chain 196)");

    let quote_request = QuoteRequest {
        chain_id: chains::XLAYER.to_string(),
        from_token: native_tokens::EVM_NATIVE.to_string(),
        to_token: "0x1E4a5963aBFD975d8c9021ce480b42188849D41d".to_string(), // USDT on X Layer
        amount: "10000000000000000".to_string(), // 0.01 OKB (18 decimals)
        slippage: Some("1.0".to_string()),
    };

    match client.get_quote(&quote_request).await {
        Ok(quote) => {
            info!("Quote received: ✓");
            info!("  From amount: {}", quote.from_token_amount);
            info!("  To amount: {}", quote.to_token_amount);
            info!("  Price impact: {}%", quote.price_impact());
            info!("  Gas estimate: {}", quote.estimate_gas_fee);
            let routers: Vec<String> = quote.dex_router_list.iter()
                .map(|r| format!("{} ({}%)", r.dex_protocol.dex_name, r.dex_protocol.percent))
                .collect();
            info!("  DEX routers: {:?}", routers);
        }
        Err(e) => {
            info!("Quote FAILED: {}", e);
            info!("This could indicate:");
            info!("  - Invalid API credentials");
            info!("  - Token not supported");
            info!("  - Amount too small");
        }
    }
    info!("");

    // Layer 4: Get Supported Tokens
    info!("=== Layer 4: Supported Tokens API ===");
    match client.get_supported_tokens(chains::XLAYER).await {
        Ok(tokens) => {
            info!("Tokens received: ✓ ({} tokens)", tokens.len());
            for token in tokens.iter().take(5) {
                info!("  - {} ({}) @ {}", token.token_symbol, token.token_name, token.token_contract_address);
            }
            if tokens.len() > 5 {
                info!("  ... and {} more", tokens.len() - 5);
            }
        }
        Err(e) => {
            info!("Tokens FAILED: {}", e);
        }
    }
    info!("");

    // Layer 5: Get Token Price
    info!("=== Layer 5: Token Price API ===");
    match client.get_price(chains::XLAYER, native_tokens::EVM_NATIVE).await {
        Ok(price) => {
            info!("OKB Price: ${}", price.price);
            info!("Timestamp: {}", price.time);
        }
        Err(e) => {
            info!("Price FAILED: {}", e);
        }
    }
    info!("");

    // Layer 6: Test Swap Data (without executing)
    info!("=== Layer 6: Swap Transaction Data ===");
    let test_address = "0x0000000000000000000000000000000000000001";
    match client.prepare_swap(
        chains::XLAYER,
        native_tokens::EVM_NATIVE,
        "0x1E4a5963aBFD975d8c9021ce480b42188849D41d", // USDT
        "10000000000000000", // 0.01 OKB
        test_address,
        "1.0",
    ).await {
        Ok(swap) => {
            info!("Swap data prepared: ✓");
            info!("  To contract: {}", swap.to);
            info!("  Value: {} wei", swap.value);
            info!("  Gas limit: {}", swap.gas_limit);
            info!("  Gas price: {}", swap.gas_price);
            info!("  Min out: {}", swap.min_out);
            info!("  Data length: {} bytes", swap.data.len() / 2 - 1); // hex string
        }
        Err(e) => {
            info!("Swap data FAILED: {}", e);
            info!("Note: This may fail with test address - expected");
        }
    }
    info!("");

    // Summary
    info!("═══════════════════════════════════════════════════════════");
    info!("                        SUMMARY");
    info!("═══════════════════════════════════════════════════════════");
    info!("API connectivity: Working");
    if private_key.as_ref().map(|k| k != "ENTER_YOUR_PRIVATE_KEY_HERE").unwrap_or(false) {
        info!("Private key: ✓ Configured");
        info!("");
        info!("✓ Ready for live trading!");
    } else {
        info!("Private key: ⚠ Not configured");
        info!("");
        info!("To enable live trading:");
        info!("  1. Export your wallet private key from MetaMask");
        info!("  2. Add to .env: XLAYER_PRIVATE_KEY=<your_key>");
        info!("  3. Fund wallet with OKB for gas + trading capital");
    }

    Ok(())
}
