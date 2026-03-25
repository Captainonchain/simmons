//! Simmons Brain - Claude integration layer
//!
//! Provides file-based IPC between Rust engine and Claude Code skill.
//! Also includes learning engine and feedback loop for continuous improvement.

pub mod bridge;
pub mod feedback;
pub mod learning;
pub mod types;

pub use bridge::BrainBridge;
pub use feedback::{FeedbackLoop, PerformanceReport, StrategyBreakdown, HealthReport, StrategyHealth, StrategyHealthEntry};
pub use learning::{LearningEngine, MarketConditions, LearningInsights, PatternStats, StrategyStats};
pub use types::*;
