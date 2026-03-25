"use client";

import { memo, useState } from "react";

interface AgentVote {
  agent: string;
  recommendation: string;
  confidence: number;
}

interface Trade {
  id: string;
  symbol: string;
  side: "long" | "short";
  entryPrice: number;
  exitPrice?: number;
  pnl?: number;
  pnlPct?: number;
  outcome: "win" | "loss" | "breakeven" | "open";
  openedAt: string;
  closedAt?: string;
  reasoning: string;
  agentVotes?: AgentVote[];
}

interface TradeHistoryProps {
  trades?: Trade[];
}

const MOCK_TRADES: Trade[] = [
  {
    id: "t001",
    symbol: "BTC-USDT",
    side: "long",
    entryPrice: 67250,
    outcome: "open",
    openedAt: new Date().toISOString(),
    reasoning: "Multi-agent consensus: 4/4 analysts bullish",
    agentVotes: [
      { agent: "Technical", recommendation: "BUY", confidence: 0.72 },
      { agent: "Sentiment", recommendation: "BUY", confidence: 0.68 },
      { agent: "On-chain", recommendation: "BUY", confidence: 0.85 },
    ],
  },
  {
    id: "t002",
    symbol: "ETH-USDT",
    side: "long",
    entryPrice: 3420,
    exitPrice: 3485,
    pnl: 65,
    pnlPct: 1.9,
    outcome: "win",
    openedAt: new Date(Date.now() - 3600000).toISOString(),
    closedAt: new Date(Date.now() - 1800000).toISOString(),
    reasoning: "RSI oversold + whale accumulation",
    agentVotes: [
      { agent: "Technical", recommendation: "BUY", confidence: 0.78 },
      { agent: "On-chain", recommendation: "BUY", confidence: 0.82 },
    ],
  },
  {
    id: "t003",
    symbol: "SOL-USDT",
    side: "short",
    entryPrice: 142,
    exitPrice: 145,
    pnl: -30,
    pnlPct: -2.1,
    outcome: "loss",
    openedAt: new Date(Date.now() - 7200000).toISOString(),
    closedAt: new Date(Date.now() - 5400000).toISOString(),
    reasoning: "Resistance rejection (invalidated)",
    agentVotes: [
      { agent: "Technical", recommendation: "SELL", confidence: 0.55 },
    ],
  },
];

