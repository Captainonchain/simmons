//! Technical Analysis Brain - Nunchi Strategy Suite Integration
//!
//! Implements RADAR (opportunity screening), PULSE (momentum detection),
//! GUARD (trailing stops), and orchestrates 14 trading strategies.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::{Regime, Signal, StrategySignal};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// RADAR opportunity score (0-400)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarScore {
    /// Total score (0-400)
    pub score: u16,
    /// Market structure contribution (0-140)
    pub market_structure: u16,
    /// Technicals contribution (0-160)
    pub technicals: u16,
    /// Funding contribution (0-100)
    pub funding: u16,
    /// Priority tier
    pub tier: RadarTier,
    /// Recommended strategy
    pub recommended_strategy: Option<String>,
    /// Symbol being analyzed
    pub symbol: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl RadarScore {
    /// Is this an elite opportunity (250+)?
    pub fn is_elite(&self) -> bool {
        self.score >= 250
    }

    /// Is this a solid opportunity (170+)?
    pub fn is_solid(&self) -> bool {
        self.score >= 170
    }

    /// Is this marginal (140-170)?
    pub fn is_marginal(&self) -> bool {
        self.score >= 140 && self.score < 170
    }

    /// Should skip (< 140)?
    pub fn should_skip(&self) -> bool {
        self.score < 140
    }
}

/// RADAR priority tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadarTier {
    /// Score 250-400: Immediate entry
    Elite,
    /// Score 170-250: Entry with PULSE confirmation
    Solid,
    /// Score 140-170: Queue only
    Marginal,
    /// Score < 140: Skip
    Skip,
}

impl RadarTier {
    pub fn from_score(score: u16) -> Self {
        match score {
            250..=400 => RadarTier::Elite,
            170..=249 => RadarTier::Solid,
            140..=169 => RadarTier::Marginal,
            _ => RadarTier::Skip,
        }
    }
}

/// PULSE momentum signal (6 tiers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseSignal {
    /// Pulse tier (1-6)
    pub tier: u8,
    /// Immediate mover score (0-100)
    pub immediate_mover: u8,
    /// Volume surge score (0-100)
    pub volume_surge: u8,
    /// Momentum direction
    pub direction: PulseDirection,
    /// Strength (0.0-1.0)
    pub strength: Decimal,
    /// Symbol
    pub symbol: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl PulseSignal {
    /// Is this a strong pulse (tier 4+)?
    pub fn is_strong(&self) -> bool {
        self.tier >= 4
    }

    /// Is this an immediate mover?
    pub fn is_immediate_mover(&self) -> bool {
        self.immediate_mover >= 80
    }

    /// Is there a volume surge?
    pub fn has_volume_surge(&self) -> bool {
        self.volume_surge >= 70
    }
}

/// PULSE direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PulseDirection {
    Up,
    Down,
    Neutral,
}

/// GUARD trailing stop state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardState {
    /// Position ID
    pub position_id: String,
    /// Symbol
    pub symbol: String,
    /// Current phase (1 or 2)
    pub phase: u8,
    /// Current trailing stop price
    pub stop_price: Decimal,
    /// High watermark price
    pub high_watermark: Decimal,
    /// Number of phase breaches
    pub breach_count: u8,
    /// Entry price
    pub entry_price: Decimal,
    /// Current ROE %
    pub roe_pct: Decimal,
    /// Stagnation timer (minutes at current level)
    pub stagnation_minutes: u32,
    /// Timestamp
    pub last_update: DateTime<Utc>,
}

impl GuardState {
    /// Create new GUARD state for a position
    pub fn new(position_id: String, symbol: String, entry_price: Decimal) -> Self {
        Self {
            position_id,
            symbol,
            phase: 1,
            stop_price: entry_price * dec!(0.97), // 3% initial stop
            high_watermark: entry_price,
            breach_count: 0,
            entry_price,
            roe_pct: Decimal::ZERO,
            stagnation_minutes: 0,
            last_update: Utc::now(),
        }
    }

    /// Update GUARD state with new price
    pub fn update(&mut self, current_price: Decimal, config: &GuardConfig) {
        // Update ROE
        self.roe_pct = ((current_price - self.entry_price) / self.entry_price) * dec!(100);
        self.last_update = Utc::now();

        // Update high watermark
        if current_price > self.high_watermark {
            self.high_watermark = current_price;
            self.stagnation_minutes = 0;
        } else {
            self.stagnation_minutes += 1;
        }

        // Phase-specific trailing logic
        let retrace_pct = match self.phase {
            1 => config.phase1_retrace,
            2 => config.phase2_retrace,
            _ => config.phase2_retrace,
        };

        // Calculate new trailing stop
        let new_stop = self.high_watermark * (Decimal::ONE - retrace_pct);
        if new_stop > self.stop_price {
            self.stop_price = new_stop;
        }

        // Check for phase transition (phase 1 -> 2 at 8% ROE)
        if self.phase == 1 && self.roe_pct >= dec!(8) {
            self.phase = 2;
            self.breach_count = 0;
            info!(
                "GUARD phase transition: {} moved to phase 2 at {:.1}% ROE",
                self.symbol, self.roe_pct
            );
        }
    }

