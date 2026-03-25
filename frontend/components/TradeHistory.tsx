"use client";

import { useState } from "react";

interface AgentVote {
  agent: string;
  recommendation: string;
  confidence: number;
}

interface Trade {
  id: string;
  symbol: string;
  side: string;
  entryPrice: number;
  exitPrice?: number;
  pnl?: number;
  pnlPct?: number;
  outcome?: "win" | "loss" | "breakeven" | "open";
  openedAt: string;
  closedAt?: string;
  reasoning: string;
  agentVotes?: AgentVote[];
}

interface TradeHistoryProps {
  trades?: Trade[];
}

export function TradeHistory({ trades = [] }: TradeHistoryProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const getOutcomeColor = (outcome?: string) => {
    switch (outcome) {
      case "win": return "text-bb-green";
      case "loss": return "text-bb-red";
      case "breakeven": return "text-bb-amber";
      case "open": return "text-bb-cyan";
      default: return "text-bb-dim";
    }
  };

  const getOutcomeBg = (outcome?: string) => {
    switch (outcome) {
      case "win": return "border-bb-green/30";
      case "loss": return "border-bb-red/30";
      case "open": return "border-bb-cyan/30";
      default: return "border-bb-border";
    }
  };

  const formatTime = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col">
      <div className="bg-bb-panel px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <span className="text-[10px] text-bb-orange font-semibold tracking-wider">TRADE HISTORY</span>
        <span className="text-[9px] text-bb-dim">{trades.length} trades</span>
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-1.5 text-[10px]">
        {trades.length === 0 ? (
          <div className="text-bb-dim text-center py-4">No trades yet</div>
        ) : (
          trades.map((trade) => (
            <div
              key={trade.id}
              className={`bg-bb-raised border ${getOutcomeBg(trade.outcome)} cursor-pointer transition-colors hover:bg-bb-panel`}
              onClick={() => setExpandedId(expandedId === trade.id ? null : trade.id)}
            >
              {/* Trade Summary Row */}
              <div className="p-1.5 flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className={trade.side === "long" ? "text-bb-green" : "text-bb-red"}>
                    {trade.side === "long" ? "▲" : "▼"}
                  </span>
                  <span className="text-bb-white font-medium">{trade.symbol}</span>
                  <span className={`text-[9px] uppercase ${getOutcomeColor(trade.outcome)}`}>
                    {trade.outcome || "pending"}
                  </span>
                </div>
                <div className="flex items-center gap-3">
                  {trade.pnl !== undefined && (
                    <span className={trade.pnl >= 0 ? "text-bb-green" : "text-bb-red"}>
                      {trade.pnl >= 0 ? "+" : ""}{trade.pnl.toFixed(2)}
                      {trade.pnlPct !== undefined && (
                        <span className="text-bb-dim ml-1">
                          ({trade.pnlPct >= 0 ? "+" : ""}{trade.pnlPct.toFixed(1)}%)
                        </span>
                      )}
                    </span>
                  )}
                  <span className="text-bb-dim text-[9px]">{formatTime(trade.openedAt)}</span>
                  <span className="text-bb-dim">{expandedId === trade.id ? "▼" : "▶"}</span>
                </div>
              </div>

              {/* Expanded Details */}
              {expandedId === trade.id && (
                <div className="border-t border-bb-border p-2 space-y-2">
                  {/* Prices */}
                  <div className="grid grid-cols-2 gap-2 text-[9px]">
                    <div>
                      <span className="text-bb-dim">Entry:</span>
                      <span className="text-bb-white ml-1">${trade.entryPrice.toFixed(2)}</span>
                    </div>
                    {trade.exitPrice && (
                      <div>
                        <span className="text-bb-dim">Exit:</span>
                        <span className="text-bb-white ml-1">${trade.exitPrice.toFixed(2)}</span>
                      </div>
                    )}
                  </div>

                  {/* Reasoning */}
                  <div className="text-[9px]">
                    <span className="text-bb-dim">Reasoning:</span>
                    <div className="text-bb-white mt-0.5">{trade.reasoning}</div>
                  </div>

                  {/* Agent Votes */}
                  {trade.agentVotes && trade.agentVotes.length > 0 && (
                    <div>
                      <span className="text-bb-dim text-[9px]">Agent Votes:</span>
                      <div className="grid grid-cols-2 gap-1 mt-1">
                        {trade.agentVotes.map((vote, i) => (
                          <div
                            key={i}
                            className="bg-bb-surface border border-bb-border p-1 text-[9px]"
                          >
                            <div className="flex items-center justify-between">
                              <span className="text-bb-cyan truncate">{vote.agent}</span>
                              <span
                                className={
                                  vote.recommendation.toLowerCase().includes("buy")
                                    ? "text-bb-green"
                                    : vote.recommendation.toLowerCase().includes("sell")
                                    ? "text-bb-red"
                                    : "text-bb-amber"
                                }
                              >
                                {vote.recommendation}
                              </span>
                            </div>
                            <div className="h-0.5 bg-bb-border mt-0.5">
                              <div
                                className="h-full bg-bb-cyan"
                                style={{ width: `${vote.confidence * 100}%` }}
                              />
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
