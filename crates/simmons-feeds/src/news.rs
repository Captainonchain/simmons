//! News and Social Sentiment Feed
//!
//! Aggregates sentiment from news sources and social media.
//! Integrates with OnchainOS signals for smart money / whale / KOL activity.

use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_infra::OnchainOSCli;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Sentiment source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SentimentSource {
    Twitter,
    Reddit,
    News,
    Telegram,
    Discord,
}

/// Sentiment level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SentimentLevel {
    VeryBullish,
    Bullish,
    Neutral,
    Bearish,
    VeryBearish,
}

impl SentimentLevel {
    pub fn to_score(&self) -> Decimal {
        match self {
            SentimentLevel::VeryBullish => dec!(1.0),
            SentimentLevel::Bullish => dec!(0.5),
            SentimentLevel::Neutral => dec!(0.0),
            SentimentLevel::Bearish => dec!(-0.5),
            SentimentLevel::VeryBearish => dec!(-1.0),
        }
    }

    pub fn from_score(score: Decimal) -> Self {
        match score {
            s if s > dec!(0.6) => SentimentLevel::VeryBullish,
            s if s > dec!(0.2) => SentimentLevel::Bullish,
            s if s < dec!(-0.6) => SentimentLevel::VeryBearish,
            s if s < dec!(-0.2) => SentimentLevel::Bearish,
            _ => SentimentLevel::Neutral,
        }
    }
}

/// News/social sentiment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsFeedConfig {
    /// Source weights
    pub source_weights: HashMap<SentimentSource, Decimal>,
    /// Keywords to track
    pub tracked_keywords: Vec<String>,
    /// Minimum posts to consider
    pub min_sample_size: usize,
    /// Time window for aggregation (seconds)
    pub aggregation_window_secs: u64,
    /// Enable velocity detection
    pub detect_velocity: bool,
}

impl Default for NewsFeedConfig {
    fn default() -> Self {
        let mut source_weights = HashMap::new();
        source_weights.insert(SentimentSource::Twitter, dec!(0.30));
        source_weights.insert(SentimentSource::News, dec!(0.30));
        source_weights.insert(SentimentSource::Reddit, dec!(0.20));
        source_weights.insert(SentimentSource::Telegram, dec!(0.10));
        source_weights.insert(SentimentSource::Discord, dec!(0.10));

        Self {
            source_weights,
            tracked_keywords: vec![
                "bitcoin".to_string(),
                "btc".to_string(),
                "crypto".to_string(),
                "ethereum".to_string(),
                "eth".to_string(),
            ],
            min_sample_size: 10,
            aggregation_window_secs: 3600, // 1 hour
            detect_velocity: true,
        }
    }
}

/// Individual sentiment item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentItem {
    pub source: SentimentSource,
    pub text: String,
    pub sentiment: SentimentLevel,
    pub score: Decimal,
    pub engagement: u64,
    pub author_influence: Decimal,
    pub timestamp: i64,
    pub keywords: Vec<String>,
}

/// Aggregated sentiment snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentSnapshot {
    pub overall_score: Decimal,
    pub overall_level: SentimentLevel,
    pub confidence: Decimal,
    pub by_source: HashMap<SentimentSource, SourceSentiment>,
    pub velocity: SentimentVelocity,
    pub sample_size: usize,
    pub time_range_secs: u64,
    pub timestamp: i64,
}

/// Sentiment from a single source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSentiment {
    pub source: SentimentSource,
    pub score: Decimal,
    pub level: SentimentLevel,
    pub count: usize,
    pub engagement: u64,
}

/// Sentiment change velocity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SentimentVelocity {
    /// Change per hour
    pub change_per_hour: Decimal,
    /// Acceleration (change of change)
    pub acceleration: Decimal,
    /// Is sentiment shifting?
    pub is_shifting: bool,
    /// Direction of shift
    pub shift_direction: ShiftDirection,
}

impl Default for SentimentVelocity {
    fn default() -> Self {
        Self {
            change_per_hour: Decimal::ZERO,
            acceleration: Decimal::ZERO,
            is_shifting: false,
            shift_direction: ShiftDirection::Stable,
        }
    }
}

