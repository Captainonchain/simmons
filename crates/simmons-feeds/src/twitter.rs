//! Twitter/X API Integration
//!
//! Provides sentiment analysis and KOL tracking via Twitter API v2.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use reqwest::Client;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use tracing::{debug, warn};

/// Twitter feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterConfig {
    /// Tracked accounts (KOLs)
    pub tracked_accounts: Vec<String>,
    /// Tracked tokens/keywords
    pub tracked_tokens: Vec<String>,
    /// Sentiment analysis window in hours
    pub window_hours: u32,
    /// Poll interval in seconds
    pub poll_interval_secs: u64,
    /// Minimum follower count for KOL status
    pub min_kol_followers: u64,
}

impl Default for TwitterConfig {
    fn default() -> Self {
        Self {
            tracked_accounts: vec![
                "solana".to_string(),
                "base".to_string(),
                "coinbase".to_string(),
                "binance".to_string(),
                "ethereum".to_string(),
                "aaboronin".to_string(),
                "DegenSpartan".to_string(),
                "CryptoCobain".to_string(),
            ],
            tracked_tokens: vec![
                "BTC".to_string(),
                "ETH".to_string(),
                "SOL".to_string(),
            ],
            window_hours: 4,
            poll_interval_secs: 60,
            min_kol_followers: 10000,
        }
    }
}

/// Tweet data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tweet {
    /// Tweet ID
    pub id: String,
    /// Author handle
    pub author: String,
    /// Author follower count
    pub followers: u64,
    /// Tweet text
    pub text: String,
    /// Sentiment score (-1 to +1)
    pub sentiment: Decimal,
    /// Mentioned tokens
    pub tokens: Vec<String>,
    /// Is this from a KOL?
    pub is_kol: bool,
    /// Retweet count
    pub retweets: u64,
    /// Like count
    pub likes: u64,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// KOL mention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KolMention {
    /// Account handle
    pub handle: String,
    /// Follower count
    pub followers: u64,
    /// Sentiment of the mention
    pub sentiment: MentionSentiment,
    /// Tweet text
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

impl MentionSentiment {
    pub fn from_score(score: Decimal) -> Self {
        if score > dec!(0.2) {
            MentionSentiment::Positive
        } else if score < dec!(-0.2) {
            MentionSentiment::Negative
        } else {
            MentionSentiment::Neutral
        }
    }
}

/// Aggregated Twitter sentiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterSentiment {
    /// Token symbol
    pub token: String,
    /// Overall sentiment score (-1 to +1)
    pub sentiment_score: Decimal,
    /// Number of mentions
    pub mention_count: u32,
    /// KOL mentions
    pub kol_mentions: Vec<KolMention>,
    /// Trending score (0-100)
    pub trending_score: u8,
    /// Time window analyzed
    pub window_hours: u32,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Twitter feed client
pub struct TwitterFeed {
    /// Configuration
    config: TwitterConfig,
    /// HTTP client
    client: Client,
    /// Bearer token
    bearer_token: Option<String>,
    /// Cache of recent tweets
    tweet_cache: HashMap<String, Vec<Tweet>>,
    /// Last fetch timestamp per token
    last_fetch: HashMap<String, DateTime<Utc>>,
}

