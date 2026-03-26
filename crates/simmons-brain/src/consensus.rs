//! Consensus Layer - Merges TA and Fundamental Brain outputs
//!
//! Implements weighted merging, conflict detection, and adaptive weighting
//! based on historical performance.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::Regime;
use tracing::{debug, info, warn};

use crate::fund_brain::{FundAction, FundBrainOutput, FundRecommendation};
use crate::ta_brain::{TAAction, TABrainOutput, TARecommendation};

/// Merged context for Claude orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedContext {
    /// Symbol analyzed
    pub symbol: String,
    /// Chain
    pub chain: String,
    /// TA Brain output
    pub ta: TABrainOutput,
    /// Fund Brain output
    pub fund: FundBrainOutput,
    /// Merged sentiment (-1 to +1)
    pub merged_sentiment: Decimal,
    /// Merged confidence (0-1)
    pub merged_confidence: Decimal,
    /// Consensus action
    pub consensus_action: ConsensusAction,
    /// Are brains in conflict?
    pub is_conflict: bool,
    /// Conflict details if applicable
    pub conflict_reason: Option<String>,
    /// Position size factor (0-1)
    pub size_factor: Decimal,
    /// Current regime
    pub regime: Regime,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Consensus action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusAction {
    /// Long with conviction
    Long,
    /// Short with conviction
    Short,
    /// Hold current position
    Hold,
    /// Close existing position
    Close,
    /// Wait - no action
    Wait,
    /// Blocked by security
    Blocked,
    /// Debate required - brains in conflict
    Debate,
}

impl ConsensusAction {
    /// Is this an actionable decision?
    pub fn is_actionable(&self) -> bool {
        matches!(self, Self::Long | Self::Short | Self::Close)
    }

    /// Requires debate?
    pub fn requires_debate(&self) -> bool {
        matches!(self, Self::Debate)
    }
}

/// Consensus engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// TA brain weight (0-1)
    pub ta_weight: Decimal,
    /// Fund brain weight (0-1)
    pub fund_weight: Decimal,
    /// Minimum confidence for action
    pub min_confidence: Decimal,
    /// Conflict threshold (sentiment difference)
    pub conflict_threshold: Decimal,
    /// Reduce size on conflict
    pub conflict_reduces_size: bool,
    /// Size reduction on conflict (multiplier)
    pub conflict_size_multiplier: Decimal,
    /// Enable adaptive weights
    pub adaptive_weights: bool,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            ta_weight: dec!(0.6),
            fund_weight: dec!(0.4),
            min_confidence: dec!(0.5),
            conflict_threshold: dec!(0.5),
            conflict_reduces_size: true,
            conflict_size_multiplier: dec!(0.5),
            adaptive_weights: true,
        }
    }
}

/// Performance tracking for adaptive weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainPerformance {
    /// Total trades where this brain was correct
    pub wins: u32,
    /// Total trades where this brain was wrong
    pub losses: u32,
    /// Win rate
    pub win_rate: Decimal,
    /// Profit factor
    pub profit_factor: Decimal,
    /// Recent performance (last 10 trades)
    pub recent_wins: u32,
    pub recent_losses: u32,
}

impl Default for BrainPerformance {
    fn default() -> Self {
        Self {
            wins: 0,
            losses: 0,
            win_rate: dec!(0.5),
            profit_factor: Decimal::ONE,
            recent_wins: 0,
            recent_losses: 0,
        }
    }
}

impl BrainPerformance {
    /// Update with trade outcome
    pub fn record(&mut self, is_win: bool) {
        if is_win {
            self.wins += 1;
            self.recent_wins = (self.recent_wins + 1).min(10);
        } else {
            self.losses += 1;
            self.recent_losses = (self.recent_losses + 1).min(10);
        }

        // Recalculate win rate
        let total = self.wins + self.losses;
        if total > 0 {
            self.win_rate = Decimal::from(self.wins) / Decimal::from(total);
        }
    }

