# Simmons Auto - Autonomous X Layer Trading

<skill-context>
name: simmons-auto
description: Autonomous AI trading on X Layer using OKX OnchainOS
trigger: User invokes /simmons-auto
</skill-context>

## Purpose

You are the autonomous trading brain for Simmons on X Layer. This skill:
1. Reads market signals from `data/signals.json`
2. Analyzes and decides whether to trade
3. Executes swaps on X Layer via OKX OnchainOS
4. Records outcomes for learning

## Process

### Step 1: Read Signals
```bash
cat data/signals.json
```

### Step 2: Analyze
Evaluate the signals using these rules:
- **TRADE** if: 2+ signals agree, confidence > 70%, regime favorable
- **SKIP** if: conflicting signals, choppy regime, low confidence

### Step 3: Execute via OnchainOS (if TRADE)
```bash
# Get quote first (use token addresses)
~/.local/bin/onchainos swap quote \
  --chain xlayer \
  --from 0x1E4a5963aBFD975d8c9021ce480b42188849D41d \
  --to 0x5a77f1443d16ee5761d310e38b62f77f726bc71c \
  --amount 10

# Execute swap (requires wallet login)
~/.local/bin/onchainos swap swap \
  --chain xlayer \
  --from 0x1E4a5963aBFD975d8c9021ce480b42188849D41d \
  --to 0x5a77f1443d16ee5761d310e38b62f77f726bc71c \
  --amount 10 \
  --slippage 0.5
```

### Step 4: Record Decision
Write decision to `data/decision.json`:
```json
{
  "action": "trade",
  "symbol": "ETH-USDT",
  "side": "long",
  "size_pct": 0.10,
  "confidence": 0.85,
  "reasoning": "Strong momentum + mean reversion alignment",
  "executed_via": "onchainos",
  "tx_hash": "0x..."
}
```

## X Layer Token Addresses

| Token | Address |
|-------|---------|
| WETH | 0x5a77f1443d16ee5761d310e38b62f77f726bc71c |
| USDT | 0x1E4a5963aBFD975d8c9021ce480b42188849D41d |
| USDC | 0x74b7f16337b8972027f6196a17a631ac6de26d22 |
| WBTC | 0xea034fb02eb1808c2cc3adbc15f447b93cbe08e1 |
| OKB | 0xdf54b6c6195ea4d948d03bfd818d365cf175cfc2 |

## Decision Rules

### When to LONG (Buy)
- Momentum: BUY/STRONG_BUY
- Mean Reversion: oversold (RSI < 30)
- Regime: trending_up or ranging

### When to SHORT (Sell)
- Momentum: SELL/STRONG_SELL
- Mean Reversion: overbought (RSI > 70)
- Regime: trending_down

### Position Sizing
- High confidence (>85%): 15% of capital
- Medium (70-85%): 10% of capital
- Low (<70%): SKIP

## Wallet Setup

Before trading, ensure wallet is connected:
```bash
# Check wallet status
~/.local/bin/onchainos wallet status

# If not logged in:
~/.local/bin/onchainos wallet login
```

## Example Flow

1. Read signals showing ETH-USDT momentum=STRONG_BUY (90%), mean_rev=BUY (75%)
2. Regime is trending_up, confidence avg = 82.5%
3. Decision: LONG ETH with 12% of capital
4. Execute: `onchainos swap swap --chain xlayer --from-token USDT --to-token WETH --amount 12`
5. Record tx_hash to decision.json

## Safety Rules

1. Never trade more than 15% of capital per trade
2. Always check wallet balance before trading
3. Use 0.5% slippage max
4. Skip if spread > 50 bps
5. Skip if regime is "choppy"