impl TwitterFeed {
    /// Create new Twitter feed
    pub fn new(config: TwitterConfig) -> Self {
        let bearer_token = env::var("TWITTER_BEARER_TOKEN").ok();

        if bearer_token.is_none() {
            warn!("TWITTER_BEARER_TOKEN not set - Twitter feed will use mock data");
        }

        Self {
            config,
            client: Client::new(),
            bearer_token,
            tweet_cache: HashMap::new(),
            last_fetch: HashMap::new(),
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(TwitterConfig::default())
    }

    /// Check if API is available
    pub fn is_available(&self) -> bool {
        self.bearer_token.is_some()
    }

    /// Fetch recent tweets for a token
    pub async fn fetch_tweets(&mut self, token: &str) -> Result<Vec<Tweet>> {
        let key = token.to_lowercase();

        // Check cache freshness
        if let Some(last) = self.last_fetch.get(&key) {
            let elapsed = Utc::now() - *last;
            if elapsed < Duration::seconds(self.config.poll_interval_secs as i64) {
                if let Some(cached) = self.tweet_cache.get(&key) {
                    return Ok(cached.clone());
                }
            }
        }

        // Fetch from API or generate mock
        let tweets = if let Some(ref bearer) = self.bearer_token {
            self.fetch_from_api(token, bearer).await?
        } else {
            self.generate_mock_tweets(token)
        };

        // Update cache
        self.tweet_cache.insert(key.clone(), tweets.clone());
        self.last_fetch.insert(key, Utc::now());

        Ok(tweets)
    }

    /// Fetch tweets from Twitter API v2
    async fn fetch_from_api(&self, token: &str, bearer: &str) -> Result<Vec<Tweet>> {
        let query = format!("{} OR ${} -is:retweet lang:en", token, token);
        let url = format!(
            "https://api.twitter.com/2/tweets/search/recent?query={}&max_results=100&tweet.fields=created_at,public_metrics,author_id&expansions=author_id&user.fields=username,public_metrics",
            percent_encode(query.as_bytes(), NON_ALPHANUMERIC)
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", bearer))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            debug!("Twitter API error {}: {}", status, text);
            return Ok(self.generate_mock_tweets(token));
        }

        let data: TwitterApiResponse = response.json().await?;

        // Build author lookup
        let mut authors: HashMap<String, TwitterUser> = HashMap::new();
        if let Some(includes) = &data.includes {
            if let Some(users) = &includes.users {
                for user in users {
                    authors.insert(user.id.clone(), user.clone());
                }
            }
        }

        let mut tweets = Vec::new();
        if let Some(data_tweets) = data.data {
            for tweet in data_tweets {
                let author = authors.get(&tweet.author_id);
                let username = author.map_or("unknown".to_string(), |a| a.username.clone());
                let followers = author.map_or(0, |a| {
                    a.public_metrics.as_ref().map_or(0, |m| m.followers_count)
                });

                let sentiment = self.analyze_sentiment(&tweet.text);
                let is_kol = followers >= self.config.min_kol_followers;

                let public_metrics = tweet.public_metrics.unwrap_or_default();

                let tokens = self.extract_tokens(&tweet.text);
                tweets.push(Tweet {
                    id: tweet.id,
                    author: username,
                    followers,
                    text: tweet.text,
                    sentiment,
                    tokens,
                    is_kol,
                    retweets: public_metrics.retweet_count,
                    likes: public_metrics.like_count,
                    created_at: tweet.created_at.parse().unwrap_or_else(|_| Utc::now()),
                });
            }
        }

        Ok(tweets)
    }

    /// Generate mock tweets for testing
    fn generate_mock_tweets(&self, token: &str) -> Vec<Tweet> {
        let now = Utc::now();
        let sentiments = [
            (dec!(0.7), "Bullish on $TOKEN! Breaking out soon"),
            (dec!(0.5), "$TOKEN looking good for a long entry"),
            (dec!(0.3), "Added more $TOKEN to my portfolio"),
            (dec!(0.0), "$TOKEN consolidating around support"),
            (dec!(-0.3), "$TOKEN might retest lower levels"),
            (dec!(-0.5), "Taking profits on $TOKEN here"),
        ];

        sentiments
            .iter()
            .enumerate()
            .map(|(i, (sentiment, template))| {
                let text = template.replace("$TOKEN", &format!("${}", token));
                Tweet {
                    id: format!("mock_{}_{}_{}", token, i, now.timestamp()),
                    author: format!("mock_user_{}", i),
                    followers: 10000 + (i as u64 * 5000),
                    text,
                    sentiment: *sentiment,
                    tokens: vec![token.to_uppercase()],
                    is_kol: i % 2 == 0,
                    retweets: (i * 50) as u64,
                    likes: (i * 200) as u64,
                    created_at: now - Duration::hours(i as i64),
                }
            })
            .collect()
    }

    /// Analyze sentiment of text (simple keyword-based)
    fn analyze_sentiment(&self, text: &str) -> Decimal {
        let text_lower = text.to_lowercase();

        let bullish_words = [
            "bullish", "moon", "pump", "buy", "long", "breakout", "ath",
            "accumulate", "hodl", "bullrun", "gem", "rocket", "surge",
        ];
        let bearish_words = [
            "bearish", "dump", "sell", "short", "crash", "down", "rekt",
            "scam", "rug", "dead", "avoid", "warning", "dump",
        ];

        let mut score = Decimal::ZERO;

        for word in &bullish_words {
            if text_lower.contains(word) {
                score += dec!(0.15);
            }
        }

        for word in &bearish_words {
            if text_lower.contains(word) {
                score -= dec!(0.15);
            }
        }

        // Clamp to -1 to 1
        score.min(Decimal::ONE).max(dec!(-1))
    }

    /// Extract token mentions from text
    fn extract_tokens(&self, text: &str) -> Vec<String> {
        let mut tokens = Vec::new();

        // Find $TICKER patterns
        for word in text.split_whitespace() {
            if word.starts_with('$') && word.len() > 1 {
                let ticker = word[1..].trim_end_matches(|c: char| !c.is_alphanumeric());
                if ticker.len() >= 2 && ticker.len() <= 10 {
                    tokens.push(ticker.to_uppercase());
                }
            }
        }

        // Check for known tokens without $ prefix
        let text_upper = text.to_uppercase();
        for tracked in &self.config.tracked_tokens {
            if text_upper.contains(tracked) && !tokens.contains(tracked) {
                tokens.push(tracked.clone());
            }
        }

        tokens
    }

    /// Get aggregated sentiment for a token
    pub async fn get_sentiment(&mut self, token: &str) -> Result<TwitterSentiment> {
        let tweets = self.fetch_tweets(token).await?;

        let cutoff = Utc::now() - Duration::hours(self.config.window_hours as i64);
        let recent_tweets: Vec<&Tweet> = tweets
            .iter()
            .filter(|t| t.created_at > cutoff)
            .collect();

        if recent_tweets.is_empty() {
            return Ok(TwitterSentiment {
                token: token.to_uppercase(),
                sentiment_score: Decimal::ZERO,
                mention_count: 0,
                kol_mentions: vec![],
                trending_score: 0,
                window_hours: self.config.window_hours,
                timestamp: Utc::now(),
            });
        }

        // Calculate weighted sentiment (KOLs and engagement weighted higher)
        let mut weighted_sum = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for tweet in &recent_tweets {
            let weight = if tweet.is_kol {
                dec!(2.0)
            } else {
                Decimal::ONE
            };

            // Add engagement weight
            let engagement_weight = Decimal::from(1 + tweet.likes / 1000 + tweet.retweets / 500);
            let final_weight = weight * engagement_weight;

            weighted_sum += tweet.sentiment * final_weight;
            total_weight += final_weight;
        }

        let sentiment_score = if total_weight.is_zero() {
            Decimal::ZERO
        } else {
            (weighted_sum / total_weight).min(Decimal::ONE).max(dec!(-1))
        };

        // Extract KOL mentions
        let kol_mentions: Vec<KolMention> = recent_tweets
            .iter()
            .filter(|t| t.is_kol)
            .map(|t| KolMention {
                handle: t.author.clone(),
                followers: t.followers,
                sentiment: MentionSentiment::from_score(t.sentiment),
                text: t.text.clone(),
                timestamp: t.created_at,
            })
            .collect();

        // Calculate trending score
        let mention_count = recent_tweets.len() as u32;
        let kol_count = kol_mentions.len();
        let trending_score = self.calculate_trending_score(mention_count, kol_count, &recent_tweets);

        Ok(TwitterSentiment {
            token: token.to_uppercase(),
            sentiment_score,
            mention_count,
            kol_mentions,
            trending_score,
            window_hours: self.config.window_hours,
            timestamp: Utc::now(),
        })
    }

    /// Calculate trending score (0-100)
    fn calculate_trending_score(&self, mentions: u32, kol_count: usize, tweets: &[&Tweet]) -> u8 {
        let mut score: u32 = 0;

        // Mention volume contribution (0-40)
        score += (mentions * 2).min(40);

        // KOL involvement contribution (0-30)
        score += (kol_count as u32 * 10).min(30);

        // Engagement contribution (0-30)
        let total_engagement: u64 = tweets.iter().map(|t| t.likes + t.retweets * 2).sum();
        score += ((total_engagement / 1000) as u32).min(30);

        score.min(100) as u8
    }

    /// Get mentions for multiple tokens
    pub async fn get_mentions_batch(&mut self, tokens: &[&str]) -> Result<HashMap<String, TwitterSentiment>> {
        let mut results = HashMap::new();

        for token in tokens {
            match self.get_sentiment(token).await {
                Ok(sentiment) => {
                    results.insert(token.to_uppercase(), sentiment);
                }
                Err(e) => {
                    debug!("Failed to get Twitter sentiment for {}: {}", token, e);
                }
            }
        }

        Ok(results)
    }

    /// Get trending tokens from tracked accounts
    pub async fn get_trending_tokens(&mut self) -> Result<Vec<TrendingToken>> {
        let mut token_mentions: HashMap<String, (u32, Decimal)> = HashMap::new();

        // Fetch tweets from tracked accounts
        for token in &self.config.tracked_tokens.clone() {
            let tweets = self.fetch_tweets(token).await?;

            for tweet in tweets {
                for mentioned in tweet.tokens {
                    let entry = token_mentions.entry(mentioned).or_insert((0, Decimal::ZERO));
                    entry.0 += 1;
                    entry.1 += tweet.sentiment;
                }
            }
        }

        let mut trending: Vec<TrendingToken> = token_mentions
            .into_iter()
            .filter(|(_, (count, _))| *count >= 3)
            .map(|(token, (count, sentiment_sum))| {
                let avg_sentiment = sentiment_sum / Decimal::from(count);
                TrendingToken {
                    token,
                    mentions: count,
                    sentiment: avg_sentiment,
                    score: (count as u8 * 10).min(100),
                }
            })
            .collect();

        trending.sort_by(|a, b| b.mentions.cmp(&a.mentions));
        Ok(trending.into_iter().take(20).collect())
    }

    /// Get config
    pub fn config(&self) -> &TwitterConfig {
        &self.config
    }
}

/// Trending token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingToken {
    /// Token symbol
    pub token: String,
    /// Number of mentions
    pub mentions: u32,
    /// Average sentiment
    pub sentiment: Decimal,
    /// Trending score (0-100)
    pub score: u8,
}

