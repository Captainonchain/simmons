//! REFLECT - Self-Learning System
//!
//! Analyzes trade outcomes, generates reflections, and updates brain weights.
//! Implements the REFLECT pattern from Nunchi for continuous improvement.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::consensus::{ConsensusAction, ConsensusEngine, MergedContext};
use crate::fund_brain::FundAction;
use crate::ta_brain::TAAction;

/// Trade record for reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    /// Unique trade ID
    pub id: String,
    /// Symbol
    pub symbol: String,
    /// Entry timestamp
    pub entry_time: DateTime<Utc>,
    /// Exit timestamp
    pub exit_time: Option<DateTime<Utc>>,
    /// Entry price
    pub entry_price: Decimal,
    /// Exit price
    pub exit_price: Option<Decimal>,
    /// Side (long/short)
    pub side: TradeSide,
    /// Size as percentage of capital
    pub size_pct: Decimal,
    /// P&L in USD
    pub pnl_usd: Option<Decimal>,
    /// P&L percentage
    pub pnl_pct: Option<Decimal>,
    /// Outcome
    pub outcome: Option<TradeOutcome>,
    /// Context at entry
    pub entry_context: MergedContext,
    /// TA Brain agreed with trade
    pub ta_agreed: bool,
    /// Fund Brain agreed with trade
    pub fund_agreed: bool,
    /// Reflection notes
    pub reflection: Option<String>,
}

/// Trade side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeSide {
    Long,
    Short,
}

/// Trade outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeOutcome {
    Win,
    Loss,
    Breakeven,
}

/// Reflection entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    /// Trade ID
    pub trade_id: String,
    /// Symbol
    pub symbol: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Outcome
    pub outcome: TradeOutcome,
    /// What worked
    pub what_worked: Vec<String>,
    /// What didn't work
    pub what_failed: Vec<String>,
    /// Lessons learned
    pub lessons: Vec<String>,
    /// Suggested adjustments
    pub adjustments: Vec<Adjustment>,
    /// Confidence in reflection
    pub confidence: Decimal,
}

/// Suggested adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adjustment {
    /// Adjustment type
    pub adjustment_type: AdjustmentType,
    /// Current value
    pub current_value: String,
    /// Suggested value
    pub suggested_value: String,
    /// Reason
    pub reason: String,
    /// Priority (1-5, 5 being highest)
    pub priority: u8,
}

/// Adjustment types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentType {
    /// Adjust TA brain weight
    TAWeight,
    /// Adjust Fund brain weight
    FundWeight,
    /// Adjust position size
    PositionSize,
    /// Adjust stop loss
    StopLoss,
    /// Adjust take profit
    TakeProfit,
    /// Adjust RADAR threshold
    RadarThreshold,
    /// Adjust confidence threshold
    ConfidenceThreshold,
    /// Avoid specific setup
    AvoidSetup,
}

/// Mistake entry for mistakes.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mistake {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Symbol
    pub symbol: String,
    /// Mistake type
    pub mistake_type: MistakeType,
    /// Description
    pub description: String,
    /// Conditions that led to mistake
    pub conditions: MistakeConditions,
    /// Rule to add
    pub avoid_rule: String,
    /// Severity (1-5)
    pub severity: u8,
}

/// Mistake types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MistakeType {
    /// Entered despite conflict
    IgnoredConflict,
    /// Entered with low confidence
    LowConfidence,
    /// Entered choppy market
    ChoppyRegime,
    /// Ignored security warning
    SecurityWarning,
    /// Position too large
    OversizedPosition,
    /// Stop too tight
    TightStop,
    /// Stop too loose
    LooseStop,
    /// Ignored whale selling
    IgnoredWhaleSelling,
    /// Bad timing
    BadTiming,
    /// Other
    Other,
}

