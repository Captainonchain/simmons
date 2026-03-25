//! Simmons MCP Server - Model Context Protocol integration for Claude
//!
//! This crate exposes the Simmons trading engine as an MCP server,
//! allowing Claude to read signals, submit trades, and manage portfolio.

pub mod dex;
pub mod memory;
pub mod server;
pub mod state;
pub mod tools;

pub use dex::{Chain, DexConfig, PreTradeCheck, SecurityScanResult, SwapQuote, SwapResult};
pub use memory::MemorySystem;
pub use server::SimmonsServer;
pub use state::TradingState;
