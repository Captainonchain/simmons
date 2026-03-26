//! Fundamental Brain - News, Whale Activity, and Social Sentiment
//!
//! Aggregates signals from multiple fundamental data sources:
//! - OnchainOS smart money signals
//! - Twitter/X sentiment and KOL tracking
//! - News RSS feeds (CoinDesk, The Block, Decrypt)
//! - Security scanning

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Whale activity signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhaleSignal {
    /// Whale address
    pub address: String,
    /// Action type
    pub action: WhaleAction,
    /// Token symbol
    pub token: String,
    /// Chain
    pub chain: String,
    /// USD value of transaction
    pub value_usd: Decimal,
    /// Is this a known smart money wallet?
    pub is_smart_money: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Whale action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhaleAction {
    Buy,
    Sell,
    Transfer,
    Mint,
    Burn,
    AddLiquidity,
    RemoveLiquidity,
}

impl WhaleAction {
    /// Is this a bullish action?
    pub fn is_bullish(&self) -> bool {
        matches!(self, Self::Buy | Self::AddLiquidity | Self::Mint)
    }

    /// Is this a bearish action?
    pub fn is_bearish(&self) -> bool {
        matches!(self, Self::Sell | Self::RemoveLiquidity | Self::Burn)
    }
}

/// Twitter/X sentiment data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterSentiment {
    /// Token symbol
    pub token: String,
    /// Overall sentiment score (-1 to +1)
    pub sentiment_score: Decimal,
    /// Number of mentions
    pub mention_count: u32,
    /// KOL mentions (influential accounts)
    pub kol_mentions: Vec<KolMention>,
    /// Trending score (0-100)
    pub trending_score: u8,
    /// Time window analyzed (hours)
    pub window_hours: u32,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// KOL (Key Opinion Leader) mention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KolMention {
    /// Account handle
    pub handle: String,
    /// Follower count
    pub followers: u64,
    /// Sentiment of the mention
    pub sentiment: MentionSentiment,
    /// Tweet text (truncated)
    pub text: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Mention sentiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MentionSentiment {
    Positive,
    Neutral,
    Negative,
}

/// News headline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsHeadline {
    /// Headline title
    pub title: String,
    /// Source (CoinDesk, The Block, etc.)
    pub source: String,
    /// URL
    pub url: String,
    /// Sentiment score (-1 to +1)
    pub sentiment: Decimal,
    /// Relevance score (0-1)
    pub relevance: Decimal,
    /// Mentioned tokens
    pub tokens: Vec<String>,
    /// Published timestamp
    pub published_at: DateTime<Utc>,
}

