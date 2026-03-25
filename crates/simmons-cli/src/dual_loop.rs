//! Dual Brain Loop - Runs TA and Fund brains in parallel
//!
//! This module coordinates both brains, feeds signals to the consensus layer,
//! and writes context for Claude orchestration.

use anyhow::Result;
use chrono::Utc;
use num_traits::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_brain::{
    consensus::{ConsensusAction, ConsensusEngine, MergedContext},
    fund_brain::{FundBrain, FundBrainConfig, SecurityAssessment, WhaleAction, WhaleSignal},
    ta_brain::{TABrain, TABrainConfig},
};
use simmons_core::{Config, Regime, TradingMode};
use simmons_feeds::{
    twitter::{TwitterFeed, TwitterSentiment},
    MarketAggregator, OnchainFeed,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Dual brain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualBrainConfig {
    /// Trading mode
    pub mode: TradingMode,
    /// Supported chains
    pub chains: Vec<String>,
    /// Capital in USD
    pub capital_usd: Decimal,
    /// Symbols to track
    pub symbols: Vec<String>,
    /// TA brain config
    pub ta_config: TABrainConfig,
    /// Fund brain config
    pub fund_config: FundBrainConfig,
    /// Update interval in seconds
    pub update_interval_secs: u64,
    /// Data directory for context files
    pub data_dir: PathBuf,
    /// Auto-invoke Claude skill
    pub auto_invoke: bool,
}

impl Default for DualBrainConfig {
    fn default() -> Self {
        Self {
            mode: TradingMode::Paper,
            chains: vec!["ethereum".to_string(), "solana".to_string(), "base".to_string()],
            capital_usd: dec!(1000),
            symbols: vec!["BTC-USDT".to_string(), "ETH-USDT".to_string(), "SOL-USDT".to_string()],
            ta_config: TABrainConfig::default(),
            fund_config: FundBrainConfig::default(),
            update_interval_secs: 30,
            data_dir: PathBuf::from("data"),
            auto_invoke: false,
        }
    }
}

impl From<&Config> for DualBrainConfig {
    fn from(config: &Config) -> Self {
        Self {
            mode: config.mode,
            chains: vec!["ethereum".to_string(), "solana".to_string()],
            capital_usd: config.capital_usd,
            symbols: config.symbols.clone(),
            ta_config: TABrainConfig::default(),
            fund_config: FundBrainConfig::default(),
            update_interval_secs: config.feeds.update_interval_ms / 1000,
            data_dir: PathBuf::from(&config.brain.data_dir),
            auto_invoke: config.brain.auto_invoke,
        }
    }
}

/// Dual brain state
pub struct DualBrainState {
    /// Latest merged contexts by symbol
    pub contexts: HashMap<String, MergedContext>,
    /// Best opportunity
    pub best_opportunity: Option<String>,
    /// Is running
    pub running: bool,
    /// Last update timestamp
    pub last_update: i64,
    /// Error count
    pub error_count: u32,
}

impl Default for DualBrainState {
    fn default() -> Self {
        Self {
            contexts: HashMap::new(),
            best_opportunity: None,
            running: false,
            last_update: 0,
            error_count: 0,
        }
    }
}

/// Dual Brain Loop orchestrator
pub struct DualBrainLoop {
    config: DualBrainConfig,
    ta_brain: TABrain,
    fund_brain: FundBrain,
    consensus: ConsensusEngine,
    aggregator: Arc<MarketAggregator>,
    onchain: OnchainFeed,
    twitter: TwitterFeed,
    state: Arc<RwLock<DualBrainState>>,
}

impl DualBrainLoop {
    /// Create new dual brain loop
    pub fn new(config: DualBrainConfig, aggregator: Arc<MarketAggregator>) -> Self {
        Self {
            ta_brain: TABrain::new(config.ta_config.clone()),
            fund_brain: FundBrain::new(config.fund_config.clone()),
            consensus: ConsensusEngine::with_defaults(),
            aggregator,
            onchain: OnchainFeed::new(),
            twitter: TwitterFeed::with_defaults(),
            state: Arc::new(RwLock::new(DualBrainState::default())),
            config,
        }
    }

    /// Get shared state
    pub fn state(&self) -> Arc<RwLock<DualBrainState>> {
        self.state.clone()
    }

    /// Run the dual brain loop
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting dual brain loop...");
        info!("Mode: {:?}", self.config.mode);
        info!("Symbols: {:?}", self.config.symbols);
        info!("Chains: {:?}", self.config.chains);

        {
            let mut state = self.state.write().await;
            state.running = true;
        }

        let mut update_timer = interval(Duration::from_secs(self.config.update_interval_secs));

        loop {
            update_timer.tick().await;

            if let Err(e) = self.update_cycle().await {
                error!("Dual brain update error: {}", e);
                let mut state = self.state.write().await;
                state.error_count += 1;

                if state.error_count >= 10 {
                    error!("Too many errors, pausing for 60 seconds");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    state.error_count = 0;
                }
            }
        }
    }

    /// Single update cycle
    async fn update_cycle(&mut self) -> Result<()> {
        let mut contexts = HashMap::new();
        let mut best_score: i16 = -1000;
        let mut best_symbol: Option<String> = None;

        for symbol in &self.config.symbols.clone() {
            match self.analyze_symbol(symbol).await {
                Ok(ctx) => {
                    // Track best opportunity
                    let score = self.calculate_opportunity_score(&ctx);
                    if score > best_score {
                        best_score = score;
                        best_symbol = Some(symbol.clone());
                    }

                    contexts.insert(symbol.clone(), ctx);
                }
                Err(e) => {
                    warn!("Failed to analyze {}: {}", symbol, e);
                }
            }
        }

        // Update state
        {
            let mut state = self.state.write().await;
            state.contexts = contexts.clone();
            state.best_opportunity = best_symbol.clone();
            state.last_update = Utc::now().timestamp();
        }

        // Write context for Claude
        if !contexts.is_empty() {
            self.write_context_file(&contexts, best_symbol.as_deref()).await?;
        }

        // Log best opportunity
        if let Some(ref symbol) = best_symbol {
            if let Some(ctx) = contexts.get(symbol) {
                info!(
                    "[DualBrain] Best: {} | {:?} | Sentiment: {:.2} | Confidence: {:.0}%",
                    symbol,
                    ctx.consensus_action,
                    ctx.merged_sentiment,
                    ctx.merged_confidence * dec!(100)
                );
            }
        }

        Ok(())
    }

    /// Analyze a single symbol with both brains
    async fn analyze_symbol(&mut self, symbol: &str) -> Result<MergedContext> {
        // Get price data from aggregator
        let prices = self.aggregator.get_prices(symbol).unwrap_or_default();
        let volumes = vec![dec!(1000000); prices.len()]; // Placeholder volumes

        if prices.len() < 20 {
            return Err(anyhow::anyhow!("Not enough price data for {}", symbol));
        }

        // Detect regime
        let regime = self.detect_regime(&prices);

        // Get volume and OI (placeholders for now)
        let volume_24h = dec!(50_000_000);
        let open_interest = Some(dec!(100_000_000));
        let funding_rate = Some(dec!(0.0001));

        // Run TA Brain
        let ta_output = self.ta_brain.analyze(
            symbol,
            &prices,
            &volumes,
            regime,
            volume_24h,
            open_interest,
            funding_rate,
        );

        // Extract token and chain from symbol
        let (token, chain) = self.parse_symbol(symbol);

        // Run Fund Brain analysis with external data
        let fund_output = self.analyze_fundamentals(&token, &chain).await?;

        // Merge with consensus
        let merged = self.consensus.merge(&ta_output, &fund_output);

        Ok(merged)
    }

    /// Analyze fundamentals for a token
    async fn analyze_fundamentals(
        &mut self,
        token: &str,
        chain: &str,
    ) -> Result<simmons_brain::fund_brain::FundBrainOutput> {
        // Fetch whale signals from OnchainOS
        let whale_signals = match self.onchain.get_smart_money_signals(chain, 50).await {
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
                    timestamp: Utc::now(),
                })
                .collect(),
            Err(e) => {
                debug!("Failed to get whale signals: {}", e);
                vec![]
            }
        };

        // Fetch Twitter sentiment
        let twitter = match self.twitter.get_sentiment(token).await {
            Ok(sentiment) => Some(simmons_brain::fund_brain::TwitterSentiment {
                token: token.to_string(),
                sentiment_score: sentiment.sentiment_score,
                mention_count: sentiment.mention_count,
                kol_mentions: sentiment
                    .kol_mentions
                    .into_iter()
                    .map(|k| simmons_brain::fund_brain::KolMention {
                        handle: k.handle,
                        followers: k.followers,
                        sentiment: match k.sentiment {
                            simmons_feeds::twitter::MentionSentiment::Positive => {
                                simmons_brain::fund_brain::MentionSentiment::Positive
                            }
                            simmons_feeds::twitter::MentionSentiment::Negative => {
                                simmons_brain::fund_brain::MentionSentiment::Negative
                            }
                            simmons_feeds::twitter::MentionSentiment::Neutral => {
                                simmons_brain::fund_brain::MentionSentiment::Neutral
                            }
                        },
                        text: k.text,
                        timestamp: k.timestamp,
                    })
                    .collect(),
                trending_score: sentiment.trending_score,
                window_hours: sentiment.window_hours,
                timestamp: sentiment.timestamp,
            }),
            Err(e) => {
                debug!("Failed to get Twitter sentiment: {}", e);
                None
            }
        };

        // Fetch security assessment
        let security = match self.onchain.check_security(chain, token).await {
            Ok(result) => Some(SecurityAssessment {
                token: token.to_string(),
                chain: chain.to_string(),
                is_honeypot: result.is_honeypot,
                buy_tax: result.buy_tax,
                sell_tax: result.sell_tax,
                can_take_ownership: false, // Not in basic result
                can_change_balance: false,
                is_mintable: result.is_mintable,
                liquidity_usd: None,
                risk_score: result.risk_score,
                red_flags: vec![],
                timestamp: Utc::now(),
            }),
            Err(e) => {
                debug!("Failed to get security: {}", e);
                None
            }
        };

        // Use Fund Brain with collected data
        let output =
            self.fund_brain
                .analyze_with_data(token, chain, whale_signals, twitter, None, security);

        Ok(output)
    }

    /// Parse symbol into token and chain
    fn parse_symbol(&self, symbol: &str) -> (String, String) {
        // BTC-USDT -> (BTC, ethereum)
        let token = symbol.split('-').next().unwrap_or(symbol).to_string();

        // Default chain mapping
        let chain = match token.to_uppercase().as_str() {
            "SOL" => "solana",
            "ETH" | "WETH" => "ethereum",
            "BTC" | "WBTC" => "ethereum",
            _ => "ethereum",
        }
        .to_string();

        (token, chain)
    }

    /// Detect market regime from prices
    fn detect_regime(&self, prices: &[Decimal]) -> Regime {
        if prices.len() < 20 {
            return Regime::default();
        }

        // Calculate SMA20 and current price
        let sma20: Decimal = prices.iter().rev().take(20).sum::<Decimal>() / dec!(20);
        let current = *prices.last().unwrap();

        // Calculate volatility (simplified)
        let returns: Vec<Decimal> = prices
            .windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect();
        let avg_return: Decimal = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
        let variance: Decimal = returns
            .iter()
            .map(|r| {
                let diff = *r - avg_return;
                diff * diff
            })
            .sum::<Decimal>()
            / Decimal::from(returns.len());

        // Classify regime
        let deviation = (current - sma20) / sma20;

        if variance > dec!(0.001) {
            Regime::HighVolatility
        } else if variance < dec!(0.0001) {
            Regime::LowVolatility
        } else if deviation > dec!(0.03) {
            Regime::TrendingUp
        } else if deviation < dec!(-0.03) {
            Regime::TrendingDown
        } else if variance > dec!(0.0005) {
            Regime::Choppy
        } else {
            Regime::MeanReverting
        }
    }

    /// Calculate opportunity score for ranking
    fn calculate_opportunity_score(&self, ctx: &MergedContext) -> i16 {
        let mut score: i16 = 0;

        // Base from consensus action
        score += match ctx.consensus_action {
            ConsensusAction::Long | ConsensusAction::Short => 100,
            ConsensusAction::Close => 50,
            ConsensusAction::Hold => 0,
            ConsensusAction::Wait => -50,
            ConsensusAction::Debate => -25,
            ConsensusAction::Blocked => -1000,
        };

        // Add sentiment strength
        score += (ctx.merged_sentiment.abs() * dec!(50)).to_i64().unwrap_or(0) as i16;

        // Add confidence
        score += (ctx.merged_confidence * dec!(50)).to_i64().unwrap_or(0) as i16;

        // Subtract for conflict
        if ctx.is_conflict {
            score -= 30;
        }

        // Add RADAR score contribution
        score += (ctx.ta.radar_score.score as i16 / 4);

        score
    }

    /// Write context file for Claude
    async fn write_context_file(
        &self,
        contexts: &HashMap<String, MergedContext>,
        best_symbol: Option<&str>,
    ) -> Result<()> {
        let context_file = self.config.data_dir.join("dual_brain_context.json");

        let output = DualBrainContextFile {
            timestamp: Utc::now(),
            mode: self.config.mode,
            best_opportunity: best_symbol.map(String::from),
            contexts: contexts.clone(),
            summary: self.generate_summary(contexts, best_symbol),
        };

        let json = serde_json::to_string_pretty(&output)?;
        std::fs::write(&context_file, json)?;

        debug!("Wrote context to {:?}", context_file);
        Ok(())
    }

    /// Generate human-readable summary
    fn generate_summary(
        &self,
        contexts: &HashMap<String, MergedContext>,
        best_symbol: Option<&str>,
    ) -> String {
        let mut summary = String::from("# Dual Brain Summary\n\n");

        if let Some(symbol) = best_symbol {
            if let Some(ctx) = contexts.get(symbol) {
                summary.push_str(&format!("## Best Opportunity: {}\n\n", symbol));
                summary.push_str(&ctx.summary());
            }
        }

        summary.push_str("\n## All Symbols\n\n");
        for (symbol, ctx) in contexts {
            summary.push_str(&format!(
                "- **{}**: {:?} | Sentiment {:.2} | Confidence {:.0}%\n",
                symbol,
                ctx.consensus_action,
                ctx.merged_sentiment,
                ctx.merged_confidence * dec!(100)
            ));
        }

        summary
    }
}

