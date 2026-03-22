//! Simmons Feeds - Data ingestion layer
//!
//! Handles WebSocket connections to exchanges and aggregates market data.

pub mod aggregator;
pub mod news;
pub mod nunchi;
pub mod okx;
pub mod onchain;
pub mod xlayer;

pub use aggregator::MarketAggregator;
pub use news::NewsFeed;
pub use nunchi::NunchiSignals;
pub use okx::OkxFeed;
pub use onchain::OnchainFeed;
pub use xlayer::XLayerFeed;
