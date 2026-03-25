//! Simmons Feeds - Data ingestion layer
//!
//! Handles WebSocket connections to exchanges and aggregates market data.
//! Integrates with OnchainOS for X Layer and sentiment signals.

pub mod aggregator;
pub mod news;
pub mod nunchi;
pub mod okx;
pub mod onchain;
pub mod xlayer;

pub use aggregator::MarketAggregator;
pub use news::{create_integrated_news_feed, NewsFeed, SentimentSnapshot, SignalSentimentFeed};
pub use nunchi::{NunchiConfig, NunchiRecommendation, NunchiScore, NunchiSignals, TradeDecision};
pub use okx::OkxFeed;
pub use onchain::OnchainFeed;
pub use xlayer::{XLayerDataFetcher, XLayerFeed, XLayerFeedConfig};
