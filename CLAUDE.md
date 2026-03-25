# Simmons v3.0 - Dual Brain Autonomous Trading Engine

## Overview

Simmons is a fully autonomous AI trading engine using Claude as the reasoning brain. v3.0 introduces the **Dual Brain Architecture** - two autonomous brains (TA + Fundamental) running in parallel, feeding signals to Claude for final decisions.

## Architecture v3.0 - Dual Brain

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                         SIMMONS DUAL BRAIN v3.0                                │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                                │
│  ┌─────────────────────────────────┐     ┌──────────────────────────────────┐ │
│  │   TA BRAIN (Nunchi 14 Strats)   │     │   FUNDAMENTAL BRAIN (Multi-Src)  │ │
│  │                                 │     │                                  │ │
│  │  ┌───────────────────────────┐  │     │  ┌────────────────────────────┐  │ │
│  │  │ RADAR: 0-400 score        │  │     │  │ WHALE: OnchainOS signals   │  │ │
│  │  │ PULSE: 6-tier momentum    │  │     │  │ TWITTER: KOL sentiment     │  │ │
│  │  │ GUARD: 2-phase stops      │  │     │  │ NEWS: RSS feeds            │  │ │
│  │  └───────────────────────────┘  │     │  │ SECURITY: Honeypot scan    │  │ │
│  │                                 │     │  └────────────────────────────┘  │ │
│  │  14 Strategies: MM/ARB/DIR     │     │                                  │ │
│  └────────────────┬────────────────┘     └───────────────┬──────────────────┘ │
│                   │                                       │                    │
│                   └─────────────────┬─────────────────────┘                    │
│                                     ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │                        CONSENSUS LAYER                                  │   │
│  │  - Merge: TA (60%) + Fund (40%) weighted                               │   │
│  │  - Conflict detection + debate trigger                                 │   │
│  │  - Adaptive weights via REFLECT                                        │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                     │                                          │
│                                     ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │                  CLAUDE ORCHESTRATOR (/simmons-dual)                    │   │
│  │  - Multi-agent debate (Bull/Bear/Risk)                                 │   │
│  │  - Final decision with GUARD stops                                     │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                     │                                          │
│                                     ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │                        EXECUTION LAYER                                  │   │
│  │  Paper: Simmons Rust engine | Live DEX: OnchainOS swap                 │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                │
└───────────────────────────────────────────────────────────────────────────────┘
```

## Running Modes

### Dual Brain Mode (NEW)
```bash
# Build
cargo build --release

# Run dual brain loop
./target/release/simmons dual

# Run with dashboard
./target/release/simmons dual --dashboard
# Dashboard: http://localhost:3456
```

### MCP Server Mode
```bash
./target/release/simmons mcp

# Register with Claude Code
claude mcp add simmons ./target/release/simmons mcp
```

### Dashboard Mode (Default)
```bash
./target/release/simmons
# Open http://localhost:3000
```

### Test Individual Brains
```bash
# Test TA Brain
./target/release/simmons test-ta-brain --symbol BTC-USDT

# Test Fund Brain
./target/release/simmons test-fund-brain --token BTC --chain ethereum
```

## Claude Skills

| Skill | Description |
|-------|-------------|
| `/simmons-dual` | **NEW** Dual brain orchestrator - reads context from both brains |
| `/simmons` | Legacy orchestrator - multi-agent trading decision |
| `/simmons-brain` | Legacy file-based decision skill |

## TA Brain Components

### RADAR Score (0-400)
Opportunity screening every 15 minutes:

| Pillar | Weight | Metrics |
|--------|--------|---------|
| Market Structure | 35% (140) | Volume, OI, depth |
| Technicals | 40% (160) | Trend, RSI, patterns |
| Funding | 25% (100) | Rate direction, extremes |

**Thresholds:**
- 250-400: Elite → immediate entry
- 170-250: Solid → entry with PULSE confirm
- 140-170: Marginal → queue only
- <140: Skip

### PULSE Signal (6 tiers)
Momentum detection every 60 seconds:
- Tier 6: Extreme momentum
- Tier 5: Strong momentum
- Tier 4: Moderate momentum
- Tier 3: Mild momentum
- Tier 2: Weak momentum
- Tier 1: No momentum

### GUARD Stops (2-phase)
- **Phase 1**: 3% retrace, 3 breaches max
- **Phase 2**: 1.5% retrace (at 8% ROE), 2 breaches max
- **Stagnation exit**: 60 min at same level with 8%+ ROE

### 14 Strategies
- **Market Making (6)**: engine_mm, avellaneda_mm, regime_mm, simple_mm, grid_mm, liquidation_mm
- **Arbitrage (2)**: funding_arb, basis_arb
- **Directional (3)**: momentum_breakout, mean_reversion, aggressive_taker
- **Infrastructure (3)**: hedge_agent, rfq_agent, claude_agent

## Fund Brain Components

### Data Sources

| Source | Weight | Data |
|--------|--------|------|
| OnchainOS | 50% | Whale/smart money signals |
| Twitter | 30% | KOL mentions, sentiment |
| News | 20% | Headlines sentiment |

### Security Scanner
**BLOCK trading if:**
- `is_honeypot = true`
- `buy_tax > 20%` or `sell_tax > 20%`
- `can_take_ownership = true`
- `risk_score >= 80`

**WARN (reduce size) if:**
- `tax > 5%`
- `is_mintable = true`
- `risk_score >= 50`

## Consensus Layer

Merges both brain outputs:
- Default weights: TA 60%, Fund 40%
- Conflict detection when sentiment diverges > 0.5
- Adaptive weights based on historical accuracy

## REFLECT Learning System

After each trade:
1. Generate reflection (what worked/failed)
2. Log mistakes to `data/mistakes.json`
3. Update brain weights based on accuracy
4. Create avoid rules for common mistakes

## Configuration

### Dual Brain Config (`config/brains.toml`)
```toml
[trading]
mode = "paper"
chains = ["solana", "base", "ethereum"]
capital_usd = 1000

