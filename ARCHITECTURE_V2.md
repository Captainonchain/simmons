# Simmons v2.0: Autonomous Trading Engine Architecture

**Date:** 2026-03-25
**Based on:** OKX OnchainOS Skills, TauricResearch TradingAgents, Claude Code Best Practices

---

## Executive Summary

This architecture redesigns Simmons as a **multi-agent autonomous trading engine** combining:

1. **TradingAgents Pattern** - Hierarchical agent teams (analysts → researchers → traders → risk)
2. **OKX OnchainOS Skills** - Production-ready MCP tools for DEX execution across 25+ chains
3. **Claude Code Skills** - Custom skills for reasoning and decision-making
4. **Rust Engine** - High-performance signal generation and execution layer

The key innovation is replacing file-based IPC with **MCP server integration** and adopting the **debate-driven decision flow** proven by TradingAgents research (2.63 Sharpe ratio in backtests) [1].

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SIMMONS v2.0 ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                        CLAUDE ORCHESTRATOR                            │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │  │                    SKILL: /simmons-orchestrator                 │ │   │
│  │  │  • Reads signals from MCP server                                │ │   │
│  │  │  • Spawns analyst/researcher agents (parallel)                  │ │   │
│  │  │  • Manages debate rounds                                        │ │   │
│  │  │  • Final decision aggregation                                   │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  │                              │                                        │   │
│  │                              ▼                                        │   │
│  │  ┌───────────────────────────────────────────────────────────────┐   │   │
│  │  │                    AGENT TEAMS (Subagents)                    │   │   │
│  │  │                                                               │   │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │   │   │
│  │  │  │  ANALYSTS   │  │ RESEARCHERS │  │ RISK MANAGEMENT     │  │   │   │
│  │  │  │  (Parallel) │  │  (Debate)   │  │    (Debate)         │  │   │   │
│  │  │  ├─────────────┤  ├─────────────┤  ├─────────────────────┤  │   │   │
│  │  │  │ Technical   │  │ Bull Agent  │  │ Aggressive Debator  │  │   │   │
│  │  │  │ Fundamental │  │ Bear Agent  │  │ Conservative Debator│  │   │   │
│  │  │  │ Sentiment   │  │             │  │ Neutral Debator     │  │   │   │
│  │  │  │ On-chain    │  │             │  │                     │  │   │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────────────┘  │   │   │
│  │  └───────────────────────────────────────────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                    │                                         │
│                                    ▼                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                         MCP SERVER LAYER                              │   │
│  │                                                                       │   │
│  │  ┌─────────────┐  ┌─────────────────┐  ┌─────────────────────────┐  │   │
│  │  │   SIMMONS   │  │  OKX ONCHAINOS  │  │    EXTERNAL MCP         │  │   │
│  │  │  MCP SERVER │  │   MCP SERVER    │  │      SERVERS            │  │   │
│  │  │  (Rust)     │  │   (Rust CLI)    │  │                         │  │   │
│  │  ├─────────────┤  ├─────────────────┤  ├─────────────────────────┤  │   │
│  │  │ get_signals │  │ dex_swap        │  │ VARRD (quant research)  │  │   │
│  │  │ get_state   │  │ dex_quote       │  │ Alpaca (equities)       │  │   │
│  │  │ submit_trade│  │ token_security  │  │ Polygon (data)          │  │   │
│  │  │ get_portfolio│ │ wallet_balance  │  │                         │  │   │
│  │  │ record_outcome││ market_prices   │  │                         │  │   │
│  │  │ get_history │  │ gas_estimate    │  │                         │  │   │
│  │  └─────────────┘  │ smart_money     │  └─────────────────────────┘  │   │
│  │                   │ meme_scanner    │                               │   │
│  │                   │ tx_simulate     │                               │   │
│  │                   │ broadcast       │                               │   │
│  │                   └─────────────────┘                               │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                    │                                         │
│                                    ▼                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                         RUST ENGINE LAYER                             │   │
│  │                                                                       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │   │
│  │  │   FEEDS     │  │    ALPHA    │  │    RISK     │  │    EXEC     │ │   │
│  │  ├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤ │   │
│  │  │ OKX WS      │  │ Momentum    │  │ Kelly Sizing│  │ Paper Trade │ │   │
│  │  │ DEX Prices  │  │ Mean Rev    │  │ Position Lim│  │ Live Trade  │ │   │
│  │  │ On-chain    │  │ Regime      │  │ Drawdown    │  │ MEV Protect │ │   │
│  │  │ News        │  │ Arbitrage   │  │ Circuit Brkr│  │ Smart Route │ │   │
│  │  │ Smart Money │  │ Patterns    │  │ Daily Limit │  │ Split Orders│ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Component Details

