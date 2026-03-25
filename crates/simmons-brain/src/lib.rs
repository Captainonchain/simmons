//! Simmons Brain - Claude integration layer
//!
//! Provides file-based IPC between Rust engine and Claude Code skill.
//! Also includes learning engine and feedback loop for continuous improvement.
//!
//! ## Dual Brain Architecture
//!
//! The brain module now includes:
//! - **TA Brain**: Technical analysis using RADAR/PULSE/GUARD and 14 Nunchi strategies
//! - **Fund Brain**: Fundamental analysis from whale signals, Twitter, and news
//! - **Consensus Layer**: Merges both brains with adaptive weighting
//! - **REFLECT**: Self-learning system for continuous improvement

pub mod bridge;
pub mod consensus;
pub mod feedback;
pub mod fund_brain;
pub mod learning;
pub mod reflect;
pub mod ta_brain;
pub mod types;

// Legacy exports
pub use bridge::BrainBridge;
pub use feedback::{FeedbackLoop, HealthReport, PerformanceReport, StrategyBreakdown, StrategyHealth, StrategyHealthEntry};
pub use learning::{LearningEngine, LearningInsights, MarketConditions, PatternStats, StrategyStats};
pub use types::*;

// Dual Brain exports
pub use consensus::{ConsensusAction, ConsensusConfig, ConsensusEngine, MergedContext};
pub use fund_brain::{
    FundAction, FundBrain, FundBrainConfig, FundBrainOutput, FundRecommendation,
    KolMention, MentionSentiment, NewsHeadline, NewsSentiment, SecurityAssessment,
    SourceWeights, TwitterSentiment, WhaleAction, WhaleSignal,
};
pub use reflect::{
    Adjustment, AdjustmentType, Mistake, MistakeConditions, MistakeType,
    PerformanceStats, ReflectSystem, Reflection, TradeOutcome, TradeRecord, TradeSide,
};
pub use ta_brain::{
    GuardConfig, GuardState, PulseDirection, PulseSignal, RadarScore, RadarTier,
    StrategyType, TAAction, TABrain, TABrainConfig, TABrainOutput, TARecommendation,
    TAStrategySignal,
};