/// News sentiment aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsSentiment {
    /// Token symbol
    pub token: String,
    /// Overall sentiment (-1 to +1)
    pub sentiment_score: Decimal,
    /// Number of relevant headlines
    pub headline_count: u32,
    /// Recent headlines
    pub headlines: Vec<NewsHeadline>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Security assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAssessment {
    /// Token address
    pub token: String,
    /// Chain
    pub chain: String,
    /// Is honeypot
    pub is_honeypot: bool,
    /// Buy tax percentage
    pub buy_tax: Option<Decimal>,
    /// Sell tax percentage
    pub sell_tax: Option<Decimal>,
    /// Can owner take back ownership
    pub can_take_ownership: bool,
    /// Can owner change balance
    pub can_change_balance: bool,
    /// Is mintable
    pub is_mintable: bool,
    /// Liquidity USD
    pub liquidity_usd: Option<Decimal>,
    /// Risk score (0-100, higher = riskier)
    pub risk_score: u8,
    /// Red flags
    pub red_flags: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl SecurityAssessment {
    /// Should block trading?
    pub fn should_block(&self) -> bool {
        self.is_honeypot
            || self.can_take_ownership
            || self.can_change_balance
            || self.buy_tax.map_or(false, |t| t > dec!(20))
            || self.sell_tax.map_or(false, |t| t > dec!(20))
            || self.risk_score >= 80
    }

    /// Should warn but allow?
    pub fn should_warn(&self) -> bool {
        !self.should_block()
            && (self.buy_tax.map_or(false, |t| t > dec!(5))
                || self.sell_tax.map_or(false, |t| t > dec!(5))
                || self.is_mintable
                || self.risk_score >= 50)
    }

    /// Is safe to trade?
    pub fn is_safe(&self) -> bool {
        !self.should_block() && !self.should_warn()
    }
}

/// Fundamental Brain output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundBrainOutput {
    /// Token/symbol analyzed
    pub symbol: String,
    /// Chain
    pub chain: String,
    /// Whale sentiment (-1 to +1)
    pub whale_sentiment: Decimal,
    /// Whale signals
    pub whale_signals: Vec<WhaleSignal>,
    /// Twitter sentiment (-1 to +1)
    pub twitter_sentiment: Decimal,
    /// Twitter data
    pub twitter_data: Option<TwitterSentiment>,
    /// News sentiment (-1 to +1)
    pub news_sentiment: Decimal,
    /// News data
    pub news_data: Option<NewsSentiment>,
    /// Security assessment
    pub security: Option<SecurityAssessment>,
    /// Overall fundamental sentiment (-1 to +1)
    pub overall_sentiment: Decimal,
    /// Overall confidence (0-1)
    pub overall_confidence: Decimal,
    /// Recommendation
    pub recommendation: FundRecommendation,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Fundamental brain recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundRecommendation {
    /// Action type
    pub action: FundAction,
    /// Confidence (0-1)
    pub confidence: Decimal,
    /// Size modifier (0-1)
    pub size_modifier: Decimal,
    /// Reasoning
    pub reasoning: String,
    /// Security warnings
    pub security_warnings: Vec<String>,
}

/// Fundamental action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FundAction {
    /// Bullish signal - go long
    Bullish,
    /// Bearish signal - go short or avoid
    Bearish,
    /// Neutral - no strong signal
    Neutral,
    /// Block - security concerns
    Block,
}

/// Source weights for sentiment aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceWeights {
    /// Whale/smart money weight
    pub whale: Decimal,
    /// Twitter/social weight
    pub twitter: Decimal,
    /// News weight
    pub news: Decimal,
}

impl Default for SourceWeights {
    fn default() -> Self {
        Self {
            whale: dec!(0.5),   // 50% - most reliable
            twitter: dec!(0.3), // 30% - social sentiment
            news: dec!(0.2),    // 20% - news sentiment
        }
    }
}

/// Fund Brain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundBrainConfig {
    /// Update interval in seconds
    pub update_interval_secs: u64,
    /// Source weights
    pub weights: SourceWeights,
    /// Minimum whale trade size USD
    pub min_whale_trade_usd: Decimal,
    /// Whale lookback hours
    pub whale_lookback_hours: u32,
    /// Twitter sentiment window hours
    pub twitter_window_hours: u32,
    /// Tracked Twitter accounts
    pub tracked_accounts: Vec<String>,
    /// News RSS feeds
    pub news_feeds: Vec<String>,
    /// Maximum tax before blocking
    pub max_tax_pct: Decimal,
    /// Block honeypots
    pub block_honeypot: bool,
    /// Minimum liquidity USD
    pub min_liquidity_usd: Decimal,
}

impl Default for FundBrainConfig {
    fn default() -> Self {
        Self {
            update_interval_secs: 30,
            weights: SourceWeights::default(),
            min_whale_trade_usd: dec!(100_000),
            whale_lookback_hours: 24,
            twitter_window_hours: 4,
            tracked_accounts: vec![
                "@solana".to_string(),
                "@base".to_string(),
                "@coinbase".to_string(),
                "@binance".to_string(),
                "@ethereum".to_string(),
            ],
            news_feeds: vec![
                "https://www.coindesk.com/arc/outboundfeeds/rss/".to_string(),
                "https://www.theblock.co/rss.xml".to_string(),
                "https://decrypt.co/feed".to_string(),
            ],
            max_tax_pct: dec!(10),
            block_honeypot: true,
            min_liquidity_usd: dec!(50_000),
        }
    }
}