### 1. Claude Skill Layer (The Brain)

Following TradingAgents architecture [1], implement these skills in `.claude/commands/`:

#### 1.1 Main Orchestrator Skill: `/simmons`

```markdown
# /simmons - Main Trading Orchestrator

Orchestrates the full trading decision flow:

1. GATHER: Spawn parallel analyst agents
2. DEBATE: Run bull/bear research debate
3. DECIDE: Synthesize into trade decision
4. RISK: Run risk management debate
5. EXECUTE: Submit final decision via MCP
```

**Key behaviors:**
- Uses `Task` tool to spawn subagents in parallel
- Manages debate rounds (configurable: 1-3 rounds)
- Aggregates analyst reports into unified context
- Makes final BUY/HOLD/SELL decision with position sizing

#### 1.2 Analyst Agents (Parallel Subagents)

| Agent | Tools Used | Output |
|-------|------------|--------|
| **Technical Analyst** | simmons_mcp.get_signals, simmons_mcp.get_history | technical_report |
| **Fundamental Analyst** | onchainos.token_info, onchainos.market_prices | fundamental_report |
| **Sentiment Analyst** | onchainos.smart_money, onchainos.meme_scanner | sentiment_report |
| **On-chain Analyst** | onchainos.wallet_portfolio, onchainos.token_security | onchain_report |

#### 1.3 Researcher Agents (Debate)

| Agent | Role | Behavior |
|-------|------|----------|
| **Bull Researcher** | Argues bullish thesis | Uses growth metrics, positive signals, upside scenarios |
| **Bear Researcher** | Argues bearish thesis | Focuses on risks, downside scenarios, negative signals |

Debate protocol (from TradingAgents [1]):
1. Bull presents initial thesis
2. Bear counters with risks
3. Bull rebuts with mitigations
4. Bear presents final concerns
5. Research Manager synthesizes

#### 1.4 Risk Management Team (Debate)

| Agent | Risk Tolerance | Focus |
|-------|----------------|-------|
| **Aggressive** | High | Maximum opportunity capture |
| **Conservative** | Low | Capital preservation first |
| **Neutral** | Balanced | Risk-adjusted returns |

---

### 2. MCP Server Layer

#### 2.1 Simmons MCP Server (New - Rust)

Replaces file-based IPC with proper MCP protocol:

```rust
// crates/simmons-mcp/src/lib.rs

pub struct SimmonsMcpServer {
    alpha_engine: Arc<AlphaEngine>,
    portfolio: Arc<Portfolio>,
    executor: Arc<Executor>,
    memory: Arc<TradingMemory>,
}

// MCP Tools exposed:
impl SimmonsMcpServer {
    // Read current signals (all strategies)
    async fn get_signals(&self) -> SignalsResponse;

    // Read portfolio state
    async fn get_portfolio(&self) -> PortfolioResponse;

    // Read trade history for memory/learning
    async fn get_history(&self, limit: usize) -> Vec<TradeRecord>;

    // Submit trade decision (Claude → Rust)
    async fn submit_trade(&self, decision: TradeDecision) -> TradeResult;

    // Record outcome for learning
    async fn record_outcome(&self, trade_id: String, outcome: TradeOutcome);

    // Get current market regime
    async fn get_regime(&self) -> RegimeState;
}
```

