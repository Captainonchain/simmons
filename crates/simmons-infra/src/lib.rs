//! Simmons Infra - OKX OnchainOS and X Layer infrastructure
//!
//! - OnchainOS: DEX aggregation (500+ DEXs), market data, swap execution
//! - X Layer: ZK-EVM bridge, DEX integration
//! - Cod3x: Lending protocol

pub mod bridge;
pub mod cod3x;
pub mod dex;
pub mod onchainos;
pub mod xlayer;

pub use bridge::OkxBridge;
pub use cod3x::Cod3xClient;
pub use dex::DexClient;
pub use onchainos::{OnchainOSClient, OnchainOSConfig};
pub use xlayer::XLayerClient;
