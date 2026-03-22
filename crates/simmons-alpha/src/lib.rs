//! Simmons Alpha - Signal generation engine
//!
//! Implements trading strategies: momentum, mean reversion, regime detection, arbitrage.
//! Also includes AI-powered alpha discovery and forecasting.

pub mod arbitrage;
pub mod autoresearch;
pub mod engine;
pub mod forecaster;
pub mod mean_reversion;
pub mod momentum;
pub mod patterns;
pub mod regime;

pub use autoresearch::AutoresearchAgent;
pub use engine::AlphaEngine;
pub use forecaster::Forecaster;
pub use patterns::{Pattern, PatternDatabase, PatternMiner};