**MCP Configuration:**
```json
{
  "mcpServers": {
    "simmons": {
      "command": "./target/release/simmons",
      "args": ["mcp"],
      "env": {
        "SIMMONS_MODE": "live",
        "OKX_API_KEY": "${OKX_API_KEY}"
      }
    },
    "onchainos": {
      "command": "onchainos",
      "args": ["mcp"]
    }
  }
}
```

#### 2.2 OKX OnchainOS Integration

Use all 11 skills from onchainos-skills [2]:

| Skill | Trading Use |
|-------|-------------|
| `okx-dex-swap` | Execute swaps across 400+ DEXs |
| `okx-dex-market` | Real-time prices, K-lines |
| `okx-dex-signal` | Smart money tracking |
| `okx-dex-trenches` | Meme token scanning (60+ filters) |
| `okx-dex-token` | Token discovery, holder analysis |
| `okx-security` | Honeypot detection, risk scoring |
| `okx-onchain-gateway` | Gas estimation, tx simulation, broadcast |
| `okx-wallet-portfolio` | Portfolio balance queries |
| `okx-agentic-wallet` | Wallet lifecycle (if custodial) |
| `okx-x402-payment` | Payment authorization |
| `okx-audit-log` | Audit trail |

**Chain Support:** Ethereum, Solana, Base, Arbitrum, Polygon, BSC, X Layer, Sui, TON, TRON (20+ chains) [2]

---

### 3. Rust Engine Layer

Keep existing crates, enhance for MCP:

| Crate | Enhancement |
|-------|-------------|
| `simmons-core` | Add MCP types, message schemas |
| `simmons-feeds` | Add OnchainOS data integration |
| `simmons-alpha` | Keep existing, expose via MCP |
| `simmons-brain` | **Remove file IPC**, add MCP server |
| `simmons-risk` | Expose risk checks via MCP |
| `simmons-exec` | Integrate onchainos swap execution |
| `simmons-mcp` | **NEW** - MCP server implementation |

---

## Decision Flow (TradingAgents Pattern)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         DECISION FLOW                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Phase 1: DATA GATHERING (Parallel)                        ~5-10 sec   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
│  │ Technical   │ │ Fundamental │ │ Sentiment   │ │ On-chain    │       │
│  │ Analyst     │ │ Analyst     │ │ Analyst     │ │ Analyst     │       │
│  └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘       │
│         └───────────────┴───────────────┴───────────────┘               │
│                                │                                         │
│                                ▼                                         │
│  Phase 2: INVESTMENT DEBATE (Sequential)                   ~10-20 sec  │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Bull: "Strong momentum + smart money accumulation suggests..."  │   │
│  │  Bear: "However, security scan shows concerning dev holdings..." │   │
│  │  Bull: "Mitigated by 6-month lock and transparent team..."      │   │
│  │  Bear: "But similar patterns preceded 3 recent rugs..."         │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                │                                         │
│                                ▼                                         │
│  Phase 3: RESEARCH SYNTHESIS                               ~5 sec      │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Research Manager: Weighs debate, creates investment_plan       │   │
│  │  Output: "BUY with 8% position, cautious due to dev concerns"   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                │                                         │
│                                ▼                                         │
│  Phase 4: TRADING DECISION                                 ~5 sec      │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Trader: Synthesizes all reports + debate                        │   │
│  │  Output: "FINAL TRANSACTION PROPOSAL: **BUY** 8% position"       │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                │                                         │
│                                ▼                                         │
│  Phase 5: RISK ASSESSMENT DEBATE                           ~10-15 sec │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Aggressive: "8% is conservative, could go 12% given signals"   │   │
│  │  Conservative: "Reduce to 5%, dev holdings are red flag"        │   │
│  │  Neutral: "6-8% appropriate, with 2% stop-loss"                 │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                │                                         │
│                                ▼                                         │
│  Phase 6: FINAL APPROVAL                                   ~5 sec      │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Portfolio Manager: Final decision with entry/exit strategy      │   │
│  │  Output: {action: "BUY", size: 0.06, stop: 0.02, target: 0.08}  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                │                                         │
│                                ▼                                         │
│  Phase 7: EXECUTION                                        ~1-5 sec    │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  1. Security scan via onchainos.token_security                   │   │
│  │  2. Gas estimate via onchainos.gateway_gas                       │   │
│  │  3. Simulate via onchainos.gateway_simulate                      │   │
│  │  4. Execute via onchainos.dex_swap                               │   │
│  │  5. Record via simmons.submit_trade                              │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  Total Time: ~45-60 seconds per decision cycle                          │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Memory & Learning System

