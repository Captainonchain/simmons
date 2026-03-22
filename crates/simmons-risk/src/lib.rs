//! Simmons Risk - Risk management layer
//!
//! Position sizing, Kelly criterion, drawdown management, portfolio optimization.

pub mod arb_router;
pub mod governor;
pub mod kelly;
pub mod portfolio;
pub mod rebalancer;

pub use arb_router::CeDefiArbRouter;
pub use governor::RiskGovernor;
pub use kelly::KellyCriterion;
pub use portfolio::Portfolio;
pub use rebalancer::PortfolioRebalancer;