/// Context file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualBrainContextFile {
    pub timestamp: chrono::DateTime<Utc>,
    pub mode: TradingMode,
    pub best_opportunity: Option<String>,
    pub contexts: HashMap<String, MergedContext>,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_symbol() {
        let config = DualBrainConfig::default();
        let aggregator = Arc::new(MarketAggregator::new(100));
        let loop_instance = DualBrainLoop::new(config, aggregator);

        let (token, chain) = loop_instance.parse_symbol("BTC-USDT");
        assert_eq!(token, "BTC");
        assert_eq!(chain, "ethereum");

        let (token, chain) = loop_instance.parse_symbol("SOL-USDT");
        assert_eq!(token, "SOL");
        assert_eq!(chain, "solana");
    }

    #[test]
    fn test_regime_detection() {
        let config = DualBrainConfig::default();
        let aggregator = Arc::new(MarketAggregator::new(100));
        let loop_instance = DualBrainLoop::new(config, aggregator);

        // Trending up prices
        let prices: Vec<Decimal> = (0..50).map(|i| dec!(60000) + Decimal::from(i * 100)).collect();
        let regime = loop_instance.detect_regime(&prices);
        assert!(matches!(regime, Regime::TrendingUp | Regime::MeanReverting));

        // Stable prices
        let prices: Vec<Decimal> = (0..50).map(|_| dec!(60000)).collect();
        let regime = loop_instance.detect_regime(&prices);
        assert_eq!(regime, Regime::LowVolatility);
    }
}