    /// Check if stop is triggered
    pub fn is_triggered(&self, current_price: Decimal) -> bool {
        current_price <= self.stop_price
    }

    /// Check for stagnation exit
    pub fn should_stagnation_exit(&self, config: &GuardConfig) -> bool {
        self.stagnation_minutes >= config.stagnation_timeout_mins
            && self.roe_pct >= config.stagnation_min_roe
    }
}

/// GUARD configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    /// Phase 1 retrace percentage (default 3%)
    pub phase1_retrace: Decimal,
    /// Phase 1 max breaches before exit
    pub phase1_max_breaches: u8,
    /// Phase 2 retrace percentage (default 1.5%)
    pub phase2_retrace: Decimal,
    /// Phase 2 max breaches before exit
    pub phase2_max_breaches: u8,
    /// Stagnation timeout in minutes
    pub stagnation_timeout_mins: u32,
    /// Minimum ROE for stagnation exit
    pub stagnation_min_roe: Decimal,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            phase1_retrace: dec!(0.03),
            phase1_max_breaches: 3,
            phase2_retrace: dec!(0.015),
            phase2_max_breaches: 2,
            stagnation_timeout_mins: 60,
            stagnation_min_roe: dec!(8),
        }
    }
}

/// Strategy types available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyType {
    // Market Making (6)
    EngineMm,
    AvellanedaMm,
    RegimeMm,
    SimpleMm,
    GridMm,
    LiquidationMm,
    // Arbitrage (2)
    FundingArb,
    BasisArb,
    // Directional (3)
    MomentumBreakout,
    MeanReversion,
    AggressiveTaker,
    // Infrastructure (3)
    HedgeAgent,
    RfqAgent,
    ClaudeAgent,
}

impl StrategyType {
    /// Get all market making strategies
    pub fn market_making() -> Vec<Self> {
        vec![
            Self::EngineMm,
            Self::AvellanedaMm,
            Self::RegimeMm,
            Self::SimpleMm,
            Self::GridMm,
            Self::LiquidationMm,
        ]
    }

    /// Get all arbitrage strategies
    pub fn arbitrage() -> Vec<Self> {
        vec![Self::FundingArb, Self::BasisArb]
    }

    /// Get all directional strategies
    pub fn directional() -> Vec<Self> {
        vec![Self::MomentumBreakout, Self::MeanReversion, Self::AggressiveTaker]
    }

    /// Get strategy name
    pub fn name(&self) -> &'static str {
        match self {
            Self::EngineMm => "engine_mm",
            Self::AvellanedaMm => "avellaneda_mm",
            Self::RegimeMm => "regime_mm",
            Self::SimpleMm => "simple_mm",
            Self::GridMm => "grid_mm",
            Self::LiquidationMm => "liquidation_mm",
            Self::FundingArb => "funding_arb",
            Self::BasisArb => "basis_arb",
            Self::MomentumBreakout => "momentum_breakout",
            Self::MeanReversion => "mean_reversion",
            Self::AggressiveTaker => "aggressive_taker",
            Self::HedgeAgent => "hedge_agent",
            Self::RfqAgent => "rfq_agent",
            Self::ClaudeAgent => "claude_agent",
        }
    }
}

/// Strategy signal from TA Brain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TAStrategySignal {
    /// Strategy type
    pub strategy: StrategyType,
    /// Signal direction
    pub signal: Signal,
    /// Confidence (0-1)
    pub confidence: Decimal,
    /// Reason for signal
    pub reason: String,
    /// Suggested entry price
    pub entry_price: Option<Decimal>,
    /// Suggested stop loss
    pub stop_loss: Option<Decimal>,
    /// Suggested take profit
    pub take_profit: Option<Decimal>,
}

/// TA Brain output for consensus layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TABrainOutput {
    /// Symbol analyzed
    pub symbol: String,
    /// RADAR score (0-400)
    pub radar_score: RadarScore,
    /// PULSE signal
    pub pulse_signal: PulseSignal,
    /// Current regime
    pub regime: Regime,
    /// Strategy signals
    pub strategy_signals: Vec<TAStrategySignal>,
    /// GUARD states for active positions
    pub guard_states: Vec<GuardState>,
    /// Overall sentiment (-1 to +1)
    pub overall_sentiment: Decimal,
    /// Overall confidence (0-1)
    pub overall_confidence: Decimal,
    /// Recommended action
    pub recommended_action: TARecommendation,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// TA Brain recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TARecommendation {
    /// Action type
    pub action: TAAction,
    /// Recommended strategy
    pub strategy: Option<StrategyType>,
    /// Position size factor (0-1)
    pub size_factor: Decimal,
    /// Confidence
    pub confidence: Decimal,
    /// Reasoning
    pub reasoning: String,
}

