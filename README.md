# Simmons v3.0 - Dual Brain Autonomous Trading Engine

A high-performance Rust trading engine with **Dual Brain Architecture**: TA Brain (Technical Analysis) + Fundamental Brain running in parallel, with Claude as the orchestrator.

## Architecture

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                         SIMMONS DUAL BRAIN v3.0                                │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                                                │
│  ┌─────────────────────────────────┐     ┌──────────────────────────────────┐ │
│  │   TA BRAIN (Nunchi 14 Strats)   │     │   FUNDAMENTAL BRAIN (Multi-Src)  │ │
│  │                                 │     │                                  │ │
│  │  RADAR: 0-400 score (15m)       │     │  WHALE: OnchainOS signals        │ │
│  │  PULSE: 6-tier momentum (60s)   │     │  TWITTER: KOL sentiment          │ │
│  │  GUARD: 2-phase stops (tick)    │     │  NEWS: RSS feeds                 │ │
│  │                                 │     │  SECURITY: Honeypot scan         │ │
│  │  14 Strategies: MM/ARB/DIR      │     │                                  │ │
│  └────────────────┬────────────────┘     └───────────────┬──────────────────┘ │
│                   │                                       │                    │
│                   └─────────────────┬─────────────────────┘                    │
│                                     ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │                        CONSENSUS LAYER                                  │   │
│  │  TA (60%) + Fund (40%) weighted | Conflict detection + debate trigger  │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                     │                                          │
│                                     ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │                  CLAUDE ORCHESTRATOR (/simmons-dual)                    │   │
│  │  Multi-agent debate (Bull/Bear/Risk) | Position sizing via Kelly       │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                     │                                          │
│                                     ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │  EXECUTION: Paper (Rust engine) | Live DEX (OnchainOS swap)            │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                │
└───────────────────────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# Build
cargo build --release

# Run dual brain loop with dashboard
./target/release/simmons dual --dashboard
# Dashboard: http://localhost:3456

# Or run dashboard only
./target/release/simmons dashboard --port 3456

# Test individual brains
./target/release/simmons test-ta-brain --symbol BTC-USDT
./target/release/simmons test-fund-brain --token BTC --chain ethereum

# Use Claude skill for trading decisions
cd ~/simmons && claude
> /simmons-dual
```

## Project Structure

```
simmons/
├── Cargo.toml                    # Workspace
├── CLAUDE.md                     # Full v3.0 documentation
├── crates/
│   ├── simmons-core/             # Types, config, errors
│   ├── simmons-feeds/            # OKX, Twitter, News, OnchainOS
│   ├── simmons-alpha/            # Signal generation, regime detection
│   ├── simmons-brain/            # TA Brain, Fund Brain, Consensus, Reflect
│   ├── simmons-risk/             # Portfolio, Kelly, Risk Governor
│   ├── simmons-exec/             # Paper/Live execution
│   ├── simmons-infra/            # OnchainOS, X Layer, DEX
│   ├── simmons-mcp/              # MCP server
│   └── simmons-cli/              # Binary, dual loop, web dashboard
├── frontend/                     # Next.js dashboard
├── skills/
│   └── simmons-dual.md           # Claude orchestrator skill
├── config/
│   ├── settings.toml             # Main config
│   └── brains.toml               # Dual brain config
└── data/
    ├── state.json                # Trade state
    ├── trades.json               # Trade history
    └── dual_brain_context.json   # Context for Claude
```

## TA Brain Components

| Component | Interval | Output |
|-----------|----------|--------|
| **RADAR** | 15 min | Score 0-400 (Elite >250, Solid >170) |
| **PULSE** | 60 sec | Momentum tier 1-6 |
| **GUARD** | Every tick | 2-phase trailing stops |

### 14 Trading Strategies

- **Market Making (6)**: engine_mm, avellaneda_mm, regime_mm, simple_mm, grid_mm, liquidation_mm
- **Arbitrage (2)**: funding_arb, basis_arb
- **Directional (3)**: momentum_breakout, mean_reversion, aggressive_taker
- **Infrastructure (3)**: hedge_agent, rfq_agent, claude_agent

## Fund Brain Sources

| Source | Weight | Data |
|--------|--------|------|
| OnchainOS | 50% | Whale/smart money signals |
| Twitter | 30% | KOL mentions, sentiment |
| News | 20% | Headlines sentiment |

## Risk Management

| Limit | Value |
|-------|-------|
| Max Position | 15% of capital |
| Max Drawdown | 20% (circuit breaker) |
| Max Consecutive Losses | 3 (circuit breaker) |
| Conflict Size Reduction | 50% |

## Environment Variables

```bash
# Optional - for Fund Brain live data
export TWITTER_BEARER_TOKEN=xxx

# Optional - for live DEX trading
export OKX_API_KEY=xxx
export OKX_SECRET_KEY=xxx
export OKX_PASSPHRASE=xxx
```

## License

MIT
