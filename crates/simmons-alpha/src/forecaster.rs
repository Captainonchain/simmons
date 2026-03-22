//! LLM + ML Forecasting
//!
//! Combines Claude reasoning with ML models for price and volatility forecasting.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use simmons_core::{MarketState, Regime};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Forecast horizon
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForecastHorizon {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
}

impl ForecastHorizon {
    pub fn to_secs(&self) -> u64 {
        match self {
            ForecastHorizon::OneMinute => 60,
            ForecastHorizon::FiveMinutes => 300,
            ForecastHorizon::FifteenMinutes => 900,
            ForecastHorizon::OneHour => 3600,
            ForecastHorizon::FourHours => 14400,
            ForecastHorizon::OneDay => 86400,
        }
    }
}

/// Direction forecast
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
    Sideways,
}

/// Price forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceForecast {
    pub symbol: String,
    pub current_price: Decimal,
    pub horizon: ForecastHorizon,
    pub direction: Direction,
    pub direction_confidence: Decimal,
    pub predicted_change_pct: Decimal,
    pub price_target: Decimal,
    pub support_level: Decimal,
    pub resistance_level: Decimal,
    pub reasoning: String,
    pub timestamp: i64,
}

/// Volatility forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilityForecast {
    pub symbol: String,
    pub current_volatility: Decimal,
    pub horizon: ForecastHorizon,
    pub predicted_volatility: Decimal,
    pub volatility_direction: Direction,
    pub confidence: Decimal,
    pub vol_regime: VolatilityRegime,
    pub reasoning: String,
    pub timestamp: i64,
}

/// Volatility regime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VolatilityRegime {
    Compression,
    Expansion,
    Stable,
    Extreme,
}

/// Regime transition forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeTransition {
    pub current_regime: Regime,
    pub most_likely_next: Regime,
    pub transition_probability: Decimal,
    pub expected_duration_mins: u64,
    pub confidence: Decimal,
    pub reasoning: String,
}

/// Forecaster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecasterConfig {
    /// History window for ML features
    pub history_window: usize,
    /// Minimum confidence to output forecast
    pub min_confidence: Decimal,
    /// Enable Claude reasoning
    pub use_llm_reasoning: bool,
    /// Feature weights
    pub feature_weights: HashMap<String, Decimal>,
}

impl Default for ForecasterConfig {
    fn default() -> Self {
        let mut weights = HashMap::new();
        weights.insert("momentum".to_string(), dec!(0.25));
        weights.insert("volatility".to_string(), dec!(0.20));
        weights.insert("volume".to_string(), dec!(0.15));
        weights.insert("regime".to_string(), dec!(0.20));
        weights.insert("sentiment".to_string(), dec!(0.10));
        weights.insert("orderflow".to_string(), dec!(0.10));

        Self {
            history_window: 100,
            min_confidence: dec!(0.5),
            use_llm_reasoning: true,
            feature_weights: weights,
        }
    }
}

/// LLM + ML Forecaster
pub struct Forecaster {
    config: ForecasterConfig,
    price_history: Vec<PricePoint>,
    recent_forecasts: Vec<ForecastRecord>,
}

/// Price point for history
#[derive(Debug, Clone)]
struct PricePoint {
    price: Decimal,
    volume: Decimal,
    timestamp: i64,
}

/// Forecast record for tracking accuracy
#[derive(Debug, Clone)]
struct ForecastRecord {
    forecast: PriceForecast,
    actual_price: Option<Decimal>,
    was_correct: Option<bool>,
}

