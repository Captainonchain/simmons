//! Simmons Core - Shared types, configuration, and error handling
//!
//! This crate provides the foundational types used across all Simmons components.

pub mod config;
pub mod error;
pub mod types;

pub use config::Config;
pub use error::{Error, Result};
pub use types::*;
