//! Memory system for agent learning
//!
//! Stores learnings, mistakes, and patterns for each agent type.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Memory storage path
const MEMORY_FILE: &str = "data/memory.json";

/// Agent types that can have memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    TechnicalAnalyst,
    FundamentalAnalyst,
    SentimentAnalyst,
    OnchainAnalyst,
    BullResearcher,
    BearResearcher,
    ResearchManager,
    AggressiveRisk,
    ConservativeRisk,
    NeutralRisk,
    Orchestrator,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::TechnicalAnalyst => write!(f, "technical_analyst"),
            AgentType::FundamentalAnalyst => write!(f, "fundamental_analyst"),
            AgentType::SentimentAnalyst => write!(f, "sentiment_analyst"),
            AgentType::OnchainAnalyst => write!(f, "onchain_analyst"),
            AgentType::BullResearcher => write!(f, "bull_researcher"),
            AgentType::BearResearcher => write!(f, "bear_researcher"),
            AgentType::ResearchManager => write!(f, "research_manager"),
            AgentType::AggressiveRisk => write!(f, "aggressive_risk"),
            AgentType::ConservativeRisk => write!(f, "conservative_risk"),
            AgentType::NeutralRisk => write!(f, "neutral_risk"),
            AgentType::Orchestrator => write!(f, "orchestrator"),
        }
    }
}

/// A single learning entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learning {
    pub id: String,
    pub agent: AgentType,
    pub category: LearningCategory,
    pub description: String,
    pub context: String,
    pub trade_id: Option<String>,
    pub outcome: Option<String>,
    pub confidence_delta: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub times_applied: u32,
    pub success_rate: Option<Decimal>,
}

/// Categories of learnings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LearningCategory {
    /// Pattern that led to successful trades
    WinningPattern,
    /// Mistake to avoid
    Mistake,
    /// Market condition insight
    MarketInsight,
    /// Risk management lesson
    RiskLesson,
    /// Signal reliability
    SignalReliability,
    /// Timing insight
    TimingInsight,
}

/// Trade reflection for post-trade learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeReflection {
    pub trade_id: String,
    pub symbol: String,
    pub side: String,
    pub entry_price: Decimal,
    pub exit_price: Decimal,
    pub pnl: Decimal,
    pub pnl_pct: Decimal,
    pub outcome: String,
    pub duration_minutes: i64,
    pub agent_predictions: HashMap<String, AgentPrediction>,
    pub what_worked: Vec<String>,
    pub what_failed: Vec<String>,
    pub lessons: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Individual agent's prediction for a trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPrediction {
    pub recommendation: String,
    pub confidence: Decimal,
    pub was_correct: bool,
    pub key_reason: String,
}

/// Statistics for an agent
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentStats {
    pub total_predictions: u32,
    pub correct_predictions: u32,
    pub accuracy: Decimal,
    pub avg_confidence_when_correct: Decimal,
    pub avg_confidence_when_wrong: Decimal,
    pub best_pattern: Option<String>,
    pub worst_pattern: Option<String>,
}

/// Complete memory storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryStorage {
    pub learnings: Vec<Learning>,
    pub reflections: Vec<TradeReflection>,
    pub agent_stats: HashMap<String, AgentStats>,
    pub global_insights: Vec<String>,
    pub avoid_patterns: Vec<String>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// Memory system manager
pub struct MemorySystem {
    inner: Arc<RwLock<MemoryStorage>>,
    file_path: String,
}

impl Clone for MemorySystem {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            file_path: self.file_path.clone(),
        }
    }
}

impl std::fmt::Debug for MemorySystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let storage = self.inner.read();
        f.debug_struct("MemorySystem")
            .field("file_path", &self.file_path)
            .field("learnings", &storage.learnings.len())
            .field("reflections", &storage.reflections.len())
            .finish()
    }
}

impl Default for MemorySystem {
    fn default() -> Self {
        Self::new(MEMORY_FILE)
    }
}

impl MemorySystem {
    /// Create new memory system, loading from file if exists
    pub fn new(file_path: &str) -> Self {
        let storage = if Path::new(file_path).exists() {
            match fs::read_to_string(file_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => MemoryStorage::default(),
            }
        } else {
            MemoryStorage::default()
        };

        Self {
            inner: Arc::new(RwLock::new(storage)),
            file_path: file_path.to_string(),
        }
    }