/// TA action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TAAction {
    /// Open long position
    Long,
    /// Open short position
    Short,
    /// Hold current position
    Hold,
    /// Close position
    Close,
    /// No action - wait
    Wait,
}

/// Technical Analysis Brain
pub struct TABrain {
    /// Configuration
    config: TABrainConfig,
    /// GUARD states for active positions
    guard_states: HashMap<String, GuardState>,
    /// Recent RADAR scores
    recent_radar: Vec<RadarScore>,
    /// Recent PULSE signals
    recent_pulse: Vec<PulseSignal>,
}

/// TA Brain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TABrainConfig {
    /// Update interval in seconds
    pub update_interval_secs: u64,
    /// RADAR elite threshold
    pub radar_elite_threshold: u16,
    /// RADAR solid threshold
    pub radar_solid_threshold: u16,
    /// RADAR marginal threshold
    pub radar_marginal_threshold: u16,
    /// PULSE interval in seconds
    pub pulse_interval_secs: u64,
    /// PULSE immediate mover threshold
    pub pulse_immediate_threshold: u8,
    /// PULSE volume surge threshold
    pub pulse_volume_threshold: u8,
    /// GUARD configuration
    pub guard_config: GuardConfig,
    /// Enabled strategies
    pub enabled_strategies: Vec<StrategyType>,
    /// Max concurrent positions
    pub max_positions: usize,
    /// Leverage
    pub leverage: u8,
}

impl Default for TABrainConfig {
    fn default() -> Self {
        Self {
            update_interval_secs: 10,
            radar_elite_threshold: 250,
            radar_solid_threshold: 170,
            radar_marginal_threshold: 140,
            pulse_interval_secs: 60,
            pulse_immediate_threshold: 80,
            pulse_volume_threshold: 70,
            guard_config: GuardConfig::default(),
            enabled_strategies: vec![
                StrategyType::EngineMm,
                StrategyType::AvellanedaMm,
                StrategyType::RegimeMm,
                StrategyType::MomentumBreakout,
                StrategyType::MeanReversion,
                StrategyType::FundingArb,
                StrategyType::BasisArb,
            ],
            max_positions: 3,
            leverage: 10,
        }
    }
}