[ta_brain.radar]
elite_threshold = 250
solid_threshold = 170
marginal_threshold = 140

[ta_brain.guard]
phase1_retrace = 0.03
phase2_retrace = 0.015
stagnation_timeout_mins = 60

[fund_brain.sources]
onchain_weight = 0.5
twitter_weight = 0.3
news_weight = 0.2

[consensus]
ta_weight = 0.6
fund_weight = 0.4
conflict_reduces_size = true
adaptive_weights = true
```

## MCP Tools

### Simmons MCP Server

| Tool | Description |
|------|-------------|
| `get_signals` | Current market signals, regime, opportunities |
| `get_portfolio` | Capital, positions, drawdown, risk level |
| `get_history` | Recent trade history for learning |
| `submit_trade` | Submit trade decision (trade/skip/close) |
| `record_outcome` | Record outcome for learning |
| `get_regime` | Market regime classification |
| `check_circuit_breaker` | Check if trading enabled |

### OnchainOS MCP Server (DEX Execution)

| Tool | Description |
|------|-------------|
| `security_token_scan` | Honeypot, tax, mint risk detection |
| `signal_list` | Smart money / whale / KOL signals |
| `swap_quote` | DEX swap quote (read-only) |
| `swap_swap` | Execute DEX swap |
| `market_prices` | Real-time token prices |

**Supported Chains:** solana, ethereum, base, bsc, arbitrum, polygon, avalanche, optimism (25+ total)

## Decision Flow

```
1. READ: data/dual_brain_context.json
2. CHECK: data/mistakes.json (avoid rules)
3. ANALYZE: TA Brain (RADAR, PULSE, strategies)
4. ANALYZE: Fund Brain (whale, twitter, news, security)
5. MERGE: Consensus layer
6. DEBATE: If conflict detected, run Bull/Bear agents
7. SIZE: Calculate position with all modifiers
8. EXECUTE: Paper (Simmons MCP) or Live (OnchainOS)
9. REFLECT: Record outcome, update weights
```

## Risk Management

| Limit | Value |
|-------|-------|
| Max Position | 15% of capital |
| Max Drawdown | 20% (circuit breaker) |
| Max Consecutive Losses | 3 (circuit breaker) |
| Conflict Size Reduction | 50% |
| Security Warning Reduction | 30% |

## Crates

| Crate | Purpose |
|-------|---------|
| simmons-core | Types, config, common utilities |
| simmons-feeds | OKX, Twitter, News, OnchainOS |
| simmons-alpha | Signal generation, regime detection |
| **simmons-brain** | **Dual brain (TA + Fund), consensus, REFLECT** |
| simmons-risk | Portfolio, Kelly sizing, risk limits |
| simmons-exec | Order execution |
| simmons-infra | Bridge, DEX protocols |
| simmons-mcp | MCP server |
| simmons-cli | Main binary, orchestrator, dual loop |

## Quick Start

```bash
# 1. Build
cargo build --release

# 2. Run dual brain loop
./target/release/simmons dual --dashboard

# 3. In another terminal, use /simmons-dual skill
cd ~/simmons
claude
> /simmons-dual
```

## Environment Variables

```bash
# Required for Fund Brain
export TWITTER_BEARER_TOKEN=xxx

# Required for OnchainOS (live trading)
export OKX_API_KEY=xxx
export OKX_SECRET_KEY=xxx
export OKX_PASSPHRASE=xxx
```

## Development

```bash
cargo build           # Debug build
cargo build --release # Release build
cargo test            # Run tests
cargo fmt             # Format
cargo clippy          # Lint
```

## Files Summary

| File | Purpose |
|------|---------|
| `crates/simmons-brain/src/ta_brain.rs` | TA Brain (RADAR/PULSE/GUARD) |
| `crates/simmons-brain/src/fund_brain.rs` | Fund Brain (whale/twitter/news) |
| `crates/simmons-brain/src/consensus.rs` | Consensus layer |
| `crates/simmons-brain/src/reflect.rs` | Self-learning system |
| `crates/simmons-feeds/src/twitter.rs` | Twitter API integration |
| `crates/simmons-feeds/src/onchain.rs` | OnchainOS integration |
| `crates/simmons-cli/src/dual_loop.rs` | Dual brain loop |
| `skills/simmons-dual.md` | Claude skill |
| `config/brains.toml` | Brain configuration |
| `data/dual_brain_context.json` | Context file for Claude |
| `data/mistakes.json` | Learned avoid rules |