/// Conditions when mistake happened
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistakeConditions {
    pub radar_score: Option<u16>,
    pub pulse_tier: Option<u8>,
    pub regime: Option<String>,
    pub ta_sentiment: Option<Decimal>,
    pub fund_sentiment: Option<Decimal>,
    pub was_conflict: bool,
    pub confidence: Option<Decimal>,
}

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// Total trades
    pub total_trades: u32,
    /// Wins
    pub wins: u32,
    /// Losses
    pub losses: u32,
    /// Win rate
    pub win_rate: Decimal,
    /// Total P&L
    pub total_pnl: Decimal,
    /// Average win
    pub avg_win: Decimal,
    /// Average loss
    pub avg_loss: Decimal,
    /// Profit factor
    pub profit_factor: Decimal,
    /// Max drawdown
    pub max_drawdown: Decimal,
    /// Sharpe ratio (simplified)
    pub sharpe_ratio: Decimal,
    /// TA brain accuracy
    pub ta_accuracy: Decimal,
    /// Fund brain accuracy
    pub fund_accuracy: Decimal,
    /// Best performing setup
    pub best_setup: Option<String>,
    /// Worst performing setup
    pub worst_setup: Option<String>,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            total_trades: 0,
            wins: 0,
            losses: 0,
            win_rate: Decimal::ZERO,
            total_pnl: Decimal::ZERO,
            avg_win: Decimal::ZERO,
            avg_loss: Decimal::ZERO,
            profit_factor: Decimal::ONE,
            max_drawdown: Decimal::ZERO,
            sharpe_ratio: Decimal::ZERO,
            ta_accuracy: dec!(0.5),
            fund_accuracy: dec!(0.5),
            best_setup: None,
            worst_setup: None,
        }
    }
}

/// REFLECT learning system
pub struct ReflectSystem {
    /// Data directory
    data_dir: PathBuf,
    /// Trade history
    trades: Vec<TradeRecord>,
    /// Reflections
    reflections: Vec<Reflection>,
    /// Mistakes
    mistakes: Vec<Mistake>,
    /// Performance stats
    stats: PerformanceStats,
    /// Reference to consensus engine for weight updates
    consensus_engine: Option<ConsensusEngine>,
}

impl ReflectSystem {
    /// Create new REFLECT system
    pub fn new(data_dir: PathBuf) -> Self {
        let mut system = Self {
            data_dir: data_dir.clone(),
            trades: Vec::new(),
            reflections: Vec::new(),
            mistakes: Vec::new(),
            stats: PerformanceStats::default(),
            consensus_engine: None,
        };

        // Load existing data
        system.load().ok();
        system
    }

    /// Set consensus engine for adaptive weights
    pub fn set_consensus_engine(&mut self, engine: ConsensusEngine) {
        self.consensus_engine = Some(engine);
    }

    /// Record a new trade entry
    pub fn record_entry(&mut self, trade: TradeRecord) {
        info!("REFLECT: Recording trade entry {}", trade.id);
        self.trades.push(trade);
        self.save().ok();
    }

    /// Record trade exit and generate reflection
    pub fn record_exit(
        &mut self,
        trade_id: &str,
        exit_price: Decimal,
        pnl_usd: Decimal,
        pnl_pct: Decimal,
    ) -> Option<Reflection> {
        // Find trade index
        let trade_idx = self.trades.iter().position(|t| t.id == trade_id)?;

        // Determine outcome
        let outcome = if pnl_pct > dec!(0.5) {
            TradeOutcome::Win
        } else if pnl_pct < dec!(-0.5) {
            TradeOutcome::Loss
        } else {
            TradeOutcome::Breakeven
        };

        // Update trade record
        {
            let trade = &mut self.trades[trade_idx];
            trade.exit_time = Some(Utc::now());
            trade.exit_price = Some(exit_price);
            trade.pnl_usd = Some(pnl_usd);
            trade.pnl_pct = Some(pnl_pct);
            trade.outcome = Some(outcome);
        }

        // Generate reflection (immutable borrow)
        let reflection = self.generate_reflection(&self.trades[trade_idx].clone());

        // Update stats
        self.update_stats();

        // Get values needed for consensus update
        let (ta_agreed, fund_agreed) = {
            let trade = &self.trades[trade_idx];
            (trade.ta_agreed, trade.fund_agreed)
        };

        // Update consensus weights if engine available
        if let Some(ref mut engine) = self.consensus_engine {
            engine.record_outcome(ta_agreed, fund_agreed, outcome == TradeOutcome::Win);
        }

        // Check for mistakes
        if outcome == TradeOutcome::Loss {
            let trade_clone = self.trades[trade_idx].clone();
            if let Some(mistake) = self.detect_mistake(&trade_clone) {
                self.mistakes.push(mistake);
            }
        }

        // Save
        self.reflections.push(reflection.clone());
        self.save().ok();

        info!(
            "REFLECT: Trade {} closed with {:?}, P&L: {:.2}%",
            trade_id, outcome, pnl_pct
        );

        Some(reflection)
    }