/// Direction of sentiment shift
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShiftDirection {
    TowardsBullish,
    TowardsBearish,
    Stable,
}

/// News and social feed aggregator
pub struct NewsFeed {
    config: NewsFeedConfig,
    items: Vec<SentimentItem>,
    snapshots: Vec<SentimentSnapshot>,
}

impl NewsFeed {
    pub fn new(config: NewsFeedConfig) -> Self {
        Self {
            config,
            items: Vec::new(),
            snapshots: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(NewsFeedConfig::default())
    }

    /// Add a sentiment item
    pub fn add_item(&mut self, item: SentimentItem) {
        self.items.push(item);
        self.cleanup_old_items();
    }

    /// Remove items outside the aggregation window
    fn cleanup_old_items(&mut self) {
        let cutoff = chrono::Utc::now().timestamp() - (self.config.aggregation_window_secs * 2) as i64;
        self.items.retain(|i| i.timestamp > cutoff);
    }

    /// Aggregate current sentiment
    pub fn aggregate(&self) -> SentimentSnapshot {
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - self.config.aggregation_window_secs as i64;

        let recent_items: Vec<&SentimentItem> = self
            .items
            .iter()
            .filter(|i| i.timestamp > cutoff)
            .collect();

        if recent_items.len() < self.config.min_sample_size {
            return SentimentSnapshot {
                overall_score: Decimal::ZERO,
                overall_level: SentimentLevel::Neutral,
                confidence: Decimal::ZERO,
                by_source: HashMap::new(),
                velocity: SentimentVelocity::default(),
                sample_size: recent_items.len(),
                time_range_secs: self.config.aggregation_window_secs,
                timestamp: now,
            };
        }

        // Aggregate by source
        let mut by_source: HashMap<SentimentSource, SourceSentiment> = HashMap::new();
        let mut source_scores: HashMap<SentimentSource, Vec<Decimal>> = HashMap::new();
        let mut source_engagement: HashMap<SentimentSource, u64> = HashMap::new();

        for item in &recent_items {
            source_scores
                .entry(item.source)
                .or_insert_with(Vec::new)
                .push(item.score * item.author_influence);

            *source_engagement.entry(item.source).or_insert(0) += item.engagement;
        }

        for (source, scores) in source_scores {
            let avg_score = scores.iter().sum::<Decimal>() / Decimal::from(scores.len());
            by_source.insert(
                source,
                SourceSentiment {
                    source,
                    score: avg_score,
                    level: SentimentLevel::from_score(avg_score),
                    count: scores.len(),
                    engagement: source_engagement.get(&source).copied().unwrap_or(0),
                },
            );
        }

        // Calculate weighted overall score
        let mut weighted_sum = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for (source, sentiment) in &by_source {
            let weight = self
                .config
                .source_weights
                .get(source)
                .copied()
                .unwrap_or(dec!(0.1));
            weighted_sum += sentiment.score * weight;
            total_weight += weight;
        }

        let overall_score = if total_weight.is_zero() {
            Decimal::ZERO
        } else {
            weighted_sum / total_weight
        };

        // Calculate confidence based on sample size and agreement
        let confidence = self.calculate_confidence(&by_source, recent_items.len());

        // Calculate velocity
        let velocity = if self.config.detect_velocity {
            self.calculate_velocity()
        } else {
            SentimentVelocity::default()
        };

        SentimentSnapshot {
            overall_score,
            overall_level: SentimentLevel::from_score(overall_score),
            confidence,
            by_source,
            velocity,
            sample_size: recent_items.len(),
            time_range_secs: self.config.aggregation_window_secs,
            timestamp: now,
        }
    }

    /// Calculate confidence in sentiment
    fn calculate_confidence(
        &self,
        by_source: &HashMap<SentimentSource, SourceSentiment>,
        sample_size: usize,
    ) -> Decimal {
        // Factor 1: Sample size (more is better)
        let size_confidence = (Decimal::from(sample_size) / Decimal::from(100))
            .min(Decimal::ONE);

        // Factor 2: Source agreement
        let scores: Vec<Decimal> = by_source.values().map(|s| s.score).collect();
        let agreement_confidence = if scores.len() < 2 {
            dec!(0.5)
        } else {
            let mean = scores.iter().sum::<Decimal>() / Decimal::from(scores.len());
            let variance: Decimal = scores
                .iter()
                .map(|s| (*s - mean).abs())
                .sum::<Decimal>()
                / Decimal::from(scores.len());
            // Lower variance = higher agreement = higher confidence
            (Decimal::ONE - variance.min(Decimal::ONE))
        };

        // Factor 3: Source diversity
        let diversity_confidence = (Decimal::from(by_source.len()) / Decimal::from(5))
            .min(Decimal::ONE);

        // Weighted average of factors
        (size_confidence * dec!(0.3) + agreement_confidence * dec!(0.4) + diversity_confidence * dec!(0.3))
    }

    /// Calculate sentiment velocity
    fn calculate_velocity(&self) -> SentimentVelocity {
        if self.snapshots.len() < 2 {
            return SentimentVelocity::default();
        }

        // Get last two snapshots
        let current = self.snapshots.last().unwrap();
        let previous = &self.snapshots[self.snapshots.len() - 2];

        let time_diff_hours = (current.timestamp - previous.timestamp) as f64 / 3600.0;
        if time_diff_hours < 0.1 {
            return SentimentVelocity::default();
        }

        let score_diff = current.overall_score - previous.overall_score;
        let change_per_hour = score_diff / Decimal::from_f64_retain(time_diff_hours).unwrap_or(Decimal::ONE);

        // Calculate acceleration if we have 3+ snapshots
        let acceleration = if self.snapshots.len() >= 3 {
            let prev_prev = &self.snapshots[self.snapshots.len() - 3];
            let prev_diff = previous.overall_score - prev_prev.overall_score;
            let prev_time_diff = (previous.timestamp - prev_prev.timestamp) as f64 / 3600.0;
            let prev_velocity = if prev_time_diff > 0.1 {
                prev_diff / Decimal::from_f64_retain(prev_time_diff).unwrap_or(Decimal::ONE)
            } else {
                Decimal::ZERO
            };
            change_per_hour - prev_velocity
        } else {
            Decimal::ZERO
        };

        let is_shifting = change_per_hour.abs() > dec!(0.1);
        let shift_direction = if change_per_hour > dec!(0.05) {
            ShiftDirection::TowardsBullish
        } else if change_per_hour < dec!(-0.05) {
            ShiftDirection::TowardsBearish
        } else {
            ShiftDirection::Stable
        };

        SentimentVelocity {
            change_per_hour,
            acceleration,
            is_shifting,
            shift_direction,
        }
    }

    /// Record current snapshot for velocity tracking
    pub fn record_snapshot(&mut self) {
        let snapshot = self.aggregate();
        self.snapshots.push(snapshot);

        // Keep only last 24 snapshots
        if self.snapshots.len() > 24 {
            self.snapshots.remove(0);
        }
    }

    /// Get sentiment for specific keyword
    pub fn get_keyword_sentiment(&self, keyword: &str) -> Option<Decimal> {
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - self.config.aggregation_window_secs as i64;

        let matching: Vec<&SentimentItem> = self
            .items
            .iter()
            .filter(|i| i.timestamp > cutoff && i.keywords.iter().any(|k| k.eq_ignore_ascii_case(keyword)))
            .collect();

        if matching.is_empty() {
            return None;
        }

        let total: Decimal = matching.iter().map(|i| i.score).sum();
        Some(total / Decimal::from(matching.len()))
    }

    /// Detect sentiment extremes (potential reversal signals)
    pub fn detect_extremes(&self) -> Option<SentimentExtreme> {
        let snapshot = self.aggregate();

        if snapshot.confidence < dec!(0.5) {
            return None;
        }

        if snapshot.overall_score > dec!(0.7) {
            return Some(SentimentExtreme {
                extreme_type: ExtremeType::ExcessiveBullish,
                score: snapshot.overall_score,
                warning: "Extreme bullishness may indicate overbought conditions".to_string(),
            });
        }

        if snapshot.overall_score < dec!(-0.7) {
            return Some(SentimentExtreme {
                extreme_type: ExtremeType::ExcessiveBearish,
                score: snapshot.overall_score,
                warning: "Extreme bearishness may indicate oversold conditions".to_string(),
            });
        }

        None
    }
}

/// Sentiment extreme detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentExtreme {
    pub extreme_type: ExtremeType,
    pub score: Decimal,
    pub warning: String,
}