/// Fundamental Analysis Brain
pub struct FundBrain {
    /// Configuration
    config: FundBrainConfig,
    /// Recent whale signals cache
    whale_cache: HashMap<String, Vec<WhaleSignal>>,
    /// Recent Twitter sentiment cache
    twitter_cache: HashMap<String, TwitterSentiment>,
    /// Recent news cache
    news_cache: HashMap<String, NewsSentiment>,
    /// Security cache
    security_cache: HashMap<String, SecurityAssessment>,
}

impl FundBrain {
    /// Create new Fund Brain
    pub fn new(config: FundBrainConfig) -> Self {
        Self {
            config,
            whale_cache: HashMap::new(),
            twitter_cache: HashMap::new(),
            news_cache: HashMap::new(),
            security_cache: HashMap::new(),
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(FundBrainConfig::default())
    }

    /// Update whale signals from external source
    pub fn update_whale_signals(&mut self, token: &str, signals: Vec<WhaleSignal>) {
        let key = token.to_lowercase();

        // Filter to recent signals
        let cutoff = Utc::now() - Duration::hours(self.config.whale_lookback_hours as i64);
        let filtered: Vec<WhaleSignal> = signals
            .into_iter()
            .filter(|s| s.timestamp > cutoff && s.value_usd >= self.config.min_whale_trade_usd)
            .collect();

        self.whale_cache.insert(key, filtered);
    }

    /// Update Twitter sentiment from external source
    pub fn update_twitter_sentiment(&mut self, token: &str, sentiment: TwitterSentiment) {
        self.twitter_cache.insert(token.to_lowercase(), sentiment);
    }

    /// Update news sentiment from external source
    pub fn update_news_sentiment(&mut self, token: &str, sentiment: NewsSentiment) {
        self.news_cache.insert(token.to_lowercase(), sentiment);
    }

    /// Update security assessment from external source
    pub fn update_security(&mut self, token: &str, chain: &str, assessment: SecurityAssessment) {
        let key = format!("{}:{}", chain.to_lowercase(), token.to_lowercase());
        self.security_cache.insert(key, assessment);
    }

    /// Calculate whale sentiment for a token
    pub fn calculate_whale_sentiment(&self, token: &str) -> (Decimal, Vec<WhaleSignal>) {
        let key = token.to_lowercase();
        let signals = self.whale_cache.get(&key).cloned().unwrap_or_default();

        if signals.is_empty() {
            return (Decimal::ZERO, signals);
        }

        // Weighted by value
        let mut bullish_value = Decimal::ZERO;
        let mut bearish_value = Decimal::ZERO;

        for signal in &signals {
            let weight = if signal.is_smart_money {
                dec!(1.5) // Smart money weighted more heavily
            } else {
                Decimal::ONE
            };

            if signal.action.is_bullish() {
                bullish_value += signal.value_usd * weight;
            } else if signal.action.is_bearish() {
                bearish_value += signal.value_usd * weight;
            }
        }

        let total = bullish_value + bearish_value;
        let sentiment = if total.is_zero() {
            Decimal::ZERO
        } else {
            (bullish_value - bearish_value) / total
        };

        (sentiment.min(Decimal::ONE).max(dec!(-1)), signals)
    }

    /// Get Twitter sentiment for a token
    pub fn get_twitter_sentiment(&self, token: &str) -> (Decimal, Option<TwitterSentiment>) {
        let key = token.to_lowercase();
        match self.twitter_cache.get(&key) {
            Some(data) => (data.sentiment_score, Some(data.clone())),
            None => (Decimal::ZERO, None),
        }
    }

    /// Get news sentiment for a token
    pub fn get_news_sentiment(&self, token: &str) -> (Decimal, Option<NewsSentiment>) {
        let key = token.to_lowercase();
        match self.news_cache.get(&key) {
            Some(data) => (data.sentiment_score, Some(data.clone())),
            None => (Decimal::ZERO, None),
        }
    }

    /// Get security assessment
    pub fn get_security(&self, token: &str, chain: &str) -> Option<SecurityAssessment> {
        let key = format!("{}:{}", chain.to_lowercase(), token.to_lowercase());
        self.security_cache.get(&key).cloned()
    }

    /// Calculate overall fundamental sentiment
    pub fn calculate_overall_sentiment(
        &self,
        whale: Decimal,
        twitter: Decimal,
        news: Decimal,
    ) -> Decimal {
        let weights = &self.config.weights;
        let weighted = whale * weights.whale + twitter * weights.twitter + news * weights.news;
        weighted.min(Decimal::ONE).max(dec!(-1))
    }

    /// Calculate confidence based on data availability
    pub fn calculate_confidence(
        &self,
        has_whale: bool,
        has_twitter: bool,
        has_news: bool,
        whale_count: usize,
    ) -> Decimal {
        let mut confidence = dec!(0.3); // Base confidence

        // More sources = higher confidence
        if has_whale {
            confidence += dec!(0.25);
            // More whale signals = higher confidence
            if whale_count >= 5 {
                confidence += dec!(0.1);
            }
        }
        if has_twitter {
            confidence += dec!(0.2);
        }
        if has_news {
            confidence += dec!(0.15);
        }

        confidence.min(Decimal::ONE)
    }

    /// Generate recommendation
    fn generate_recommendation(
        &self,
        sentiment: Decimal,
        confidence: Decimal,
        security: &Option<SecurityAssessment>,
    ) -> FundRecommendation {
        // Check security first
        if let Some(sec) = security {
            if sec.should_block() {
                return FundRecommendation {
                    action: FundAction::Block,
                    confidence: dec!(1.0),
                    size_modifier: Decimal::ZERO,
                    reasoning: format!("Security block: {}", sec.red_flags.join(", ")),
                    security_warnings: sec.red_flags.clone(),
                };
            }
        }

        // Determine action based on sentiment
        let (action, reasoning) = if sentiment > dec!(0.3) {
            (
                FundAction::Bullish,
                format!("Bullish fundamentals: sentiment {:.2}", sentiment),
            )
        } else if sentiment < dec!(-0.3) {
            (
                FundAction::Bearish,
                format!("Bearish fundamentals: sentiment {:.2}", sentiment),
            )
        } else {
            (
                FundAction::Neutral,
                format!("Neutral fundamentals: sentiment {:.2}", sentiment),
            )
        };

        // Size modifier based on confidence
        let size_modifier = confidence;

        // Collect security warnings
        let security_warnings = security
            .as_ref()
            .filter(|s| s.should_warn())
            .map(|s| s.red_flags.clone())
            .unwrap_or_default();

        FundRecommendation {
            action,
            confidence,
            size_modifier,
            reasoning,
            security_warnings,
        }
    }

    /// Analyze and produce full Fund Brain output
    pub fn analyze(&self, symbol: &str, chain: &str) -> FundBrainOutput {
        // Get all sentiment data
        let (whale_sentiment, whale_signals) = self.calculate_whale_sentiment(symbol);
        let (twitter_sentiment, twitter_data) = self.get_twitter_sentiment(symbol);
        let (news_sentiment, news_data) = self.get_news_sentiment(symbol);
        let security = self.get_security(symbol, chain);

        // Calculate overall sentiment
        let overall_sentiment =
            self.calculate_overall_sentiment(whale_sentiment, twitter_sentiment, news_sentiment);

        // Calculate confidence
        let overall_confidence = self.calculate_confidence(
            !whale_signals.is_empty(),
            twitter_data.is_some(),
            news_data.is_some(),
            whale_signals.len(),
        );

        // Generate recommendation
        let recommendation =
            self.generate_recommendation(overall_sentiment, overall_confidence, &security);

        FundBrainOutput {
            symbol: symbol.to_string(),
            chain: chain.to_string(),
            whale_sentiment,
            whale_signals,
            twitter_sentiment,
            twitter_data,
            news_sentiment,
            news_data,
            security,
            overall_sentiment,
            overall_confidence,
            recommendation,
            timestamp: Utc::now(),
        }
    }

    /// Analyze with provided data (for testing or external data)
    pub fn analyze_with_data(
        &self,
        symbol: &str,
        chain: &str,
        whale_signals: Vec<WhaleSignal>,
        twitter: Option<TwitterSentiment>,
        news: Option<NewsSentiment>,
        security: Option<SecurityAssessment>,
    ) -> FundBrainOutput {
        // Calculate whale sentiment
        let (whale_sentiment, filtered_signals) = if whale_signals.is_empty() {
            (Decimal::ZERO, vec![])
        } else {
            let mut bullish_value = Decimal::ZERO;
            let mut bearish_value = Decimal::ZERO;

            for signal in &whale_signals {
                let weight = if signal.is_smart_money { dec!(1.5) } else { Decimal::ONE };
                if signal.action.is_bullish() {
                    bullish_value += signal.value_usd * weight;
                } else if signal.action.is_bearish() {
                    bearish_value += signal.value_usd * weight;
                }
            }

            let total = bullish_value + bearish_value;
            let sentiment = if total.is_zero() {
                Decimal::ZERO
            } else {
                (bullish_value - bearish_value) / total
            };

            (sentiment.min(Decimal::ONE).max(dec!(-1)), whale_signals)
        };

        let twitter_sentiment = twitter.as_ref().map_or(Decimal::ZERO, |t| t.sentiment_score);
        let news_sentiment = news.as_ref().map_or(Decimal::ZERO, |n| n.sentiment_score);

        let overall_sentiment =
            self.calculate_overall_sentiment(whale_sentiment, twitter_sentiment, news_sentiment);

        let overall_confidence = self.calculate_confidence(
            !filtered_signals.is_empty(),
            twitter.is_some(),
            news.is_some(),
            filtered_signals.len(),
        );

        let recommendation =
            self.generate_recommendation(overall_sentiment, overall_confidence, &security);

        FundBrainOutput {
            symbol: symbol.to_string(),
            chain: chain.to_string(),
            whale_sentiment,
            whale_signals: filtered_signals,
            twitter_sentiment,
            twitter_data: twitter,
            news_sentiment,
            news_data: news,
            security,
            overall_sentiment,
            overall_confidence,
            recommendation,
            timestamp: Utc::now(),
        }
    }

    /// Quick check if token should be blocked
    pub fn should_block(&self, token: &str, chain: &str) -> Option<String> {
        if let Some(security) = self.get_security(token, chain) {
            if security.should_block() {
                return Some(format!("Security block: {}", security.red_flags.join(", ")));
            }
        }
        None
    }

    /// Get config
    pub fn config(&self) -> &FundBrainConfig {
        &self.config
    }
}

/// Aggregate multiple Fund Brain outputs
pub fn aggregate_fund_outputs(outputs: &[FundBrainOutput]) -> (Decimal, Decimal) {
    if outputs.is_empty() {
        return (Decimal::ZERO, Decimal::ZERO);
    }

    let sentiment_sum: Decimal = outputs.iter().map(|o| o.overall_sentiment).sum();
    let confidence_sum: Decimal = outputs.iter().map(|o| o.overall_confidence).sum();

    let count = Decimal::from(outputs.len());
    (sentiment_sum / count, confidence_sum / count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_whale_signal(action: WhaleAction, value: Decimal, smart_money: bool) -> WhaleSignal {
        WhaleSignal {
            address: "0x123".to_string(),
            action,
            token: "BTC".to_string(),
            chain: "ethereum".to_string(),
            value_usd: value,
            is_smart_money: smart_money,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_whale_sentiment_bullish() {
        let mut brain = FundBrain::with_defaults();

        let signals = vec![
            make_whale_signal(WhaleAction::Buy, dec!(500_000), true),
            make_whale_signal(WhaleAction::Buy, dec!(200_000), false),
        ];

        brain.update_whale_signals("BTC", signals);
        let (sentiment, _) = brain.calculate_whale_sentiment("BTC");

        assert!(sentiment > Decimal::ZERO);
    }

    #[test]
    fn test_whale_sentiment_bearish() {
        let mut brain = FundBrain::with_defaults();

        let signals = vec![
            make_whale_signal(WhaleAction::Sell, dec!(500_000), true),
            make_whale_signal(WhaleAction::RemoveLiquidity, dec!(200_000), false),
        ];

        brain.update_whale_signals("BTC", signals);
        let (sentiment, _) = brain.calculate_whale_sentiment("BTC");

        assert!(sentiment < Decimal::ZERO);
    }

    #[test]
    fn test_security_block() {
        let assessment = SecurityAssessment {
            token: "SCAM".to_string(),
            chain: "ethereum".to_string(),
            is_honeypot: true,
            buy_tax: Some(dec!(50)),
            sell_tax: Some(dec!(90)),
            can_take_ownership: true,
            can_change_balance: true,
            is_mintable: true,
            liquidity_usd: Some(dec!(1000)),
            risk_score: 95,
            red_flags: vec!["Honeypot detected".to_string(), "High tax".to_string()],
            timestamp: Utc::now(),
        };

        assert!(assessment.should_block());
        assert!(!assessment.is_safe());
    }

    #[test]
    fn test_security_safe() {
        let assessment = SecurityAssessment {
            token: "SAFE".to_string(),
            chain: "ethereum".to_string(),
            is_honeypot: false,
            buy_tax: Some(dec!(1)),
            sell_tax: Some(dec!(1)),
            can_take_ownership: false,
            can_change_balance: false,
            is_mintable: false,
            liquidity_usd: Some(dec!(1_000_000)),
            risk_score: 10,
            red_flags: vec![],
            timestamp: Utc::now(),
        };

        assert!(assessment.is_safe());
        assert!(!assessment.should_block());
        assert!(!assessment.should_warn());
    }

    #[test]
    fn test_overall_sentiment() {
        let brain = FundBrain::with_defaults();

        // All bullish
        let sentiment = brain.calculate_overall_sentiment(dec!(0.8), dec!(0.6), dec!(0.4));
        assert!(sentiment > dec!(0.5));

        // All bearish
        let sentiment = brain.calculate_overall_sentiment(dec!(-0.8), dec!(-0.6), dec!(-0.4));
        assert!(sentiment < dec!(-0.5));

        // Mixed
        let sentiment = brain.calculate_overall_sentiment(dec!(0.5), dec!(-0.3), dec!(0.1));
        // Should be slightly positive due to whale weight
        assert!(sentiment > dec!(-0.1) && sentiment < dec!(0.3));
    }

    #[test]
    fn test_analyze_with_data() {
        let brain = FundBrain::with_defaults();

        let whale_signals = vec![
            make_whale_signal(WhaleAction::Buy, dec!(500_000), true),
        ];

        let twitter = Some(TwitterSentiment {
            token: "BTC".to_string(),
            sentiment_score: dec!(0.6),
            mention_count: 1000,
            kol_mentions: vec![],
            trending_score: 75,
            window_hours: 4,
            timestamp: Utc::now(),
        });

        let output = brain.analyze_with_data("BTC", "ethereum", whale_signals, twitter, None, None);

        assert!(output.overall_sentiment > Decimal::ZERO);
        assert!(output.overall_confidence > dec!(0.4));
        assert_eq!(output.recommendation.action, FundAction::Bullish);
    }
}