    /// Generate reflection for a trade
    fn generate_reflection(&self, trade: &TradeRecord) -> Reflection {
        let outcome = trade.outcome.unwrap_or(TradeOutcome::Breakeven);
        let ctx = &trade.entry_context;

        let mut what_worked = Vec::new();
        let mut what_failed = Vec::new();
        let mut lessons = Vec::new();
        let mut adjustments = Vec::new();

        // Analyze based on outcome
        match outcome {
            TradeOutcome::Win => {
                // What worked
                if trade.ta_agreed {
                    what_worked.push("TA brain signal was correct".to_string());
                }
                if trade.fund_agreed {
                    what_worked.push("Fund brain signal was correct".to_string());
                }
                if ctx.ta.radar_score.score >= 200 {
                    what_worked.push(format!(
                        "High RADAR score {} provided conviction",
                        ctx.ta.radar_score.score
                    ));
                }
                if ctx.merged_confidence > dec!(0.7) {
                    what_worked.push("High confidence entry was justified".to_string());
                }

                lessons.push("Continue trusting this setup pattern".to_string());
            }
            TradeOutcome::Loss => {
                // What failed
                if !trade.ta_agreed {
                    what_failed.push("Entered against TA brain signal".to_string());
                    adjustments.push(Adjustment {
                        adjustment_type: AdjustmentType::TAWeight,
                        current_value: "0.6".to_string(),
                        suggested_value: "0.65".to_string(),
                        reason: "TA was right, increase weight".to_string(),
                        priority: 3,
                    });
                }
                if !trade.fund_agreed {
                    what_failed.push("Entered against Fund brain signal".to_string());
                    adjustments.push(Adjustment {
                        adjustment_type: AdjustmentType::FundWeight,
                        current_value: "0.4".to_string(),
                        suggested_value: "0.45".to_string(),
                        reason: "Fund was right, increase weight".to_string(),
                        priority: 3,
                    });
                }
                if ctx.is_conflict {
                    what_failed.push("Entered despite brain conflict".to_string());
                    lessons.push("Avoid trading when brains disagree".to_string());
                }
                if ctx.ta.radar_score.score < 170 {
                    what_failed.push(format!(
                        "Low RADAR score {} should have been skipped",
                        ctx.ta.radar_score.score
                    ));
                    adjustments.push(Adjustment {
                        adjustment_type: AdjustmentType::RadarThreshold,
                        current_value: "170".to_string(),
                        suggested_value: "180".to_string(),
                        reason: "Raise RADAR threshold to filter weak setups".to_string(),
                        priority: 4,
                    });
                }
                if ctx.merged_confidence < dec!(0.6) {
                    what_failed.push("Low confidence entry resulted in loss".to_string());
                    lessons.push("Require higher confidence for entries".to_string());
                }
                if matches!(ctx.regime, simmons_core::Regime::Choppy) {
                    what_failed.push("Traded in choppy regime".to_string());
                    lessons.push("Avoid trading in choppy conditions".to_string());
                }

                lessons.push("Review entry criteria for this setup".to_string());
            }
            TradeOutcome::Breakeven => {
                lessons.push("Consider tighter profit targets".to_string());
            }
        }

        // Calculate reflection confidence
        let confidence = if what_worked.len() + what_failed.len() >= 3 {
            dec!(0.8)
        } else {
            dec!(0.5)
        };

        Reflection {
            trade_id: trade.id.clone(),
            symbol: trade.symbol.clone(),
            timestamp: Utc::now(),
            outcome,
            what_worked,
            what_failed,
            lessons,
            adjustments,
            confidence,
        }
    }

