# Simmons - Max ROI DeFi Trading System

A high-performance Rust trading engine with **Claude as the reasoning brain** for decision-making.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         SIMMONS                                  │
│                    (Rust Trading Engine)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────┐   ┌─────────┐   ┌─────────────┐   ┌─────────┐     │
│  │ Layer 1 │──▶│ Layer 2 │──▶│  CLAUDE     │──▶│ Layer 4 │     │
│  │  Feeds  │   │  Alpha  │   │   BRAIN     │   │  Exec   │     │
│  │  (Rust) │   │  (Rust) │   │  (Skill)    │   │  (Rust) │     │
│  └─────────┘   └─────────┘   └─────────────┘   └─────────┘     │
│       │              │              │                │          │
│       │              │              ▼                │          │
│       │              │       ┌─────────────┐         │          │
│       │              │       │  Layer 3    │         │          │
│       │              └──────▶│   Risk      │◀────────┘          │
│       │                      │  (Rust)     │                    │
│       │                      └─────────────┘                    │
│       └─────────────────────────────────────────────────────────┘
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                    Claude Skill: /simmons-brain                  │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# Build
cargo build --release

# Run paper trading
./target/release/simmons --mode paper --capital 100

# Run simulation
./target/release/simmons sim --duration 300

# Show current signals
./target/release/simmons signals

# Show portfolio status
./target/release/simmons status
```

## Interactive Trading with Claude

1. **Terminal 1**: Start Rust engine
```bash
cd ~/simmons
./target/release/simmons
# Shows: "Signals updated → data/signals.json"
```

2. **Terminal 2**: Claude Code for decisions
```bash
cd ~/simmons
claude
> /simmons-brain
# Claude reads signals, reasons, writes decision
# Rust engine executes automatically
```

## Project Structure

```
simmons/
├── Cargo.toml              # Workspace
├── crates/
│   ├── simmons-core/       # Shared types, config
│   ├── simmons-feeds/      # Layer 1: Data ingestion (OKX WebSocket)
│   ├── simmons-alpha/      # Layer 2: Signal generation
│   ├── simmons-brain/      # Claude Brain: File-based IPC
│   ├── simmons-risk/       # Layer 3: Risk management, Kelly
│   ├── simmons-exec/       # Layer 4: Execution engine
│   ├── simmons-infra/      # Layer 5: X Layer, DeFi
│   └── simmons-cli/        # Binary entry point
├── .claude/commands/
│   └── simmons-brain.md    # Claude skill definition
├── config/
│   └── settings.toml       # Configuration
├── data/
│   ├── signals.json        # Rust writes, Claude reads
│   ├── decision.json       # Claude writes, Rust reads
│   └── state.json          # Shared state
└── skills/
    └── simmons-brain.md    # Skill documentation
```

## Signal Generation

| Strategy | Description |
|----------|-------------|
| Momentum | RSI, ROC, price momentum |
| Mean Reversion | Z-score, Bollinger Bands |
| Regime Detection | Trend, volatility, choppiness |
| Arbitrage | CeDeFi spreads |

## Risk Management

- **Kelly Criterion**: Optimal position sizing
- **Max Drawdown**: 20% halt threshold
- **Position Limits**: Max 15% per trade
- **Daily Loss Limit**: Configurable

## Performance Targets

| Component | Target |
|-----------|--------|
| WebSocket tick | <1ms |
| Signal generation | <0.1ms |
| Risk calculations | <0.1ms |
| Memory per symbol | <5KB |
| Max symbols | 500+ |

## Configuration

Edit `config/settings.toml`:

```toml
mode = "paper"           # paper, live, simulation
capital_usd = 100
symbols = ["BTC-USDT", "ETH-USDT", "SOL-USDT"]

[risk]
max_position_pct = 0.15  # 15% max position
max_drawdown = 0.20      # 20% max drawdown
kelly_fraction = 0.25    # Quarter Kelly

[brain]
data_dir = "data"
timeout_secs = 60
auto_invoke = false      # Manual Claude invocation
```

## License

MIT
