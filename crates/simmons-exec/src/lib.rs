//! Simmons Exec - Execution layer
//!
//! Smart order routing, MEV protection, paper and live trading.

pub mod cod3x_exec;
pub mod engine;
pub mod gas;
pub mod mev;
pub mod paper;
pub mod router;

pub use cod3x_exec::Cod3xExecutor;
pub use engine::ExecutionEngine;
pub use gas::GasOptimizer;
pub use paper::PaperTrader;
pub use router::SmartOrderRouter;