// Twitter API response types
#[derive(Debug, Deserialize)]
struct TwitterApiResponse {
    data: Option<Vec<TwitterTweet>>,
    includes: Option<TwitterIncludes>,
}

#[derive(Debug, Deserialize)]
struct TwitterTweet {
    id: String,
    text: String,
    author_id: String,
    created_at: String,
    public_metrics: Option<TweetMetrics>,
}

#[derive(Debug, Deserialize, Default)]
struct TweetMetrics {
    retweet_count: u64,
    like_count: u64,
}

#[derive(Debug, Deserialize)]
struct TwitterIncludes {
    users: Option<Vec<TwitterUser>>,
}

#[derive(Debug, Clone, Deserialize)]
struct TwitterUser {
    id: String,
    username: String,
    public_metrics: Option<UserMetrics>,
}

#[derive(Debug, Clone, Deserialize)]
struct UserMetrics {
    followers_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentiment_analysis() {
        let feed = TwitterFeed::with_defaults();

        let bullish = feed.analyze_sentiment("Super bullish on BTC! Moon soon!");
        assert!(bullish > dec!(0.2));

        let bearish = feed.analyze_sentiment("BTC dump incoming, sell everything");
        assert!(bearish < dec!(-0.2));

        let neutral = feed.analyze_sentiment("BTC is trading at 67000");
        assert!(neutral.abs() < dec!(0.2));
    }

    #[test]
    fn test_extract_tokens() {
        let feed = TwitterFeed::with_defaults();

        let tokens = feed.extract_tokens("Buying $BTC and $ETH today! SOL also looking good");
        assert!(tokens.contains(&"BTC".to_string()));
        assert!(tokens.contains(&"ETH".to_string()));
        assert!(tokens.contains(&"SOL".to_string()));
    }

    #[test]
    fn test_mock_tweets() {
        let feed = TwitterFeed::with_defaults();
        let tweets = feed.generate_mock_tweets("BTC");

        assert!(!tweets.is_empty());
        assert!(tweets.iter().any(|t| t.is_kol));
    }

    #[test]
    fn test_mention_sentiment() {
        assert_eq!(MentionSentiment::from_score(dec!(0.5)), MentionSentiment::Positive);
        assert_eq!(MentionSentiment::from_score(dec!(0.0)), MentionSentiment::Neutral);
        assert_eq!(MentionSentiment::from_score(dec!(-0.5)), MentionSentiment::Negative);
    }
}