Following TradingAgents' reflection mechanism [1] and Anthropic's long-running agent best practices [3]:

### Memory Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         MEMORY SYSTEM                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    SEMANTIC MEMORY (Per Agent)                   │   │
│  │                                                                  │   │
│  │  Storage: data/memory/{agent_name}.json                         │   │
│  │  Format: [(situation, reflection, outcome, timestamp), ...]     │   │
│  │                                                                  │   │
│  │  Retrieval: get_memories(curr_situation, n_matches=3)           │   │
│  │  Method: Embedding similarity search                             │   │
│  │                                                                  │   │
│  │  Agents with memory:                                            │   │
│  │  - bull_researcher                                              │   │
│  │  - bear_researcher                                              │   │
│  │  - trader                                                       │   │
│  │  - portfolio_manager                                            │   │
│  │  - risk_neutral                                                 │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    REFLECTION LOOP                               │   │
│  │                                                                  │   │
│  │  Trigger: After trade closes (win or loss)                       │   │
│  │                                                                  │   │
│  │  Process:                                                        │   │
│  │  1. Retrieve original decision context                          │   │
│  │  2. Compare prediction vs outcome                               │   │
│  │  3. Generate reflection: "What did we miss?"                    │   │
│  │  4. Store (context, reflection, outcome) tuple                  │   │
│  │  5. Update strategy weights                                     │   │
│  │                                                                  │   │
│  │  Skill: /simmons-reflect                                        │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    STATE PERSISTENCE                             │   │
│  │                                                                  │   │
│  │  Files (Anthropic best practice [3]):                           │   │
│  │  - data/progress.json: Current cycle state                      │   │
│  │  - data/state.json: Portfolio + performance                     │   │
│  │  - data/decisions.log: Append-only decision log                 │   │
│  │  - data/mistakes.json: Learned mistakes (from brain_learning)   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Skill Definitions

### Main Skills (`.claude/commands/`)

#### `/simmons` - Main Orchestrator

```markdown
# Simmons Trading Orchestrator

<skill-context>
name: simmons
description: Autonomous multi-agent trading decision engine
</skill-context>

## Process

1. **GATHER** - Spawn parallel analyst agents
   - Use Task tool with 4 subagents (technical, fundamental, sentiment, onchain)
   - Each reads via simmons MCP + onchainos MCP
   - Timeout: 10 seconds

2. **DEBATE** - Investment thesis debate
   - Bull researcher presents bullish case
   - Bear researcher counters
   - 1-2 rounds configurable
   - Research manager synthesizes

3. **DECIDE** - Make trade decision
   - Synthesize all reports
   - Apply decision rules
   - Output: BUY/HOLD/SELL with position size

4. **RISK** - Risk management debate
   - Aggressive/Conservative/Neutral perspectives
   - Adjust position size
   - Set stop-loss/take-profit

5. **EXECUTE** - Submit via MCP
   - Pre-flight: onchainos.token_security
   - Simulate: onchainos.gateway_simulate
   - Execute: onchainos.dex_swap
   - Record: simmons.submit_trade

## MCP Tools Available

From simmons MCP:
- get_signals, get_portfolio, get_history, submit_trade, get_regime

From onchainos MCP:
- dex_swap, dex_quote, token_security, market_prices, wallet_portfolio
- gateway_gas, gateway_simulate, gateway_broadcast
- smart_money, meme_scanner, token_info
```