    /// Detect mistakes from losing trades
    fn detect_mistake(&self, trade: &TradeRecord) -> Option<Mistake> {
        let ctx = &trade.entry_context;

        // Check for common mistakes
        let (mistake_type, description, avoid_rule, severity) = if ctx.is_conflict {
            (
                MistakeType::IgnoredConflict,
                "Entered trade despite brain conflict".to_string(),
                "DO NOT trade when TA and Fund brains disagree".to_string(),
                4,
            )
        } else if ctx.merged_confidence < dec!(0.5) {
            (
                MistakeType::LowConfidence,
                format!("Entered with only {:.0}% confidence", ctx.merged_confidence * dec!(100)),
                "Require minimum 60% confidence for entries".to_string(),
                3,
            )
        } else if matches!(ctx.regime, simmons_core::Regime::Choppy) {
            (
                MistakeType::ChoppyRegime,
                "Traded in choppy market regime".to_string(),
                "Avoid trading when regime is CHOPPY".to_string(),
                4,
            )
        } else if ctx.fund.whale_sentiment < dec!(-0.5) {
            (
                MistakeType::IgnoredWhaleSelling,
                "Went long despite whale selling pressure".to_string(),
                "Do not long when whale sentiment < -0.5".to_string(),
                3,
            )
        } else if ctx.ta.radar_score.score < 140 {
            (
                MistakeType::LowConfidence,
                format!("RADAR score {} was too low", ctx.ta.radar_score.score),
                "Skip trades with RADAR < 140".to_string(),
                3,
            )
        } else {
            return None;
        };

        Some(Mistake {
            timestamp: Utc::now(),
            symbol: trade.symbol.clone(),
            mistake_type,
            description,
            conditions: MistakeConditions {
                radar_score: Some(ctx.ta.radar_score.score),
                pulse_tier: Some(ctx.ta.pulse_signal.tier),
                regime: Some(format!("{:?}", ctx.regime)),
                ta_sentiment: Some(ctx.ta.overall_sentiment),
                fund_sentiment: Some(ctx.fund.overall_sentiment),
                was_conflict: ctx.is_conflict,
                confidence: Some(ctx.merged_confidence),
            },
            avoid_rule,
            severity,
        })
    }

    /// Update performance statistics
    fn update_stats(&mut self) {
        let closed_trades: Vec<&TradeRecord> = self
            .trades
            .iter()
            .filter(|t| t.outcome.is_some())
            .collect();

        if closed_trades.is_empty() {
            return;
        }

        let total = closed_trades.len() as u32;
        let wins = closed_trades
            .iter()
            .filter(|t| t.outcome == Some(TradeOutcome::Win))
            .count() as u32;
        let losses = closed_trades
            .iter()
            .filter(|t| t.outcome == Some(TradeOutcome::Loss))
            .count() as u32;

        let win_rate = if total > 0 {
            Decimal::from(wins) / Decimal::from(total)
        } else {
            Decimal::ZERO
        };

        let total_pnl: Decimal = closed_trades
            .iter()
            .filter_map(|t| t.pnl_usd)
            .sum();

        let winning_pnl: Decimal = closed_trades
            .iter()
            .filter(|t| t.outcome == Some(TradeOutcome::Win))
            .filter_map(|t| t.pnl_usd)
            .sum();
        let losing_pnl: Decimal = closed_trades
            .iter()
            .filter(|t| t.outcome == Some(TradeOutcome::Loss))
            .filter_map(|t| t.pnl_usd)
            .sum::<Decimal>()
            .abs();

        let avg_win = if wins > 0 {
            winning_pnl / Decimal::from(wins)
        } else {
            Decimal::ZERO
        };

        let avg_loss = if losses > 0 {
            losing_pnl / Decimal::from(losses)
        } else {
            Decimal::ZERO
        };

        let profit_factor = if losing_pnl > Decimal::ZERO {
            winning_pnl / losing_pnl
        } else if winning_pnl > Decimal::ZERO {
            dec!(999)
        } else {
            Decimal::ONE
        };

        // TA accuracy
        let ta_correct = closed_trades
            .iter()
            .filter(|t| t.ta_agreed == (t.outcome == Some(TradeOutcome::Win)))
            .count() as u32;
        let ta_accuracy = Decimal::from(ta_correct) / Decimal::from(total);

        // Fund accuracy
        let fund_correct = closed_trades
            .iter()
            .filter(|t| t.fund_agreed == (t.outcome == Some(TradeOutcome::Win)))
            .count() as u32;
        let fund_accuracy = Decimal::from(fund_correct) / Decimal::from(total);

        self.stats = PerformanceStats {
            total_trades: total,
            wins,
            losses,
            win_rate,
            total_pnl,
            avg_win,
            avg_loss,
            profit_factor,
            max_drawdown: self.stats.max_drawdown, // Keep existing
            sharpe_ratio: Decimal::ZERO,           // TODO: Calculate properly
            ta_accuracy,
            fund_accuracy,
            best_setup: None,  // TODO: Track setups
            worst_setup: None,
        };
    }

    /// Get current performance stats
    pub fn get_stats(&self) -> &PerformanceStats {
        &self.stats
    }

    /// Get recent reflections
    pub fn get_recent_reflections(&self, count: usize) -> Vec<&Reflection> {
        self.reflections.iter().rev().take(count).collect()
    }

    /// Get all mistakes
    pub fn get_mistakes(&self) -> &[Mistake] {
        &self.mistakes
    }

    /// Get avoid rules from mistakes
    pub fn get_avoid_rules(&self) -> Vec<String> {
        self.mistakes.iter().map(|m| m.avoid_rule.clone()).collect()
    }

