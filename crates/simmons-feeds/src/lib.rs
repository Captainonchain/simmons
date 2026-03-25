//! Simmons Feeds - Data ingestion layer
//!
//! Handles WebSocket connections to exchanges and aggregates market data.
//! Integrates with OnchainOS for X Layer and sentiment signals.
//!
//! ## Data Sources
//!
//! - **OKX**: Real-time price feeds via WebSocket
//! - **OnchainOS**: Smart money signals, security scanning, DEX prices
//! - **Twitter**: KOL tracking and sentiment analysis
//! - **News**: RSS feeds from CoinDesk, The Block, Decrypt
//! - **X Layer**: Cross-chain data

pub mod aggregator;
pub mod news;
pub mod nunchi;
pub mod okx;
pub mod onchain;
pub mod twitter;
pub mod xlayer;

pub use aggregator::MarketAggregator;
pub use news::{create_integrated_news_feed, NewsFeed, SentimentSnapshot, SignalSentimentFeed};
pub use nunchi::{NunchiConfig, NunchiRecommendation, NunchiScore, NunchiSignals, TradeDecision};
pub use okx::OkxFeed;
pub use onchain::{MemeToken, OnchainFeed, OnchainPrice, SecurityResult, SmartMoneySignal, SwapResult, WhaleActivity};
pub use twitter::{KolMention, MentionSentiment, TrendingToken, Tweet, TwitterConfig, TwitterFeed, TwitterSentiment};
pub use xlayer::{XLayerDataFetcher, XLayerFeed, XLayerFeedConfig};