impl TABrain {
    /// Create new TA Brain
    pub fn new(config: TABrainConfig) -> Self {
        Self {
            config,
            guard_states: HashMap::new(),
            recent_radar: Vec::new(),
            recent_pulse: Vec::new(),
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(TABrainConfig::default())
    }

    /// Calculate RADAR score for a symbol
    pub fn calculate_radar(
        &self,
        symbol: &str,
        prices: &[Decimal],
        volume: Decimal,
        open_interest: Option<Decimal>,
        funding_rate: Option<Decimal>,
        regime: Regime,
    ) -> RadarScore {
        // Market Structure (35% = 140 points)
        let market_structure = self.calculate_market_structure(volume, open_interest);

        // Technicals (40% = 160 points)
        let technicals = self.calculate_technicals(prices, regime);

        // Funding (25% = 100 points)
        let funding = self.calculate_funding_score(funding_rate);

        let score = market_structure + technicals + funding;
        let tier = RadarTier::from_score(score);

        // Recommend strategy based on conditions
        let recommended_strategy = self.recommend_strategy(regime, funding_rate);

        RadarScore {
            score,
            market_structure,
            technicals,
            funding,
            tier,
            recommended_strategy,
            symbol: symbol.to_string(),
            timestamp: Utc::now(),
        }
    }

    /// Calculate market structure contribution (0-140)
    fn calculate_market_structure(&self, volume: Decimal, open_interest: Option<Decimal>) -> u16 {
        let mut score: u16 = 0;

        // Volume tiers (0-60)
        let volume_score = if volume > dec!(100_000_000) {
            60
        } else if volume > dec!(50_000_000) {
            50
        } else if volume > dec!(10_000_000) {
            40
        } else if volume > dec!(1_000_000) {
            30
        } else {
            15
        };
        score += volume_score;

        // OI surge detection (0-50)
        if let Some(oi) = open_interest {
            let oi_score = if oi > dec!(500_000_000) {
                50
            } else if oi > dec!(100_000_000) {
                40
            } else if oi > dec!(50_000_000) {
                30
            } else {
                20
            };
            score += oi_score;
        } else {
            score += 25; // Default if no OI data
        }

        // Depth/liquidity bonus (0-30)
        // In production, this would check order book depth
        score += 20; // Default reasonable depth

        score.min(140)
    }

    /// Calculate technicals contribution (0-160)
    fn calculate_technicals(&self, prices: &[Decimal], regime: Regime) -> u16 {
        let mut score: u16 = 0;

        if prices.len() < 14 {
            return 80; // Not enough data, return neutral
        }

        // Trend analysis (0-70)
        let trend_score = self.calculate_trend_score(prices);
        score += trend_score;

        // RSI analysis (0-50)
        let rsi_score = self.calculate_rsi_score(prices);
        score += rsi_score;

        // Pattern detection (0-40)
        let pattern_score = self.calculate_pattern_score(prices, regime);
        score += pattern_score;

        score.min(160)
    }

    /// Calculate trend score (0-70)
    fn calculate_trend_score(&self, prices: &[Decimal]) -> u16 {
        if prices.len() < 20 {
            return 35;
        }

        // Simple: price above/below 20-period moving average
        let sma20: Decimal = prices.iter().rev().take(20).sum::<Decimal>() / dec!(20);
        let current = *prices.last().unwrap();

        let deviation = ((current - sma20) / sma20) * dec!(100);

        if deviation.abs() > dec!(5) {
            70 // Strong trend
        } else if deviation.abs() > dec!(2) {
            50 // Moderate trend
        } else {
            30 // Weak trend
        }
    }

    /// Calculate RSI score (0-50)
    fn calculate_rsi_score(&self, prices: &[Decimal]) -> u16 {
        if prices.len() < 15 {
            return 25;
        }

        // Calculate RSI
        let mut gains = Decimal::ZERO;
        let mut losses = Decimal::ZERO;
        let period = 14;

        for i in (prices.len() - period)..prices.len() {
            let change = prices[i] - prices[i - 1];
            if change > Decimal::ZERO {
                gains += change;
            } else {
                losses += change.abs();
            }
        }

        let avg_gain = gains / Decimal::from(period);
        let avg_loss = losses / Decimal::from(period);

        let rsi = if avg_loss.is_zero() {
            dec!(100)
        } else {
            let rs = avg_gain / avg_loss;
            dec!(100) - (dec!(100) / (Decimal::ONE + rs))
        };

        // Score based on RSI extremes
        if rsi < dec!(30) || rsi > dec!(70) {
            50 // Oversold/overbought - potential reversal
        } else if rsi < dec!(40) || rsi > dec!(60) {
            35 // Approaching extremes
        } else {
            20 // Neutral
        }
    }

    /// Calculate pattern score (0-40)
    fn calculate_pattern_score(&self, prices: &[Decimal], regime: Regime) -> u16 {
        // Regime-adjusted pattern detection
        match regime {
            Regime::TrendingUp | Regime::TrendingDown => 40, // Strong patterns in trends
            Regime::MeanReverting => 30,
            Regime::HighVolatility => 20,
            Regime::LowVolatility => 25,
            Regime::Choppy => 10, // Patterns unreliable
        }
    }

    /// Calculate funding contribution (0-100)
    fn calculate_funding_score(&self, funding_rate: Option<Decimal>) -> u16 {
        match funding_rate {
            Some(rate) => {
                let rate_bps = rate * dec!(10000);
                if rate_bps.abs() > dec!(50) {
                    100 // Extreme funding - arb opportunity
                } else if rate_bps.abs() > dec!(20) {
                    70 // Elevated funding
                } else if rate_bps.abs() > dec!(10) {
                    50 // Normal
                } else {
                    30 // Low funding
                }
            }
            None => 50, // Default if no funding data
        }
    }

    /// Recommend strategy based on conditions
    fn recommend_strategy(&self, regime: Regime, funding_rate: Option<Decimal>) -> Option<String> {
        // Check for funding arb opportunity
        if let Some(rate) = funding_rate {
            if (rate * dec!(10000)).abs() > dec!(30) {
                return Some("funding_arb".to_string());
            }
        }

        // Regime-based strategy selection
        match regime {
            Regime::TrendingUp | Regime::TrendingDown => Some("momentum_breakout".to_string()),
            Regime::MeanReverting => Some("mean_reversion".to_string()),
            Regime::HighVolatility => Some("regime_mm".to_string()),
            Regime::LowVolatility => Some("avellaneda_mm".to_string()),
            Regime::Choppy => None, // Avoid trading
        }
    }

    /// Calculate PULSE signal
    pub fn calculate_pulse(
        &self,
        symbol: &str,
        prices: &[Decimal],
        volumes: &[Decimal],
    ) -> PulseSignal {
        // Calculate immediate mover score
        let immediate_mover = self.calculate_immediate_mover(prices);

        // Calculate volume surge
        let volume_surge = self.calculate_volume_surge(volumes);

        // Determine direction
        let direction = if prices.len() >= 2 {
            let last = prices[prices.len() - 1];
            let prev = prices[prices.len() - 2];
            if last > prev * dec!(1.001) {
                PulseDirection::Up
            } else if last < prev * dec!(0.999) {
                PulseDirection::Down
            } else {
                PulseDirection::Neutral
            }
        } else {
            PulseDirection::Neutral
        };

        // Calculate tier (1-6)
        let tier = self.calculate_pulse_tier(immediate_mover, volume_surge);

        // Calculate strength
        let strength = Decimal::from(immediate_mover + volume_surge) / dec!(200);

        PulseSignal {
            tier,
            immediate_mover,
            volume_surge,
            direction,
            strength,
            symbol: symbol.to_string(),
            timestamp: Utc::now(),
        }
    }

    /// Calculate immediate mover score (0-100)
    fn calculate_immediate_mover(&self, prices: &[Decimal]) -> u8 {
        if prices.len() < 5 {
            return 50;
        }

        let recent: Vec<&Decimal> = prices.iter().rev().take(5).collect();
        let oldest = *recent.last().unwrap();
        let newest = *recent.first().unwrap();

        let change_pct = ((*newest - *oldest) / *oldest).abs() * dec!(100);

        if change_pct > dec!(2) {
            100
        } else if change_pct > dec!(1) {
            80
        } else if change_pct > dec!(0.5) {
            60
        } else {
            40
        }
    }

    /// Calculate volume surge score (0-100)
    fn calculate_volume_surge(&self, volumes: &[Decimal]) -> u8 {
        if volumes.len() < 10 {
            return 50;
        }

        let recent_avg: Decimal = volumes.iter().rev().take(5).sum::<Decimal>() / dec!(5);
        let baseline_avg: Decimal = volumes.iter().sum::<Decimal>() / Decimal::from(volumes.len());

        if baseline_avg.is_zero() {
            return 50;
        }

        let surge_ratio = recent_avg / baseline_avg;

        if surge_ratio > dec!(3) {
            100
        } else if surge_ratio > dec!(2) {
            80
        } else if surge_ratio > dec!(1.5) {
            60
        } else {
            40
        }
    }

    /// Calculate PULSE tier (1-6)
    fn calculate_pulse_tier(&self, immediate: u8, volume: u8) -> u8 {
        let combined = (immediate as u16 + volume as u16) / 2;

        match combined {
            90..=100 => 6, // Extreme momentum
            75..=89 => 5,  // Strong momentum
            60..=74 => 4,  // Moderate momentum
            45..=59 => 3,  // Mild momentum
            30..=44 => 2,  // Weak momentum
            _ => 1,        // No momentum
        }
    }

    /// Register a new position for GUARD tracking
    pub fn register_position(&mut self, position_id: &str, symbol: &str, entry_price: Decimal) {
        let guard = GuardState::new(position_id.to_string(), symbol.to_string(), entry_price);
        self.guard_states.insert(position_id.to_string(), guard);
        info!("GUARD registered position {} at {}", position_id, entry_price);
    }

    /// Update GUARD state for a position
    pub fn update_guard(&mut self, position_id: &str, current_price: Decimal) -> Option<bool> {
        if let Some(guard) = self.guard_states.get_mut(position_id) {
            guard.update(current_price, &self.config.guard_config);

            if guard.is_triggered(current_price) {
                info!(
                    "GUARD triggered for {}: stop at {} hit by {}",
                    position_id, guard.stop_price, current_price
                );
                return Some(true);
            }

            if guard.should_stagnation_exit(&self.config.guard_config) {
                info!(
                    "GUARD stagnation exit for {}: {} minutes at {:.1}% ROE",
                    position_id, guard.stagnation_minutes, guard.roe_pct
                );
                return Some(true);
            }

            Some(false)
        } else {
            None
        }
    }

    /// Remove position from GUARD tracking
    pub fn unregister_position(&mut self, position_id: &str) {
        self.guard_states.remove(position_id);
    }

    /// Get all GUARD states
    pub fn get_guard_states(&self) -> Vec<GuardState> {
        self.guard_states.values().cloned().collect()
    }

    /// Generate strategy signals
    pub fn generate_signals(
        &self,
        symbol: &str,
        prices: &[Decimal],
        regime: Regime,
        funding_rate: Option<Decimal>,
    ) -> Vec<TAStrategySignal> {
        let mut signals = Vec::new();

        for strategy in &self.config.enabled_strategies {
            if let Some(signal) = self.generate_strategy_signal(*strategy, symbol, prices, regime, funding_rate) {
                signals.push(signal);
            }
        }

        signals
    }

    /// Generate signal for a specific strategy
    fn generate_strategy_signal(
        &self,
        strategy: StrategyType,
        symbol: &str,
        prices: &[Decimal],
        regime: Regime,
        funding_rate: Option<Decimal>,
    ) -> Option<TAStrategySignal> {
        match strategy {
            StrategyType::MomentumBreakout => self.momentum_breakout_signal(prices, regime),
            StrategyType::MeanReversion => self.mean_reversion_signal(prices, regime),
            StrategyType::FundingArb => self.funding_arb_signal(funding_rate),
            StrategyType::BasisArb => self.basis_arb_signal(funding_rate),
            StrategyType::EngineMm | StrategyType::AvellanedaMm | StrategyType::RegimeMm => {
                self.market_making_signal(strategy, prices, regime)
            }
            _ => None,
        }
    }

    /// Momentum breakout signal
    fn momentum_breakout_signal(&self, prices: &[Decimal], regime: Regime) -> Option<TAStrategySignal> {
        if prices.len() < 20 {
            return None;
        }

        // Only in trending regimes
        if !matches!(regime, Regime::TrendingUp | Regime::TrendingDown) {
            return None;
        }

        let current = *prices.last()?;
        let high_20 = prices.iter().rev().take(20).max()?;
        let low_20 = prices.iter().rev().take(20).min()?;

        let breakout_threshold = dec!(0.02); // 2% from high/low

        let (signal, reason) = if current > *high_20 * (Decimal::ONE - breakout_threshold) {
            (Signal::Buy, format!("Breakout above 20-period high {}", high_20))
        } else if current < *low_20 * (Decimal::ONE + breakout_threshold) {
            (Signal::Sell, format!("Breakdown below 20-period low {}", low_20))
        } else {
            (Signal::Hold, "No breakout detected".to_string())
        };

        Some(TAStrategySignal {
            strategy: StrategyType::MomentumBreakout,
            signal,
            confidence: dec!(0.7),
            reason,
            entry_price: Some(current),
            stop_loss: Some(current * dec!(0.97)),
            take_profit: Some(current * dec!(1.05)),
        })
    }

    /// Mean reversion signal
    fn mean_reversion_signal(&self, prices: &[Decimal], regime: Regime) -> Option<TAStrategySignal> {
        if prices.len() < 20 {
            return None;
        }

        // Best in mean-reverting regime
        if !matches!(regime, Regime::MeanReverting | Regime::LowVolatility) {
            return None;
        }

        let sma20: Decimal = prices.iter().rev().take(20).sum::<Decimal>() / dec!(20);
        let current = *prices.last()?;

        // Calculate standard deviation
        let variance: Decimal = prices
            .iter()
            .rev()
            .take(20)
            .map(|p| {
                let diff = *p - sma20;
                diff * diff
            })
            .sum::<Decimal>()
            / dec!(20);

        // Approximate sqrt using Newton's method
        let std_dev = if variance > Decimal::ZERO {
            // Newton-Raphson approximation: start with variance/2, iterate
            let mut x = variance / dec!(2);
            for _ in 0..10 {
                if x == Decimal::ZERO {
                    break;
                }
                x = (x + variance / x) / dec!(2);
            }
            x
        } else {
            dec!(1)
        };
        let z_score = if std_dev > Decimal::ZERO {
            (current - sma20) / std_dev
        } else {
            Decimal::ZERO
        };

        let (signal, reason, confidence) = if z_score < dec!(-2) {
            (
                Signal::StrongBuy,
                format!("Z-score {:.2} - oversold", z_score),
                dec!(0.85),
            )
        } else if z_score < dec!(-1.5) {
            (
                Signal::Buy,
                format!("Z-score {:.2} - approaching oversold", z_score),
                dec!(0.70),
            )
        } else if z_score > dec!(2) {
            (
                Signal::StrongSell,
                format!("Z-score {:.2} - overbought", z_score),
                dec!(0.85),
            )
        } else if z_score > dec!(1.5) {
            (
                Signal::Sell,
                format!("Z-score {:.2} - approaching overbought", z_score),
                dec!(0.70),
            )
        } else {
            (Signal::Hold, format!("Z-score {:.2} - neutral", z_score), dec!(0.50))
        };

        Some(TAStrategySignal {
            strategy: StrategyType::MeanReversion,
            signal,
            confidence,
            reason,
            entry_price: Some(current),
            stop_loss: Some(current * dec!(0.98)),
            take_profit: Some(sma20),
        })
    }

    /// Funding arbitrage signal
    fn funding_arb_signal(&self, funding_rate: Option<Decimal>) -> Option<TAStrategySignal> {
        let rate = funding_rate?;
        let rate_bps = rate * dec!(10000);

        if rate_bps.abs() < dec!(20) {
            return None; // Not enough edge
        }

        let (signal, reason) = if rate_bps > dec!(30) {
            (
                Signal::Sell,
                format!("High positive funding {:.1}bps - short perp, long spot", rate_bps),
            )
        } else if rate_bps < dec!(-30) {
            (
                Signal::Buy,
                format!("High negative funding {:.1}bps - long perp, short spot", rate_bps),
            )
        } else {
            (Signal::Hold, format!("Moderate funding {:.1}bps", rate_bps))
        };

        Some(TAStrategySignal {
            strategy: StrategyType::FundingArb,
            signal,
            confidence: dec!(0.80),
            reason,
            entry_price: None,
            stop_loss: None,
            take_profit: None,
        })
    }

    /// Basis arbitrage signal
    fn basis_arb_signal(&self, funding_rate: Option<Decimal>) -> Option<TAStrategySignal> {
        // Basis arb based on sustained funding extremes
        let rate = funding_rate?;
        let rate_bps = rate * dec!(10000);

        if rate_bps.abs() < dec!(50) {
            return None; // Need extreme funding for basis trade
        }

        let (signal, reason) = if rate_bps > dec!(50) {
            (
                Signal::Sell,
                format!("Contango {:.1}bps - short basis", rate_bps),
            )
        } else {
            (
                Signal::Buy,
                format!("Backwardation {:.1}bps - long basis", rate_bps),
            )
        };

        Some(TAStrategySignal {
            strategy: StrategyType::BasisArb,
            signal,
            confidence: dec!(0.75),
            reason,
            entry_price: None,
            stop_loss: None,
            take_profit: None,
        })
    }

    /// Market making signal
    fn market_making_signal(
        &self,
        strategy: StrategyType,
        prices: &[Decimal],
        regime: Regime,
    ) -> Option<TAStrategySignal> {
        // MM strategies signal based on regime suitability
        let (suitable, reason, confidence) = match strategy {
            StrategyType::AvellanedaMm => match regime {
                Regime::LowVolatility | Regime::MeanReverting => {
                    (true, "Low vol regime - optimal for Avellaneda MM", dec!(0.80))
                }
                Regime::HighVolatility => {
                    (false, "High vol - widen spreads or pause MM", dec!(0.40))
                }
                _ => (true, "Neutral regime for MM", dec!(0.60)),
            },
            StrategyType::RegimeMm => {
                // Regime MM adapts to all conditions
                (true, "Regime MM auto-adapts to current conditions", dec!(0.75))
            }
            StrategyType::EngineMm => match regime {
                Regime::Choppy => (false, "Choppy - pause engine MM", dec!(0.30)),
                _ => (true, "Engine MM active", dec!(0.70)),
            },
            _ => return None,
        };

        if !suitable {
            return Some(TAStrategySignal {
                strategy,
                signal: Signal::Hold,
                confidence,
                reason: reason.to_string(),
                entry_price: None,
                stop_loss: None,
                take_profit: None,
            });
        }

        Some(TAStrategySignal {
            strategy,
            signal: Signal::Buy, // MM is always "active"
            confidence,
            reason: reason.to_string(),
            entry_price: prices.last().copied(),
            stop_loss: None,
            take_profit: None,
        })
    }

    /// Analyze and produce full TA Brain output
    pub fn analyze(
        &mut self,
        symbol: &str,
        prices: &[Decimal],
        volumes: &[Decimal],
        regime: Regime,
        volume_24h: Decimal,
        open_interest: Option<Decimal>,
        funding_rate: Option<Decimal>,
    ) -> TABrainOutput {
        // Calculate RADAR
        let radar_score = self.calculate_radar(symbol, prices, volume_24h, open_interest, funding_rate, regime);
        self.recent_radar.push(radar_score.clone());
        if self.recent_radar.len() > 100 {
            self.recent_radar.remove(0);
        }

        // Calculate PULSE
        let pulse_signal = self.calculate_pulse(symbol, prices, volumes);
        self.recent_pulse.push(pulse_signal.clone());
        if self.recent_pulse.len() > 100 {
            self.recent_pulse.remove(0);
        }

        // Generate strategy signals
        let strategy_signals = self.generate_signals(symbol, prices, regime, funding_rate);

        // Get GUARD states
        let guard_states = self.get_guard_states();

        // Calculate overall sentiment
        let overall_sentiment = self.calculate_overall_sentiment(&strategy_signals);
        let overall_confidence = self.calculate_overall_confidence(&strategy_signals, &radar_score, &pulse_signal);

        // Generate recommendation
        let recommended_action = self.generate_recommendation(
            &radar_score,
            &pulse_signal,
            &strategy_signals,
            overall_sentiment,
            overall_confidence,
        );

        TABrainOutput {
            symbol: symbol.to_string(),
            radar_score,
            pulse_signal,
            regime,
            strategy_signals,
            guard_states,
            overall_sentiment,
            overall_confidence,
            recommended_action,
            timestamp: Utc::now(),
        }
    }

    /// Calculate overall sentiment from signals
    fn calculate_overall_sentiment(&self, signals: &[TAStrategySignal]) -> Decimal {
        if signals.is_empty() {
            return Decimal::ZERO;
        }

        let sum: Decimal = signals
            .iter()
            .map(|s| {
                let base = match s.signal {
                    Signal::StrongBuy => dec!(1.0),
                    Signal::Buy => dec!(0.5),
                    Signal::Hold => dec!(0.0),
                    Signal::Sell => dec!(-0.5),
                    Signal::StrongSell => dec!(-1.0),
                };
                base * s.confidence
            })
            .sum();

        sum / Decimal::from(signals.len())
    }

    /// Calculate overall confidence
    fn calculate_overall_confidence(
        &self,
        signals: &[TAStrategySignal],
        radar: &RadarScore,
        pulse: &PulseSignal,
    ) -> Decimal {
        let signal_confidence: Decimal = if signals.is_empty() {
            dec!(0.5)
        } else {
            signals.iter().map(|s| s.confidence).sum::<Decimal>() / Decimal::from(signals.len())
        };

        let radar_confidence = Decimal::from(radar.score) / dec!(400);
        let pulse_confidence = pulse.strength;

        // Weighted average
        (signal_confidence * dec!(0.5) + radar_confidence * dec!(0.3) + pulse_confidence * dec!(0.2))
    }

    /// Generate recommendation
    fn generate_recommendation(
        &self,
        radar: &RadarScore,
        pulse: &PulseSignal,
        signals: &[TAStrategySignal],
        sentiment: Decimal,
        confidence: Decimal,
    ) -> TARecommendation {
        // Skip if RADAR too low
        if radar.should_skip() {
            return TARecommendation {
                action: TAAction::Wait,
                strategy: None,
                size_factor: Decimal::ZERO,
                confidence: dec!(0.3),
                reasoning: format!("RADAR score {} below threshold", radar.score),
            };
        }

        // Determine action based on sentiment
        let action = if sentiment > dec!(0.3) {
            TAAction::Long
        } else if sentiment < dec!(-0.3) {
            TAAction::Short
        } else {
            TAAction::Hold
        };

        // Size factor based on RADAR tier
        let size_factor = match radar.tier {
            RadarTier::Elite => dec!(1.0),
            RadarTier::Solid => dec!(0.75),
            RadarTier::Marginal => dec!(0.5),
            RadarTier::Skip => dec!(0.0),
        };

        // Adjust for PULSE confirmation
        let size_factor = if pulse.is_strong() && pulse.direction != PulseDirection::Neutral {
            size_factor * dec!(1.2)
        } else {
            size_factor
        };

        // Find recommended strategy
        let strategy = signals
            .iter()
            .filter(|s| s.signal.is_bullish() == (action == TAAction::Long))
            .max_by_key(|s| s.confidence)
            .map(|s| s.strategy);

        let reasoning = format!(
            "RADAR {} ({:?}), PULSE tier {}, sentiment {:.2}, {} signals agree",
            radar.score,
            radar.tier,
            pulse.tier,
            sentiment,
            signals.iter().filter(|s| s.signal.is_bullish() == (sentiment > Decimal::ZERO)).count()
        );

        TARecommendation {
            action,
            strategy,
            size_factor: size_factor.min(dec!(1.0)),
            confidence,
            reasoning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_prices() -> Vec<Decimal> {
        (0..100)
            .map(|i| dec!(65000) + Decimal::from(i * 100))
            .collect()
    }

    fn sample_volumes() -> Vec<Decimal> {
        (0..100).map(|_| dec!(1000000)).collect()
    }

    #[test]
    fn test_radar_score() {
        let brain = TABrain::with_defaults();
        let radar = brain.calculate_radar(
            "BTC-USDT",
            &sample_prices(),
            dec!(50_000_000),
            Some(dec!(100_000_000)),
            Some(dec!(0.001)),
            Regime::TrendingUp,
        );

        assert!(radar.score > 0);
        assert!(radar.score <= 400);
        assert!(radar.market_structure <= 140);
        assert!(radar.technicals <= 160);
        assert!(radar.funding <= 100);
    }

    #[test]
    fn test_pulse_signal() {
        let brain = TABrain::with_defaults();
        let pulse = brain.calculate_pulse("BTC-USDT", &sample_prices(), &sample_volumes());

        assert!(pulse.tier >= 1 && pulse.tier <= 6);
        assert!(pulse.strength <= Decimal::ONE);
    }

    #[test]
    fn test_guard_state() {
        let mut guard = GuardState::new("test".to_string(), "BTC-USDT".to_string(), dec!(65000));
        let config = GuardConfig::default();

        // Price goes up
        guard.update(dec!(67000), &config);
        assert!(!guard.is_triggered(dec!(67000)));
        assert!(guard.roe_pct > Decimal::ZERO);

        // Price drops to stop
        guard.update(dec!(63000), &config);
        assert!(guard.is_triggered(dec!(63000)));
    }

    #[test]
    fn test_ta_brain_analyze() {
        let mut brain = TABrain::with_defaults();
        let output = brain.analyze(
            "BTC-USDT",
            &sample_prices(),
            &sample_volumes(),
            Regime::TrendingUp,
            dec!(50_000_000),
            Some(dec!(100_000_000)),
            Some(dec!(0.001)),
        );

        assert_eq!(output.symbol, "BTC-USDT");
        assert!(!output.strategy_signals.is_empty());
    }
}