    /// Check if conditions match a known mistake pattern
    pub fn check_mistake_pattern(&self, context: &MergedContext) -> Option<&Mistake> {
        for mistake in &self.mistakes {
            let conditions = &mistake.conditions;

            // Check regime match
            if let Some(ref regime) = conditions.regime {
                if format!("{:?}", context.regime) == *regime {
                    return Some(mistake);
                }
            }

            // Check conflict pattern
            if conditions.was_conflict && context.is_conflict {
                return Some(mistake);
            }

            // Check low confidence pattern
            if let Some(conf) = conditions.confidence {
                if conf < dec!(0.6) && context.merged_confidence < dec!(0.6) {
                    return Some(mistake);
                }
            }
        }

        None
    }

    /// Save data to files
    fn save(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.data_dir)?;

        // Save trades
        let trades_file = self.data_dir.join("trades.json");
        let trades_json = serde_json::to_string_pretty(&self.trades)?;
        fs::write(trades_file, trades_json)?;

        // Save reflections
        let reflections_file = self.data_dir.join("reflections.json");
        let reflections_json = serde_json::to_string_pretty(&self.reflections)?;
        fs::write(reflections_file, reflections_json)?;

        // Save mistakes
        let mistakes_file = self.data_dir.join("mistakes.json");
        let mistakes_json = serde_json::to_string_pretty(&self.mistakes)?;
        fs::write(mistakes_file, mistakes_json)?;

        // Save stats
        let stats_file = self.data_dir.join("performance.json");
        let stats_json = serde_json::to_string_pretty(&self.stats)?;
        fs::write(stats_file, stats_json)?;