    /// Get recent win rate
    pub fn recent_win_rate(&self) -> Decimal {
        let recent_total = self.recent_wins + self.recent_losses;
        if recent_total == 0 {
            return dec!(0.5);
        }
        Decimal::from(self.recent_wins) / Decimal::from(recent_total)
    }
}

/// Consensus Engine
pub struct ConsensusEngine {
    /// Configuration
    config: ConsensusConfig,
    /// TA brain performance
    ta_performance: BrainPerformance,
    /// Fund brain performance
    fund_performance: BrainPerformance,
    /// Current effective weights
    effective_ta_weight: Decimal,
    effective_fund_weight: Decimal,
}

impl ConsensusEngine {
    /// Create new consensus engine
    pub fn new(config: ConsensusConfig) -> Self {
        let ta_weight = config.ta_weight;
        let fund_weight = config.fund_weight;

        Self {
            config,
            ta_performance: BrainPerformance::default(),
            fund_performance: BrainPerformance::default(),
            effective_ta_weight: ta_weight,
            effective_fund_weight: fund_weight,
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(ConsensusConfig::default())
    }

    /// Merge TA and Fund brain outputs
    pub fn merge(&self, ta: &TABrainOutput, fund: &FundBrainOutput) -> MergedContext {
        // Check for security block first
        if let Some(ref security) = fund.security {
            if security.should_block() {
                return MergedContext {
                    symbol: ta.symbol.clone(),
                    chain: fund.chain.clone(),
                    ta: ta.clone(),
                    fund: fund.clone(),
                    merged_sentiment: Decimal::ZERO,
                    merged_confidence: Decimal::ONE,
                    consensus_action: ConsensusAction::Blocked,
                    is_conflict: false,
                    conflict_reason: Some(format!(
                        "Security block: {}",
                        security.red_flags.join(", ")
                    )),
                    size_factor: Decimal::ZERO,
                    regime: ta.regime,
                    timestamp: Utc::now(),
                };
            }
        }

        // Calculate merged sentiment
        let merged_sentiment = self.calculate_merged_sentiment(ta, fund);

        // Calculate merged confidence
        let merged_confidence = self.calculate_merged_confidence(ta, fund);

        // Detect conflict
        let (is_conflict, conflict_reason) = self.detect_conflict(ta, fund);

        // Determine consensus action
        let consensus_action = self.determine_action(ta, fund, merged_sentiment, merged_confidence, is_conflict);

        // Calculate position size factor
        let size_factor = self.calculate_size_factor(ta, fund, is_conflict, merged_confidence);

        MergedContext {
            symbol: ta.symbol.clone(),
            chain: fund.chain.clone(),
            ta: ta.clone(),
            fund: fund.clone(),
            merged_sentiment,
            merged_confidence,
            consensus_action,
            is_conflict,
            conflict_reason,
            size_factor,
            regime: ta.regime,
            timestamp: Utc::now(),
        }
    }

    /// Calculate merged sentiment using weighted average
    fn calculate_merged_sentiment(&self, ta: &TABrainOutput, fund: &FundBrainOutput) -> Decimal {
        let ta_sentiment = ta.overall_sentiment;
        let fund_sentiment = fund.overall_sentiment;

        let weighted = ta_sentiment * self.effective_ta_weight
            + fund_sentiment * self.effective_fund_weight;

        weighted.min(Decimal::ONE).max(dec!(-1))
    }

    /// Calculate merged confidence
    fn calculate_merged_confidence(&self, ta: &TABrainOutput, fund: &FundBrainOutput) -> Decimal {
        let ta_conf = ta.overall_confidence;
        let fund_conf = fund.overall_confidence;

        // Weight by brain weights and individual confidences
        let weighted = ta_conf * self.effective_ta_weight * ta_conf
            + fund_conf * self.effective_fund_weight * fund_conf;

        // Normalize
        let normalizer = self.effective_ta_weight * ta_conf + self.effective_fund_weight * fund_conf;

        if normalizer.is_zero() {
            return Decimal::ZERO;
        }

        (weighted / normalizer).min(Decimal::ONE)
    }

    /// Detect if brains are in conflict
    pub fn detect_conflict(&self, ta: &TABrainOutput, fund: &FundBrainOutput) -> (bool, Option<String>) {
        let ta_sentiment = ta.overall_sentiment;
        let fund_sentiment = fund.overall_sentiment;

        // Check sentiment divergence
        let sentiment_diff = (ta_sentiment - fund_sentiment).abs();
        if sentiment_diff > self.config.conflict_threshold {
            let reason = format!(
                "Sentiment divergence: TA {:.2} vs Fund {:.2}",
                ta_sentiment, fund_sentiment
            );
            return (true, Some(reason));
        }

        // Check action divergence
        let ta_action = &ta.recommended_action.action;
        let fund_action = &fund.recommendation.action;

        let action_conflict = match (ta_action, fund_action) {
            (TAAction::Long, FundAction::Bearish) => true,
            (TAAction::Short, FundAction::Bullish) => true,
            _ => false,
        };

        if action_conflict {
            let reason = format!(
                "Action conflict: TA {:?} vs Fund {:?}",
                ta_action, fund_action
            );
            return (true, Some(reason));
        }

        (false, None)
    }

    /// Determine consensus action
    fn determine_action(
        &self,
        ta: &TABrainOutput,
        fund: &FundBrainOutput,
        merged_sentiment: Decimal,
        merged_confidence: Decimal,
        is_conflict: bool,
    ) -> ConsensusAction {
        // If in conflict and debate is needed
        if is_conflict && merged_confidence < dec!(0.7) {
            return ConsensusAction::Debate;
        }

        // Check minimum confidence
        if merged_confidence < self.config.min_confidence {
            return ConsensusAction::Wait;
        }

        // Check fund brain security
        if fund.recommendation.action == FundAction::Block {
            return ConsensusAction::Blocked;
        }

        // Determine action from merged sentiment
        // Paper trading: lowered threshold to 0.15 for more active trading
        if merged_sentiment > dec!(0.15) {
            ConsensusAction::Long
        } else if merged_sentiment < dec!(-0.15) {
            ConsensusAction::Short
        } else if merged_sentiment.abs() < dec!(0.05) {
            ConsensusAction::Wait
        } else {
            ConsensusAction::Hold
        }
    }

    /// Calculate position size factor
    fn calculate_size_factor(
        &self,
        ta: &TABrainOutput,
        fund: &FundBrainOutput,
        is_conflict: bool,
        merged_confidence: Decimal,
    ) -> Decimal {
        let mut size = dec!(1.0);

        // Start with TA recommendation size
        size *= ta.recommended_action.size_factor;

        // Apply fund recommendation modifier
        size *= fund.recommendation.size_modifier;

        // Apply confidence scaling
        size *= merged_confidence;

        // Reduce on conflict
        if is_conflict && self.config.conflict_reduces_size {
            size *= self.config.conflict_size_multiplier;
            info!("Size reduced due to brain conflict: {:.1}%", size * dec!(100));
        }

        // Apply security warning reduction
        if !fund.recommendation.security_warnings.is_empty() {
            size *= dec!(0.7); // 30% reduction for security warnings
        }

        size.min(Decimal::ONE).max(Decimal::ZERO)
    }

    /// Update weights based on performance (adaptive weighting)
    pub fn update_weights(&mut self, ta_win_rate: Decimal, fund_win_rate: Decimal) {
        if !self.config.adaptive_weights {
            return;
        }

        // Calculate performance-based weights
        let total_wr = ta_win_rate + fund_win_rate;
        if total_wr.is_zero() {
            return;
        }

        // New weights based on relative performance
        let new_ta_weight = ta_win_rate / total_wr;
        let new_fund_weight = fund_win_rate / total_wr;

        // Smooth transition (90% old, 10% new)
        self.effective_ta_weight = self.effective_ta_weight * dec!(0.9) + new_ta_weight * dec!(0.1);
        self.effective_fund_weight = self.effective_fund_weight * dec!(0.9) + new_fund_weight * dec!(0.1);

        // Ensure weights sum to 1
        let total = self.effective_ta_weight + self.effective_fund_weight;
        self.effective_ta_weight /= total;
        self.effective_fund_weight /= total;

        // Clamp to reasonable bounds (don't let either dominate completely)
        self.effective_ta_weight = self.effective_ta_weight.max(dec!(0.3)).min(dec!(0.7));
        self.effective_fund_weight = dec!(1.0) - self.effective_ta_weight;

        info!(
            "Updated weights: TA {:.1}%, Fund {:.1}%",
            self.effective_ta_weight * dec!(100),
            self.effective_fund_weight * dec!(100)
        );
    }

    /// Record trade outcome for performance tracking
    pub fn record_outcome(&mut self, ta_agreed: bool, fund_agreed: bool, is_win: bool) {
        if ta_agreed {
            self.ta_performance.record(is_win);
        }
        if fund_agreed {
            self.fund_performance.record(is_win);
        }

        // Update adaptive weights periodically
        if self.config.adaptive_weights {
            self.update_weights(
                self.ta_performance.recent_win_rate(),
                self.fund_performance.recent_win_rate(),
            );
        }
    }

    /// Get current effective weights
    pub fn get_weights(&self) -> (Decimal, Decimal) {
        (self.effective_ta_weight, self.effective_fund_weight)
    }

    /// Get brain performance stats
    pub fn get_performance(&self) -> (&BrainPerformance, &BrainPerformance) {
        (&self.ta_performance, &self.fund_performance)
    }

    /// Reset performance tracking
    pub fn reset_performance(&mut self) {
        self.ta_performance = BrainPerformance::default();
        self.fund_performance = BrainPerformance::default();
        self.effective_ta_weight = self.config.ta_weight;
        self.effective_fund_weight = self.config.fund_weight;
    }
}

/// Serialize merged context to JSON for Claude
impl MergedContext {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Generate summary for Claude
    pub fn summary(&self) -> String {
        let action_emoji = match self.consensus_action {
            ConsensusAction::Long => "🟢",
            ConsensusAction::Short => "🔴",
            ConsensusAction::Hold => "🟡",
            ConsensusAction::Wait => "⏸️",
            ConsensusAction::Close => "🔻",
            ConsensusAction::Blocked => "🚫",
            ConsensusAction::Debate => "🤔",
        };

        let conflict_status = if self.is_conflict {
            format!("⚠️ CONFLICT: {}", self.conflict_reason.as_deref().unwrap_or("unknown"))
        } else {
            "✅ Aligned".to_string()
        };

        format!(
            r#"## Dual Brain Analysis: {}

**Consensus:** {} {:?}
**Sentiment:** TA {:.2} | Fund {:.2} | Merged {:.2}
**Confidence:** {:.0}%
**Size Factor:** {:.0}%
**Regime:** {:?}

**Brain Status:** {}

### TA Brain
- RADAR: {} ({:?})
- PULSE: Tier {} ({:?})
- Recommendation: {:?}

### Fund Brain
- Whale Sentiment: {:.2}
- Twitter Sentiment: {:.2}
- News Sentiment: {:.2}
- Security: {}
- Recommendation: {:?}
"#,
            self.symbol,
            action_emoji,
            self.consensus_action,
            self.ta.overall_sentiment,
            self.fund.overall_sentiment,
            self.merged_sentiment,
            self.merged_confidence * dec!(100),
            self.size_factor * dec!(100),
            self.regime,
            conflict_status,
            self.ta.radar_score.score,
            self.ta.radar_score.tier,
            self.ta.pulse_signal.tier,
            self.ta.pulse_signal.direction,
            self.ta.recommended_action.action,
            self.fund.whale_sentiment,
            self.fund.twitter_sentiment,
            self.fund.news_sentiment,
            self.fund.security.as_ref().map_or("Not scanned".to_string(), |s| {
                if s.is_safe() { "✅ Safe".to_string() }
                else if s.should_warn() { format!("⚠️ Warning: {:?}", s.red_flags) }
                else { format!("🚫 Blocked: {:?}", s.red_flags) }
            }),
            self.fund.recommendation.action,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fund_brain::{FundBrainOutput, FundRecommendation, SecurityAssessment};
    use crate::ta_brain::{
        GuardState, PulseDirection, PulseSignal, RadarScore, RadarTier, TABrainOutput,
        TARecommendation, TAStrategySignal,
    };

    fn make_ta_output(sentiment: Decimal, confidence: Decimal, action: TAAction) -> TABrainOutput {
        TABrainOutput {
            symbol: "BTC".to_string(),
            radar_score: RadarScore {
                score: 200,
                market_structure: 70,
                technicals: 80,
                funding: 50,
                tier: RadarTier::Solid,
                recommended_strategy: Some("momentum".to_string()),
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
            overall_confidence: confidence,
            recommended_action: TARecommendation {
                action,
                strategy: None,
                size_factor: dec!(0.8),
                confidence,
                reasoning: "Test".to_string(),
            },
            timestamp: Utc::now(),
        }
    }

    fn make_fund_output(sentiment: Decimal, confidence: Decimal, action: FundAction) -> FundBrainOutput {
        FundBrainOutput {
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
            overall_confidence: confidence,
            recommendation: FundRecommendation {
                action,
                confidence,
                size_modifier: dec!(0.9),
                reasoning: "Test".to_string(),
                security_warnings: vec![],
            },
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_merge_aligned_bullish() {
        let engine = ConsensusEngine::with_defaults();
        let ta = make_ta_output(dec!(0.7), dec!(0.8), TAAction::Long);
        let fund = make_fund_output(dec!(0.6), dec!(0.75), FundAction::Bullish);

        let merged = engine.merge(&ta, &fund);

        assert_eq!(merged.consensus_action, ConsensusAction::Long);
        assert!(!merged.is_conflict);
        assert!(merged.merged_sentiment > dec!(0.5));
    }

    #[test]
    fn test_merge_conflict() {
        let engine = ConsensusEngine::with_defaults();
        let ta = make_ta_output(dec!(0.7), dec!(0.8), TAAction::Long);
        let fund = make_fund_output(dec!(-0.5), dec!(0.75), FundAction::Bearish);

        let merged = engine.merge(&ta, &fund);

        assert!(merged.is_conflict);
        assert!(merged.conflict_reason.is_some());
        // Size should be reduced due to conflict
        assert!(merged.size_factor < dec!(0.5));
    }

    #[test]
    fn test_security_block() {
        let engine = ConsensusEngine::with_defaults();
        let ta = make_ta_output(dec!(0.7), dec!(0.8), TAAction::Long);
        let mut fund = make_fund_output(dec!(0.6), dec!(0.75), FundAction::Bullish);

        // Add security block
        fund.security = Some(SecurityAssessment {
            token: "BTC".to_string(),
            chain: "ethereum".to_string(),
            is_honeypot: true,
            buy_tax: Some(dec!(50)),
            sell_tax: Some(dec!(90)),
            can_take_ownership: true,
            can_change_balance: false,
            is_mintable: false,
            liquidity_usd: Some(dec!(1000)),
            risk_score: 95,
            red_flags: vec!["Honeypot".to_string()],
            timestamp: Utc::now(),
        });

        let merged = engine.merge(&ta, &fund);

        assert_eq!(merged.consensus_action, ConsensusAction::Blocked);
        assert_eq!(merged.size_factor, Decimal::ZERO);
    }

    #[test]
    fn test_adaptive_weights() {
        let mut engine = ConsensusEngine::with_defaults();

        // Record TA performing better
        for _ in 0..8 {
            engine.record_outcome(true, false, true); // TA agreed and won
        }
        for _ in 0..2 {
            engine.record_outcome(false, true, false); // Fund agreed and lost
        }

        let (ta_weight, fund_weight) = engine.get_weights();

        // TA should have higher weight now
        assert!(ta_weight > dec!(0.5));
        assert!(fund_weight < dec!(0.5));
    }
}
