//! File-based IPC bridge between Rust and Claude

use crate::types::{BrainDecision, BrainInput, BrainState};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Bridge for communication with Claude brain
pub struct BrainBridge {
    data_dir: PathBuf,
    timeout: Duration,
    auto_invoke: bool,
}

impl BrainBridge {
    /// Create a new brain bridge
    pub fn new<P: AsRef<Path>>(data_dir: P, timeout_secs: u64, auto_invoke: bool) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            timeout: Duration::from_secs(timeout_secs),
            auto_invoke,
        }
    }

    /// Ensure data directory exists
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        Ok(())
    }

    /// Write signals for Claude to read
    pub fn write_signals(&self, input: &BrainInput) -> Result<()> {
        let path = self.data_dir.join("signals.json");
        let json = serde_json::to_string_pretty(input)?;
        std::fs::write(&path, json)?;
        debug!("Wrote signals to {:?}", path);
        Ok(())
    }

    /// Read decision from Claude (non-blocking)
    pub fn read_decision(&self) -> Result<Option<BrainDecision>> {
        let path = self.data_dir.join("decision.json");
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(&path)?;
        let decision: BrainDecision = serde_json::from_str(&json)?;

        // Clear file after reading
        std::fs::remove_file(&path)?;
        info!("Read decision from {:?}", path);

        Ok(Some(decision))
    }

    /// Check for decision without removing
    pub fn peek_decision(&self) -> Result<Option<BrainDecision>> {
        let path = self.data_dir.join("decision.json");
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(&path)?;
        let decision: BrainDecision = serde_json::from_str(&json)?;
        Ok(Some(decision))
    }

    /// Wait for decision with timeout
    pub async fn wait_for_decision(&self) -> Result<BrainDecision> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(100);

        loop {
            if let Some(decision) = self.read_decision()? {
                return Ok(decision);
            }

            if start.elapsed() > self.timeout {
                return Err(anyhow!(
                    "Timeout waiting for Claude decision ({}s)",
                    self.timeout.as_secs()
                ));
            }

            sleep(poll_interval).await;
        }
    }

    /// Invoke Claude Code skill (for autonomous mode)
    pub async fn invoke_claude(&self, skill: &str) -> Result<String> {
        info!("Invoking Claude skill: {}", skill);

        // Get HOME and construct paths
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/sandeep".to_string());
        let claude_symlink = format!("{}/.local/bin/claude", home);

        // Resolve symlink to get actual binary path
        let claude_path = std::fs::canonicalize(&claude_symlink)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(claude_symlink);

        info!("Using claude CLI at: {}", claude_path);

        // Get current working directory (should be project root)
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        info!("Working directory: {:?}", cwd);

        // Add ~/.local/bin to PATH for the subprocess
        let current_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}/.local/bin:{}", home, current_path);

        let output = Command::new(&claude_path)
            .args(["--print", &format!("/{}", skill)])
            .current_dir(&cwd)
            .env("PATH", &new_path)
            .env("HOME", &home)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute claude CLI: {} (path: {})", e, claude_path))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Claude invocation failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    /// Request decision from Claude (write signals + optionally invoke)
    pub async fn request_decision(&self, input: &BrainInput) -> Result<Option<BrainDecision>> {
        self.write_signals(input)?;

        if self.auto_invoke {
            // Auto-invoke Claude skill
            self.invoke_claude("simmons-brain").await?;
            // Wait for decision
            let decision = self.wait_for_decision().await?;
            Ok(Some(decision))
        } else {
            // Interactive mode - just write signals, user will invoke Claude
            info!("Signals written. Waiting for Claude decision...");
            Ok(None)
        }
    }

    /// Load brain state
    pub fn load_state(&self) -> Result<BrainState> {
        let path = self.data_dir.join("state.json");
        if !path.exists() {
            return Ok(BrainState::default());
        }
        let json = std::fs::read_to_string(&path)?;
        let state: BrainState = serde_json::from_str(&json)?;
        Ok(state)
    }

    /// Save brain state
    pub fn save_state(&self, state: &BrainState) -> Result<()> {
        let path = self.data_dir.join("state.json");
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Get signals file path (for display)
    pub fn signals_path(&self) -> PathBuf {
        self.data_dir.join("signals.json")
    }

    /// Get decision file path (for display)
    pub fn decision_path(&self) -> PathBuf {
        self.data_dir.join("decision.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use tempfile::tempdir;

    #[test]
    fn test_write_read_signals() {
        let dir = tempdir().unwrap();
        let bridge = BrainBridge::new(dir.path(), 60, false);
        bridge.init().unwrap();

        let input = BrainInput {
            timestamp: 1234567890,
            symbol: "BTC-USDT".to_string(),
            market_state: crate::types::BrainMarketState {
                price: dec!(67000),
                spread_bps: dec!(15),
                volatility_1h: dec!(0.023),
                regime: "trending_up".to_string(),
            },
            signals: vec![],
            arbitrage: vec![],
            portfolio: crate::types::BrainPortfolio {
                capital: dec!(100),
                positions: vec![],
                drawdown: dec!(0.02),
                risk_level: "normal".to_string(),
            },
            recent_trades: vec![],
        };

        bridge.write_signals(&input).unwrap();

        let path = dir.path().join("signals.json");
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("BTC-USDT"));
    }

    #[test]
    fn test_read_decision() {
        let dir = tempdir().unwrap();
        let bridge = BrainBridge::new(dir.path(), 60, false);
        bridge.init().unwrap();

        // Write a decision file
        let decision_json = r#"{
            "action": "trade",
            "symbol": "BTC-USDT",
            "side": "long",
            "size_pct": 0.12,
            "confidence": 0.85,
            "reasoning": "Test decision",
            "stop_loss_pct": 0.03,
            "take_profit_pct": 0.08
        }"#;
        std::fs::write(dir.path().join("decision.json"), decision_json).unwrap();

        let decision = bridge.read_decision().unwrap().unwrap();
        assert_eq!(decision.symbol, Some("BTC-USDT".to_string()));
        assert_eq!(decision.confidence, dec!(0.85));

        // File should be removed after reading
        assert!(!dir.path().join("decision.json").exists());
    }

    #[test]
    fn test_state_persistence() {
        let dir = tempdir().unwrap();
        let bridge = BrainBridge::new(dir.path(), 60, false);
        bridge.init().unwrap();

        let mut state = BrainState::default();
        state.total_trades = 10;
        state.wins = 7;
        state.total_pnl = dec!(50);

        bridge.save_state(&state).unwrap();

        let loaded = bridge.load_state().unwrap();
        assert_eq!(loaded.total_trades, 10);
        assert_eq!(loaded.wins, 7);
        assert_eq!(loaded.total_pnl, dec!(50));
    }
}