impl Forecaster {
    pub fn new(config: ForecasterConfig) -> Self {
        Self {
            config,
            price_history: Vec::new(),
            recent_forecasts: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(ForecasterConfig::default())
    }

    /// Add price observation
    pub fn observe_price(&mut self, price: Decimal, volume: Decimal) {
        let point = PricePoint {
            price,
            volume,
            timestamp: chrono::Utc::now().timestamp(),
        };
        self.price_history.push(point);

        // Maintain window size
        if self.price_history.len() > self.config.history_window {
            self.price_history.remove(0);
        }
    }

    /// Forecast price direction
    pub fn forecast_direction(
        &self,
        symbol: &str,
        horizon: ForecastHorizon,
    ) -> PriceForecast {
        let current_price = self
            .price_history
            .last()
            .map(|p| p.price)
            .unwrap_or(Decimal::ZERO);

        // Calculate features
        let features = self.calculate_features();

        // Simple momentum-based prediction
        let momentum = features.get("momentum").copied().unwrap_or(Decimal::ZERO);
        let volatility = features.get("volatility").copied().unwrap_or(dec!(0.01));

        let direction = if momentum > dec!(0.005) {
            Direction::Up
        } else if momentum < dec!(-0.005) {
            Direction::Down
        } else {
            Direction::Sideways
        };

        // Confidence based on momentum magnitude and volatility
        let direction_confidence = momentum.abs().min(Decimal::ONE)
            * (Decimal::ONE - volatility.min(dec!(0.1)) / dec!(0.1));

        // Predicted change based on momentum scaled by horizon
        let horizon_factor = Decimal::from(horizon.to_secs()) / dec!(3600); // Relative to 1 hour
        let predicted_change_pct = momentum * horizon_factor;

        // Price levels
        let price_target = current_price * (Decimal::ONE + predicted_change_pct / dec!(100));
        let support_level = current_price * (Decimal::ONE - volatility * dec!(2));
        let resistance_level = current_price * (Decimal::ONE + volatility * dec!(2));

        let reasoning = if self.config.use_llm_reasoning {
            self.generate_reasoning(&features, direction, horizon)
        } else {
            format!("Momentum: {:.2}%, Volatility: {:.2}%", momentum * dec!(100), volatility * dec!(100))
        };

        PriceForecast {
            symbol: symbol.to_string(),
            current_price,
            horizon,
            direction,
            direction_confidence,
            predicted_change_pct,
            price_target,
            support_level,
            resistance_level,
            reasoning,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Forecast volatility
    pub fn forecast_volatility(
        &self,
        symbol: &str,
        horizon: ForecastHorizon,
    ) -> VolatilityForecast {
        let features = self.calculate_features();
        let current_vol = features.get("volatility").copied().unwrap_or(dec!(0.02));

        // Volatility mean reversion model
        let long_term_vol = dec!(0.025); // Historical average
        let reversion_speed = dec!(0.1);
        let predicted_vol = current_vol + (long_term_vol - current_vol) * reversion_speed;

        let vol_direction = if predicted_vol > current_vol * dec!(1.1) {
            Direction::Up
        } else if predicted_vol < current_vol * dec!(0.9) {
            Direction::Down
        } else {
            Direction::Sideways
        };

        let vol_regime = if current_vol < dec!(0.01) {
            VolatilityRegime::Compression
        } else if current_vol > dec!(0.05) {
            VolatilityRegime::Extreme
        } else if vol_direction == Direction::Up {
            VolatilityRegime::Expansion
        } else {
            VolatilityRegime::Stable
        };

        let confidence = dec!(0.6); // Base confidence

        VolatilityForecast {
            symbol: symbol.to_string(),
            current_volatility: current_vol,
            horizon,
            predicted_volatility: predicted_vol,
            volatility_direction: vol_direction,
            confidence,
            vol_regime,
            reasoning: format!(
                "Current vol {:.2}% vs long-term {:.2}%, expect {}",
                current_vol * dec!(100),
                long_term_vol * dec!(100),
                match vol_direction {
                    Direction::Up => "expansion",
                    Direction::Down => "compression",
                    Direction::Sideways => "stability",
                }
            ),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Forecast regime transition
    pub fn forecast_regime(&self, current: Regime) -> RegimeTransition {
        // Regime transition matrix (simplified)
        let (most_likely, prob) = match current {
            Regime::TrendingUp => (Regime::MeanReverting, dec!(0.4)),
            Regime::TrendingDown => (Regime::MeanReverting, dec!(0.4)),
            Regime::MeanReverting => (Regime::TrendingUp, dec!(0.35)),
            Regime::HighVolatility => (Regime::MeanReverting, dec!(0.5)),
            Regime::LowVolatility => (Regime::TrendingUp, dec!(0.4)),
            Regime::Choppy => (Regime::MeanReverting, dec!(0.45)),
        };

        let expected_duration = match current {
            Regime::TrendingUp | Regime::TrendingDown => 120, // 2 hours
            Regime::MeanReverting => 60, // 1 hour
            Regime::HighVolatility => 30, // 30 mins
            Regime::LowVolatility => 240, // 4 hours
            Regime::Choppy => 45, // 45 mins
        };

        RegimeTransition {
            current_regime: current,
            most_likely_next: most_likely,
            transition_probability: prob,
            expected_duration_mins: expected_duration,
            confidence: dec!(0.55),
            reasoning: format!(
                "Based on regime transition patterns, {} typically transitions to {} with {:.0}% probability",
                regime_str(&current),
                regime_str(&most_likely),
                prob * dec!(100)
            ),
        }
    }

    /// Calculate features from price history
    fn calculate_features(&self) -> HashMap<String, Decimal> {
        let mut features = HashMap::new();

        if self.price_history.len() < 2 {
            features.insert("momentum".to_string(), Decimal::ZERO);
            features.insert("volatility".to_string(), dec!(0.02));
            features.insert("volume_trend".to_string(), Decimal::ZERO);
            return features;
        }

        let prices: Vec<Decimal> = self.price_history.iter().map(|p| p.price).collect();
        let volumes: Vec<Decimal> = self.price_history.iter().map(|p| p.volume).collect();

        // Momentum (% change over window)
        let first = prices.first().unwrap();
        let last = prices.last().unwrap();
        let momentum = if !first.is_zero() {
            (*last - *first) / *first
        } else {
            Decimal::ZERO
        };
        features.insert("momentum".to_string(), momentum);

        // Volatility (standard deviation of returns)
        let returns: Vec<Decimal> = prices
            .windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect();
        let volatility = if returns.is_empty() {
            dec!(0.02)
        } else {
            let mean: Decimal = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
            let variance: Decimal = returns
                .iter()
                .map(|r| (*r - mean) * (*r - mean))
                .sum::<Decimal>()
                / Decimal::from(returns.len());
            sqrt_approx(variance)
        };
        features.insert("volatility".to_string(), volatility);

        // Volume trend
        let mid = volumes.len() / 2;
        if mid > 0 {
            let recent_vol: Decimal = volumes[mid..].iter().sum::<Decimal>() / Decimal::from(volumes.len() - mid);
            let earlier_vol: Decimal = volumes[..mid].iter().sum::<Decimal>() / Decimal::from(mid);
            let vol_trend = if !earlier_vol.is_zero() {
                (recent_vol - earlier_vol) / earlier_vol
            } else {
                Decimal::ZERO
            };
            features.insert("volume_trend".to_string(), vol_trend);
        }

        features
    }

    /// Generate reasoning (placeholder for Claude integration)
    fn generate_reasoning(
        &self,
        features: &HashMap<String, Decimal>,
        direction: Direction,
        horizon: ForecastHorizon,
    ) -> String {
        let momentum = features.get("momentum").copied().unwrap_or(Decimal::ZERO);
        let volatility = features.get("volatility").copied().unwrap_or(Decimal::ZERO);
        let vol_trend = features.get("volume_trend").copied().unwrap_or(Decimal::ZERO);

        let mut reasons = Vec::new();

        // Momentum analysis
        if momentum.abs() > dec!(0.005) {
            let strength = if momentum.abs() > dec!(0.02) {
                "strong"
            } else {
                "moderate"
            };
            reasons.push(format!(
                "{} {} momentum ({:.2}%)",
                strength,
                if momentum > Decimal::ZERO { "bullish" } else { "bearish" },
                momentum * dec!(100)
            ));
        } else {
            reasons.push("momentum is neutral".to_string());
        }

        // Volatility analysis
        if volatility > dec!(0.03) {
            reasons.push("elevated volatility suggests caution".to_string());
        } else if volatility < dec!(0.01) {
            reasons.push("compressed volatility may precede breakout".to_string());
        }

        // Volume analysis
        if vol_trend.abs() > dec!(0.2) {
            reasons.push(format!(
                "volume {} confirming move",
                if vol_trend > Decimal::ZERO { "increasing" } else { "declining" }
            ));
        }

        format!(
            "For {} horizon: {}. Expect {} price action.",
            horizon_str(&horizon),
            reasons.join("; "),
            match direction {
                Direction::Up => "upward",
                Direction::Down => "downward",
                Direction::Sideways => "sideways",
            }
        )
    }

    /// Record forecast for accuracy tracking
    pub fn record_forecast(&mut self, forecast: PriceForecast) {
        self.recent_forecasts.push(ForecastRecord {
            forecast,
            actual_price: None,
            was_correct: None,
        });

        // Keep only last 100
        if self.recent_forecasts.len() > 100 {
            self.recent_forecasts.remove(0);
        }
    }

    /// Update forecast with actual outcome
    pub fn record_outcome(&mut self, forecast_idx: usize, actual_price: Decimal) {
        if let Some(record) = self.recent_forecasts.get_mut(forecast_idx) {
            record.actual_price = Some(actual_price);

            let predicted_up = record.forecast.direction == Direction::Up;
            let actual_up = actual_price > record.forecast.current_price;
            let predicted_down = record.forecast.direction == Direction::Down;
            let actual_down = actual_price < record.forecast.current_price;

            record.was_correct = Some(
                (predicted_up && actual_up)
                    || (predicted_down && actual_down)
                    || (record.forecast.direction == Direction::Sideways
                        && (actual_price - record.forecast.current_price).abs()
                            < record.forecast.current_price * dec!(0.001)),
            );
        }
    }

    /// Get forecast accuracy
    pub fn accuracy(&self) -> ForecastAccuracy {
        let evaluated: Vec<_> = self
            .recent_forecasts
            .iter()
            .filter(|r| r.was_correct.is_some())
            .collect();

        if evaluated.is_empty() {
            return ForecastAccuracy::default();
        }

        let correct = evaluated.iter().filter(|r| r.was_correct.unwrap()).count();

        ForecastAccuracy {
            sample_size: evaluated.len(),
            accuracy_pct: Decimal::from(correct) / Decimal::from(evaluated.len()) * dec!(100),
        }
    }
}

/// Forecast accuracy report
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForecastAccuracy {
    pub sample_size: usize,
    pub accuracy_pct: Decimal,
}

/// Approximate square root
fn sqrt_approx(x: Decimal) -> Decimal {
    if x.is_zero() || x.is_sign_negative() {
        return Decimal::ZERO;
    }
    let mut guess = x;
    for _ in 0..15 {
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

/// Helper for regime string
fn regime_str(regime: &Regime) -> &'static str {
    match regime {
        Regime::TrendingUp => "Trending Up",
        Regime::TrendingDown => "Trending Down",
        Regime::MeanReverting => "Mean Reverting",
        Regime::HighVolatility => "High Volatility",
        Regime::LowVolatility => "Low Volatility",
        Regime::Choppy => "Choppy",
    }
}

/// Helper for horizon string
fn horizon_str(horizon: &ForecastHorizon) -> &'static str {
    match horizon {
        ForecastHorizon::OneMinute => "1m",
        ForecastHorizon::FiveMinutes => "5m",
        ForecastHorizon::FifteenMinutes => "15m",
        ForecastHorizon::OneHour => "1h",
        ForecastHorizon::FourHours => "4h",
        ForecastHorizon::OneDay => "1d",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forecast_with_no_history() {
        let forecaster = Forecaster::with_defaults();

        let forecast = forecaster.forecast_direction("BTC-USDT", ForecastHorizon::OneHour);

        assert_eq!(forecast.current_price, Decimal::ZERO);
        assert_eq!(forecast.direction, Direction::Sideways);
    }

    #[test]
    fn test_forecast_with_uptrend() {
        let mut forecaster = Forecaster::with_defaults();

        // Simulate uptrend
        for i in 0..20 {
            let price = dec!(67000) + Decimal::from(i * 50);
            forecaster.observe_price(price, dec!(100));
        }

        let forecast = forecaster.forecast_direction("BTC-USDT", ForecastHorizon::OneHour);

        assert_eq!(forecast.direction, Direction::Up);
        assert!(forecast.predicted_change_pct > Decimal::ZERO);
    }

    #[test]
    fn test_volatility_forecast() {
        let mut forecaster = Forecaster::with_defaults();

        // Add some price history
        for i in 0..20 {
            let price = dec!(67000) + Decimal::from((i % 5) * 100);
            forecaster.observe_price(price, dec!(100));
        }

        let vol_forecast = forecaster.forecast_volatility("BTC-USDT", ForecastHorizon::OneHour);

        assert!(vol_forecast.current_volatility > Decimal::ZERO);
        assert!(vol_forecast.confidence > Decimal::ZERO);
    }

    #[test]
    fn test_regime_transition() {
        let forecaster = Forecaster::with_defaults();

        let transition = forecaster.forecast_regime(Regime::TrendingUp);

        assert_eq!(transition.current_regime, Regime::TrendingUp);
        assert!(transition.transition_probability > Decimal::ZERO);
        assert!(transition.expected_duration_mins > 0);
    }
}