/// Type of extreme
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtremeType {
    ExcessiveBullish,
    ExcessiveBearish,
}

/// Signal-based sentiment source using OnchainOS
pub struct SignalSentimentFeed {
    cli: OnchainOSCli,
    news_feed: Arc<RwLock<NewsFeed>>,
    chains: Vec<String>,
    poll_interval_ms: u64,
}

impl SignalSentimentFeed {
    pub fn new(news_feed: Arc<RwLock<NewsFeed>>, chains: Vec<String>) -> Self {
        Self {
            cli: OnchainOSCli::new(),
            news_feed,
            chains,
            poll_interval_ms: 30000, // 30 seconds
        }
    }

    /// Start background signal polling
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }

    /// Run the signal fetcher loop
    async fn run(&self) {
        let mut poll_timer = interval(Duration::from_millis(self.poll_interval_ms));

        info!(
            "Signal sentiment feed started for chains: {:?}",
            self.chains
        );

        loop {
            poll_timer.tick().await;

            for chain in &self.chains {
                if let Err(e) = self.fetch_signals(chain).await {
                    debug!("Failed to fetch signals for {}: {}", chain, e);
                }
            }

            // Record snapshot for velocity tracking
            let mut feed = self.news_feed.write().await;
            feed.record_snapshot();
        }
    }

    /// Fetch signals from OnchainOS and convert to sentiment items
    async fn fetch_signals(&self, chain: &str) -> Result<()> {
        let signals = self.cli.get_signals(Some(chain)).await?;

        let mut feed = self.news_feed.write().await;

        let count = signals.len();
        for signal in signals {
            let sentiment_item = self.signal_to_sentiment(&signal);
            feed.add_item(sentiment_item);
        }

        if count > 0 {
            info!("[Signals] {} new signals from {}", count, chain);
        }

        Ok(())
    }

    /// Convert OnchainOS signal to sentiment item
    fn signal_to_sentiment(&self, signal: &simmons_infra::onchainos_cli::Signal) -> SentimentItem {
        // Determine sentiment from action
        let (sentiment, score) = match signal.action.as_str() {
            "buy" => (SentimentLevel::Bullish, dec!(0.7)),
            "sell" => (SentimentLevel::Bearish, dec!(-0.7)),
            _ => (SentimentLevel::Neutral, dec!(0.0)),
        };

        // Determine source from signal type
        let source = match signal.signal_type.as_str() {
            "smart_money" => SentimentSource::Twitter, // Use Twitter as proxy for smart money
            "whale" => SentimentSource::News,           // Use News as proxy for whale activity
            "kol" => SentimentSource::Discord,          // Use Discord as proxy for KOL
            _ => SentimentSource::Telegram,
        };

        // Estimate engagement from amount
        let engagement = signal
            .amount_usd
            .as_ref()
            .and_then(|a| a.parse::<f64>().ok())
            .map(|a| (a / 1000.0) as u64) // Scale down for engagement metric
            .unwrap_or(100);

        // Influence based on signal type
        let author_influence = match signal.signal_type.as_str() {
            "smart_money" => dec!(1.5), // High influence
            "whale" => dec!(1.3),
            "kol" => dec!(1.2),
            _ => dec!(1.0),
        };

        let keywords = signal
            .token_symbol
            .as_ref()
            .map(|s| vec![s.to_lowercase()])
            .unwrap_or_default();

        SentimentItem {
            source,
            text: format!(
                "{} {} {} {}",
                signal.signal_type,
                signal.action,
                signal.token_symbol.as_deref().unwrap_or("unknown"),
                signal.amount_usd.as_deref().unwrap_or("")
            ),
            sentiment,
            score,
            engagement,
            author_influence,
            timestamp: signal.timestamp as i64,
            keywords,
        }
    }
}

