# Simmons v2.0 - Autonomous AI Trading Engine

## Overview

Simmons is a fully autonomous AI trading engine using Claude as the reasoning brain. v2.0 introduces **MCP (Model Context Protocol)** integration for direct Claude tool access.

## Architecture v2.0

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           SIMMONS v2.0                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    CLAUDE ORCHESTRATOR                             │ │
│  │         /simmons skill + Multi-Agent Debate System                 │ │
│  │  Analysts (4) → Researchers (Bull/Bear) → Risk (3) → Execute      │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                          │                   │                           │
│                          ▼                   ▼                           │
│  ┌──────────────────────────────┐  ┌────────────────────────────────┐  │
│  │   SIMMONS MCP SERVER         │  │   ONCHAINOS MCP SERVER         │  │
│  │   get_signals, get_portfolio │  │   security_*, signal_*         │  │
│  │   submit_trade, get_regime   │  │   swap_*, market_*, portfolio  │  │
│  └──────────────────────────────┘  └────────────────────────────────┘  │
│                          │                   │                           │
│                          ▼                   ▼                           │
│  ┌──────────────────────────────┐  ┌────────────────────────────────┐  │
│  │   RUST ENGINE LAYER          │  │   OKX WEB3 BACKEND             │  │
│  │   Feeds → Alpha → Risk       │  │   400+ DEXs, 25+ Chains        │  │
│  └──────────────────────────────┘  └────────────────────────────────┘  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Running Modes

### MCP Server Mode (NEW - for Claude integration)
```bash
# Build release
cargo build --release

# Run as MCP server
./target/release/simmons mcp

# Register with Claude Code
claude mcp add simmons ./target/release/simmons mcp
```

### Dashboard Mode (Default)
```bash
./target/release/simmons
# Open http://localhost:3000
```

### Paper Trading
```bash
./target/release/simmons run --mode paper --capital 1000
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

### OnchainOS MCP Server (DEX Execution)

| Tool | Description |
|------|-------------|
| `security_token_scan` | Honeypot, tax, mint risk detection |
| `security_tx_scan` | Transaction pre-execution scan |
| `signal_list` | Smart money / whale / KOL signals |
| `swap_quote` | DEX swap quote (read-only) |
| `swap_swap` | Execute DEX swap |
| `market_prices` | Real-time token prices |
| `portfolio_balances` | Wallet balances across chains |

**OnchainOS Chains:** solana, ethereum, base, bsc, arbitrum, polygon, avalanche, optimism, sui, ton, tron (25+ total)

## Claude Skills

| Skill | Description |
|-------|-------------|
| `/simmons` | Main orchestrator - multi-agent trading decision |
| `/simmons-brain` | Legacy file-based decision skill |

## Decision Flow (TradingAgents Pattern)

```
Phase 1: GATHER (Parallel)
├── Technical Analyst → signals, momentum
├── Fundamental Analyst → metrics, TVL
├── Sentiment Analyst → smart money (onchainos signal_list)
└── On-chain Analyst → security (onchainos security_token_scan)

Phase 2: DEBATE (Sequential)
├── Bull Researcher → bullish thesis
├── Bear Researcher → bearish thesis
└── Research Manager → synthesis

Phase 3: RISK (Debate)
├── Aggressive → larger position
├── Conservative → smaller position
└── Neutral → balanced view

Phase 4: SECURITY PRE-FLIGHT (onchainos)
├── security_token_scan → honeypot, tax, mint risks
└── BLOCK if any red flags detected

Phase 5: EXECUTE
├── Paper mode → submit_trade via Simmons MCP
└── Live mode → swap_swap via OnchainOS MCP (DEX)
```

## Risk Management

| Limit | Value |
|-------|-------|
| Max Position | 15% of capital |
| Max Drawdown | 20% (circuit breaker) |
| Max Consecutive Losses | 3 (circuit breaker) |
| Default Stop Loss | 2% |
| Default Take Profit | 5% |

## Configuration

### MCP Registration (`~/.claude.json` or `.claude/settings.json`)
```json
{
  "mcpServers": {
    "simmons": {
      "command": "/path/to/simmons",
      "args": ["mcp"]
    }
  }
}
```

### Trading Config (`config/settings.toml`)
```toml
mode = "paper"
capital_usd = 1000
symbols = ["BTC-USDT", "ETH-USDT", "SOL-USDT"]

[risk]
max_position_pct = 0.15
max_drawdown = 0.20
kelly_fraction = 0.25
```

## Crates

| Crate | Purpose |
|-------|---------|
| simmons-core | Types, config, common utilities |
| simmons-feeds | OKX, X Layer, price aggregation |
| simmons-alpha | Signal generation, regime detection |
| simmons-brain | Legacy Claude IPC, learning |
| simmons-risk | Portfolio, Kelly sizing, risk limits |
| simmons-exec | Order execution |
| simmons-infra | Bridge, DEX protocols |
| **simmons-mcp** | **MCP server (NEW)** |
| simmons-cli | Main binary, orchestrator |

## Quick Start

```bash
# 1. Build
cargo build --release

# 2. Register MCP server with Claude Code
claude mcp add simmons ./target/release/simmons mcp

# 3. Use /simmons skill in Claude Code
cd ~/simmons
claude
> /simmons
```

## Development

```bash
cargo build           # Debug build
cargo build --release # Release build
cargo test            # Run tests
cargo fmt             # Format
cargo clippy          # Lint
```

## Full Architecture Doc

See `ARCHITECTURE_V2.md` for complete architecture design including:
- Multi-agent patterns from TradingAgents research
- OKX OnchainOS integration plan
- Memory and learning system
- Implementation phases