    /// Save memory to file
    pub fn save(&self) -> Result<(), std::io::Error> {
        let storage = self.inner.read();

        // Ensure directory exists
        if let Some(parent) = Path::new(&self.file_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&*storage)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&self.file_path, json)
    }

    /// Add a learning
    pub fn add_learning(&self, learning: Learning) {
        let mut storage = self.inner.write();
        storage.learnings.push(learning);
        storage.last_updated = Some(Utc::now());
    }

    /// Add a trade reflection
    pub fn add_reflection(&self, reflection: TradeReflection) {
        let mut storage = self.inner.write();

        // Update agent stats based on reflection
        for (agent_name, prediction) in &reflection.agent_predictions {
            let stats = storage.agent_stats.entry(agent_name.clone()).or_default();
            stats.total_predictions += 1;
            if prediction.was_correct {
                stats.correct_predictions += 1;
            }
            // Recalculate accuracy
            if stats.total_predictions > 0 {
                stats.accuracy = Decimal::from(stats.correct_predictions)
                    / Decimal::from(stats.total_predictions);
            }
        }

        // Add lessons as learnings
        for lesson in &reflection.lessons {
            storage.global_insights.push(lesson.clone());
        }

        // Add what failed to avoid patterns
        for failure in &reflection.what_failed {
            if !storage.avoid_patterns.contains(failure) {
                storage.avoid_patterns.push(failure.clone());
            }
        }

        storage.reflections.push(reflection);
        storage.last_updated = Some(Utc::now());
    }

    /// Get learnings for a specific agent
    pub fn get_agent_learnings(&self, agent: AgentType) -> Vec<Learning> {
        let storage = self.inner.read();
        storage.learnings
            .iter()
            .filter(|l| l.agent == agent)
            .cloned()
            .collect()
    }

    /// Get all mistakes to avoid
    pub fn get_mistakes(&self) -> Vec<Learning> {
        let storage = self.inner.read();
        storage.learnings
            .iter()
            .filter(|l| l.category == LearningCategory::Mistake)
            .cloned()
            .collect()
    }

    /// Get winning patterns
    pub fn get_winning_patterns(&self) -> Vec<Learning> {
        let storage = self.inner.read();
        storage.learnings
            .iter()
            .filter(|l| l.category == LearningCategory::WinningPattern)
            .cloned()
            .collect()
    }

    /// Get agent statistics
    pub fn get_agent_stats(&self, agent_name: &str) -> Option<AgentStats> {
        let storage = self.inner.read();
        storage.agent_stats.get(agent_name).cloned()
    }

    /// Get all agent statistics
    pub fn get_all_agent_stats(&self) -> HashMap<String, AgentStats> {
        let storage = self.inner.read();
        storage.agent_stats.clone()
    }

    /// Get recent reflections
    pub fn get_recent_reflections(&self, limit: usize) -> Vec<TradeReflection> {
        let storage = self.inner.read();
        storage.reflections
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get avoid patterns
    pub fn get_avoid_patterns(&self) -> Vec<String> {
        let storage = self.inner.read();
        storage.avoid_patterns.clone()
    }

    /// Get global insights
    pub fn get_global_insights(&self) -> Vec<String> {
        let storage = self.inner.read();
        storage.global_insights.clone()
    }

    /// Get full memory snapshot for Claude
    pub fn get_memory_snapshot(&self) -> MemorySnapshot {
        let storage = self.inner.read();

        MemorySnapshot {
            total_learnings: storage.learnings.len(),
            total_reflections: storage.reflections.len(),
            agent_stats: storage.agent_stats.clone(),
            recent_lessons: storage.global_insights.iter().rev().take(10).cloned().collect(),
            avoid_patterns: storage.avoid_patterns.clone(),
            winning_patterns: storage.learnings
                .iter()
                .filter(|l| l.category == LearningCategory::WinningPattern)
                .take(10)
                .map(|l| l.description.clone())
                .collect(),
            last_updated: storage.last_updated,
        }
    }

    /// Clear all memory (for testing)
    pub fn clear(&self) {
        let mut storage = self.inner.write();
        *storage = MemoryStorage::default();
    }
}

/// Memory snapshot for MCP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub total_learnings: usize,
    pub total_reflections: usize,
    pub agent_stats: HashMap<String, AgentStats>,
    pub recent_lessons: Vec<String>,
    pub avoid_patterns: Vec<String>,
    pub winning_patterns: Vec<String>,
    pub last_updated: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_memory_system() {
        let memory = MemorySystem::new("/tmp/test_memory.json");

        // Add a learning
        let learning = Learning {
            id: "l1".to_string(),
            agent: AgentType::TechnicalAnalyst,
            category: LearningCategory::WinningPattern,
            description: "RSI below 30 with volume spike leads to reversal".to_string(),
            context: "BTC-USDT".to_string(),
            trade_id: Some("t1".to_string()),
            outcome: Some("win".to_string()),
            confidence_delta: Some(dec!(0.1)),
            created_at: Utc::now(),
            times_applied: 0,
            success_rate: None,
        };

        memory.add_learning(learning);

        let learnings = memory.get_agent_learnings(AgentType::TechnicalAnalyst);
        assert_eq!(learnings.len(), 1);

        let winning = memory.get_winning_patterns();
        assert_eq!(winning.len(), 1);
    }
}
