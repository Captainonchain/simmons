//! Simmons Brain - Claude integration layer
//!
//! Provides file-based IPC between Rust engine and Claude Code skill.
//! Also includes learning engine and feedback loop for continuous improvement.

pub mod bridge;
pub mod feedback;
pub mod learning;
pub mod types;

pub use bridge::BrainBridge;
pub use feedback::FeedbackLoop;
pub use learning::LearningEngine;
pub use types::*;
