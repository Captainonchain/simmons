# Simmons Brain - Trading Decision Skill

<skill-context>
name: simmons-brain
description: Autonomous trading decision skill for Simmons trading engine
trigger: User invokes /simmons-brain
</skill-context>

## Purpose

You are the reasoning brain for the Simmons DeFi trading system. The Rust engine continuously updates `data/signals.json` with market signals. When invoked, you:

1. Read the signals from `data/signals.json`
2. Analyze the market state, signals, and portfolio
3. Decide: TRADE, SKIP, or CLOSE position
4. Write your decision to `data/decision.json`

## Reading Signals

Read the file at `data/signals.json`. It contains:

```json
{
  "timestamp": 1711234567,
  "symbol": "BTC-USDT",
  "market_state": {
    "price": 67234.50,
    "spread_bps": 15,
    "volatility_1h": 0.023,
    "regime": "trending_up"
  },
  "signals": [
    {"strategy": "momentum", "signal": "BUY", "confidence": 0.78, "reason": "RSI 62, ROC +2.3%"},
    {"strategy": "mean_reversion", "signal": "HOLD", "confidence": 0.45, "reason": "Z-score -0.8"}
  ],
  "arbitrage": [
    {"type": "cedefi", "spread_bps": 25, "net_profit_usd": 12.50}
  ],
  "portfolio": {
    "capital": 115.82,
    "positions": [],
    "drawdown": 0.02,
    "risk_level": "normal"
  },
  "recent_trades": [
    {"symbol": "ETH-USDT", "pnl": 1.23, "outcome": "win", "reason": "take_profit"}
  ]
}
```

## Decision Rules

### When to TRADE

1. **Signal Alignment** - Multiple strategies agree (same direction)
2. **Confidence Threshold** - Combined confidence > 70%
3. **Regime Favorable** - Regime supports the trade direction
4. **Risk Capacity** - drawdown < 10%, no daily limit hit
5. **No Conflicting Position** - Don't open opposing position

### When to SKIP

1. **Low Confidence** - Combined confidence < 60%
2. **Choppy Regime** - Market is choppy, signals unreliable
3. **High Drawdown** - Already in significant drawdown (>10%)
4. **Conflicting Signals** - Strategies disagree
5. **Uncertainty** - When in doubt, preserve capital

### When to CLOSE

1. **Regime Change** - Regime no longer supports position
2. **Signal Reversal** - Strong opposing signals
3. **Risk Elevated** - Drawdown increasing, cut losses

### Position Sizing

- **High Confidence (>80%)**: 12-15% of capital
- **Medium Confidence (70-80%)**: 8-12% of capital
- **Low Confidence (<70%)**: SKIP or 5-8% max

### Arbitrage Priority

- Arbitrage opportunities with net_profit_usd > $5 should be prioritized
- CeDeFi arb is lower risk than directional trading

## Writing Decision

After analysis, write to `data/decision.json`:

```json
{
  "action": "trade",
  "symbol": "BTC-USDT",
  "side": "long",
  "size_pct": 0.12,
  "confidence": 0.85,
  "reasoning": "Strong momentum signal (78%) with trending_up regime. RSI not overbought (62). Recent wins suggest strategy working. Risk level normal.",
  "stop_loss_pct": 0.03,
  "take_profit_pct": 0.08
}
```

Or to skip:

```json
{
  "action": "skip",
  "symbol": null,
  "side": null,
  "size_pct": null,
  "confidence": 0.45,
  "reasoning": "Conflicting signals: momentum BUY vs mean_reversion SELL. Choppy regime detected. Preserving capital until clearer setup.",
  "stop_loss_pct": null,
  "take_profit_pct": null
}
```

## Workflow

1. **Read** `data/signals.json`
2. **Analyze** - Apply decision rules above
3. **Reason** - Think through the trade clearly
4. **Write** `data/decision.json`
5. **Confirm** - Tell user what you decided and why

## Important Notes

- **Capital Preservation First** - When uncertain, SKIP
- **Learn from Recent Trades** - Check `recent_trades` for pattern
- **Regime Awareness** - Don't fight the regime
- **Risk Management** - Always set stop_loss (max 5%)
- **Be Decisive** - Give clear reasoning