/// Create a full news/sentiment feed with OnchainOS signal integration
pub fn create_integrated_news_feed(chains: Vec<String>) -> (Arc<RwLock<NewsFeed>>, tokio::task::JoinHandle<()>) {
    let news_feed = Arc::new(RwLock::new(NewsFeed::with_defaults()));
    let signal_feed = SignalSentimentFeed::new(news_feed.clone(), chains);
    let handle = signal_feed.start();
    (news_feed, handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(source: SentimentSource, score: Decimal, engagement: u64) -> SentimentItem {
        SentimentItem {
            source,
            text: "test".to_string(),
            sentiment: SentimentLevel::from_score(score),
            score,
            engagement,
            author_influence: Decimal::ONE,
            timestamp: chrono::Utc::now().timestamp(),
            keywords: vec!["bitcoin".to_string()],
        }
    }

    #[test]
    fn test_aggregate_single_source() {
        let mut feed = NewsFeed::with_defaults();

        for i in 0..20 {
            feed.add_item(make_item(SentimentSource::Twitter, dec!(0.6), 100));
        }

        let snapshot = feed.aggregate();
        assert!(snapshot.overall_score > dec!(0.3));
        assert_eq!(snapshot.overall_level, SentimentLevel::Bullish);
    }

    #[test]
    fn test_aggregate_multi_source() {
        let mut feed = NewsFeed::with_defaults();

        for _ in 0..10 {
            feed.add_item(make_item(SentimentSource::Twitter, dec!(0.5), 100));
            feed.add_item(make_item(SentimentSource::News, dec!(0.7), 50));
            feed.add_item(make_item(SentimentSource::Reddit, dec!(0.3), 200));
        }

        let snapshot = feed.aggregate();
        assert!(snapshot.overall_score > dec!(0.3));
        assert_eq!(snapshot.by_source.len(), 3);
    }

    #[test]
    fn test_low_sample_size() {
        let mut feed = NewsFeed::with_defaults();

        // Only 5 items, below minimum
        for _ in 0..5 {
            feed.add_item(make_item(SentimentSource::Twitter, dec!(0.8), 100));
        }

        let snapshot = feed.aggregate();
        assert_eq!(snapshot.confidence, Decimal::ZERO);
    }

    #[test]
    fn test_sentiment_level_conversion() {
        assert_eq!(SentimentLevel::from_score(dec!(0.8)), SentimentLevel::VeryBullish);
        assert_eq!(SentimentLevel::from_score(dec!(0.4)), SentimentLevel::Bullish);
        assert_eq!(SentimentLevel::from_score(dec!(0.0)), SentimentLevel::Neutral);
        assert_eq!(SentimentLevel::from_score(dec!(-0.4)), SentimentLevel::Bearish);
        assert_eq!(SentimentLevel::from_score(dec!(-0.8)), SentimentLevel::VeryBearish);
    }

    #[test]
    fn test_detect_extremes() {
        let mut feed = NewsFeed::with_defaults();

        // Add very bullish items
        for _ in 0..50 {
            feed.add_item(make_item(SentimentSource::Twitter, dec!(0.9), 100));
            feed.add_item(make_item(SentimentSource::News, dec!(0.85), 50));
        }

        let extreme = feed.detect_extremes();
        assert!(extreme.is_some());
        assert_eq!(extreme.unwrap().extreme_type, ExtremeType::ExcessiveBullish);
    }

    #[test]
    fn test_keyword_sentiment() {
        let mut feed = NewsFeed::with_defaults();

        let mut item = make_item(SentimentSource::Twitter, dec!(0.7), 100);
        item.keywords = vec!["bitcoin".to_string(), "btc".to_string()];
        feed.add_item(item);

        let btc_sentiment = feed.get_keyword_sentiment("bitcoin");
        assert!(btc_sentiment.is_some());
        assert_eq!(btc_sentiment.unwrap(), dec!(0.7));

        let eth_sentiment = feed.get_keyword_sentiment("ethereum");
        assert!(eth_sentiment.is_none());
    }
}