        Ok(())
    }

    /// Load data from files
    fn load(&mut self) -> std::io::Result<()> {
        // Load trades
        let trades_file = self.data_dir.join("trades.json");
        if trades_file.exists() {
            let content = fs::read_to_string(&trades_file)?;
            self.trades = serde_json::from_str(&content).unwrap_or_default();
        }

        // Load reflections
        let reflections_file = self.data_dir.join("reflections.json");
        if reflections_file.exists() {
            let content = fs::read_to_string(&reflections_file)?;
            self.reflections = serde_json::from_str(&content).unwrap_or_default();
        }

        // Load mistakes
        let mistakes_file = self.data_dir.join("mistakes.json");
        if mistakes_file.exists() {
            let content = fs::read_to_string(&mistakes_file)?;
            self.mistakes = serde_json::from_str(&content).unwrap_or_default();
        }

        // Load stats
        let stats_file = self.data_dir.join("performance.json");
        if stats_file.exists() {
            let content = fs::read_to_string(&stats_file)?;
            self.stats = serde_json::from_str(&content).unwrap_or_default();
        }

        info!(
            "REFLECT: Loaded {} trades, {} reflections, {} mistakes",
            self.trades.len(),
            self.reflections.len(),
            self.mistakes.len()
        );

        Ok(())
    }

    /// Run periodic reflection (e.g., nightly)
    pub fn run_periodic_reflection(&mut self) -> String {
        let mut report = String::from("# REFLECT Periodic Report\n\n");

        // Performance summary
        report.push_str("## Performance Summary\n\n");
        report.push_str(&format!("- Total Trades: {}\n", self.stats.total_trades));
        report.push_str(&format!(
            "- Win Rate: {:.1}%\n",
            self.stats.win_rate * dec!(100)
        ));
        report.push_str(&format!("- Total P&L: ${:.2}\n", self.stats.total_pnl));
        report.push_str(&format!("- Profit Factor: {:.2}\n", self.stats.profit_factor));
        report.push_str(&format!(
            "- TA Accuracy: {:.1}%\n",
            self.stats.ta_accuracy * dec!(100)
        ));
        report.push_str(&format!(
            "- Fund Accuracy: {:.1}%\n\n",
            self.stats.fund_accuracy * dec!(100)
        ));

        // Recent lessons
        report.push_str("## Recent Lessons\n\n");
        for reflection in self.get_recent_reflections(5) {
            for lesson in &reflection.lessons {
                report.push_str(&format!("- {}\n", lesson));
            }
        }
        report.push('\n');

        // Active avoid rules
        report.push_str("## Active Avoid Rules\n\n");
        for rule in self.get_avoid_rules() {
            report.push_str(&format!("- {}\n", rule));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_context(sentiment: Decimal, is_conflict: bool) -> MergedContext {
        use crate::fund_brain::{FundBrainOutput, FundRecommendation};
        use crate::ta_brain::{
            PulseDirection, PulseSignal, RadarScore, RadarTier, TABrainOutput, TARecommendation,
        };
        use simmons_core::Regime;

        MergedContext {
            symbol: "BTC".to_string(),
            chain: "ethereum".to_string(),
            ta: TABrainOutput {
                symbol: "BTC".to_string(),
                radar_score: RadarScore {
                    score: 200,
                    market_structure: 70,
                    technicals: 80,
                    funding: 50,
                    tier: RadarTier::Solid,
                    recommended_strategy: None,
                    symbol: "BTC".to_string(),
                    timestamp: Utc::now(),
                },
                pulse_signal: PulseSignal {
                    tier: 4,
                    immediate_mover: 70,
                    volume_surge: 60,
                    direction: PulseDirection::Up,
                    strength: dec!(0.65),
                    symbol: "BTC".to_string(),
                    timestamp: Utc::now(),
                },
                regime: Regime::TrendingUp,
                strategy_signals: vec![],
                guard_states: vec![],
                overall_sentiment: sentiment,
                overall_confidence: dec!(0.75),
                recommended_action: TARecommendation {
                    action: TAAction::Long,
                    strategy: None,
                    size_factor: dec!(0.8),
                    confidence: dec!(0.75),
                    reasoning: "Test".to_string(),
                },
                timestamp: Utc::now(),
            },
            fund: FundBrainOutput {
                symbol: "BTC".to_string(),
                chain: "ethereum".to_string(),
                whale_sentiment: sentiment,
                whale_signals: vec![],
                twitter_sentiment: sentiment,
                twitter_data: None,
                news_sentiment: sentiment,
                news_data: None,
                security: None,
                overall_sentiment: sentiment,
                overall_confidence: dec!(0.7),
                recommendation: FundRecommendation {
                    action: FundAction::Bullish,
                    confidence: dec!(0.7),
                    size_modifier: dec!(0.9),
                    reasoning: "Test".to_string(),
                    security_warnings: vec![],
                },
                timestamp: Utc::now(),
            },
            merged_sentiment: sentiment,
            merged_confidence: dec!(0.72),
            consensus_action: ConsensusAction::Long,
            is_conflict,
            conflict_reason: if is_conflict {
                Some("Test conflict".to_string())
            } else {
                None
            },
            size_factor: dec!(0.7),
            regime: Regime::TrendingUp,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_record_and_reflect() {
        let temp_dir = TempDir::new().unwrap();
        let mut reflect = ReflectSystem::new(temp_dir.path().to_path_buf());

        let trade = TradeRecord {
            id: "test_1".to_string(),
            symbol: "BTC-USDT".to_string(),
            entry_time: Utc::now(),
            exit_time: None,
            entry_price: dec!(65000),
            exit_price: None,
            side: TradeSide::Long,
            size_pct: dec!(0.1),
            pnl_usd: None,
            pnl_pct: None,
            outcome: None,
            entry_context: make_test_context(dec!(0.7), false),
            ta_agreed: true,
            fund_agreed: true,
            reflection: None,
        };

        reflect.record_entry(trade);

        let reflection = reflect.record_exit("test_1", dec!(66000), dec!(10), dec!(1.54));
        assert!(reflection.is_some());

        let stats = reflect.get_stats();
        assert_eq!(stats.total_trades, 1);
        assert_eq!(stats.wins, 1);
    }

    #[test]
    fn test_mistake_detection() {
        let temp_dir = TempDir::new().unwrap();
        let mut reflect = ReflectSystem::new(temp_dir.path().to_path_buf());

        // Create a losing trade with conflict
        let trade = TradeRecord {
            id: "test_2".to_string(),
            symbol: "BTC-USDT".to_string(),
            entry_time: Utc::now(),
            exit_time: None,
            entry_price: dec!(65000),
            exit_price: None,
            side: TradeSide::Long,
            size_pct: dec!(0.1),
            pnl_usd: None,
            pnl_pct: None,
            outcome: None,
            entry_context: make_test_context(dec!(0.5), true), // conflict = true
            ta_agreed: true,
            fund_agreed: false,
            reflection: None,
        };

        reflect.record_entry(trade);
        reflect.record_exit("test_2", dec!(64000), dec!(-10), dec!(-1.54));

        assert!(!reflect.get_mistakes().is_empty());
        assert_eq!(reflect.get_mistakes()[0].mistake_type, MistakeType::IgnoredConflict);
    }
}