export const TradeHistory = memo(function TradeHistory({ trades = MOCK_TRADES }: TradeHistoryProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [filter, setFilter] = useState<"all" | "open" | "closed">("all");

  const filteredTrades = trades.filter((t) => {
    if (filter === "open") return t.outcome === "open";
    if (filter === "closed") return t.outcome !== "open";
    return true;
  });

  const getOutcomeStyle = (outcome: string) => {
    switch (outcome) {
      case "win":
        return { color: "text-bb-green", bg: "border-bb-green/30" };
      case "loss":
        return { color: "text-bb-red", bg: "border-bb-red/30" };
      case "open":
        return { color: "text-bb-cyan", bg: "border-bb-cyan/30" };
      default:
        return { color: "text-bb-dim", bg: "border-bb-border" };
    }
  };

  const formatTime = (ts: string) => {
    const d = new Date(ts);
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  const stats = {
    total: trades.length,
    wins: trades.filter((t) => t.outcome === "win").length,
    losses: trades.filter((t) => t.outcome === "loss").length,
    open: trades.filter((t) => t.outcome === "open").length,
  };

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <div className="flex items-center gap-2">
          <span className="grid-cell-title">TRADE HISTORY</span>
          <span className="text-[8px] text-bb-dim">{stats.total} total</span>
        </div>
        <div className="flex items-center gap-1 text-[8px]">
          <span className="text-bb-green">{stats.wins}W</span>
          <span className="text-bb-dim">/</span>
          <span className="text-bb-red">{stats.losses}L</span>
          {stats.open > 0 && (
            <>
              <span className="text-bb-dim">/</span>
              <span className="text-bb-cyan">{stats.open}O</span>
            </>
          )}
        </div>
      </div>

      {/* Filter Tabs */}
      <div className="flex border-b border-bb-border shrink-0">
        {(["all", "open", "closed"] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`flex-1 py-1 text-[8px] uppercase transition-colors ${
              filter === f ? "text-bb-orange bg-bb-raised" : "text-bb-dim hover:text-bb-white"
            }`}
          >
            {f}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto">
        {filteredTrades.length === 0 ? (
          <div className="h-full flex items-center justify-center text-bb-dim text-[10px]">
            No trades
          </div>
        ) : (
          <div className="divide-y divide-bb-border">
            {filteredTrades.map((trade) => {
              const style = getOutcomeStyle(trade.outcome);
              const isExpanded = expandedId === trade.id;

              return (
                <div key={trade.id} className="bg-bb-surface hover:bg-bb-raised transition-colors">
                  {/* Summary Row */}
                  <div
                    className={`px-2 py-1.5 flex items-center justify-between cursor-pointer border-l-2 ${style.bg}`}
                    onClick={() => setExpandedId(isExpanded ? null : trade.id)}
                  >
                    <div className="flex items-center gap-2">
                      <span className={trade.side === "long" ? "text-bb-green" : "text-bb-red"}>
                        {trade.side === "long" ? "▲" : "▼"}
                      </span>
                      <span className="text-bb-white text-[10px] font-medium">
                        {trade.symbol.replace("-USDT", "")}
                      </span>
                      <span className={`text-[8px] uppercase ${style.color}`}>
                        {trade.outcome}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      {trade.pnl !== undefined && (
                        <span className={`text-[10px] stat-value ${trade.pnl >= 0 ? "text-bb-green" : "text-bb-red"}`}>
                          {trade.pnl >= 0 ? "+" : ""}${trade.pnl.toFixed(0)}
                          {trade.pnlPct !== undefined && (
                            <span className="text-bb-dim ml-0.5">
                              ({trade.pnlPct >= 0 ? "+" : ""}{trade.pnlPct.toFixed(1)}%)
                            </span>
                          )}
                        </span>
                      )}
                      <span className="text-[8px] text-bb-dim">{formatTime(trade.openedAt)}</span>
                      <span className="text-[8px] text-bb-muted">{isExpanded ? "−" : "+"}</span>
                    </div>
                  </div>

                  {/* Expanded Details */}
                  {isExpanded && (
                    <div className="px-2 py-2 bg-bb-raised border-t border-bb-border space-y-2 slide-in">
                      {/* Prices */}
                      <div className="grid grid-cols-2 gap-2 text-[9px]">
                        <div>
                          <span className="text-bb-dim">Entry: </span>
                          <span className="text-bb-white stat-value">${trade.entryPrice.toLocaleString()}</span>
                        </div>
                        {trade.exitPrice && (
                          <div>
                            <span className="text-bb-dim">Exit: </span>
                            <span className="text-bb-white stat-value">${trade.exitPrice.toLocaleString()}</span>
                          </div>
                        )}
                      </div>

                      {/* Reasoning */}
                      <div className="text-[9px]">
                        <span className="text-bb-dim">Reason: </span>
                        <span className="text-bb-white">{trade.reasoning}</span>
                      </div>

                      {/* Agent Votes */}
                      {trade.agentVotes && trade.agentVotes.length > 0 && (
                        <div>
                          <div className="text-[8px] text-bb-dim mb-1">AGENT VOTES</div>
                          <div className="flex flex-wrap gap-1">
                            {trade.agentVotes.map((v, i) => (
                              <div
                                key={i}
                                className="bg-bb-surface border border-bb-border px-1.5 py-0.5 text-[8px]"
                              >
                                <span className="text-bb-cyan">{v.agent}</span>
                                <span className="text-bb-dim mx-1">→</span>
                                <span
                                  className={
                                    v.recommendation.includes("BUY") || v.recommendation.includes("LONG")
                                      ? "text-bb-green"
                                      : v.recommendation.includes("SELL") || v.recommendation.includes("SHORT")
                                      ? "text-bb-red"
                                      : "text-bb-amber"
                                  }
                                >
                                  {v.recommendation}
                                </span>
                                <span className="text-bb-muted ml-1">
                                  {(v.confidence * 100).toFixed(0)}%
                                </span>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
});