#### `/simmons-reflect` - Learning Skill

```markdown
# Simmons Reflection & Learning

<skill-context>
name: simmons-reflect
description: Post-trade reflection and memory update
trigger: After trade closes
</skill-context>

## Process

1. Read closed trade from simmons.get_history
2. Retrieve original decision context
3. Compare prediction vs outcome
4. Generate reflection for each agent
5. Store in semantic memory
6. Update strategy weights if needed
```

---

## Risk Management

### Pre-Trade Checks (Integrated with OnchainOS)

```python
def pre_trade_validation(token: str, chain: str) -> ValidationResult:
    # 1. Security scan
    security = onchainos.token_security(token, chain)
    if security.honeypot or security.tax > 20:
        return REJECT("Security risk")

    # 2. Dev holdings check
    if security.dev_holdings > 30:
        return WARN("High dev holdings")

    # 3. Liquidity check
    if security.liquidity_usd < 50000:
        return REJECT("Low liquidity")

    # 4. Transaction simulation
    sim = onchainos.gateway_simulate(swap_tx)
    if sim.expected_slippage > 3:
        return WARN("High slippage")

    return APPROVE()
```

### Circuit Breakers (From Polymarket Experience [MEMORY.md])

| Limit | Value | Enforcement |
|-------|-------|-------------|
| Total consecutive losses | 3 | `check_circuit_breaker()` |
| Max drawdown | 20% | `check_drawdown()` |
| Max position size | 15% | Entry sizing |
| Daily loss limit | $100 | `check_daily_loss()` |
| Min balance | $50 | Manual halt |

---

## Directory Structure

```
simmons/
├── Cargo.toml
├── CLAUDE.md
├── .claude/
│   ├── commands/
│   │   ├── simmons.md           # Main orchestrator skill
│   │   ├── simmons-reflect.md   # Learning/reflection skill
│   │   └── simmons-auto.md      # Continuous loop skill
│   ├── agents/
│   │   ├── technical_analyst.md
│   │   ├── fundamental_analyst.md
│   │   ├── sentiment_analyst.md
│   │   ├── onchain_analyst.md
│   │   ├── bull_researcher.md
│   │   ├── bear_researcher.md
│   │   ├── aggressive_risk.md
│   │   ├── conservative_risk.md
│   │   └── neutral_risk.md
│   └── settings.json            # MCP server config
├── crates/
│   ├── simmons-core/            # Shared types
│   ├── simmons-feeds/           # Data ingestion
│   ├── simmons-alpha/           # Signal generation
│   ├── simmons-risk/            # Risk management
│   ├── simmons-exec/            # Order execution
│   ├── simmons-mcp/             # NEW: MCP server
│   └── simmons-cli/             # Binary
├── config/
│   ├── settings.toml
│   └── mcp.json                 # MCP server registration
└── data/
    ├── memory/                  # Agent memories
    │   ├── bull_researcher.json
    │   ├── bear_researcher.json
    │   └── trader.json
    ├── state.json               # Portfolio state
    ├── progress.json            # Cycle progress
    └── decisions.log            # Append-only log
```

---

## Configuration

### MCP Server Configuration (`config/mcp.json`)

```json
{
  "mcpServers": {
    "simmons": {
      "command": "./target/release/simmons",
      "args": ["mcp"],
      "env": {
        "SIMMONS_MODE": "paper",
        "RUST_LOG": "info"
      }
    },
    "onchainos": {
      "command": "onchainos",
      "args": ["mcp"],
      "env": {
        "OKX_API_KEY": "${OKX_API_KEY}",
        "OKX_API_SECRET": "${OKX_API_SECRET}",
        "OKX_PASSPHRASE": "${OKX_PASSPHRASE}"
      }
    }
  }
}
```

