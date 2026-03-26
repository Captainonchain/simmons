//! Simmons Exec - Execution layer
//!
//! Smart order routing, MEV protection, paper and live trading.

pub mod cod3x_exec;
pub mod engine;
pub mod gas;
pub mod live;
pub mod mev;
pub mod okx_api;
pub mod paper;
pub mod router;
pub mod signer;
pub mod xlayer_executor;

pub use cod3x_exec::Cod3xExecutor;
pub use engine::ExecutionEngine;
pub use gas::GasOptimizer;
pub use live::{LiveExecutor, LiveExecutorConfig, Venue};
pub use okx_api::{OkxApiClient, OkxApiConfig};
pub use paper::PaperTrader;
pub use router::SmartOrderRouter;
pub use signer::{SignerConfig, TxSigner};
pub use xlayer_executor::{XLayerExecutor, ExecutionResult, SwapQuote, xlayer_tokens};