### Trading Configuration (`config/settings.toml`)

```toml
mode = "paper"
capital_usd = 1000

[chains]
primary = ["solana", "base", "ethereum"]
enabled = ["arbitrum", "polygon", "bsc"]

[risk]
max_position_pct = 0.15
max_drawdown = 0.20
kelly_fraction = 0.25
max_consecutive_losses = 3
daily_loss_limit = 100

[agents]
debate_rounds = 1
risk_discuss_rounds = 1
analyst_timeout_secs = 10
decision_timeout_secs = 30

[memory]
enabled = true
max_memories_per_agent = 100
retrieval_count = 3
```

---

## Implementation Phases

### Phase 1: MCP Foundation (Week 1)

1. Create `simmons-mcp` crate
2. Implement MCP server with 6 tools
3. Register with Claude Code
4. Test basic signal retrieval

### Phase 2: Agent Skills (Week 2)

1. Write `/simmons` orchestrator skill
2. Create analyst agent definitions
3. Create researcher agent definitions
4. Test parallel agent spawning

### Phase 3: Debate System (Week 3)

1. Implement bull/bear debate protocol
2. Implement risk debate protocol
3. Add research manager synthesis
4. Test full decision flow

### Phase 4: OnchainOS Integration (Week 4)

1. Install onchainos-skills
2. Configure MCP registration
3. Integrate security scanning
4. Implement swap execution

### Phase 5: Memory & Learning (Week 5)

1. Implement semantic memory
2. Create `/simmons-reflect` skill
3. Add strategy weight updates
4. Test reflection loop

### Phase 6: Production Hardening (Week 6)

1. Add circuit breakers
2. Implement MEV protection
3. Add audit logging
4. Stress testing

---

## References

[1] TradingAgents: Multi-Agents LLM Financial Trading Framework. TauricResearch, 2024. https://arxiv.org/abs/2412.20138

[2] OKX OnchainOS Skills. OKX, 2025. https://github.com/okx/onchainos-skills

[3] Effective Harnesses for Long-Running Agents. Anthropic Engineering, 2025. https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents

[4] MCP for Trading. VARRD, 2026. https://www.varrd.com/guides/mcp-trading.html

[5] LangGraph Multi-Agent Workflows. LangChain, 2025. https://blog.langchain.com/langgraph-multi-agent-workflows/

[6] Claude Code Best Practices. Anthropic, 2026. https://code.claude.com/docs/en/best-practices

---

## Quick Start

```bash
# 1. Build
cargo build --release

# 2. Install onchainos
npx skills add okx/onchainos-skills

# 3. Configure MCP
claude mcp add simmons ./target/release/simmons mcp
claude mcp add onchainos onchainos mcp

# 4. Start paper trading
./target/release/simmons --mode paper

# 5. In another terminal, run Claude Code
cd ~/simmons
claude
> /simmons
```

---

## Comparison: v1.0 vs v2.0

| Aspect | v1.0 (Current) | v2.0 (New) |
|--------|----------------|------------|
| IPC | File-based (signals.json) | MCP protocol |
| Decision | Single Claude skill | Multi-agent teams |
| Analysis | Single-pass | Debate-driven |
| Risk | Post-hoc checks | Integrated debate |
| Memory | state.json | Semantic per-agent |
| Execution | Manual OKX API | OnchainOS MCP |
| Chains | X Layer focus | 25+ chains |
| Learning | Basic feedback | Reflection loop |

---

## Success Metrics

Based on TradingAgents research [1]:

| Metric | Target | Measurement |
|--------|--------|-------------|
| Sharpe Ratio | > 2.0 | Risk-adjusted returns |
| Max Drawdown | < 15% | Worst peak-to-trough |
| Win Rate | > 55% | Profitable trades |
| Decision Time | < 60s | Full cycle |
| Uptime | > 99% | System availability |
