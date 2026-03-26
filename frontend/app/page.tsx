"use client";

import { useCallback, useState, useEffect } from "react";
import { useWebSocket } from "@/hooks/useWebSocket";

// Core panels
import { Header } from "@/components/Header";
import { StatsBar } from "@/components/StatsBar";
import { PriceChart } from "@/components/PriceChart";
import { AgentDebate } from "@/components/AgentDebate";
import { CircuitBreaker } from "@/components/CircuitBreaker";
import { TradeHistory } from "@/components/TradeHistory";
import { MemoryInsights } from "@/components/MemoryInsights";
import { PortfolioChart } from "@/components/PortfolioChart";
import { DualBrainArchitecture } from "@/components/DualBrainArchitecture";
import { OpenPositions } from "@/components/OpenPositions";

// Types for API data
interface Trade {
  id: string;
  symbol: string;
  side: "long" | "short";
  entry_price?: number;
  exit_price?: number;
  pnl?: number;
  pnl_pct?: number;
  status: string;
  outcome: string;
  reasoning: string;
  timestamp?: string;
}

export default function Dashboard() {
  const { data, isConnected } = useWebSocket();
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" | "info" } | null>(null);
  const [trades, setTrades] = useState<Trade[]>([]);
  const [activeTab, setActiveTab] = useState<"brain" | "trading">("brain");

  const handleToast = useCallback((message: string, type: "success" | "error" | "info") => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 2000);
  }, []);

  // Fetch trades only (portfolio data comes from WebSocket now)
  useEffect(() => {
    const fetchTrades = async () => {
      try {
        const res = await fetch("/api/trades");
        if (res.ok) {
          const data = await res.json();
          if (data.trades) setTrades(data.trades);
        }
      } catch (e) {
        console.error("Failed to fetch trades:", e);
      }
    };
    fetchTrades();
    const interval = setInterval(fetchTrades, 5000);
    return () => clearInterval(interval);
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "?" && !e.ctrlKey && !e.metaKey) {
        handleToast("1=Brain 2=Trading L=Long S=Short", "info");
      }
      if (e.key === "1") setActiveTab("brain");
      if (e.key === "2") setActiveTab("trading");
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handleToast]);

  // All data from WebSocket (single source of truth)
  const l = data?.layers;
  const portfolio = l?.decision_risk.portfolio;
  const regime = l?.ai_intelligence.regime;
  const kelly = l?.decision_risk.kelly_sizing;
  const risk = l?.decision_risk.risk_metrics;
  const signals = l?.ai_intelligence.strategy_signals ?? [];
  const nunchi = l?.ai_intelligence.nunchi_score;

  // Transform signals into properly categorized agent votes
  const buildAgentVotes = () => {
    const votes: Array<{
      agent: string;
      role: "analyst" | "researcher" | "risk";
      recommendation: string;
      confidence: number;
      reason: string;
    }> = [];

    // Strategy type mapping
    const strategyConfig: Record<string, { name: string; role: "analyst" | "researcher" | "risk" }> = {
      momentum: { name: "Momentum", role: "analyst" },
      mean_reversion: { name: "Mean Reversion", role: "analyst" },
      technical: { name: "Technical", role: "analyst" },
      sentiment: { name: "Sentiment", role: "analyst" },
      regime: { name: "Regime", role: "researcher" },
      trend: { name: "Trend", role: "researcher" },
      volatility: { name: "Volatility", role: "researcher" },
    };

    // Aggregate signals by strategy (average across symbols)
    const aggregated = new Map<string, { signal: string; confidence: number; reasons: string[]; count: number }>();

    for (const s of signals) {
      const key = s.strategy;
      const existing = aggregated.get(key);
      if (existing) {
        existing.confidence += s.confidence;
        existing.count += 1;
        if (!existing.reasons.includes(s.reason)) {
          existing.reasons.push(s.reason);
        }
        // Use most bullish signal if mixed
        if (s.signal === "Buy" || s.signal === "Long") existing.signal = "BUY";
        else if (existing.signal !== "BUY" && (s.signal === "Sell" || s.signal === "Short")) existing.signal = "SELL";
      } else {
        aggregated.set(key, {
          signal: s.signal === "Buy" || s.signal === "Long" ? "BUY" : s.signal === "Sell" || s.signal === "Short" ? "SELL" : "HOLD",
          confidence: s.confidence,
          reasons: [s.reason],
          count: 1,
        });
      }
    }

    // Convert to votes
    for (const [strategy, data] of aggregated) {
      const config = strategyConfig[strategy] || { name: strategy, role: "analyst" as const };
      votes.push({
        agent: config.name,
        role: config.role,
        recommendation: data.signal,
        confidence: data.confidence / data.count,
        reason: data.reasons[0], // Use first reason
      });
    }

    // Add risk agents from Kelly sizing
    if (kelly) {
      votes.push({
        agent: "Kelly Criterion",
        role: "risk",
        recommendation: `${kelly.recommended_size_pct?.toFixed(0) ?? 10}%`,
        confidence: Math.min(1, (kelly.edge ?? 0.05) * 10 + 0.5),
        reason: `Edge ${((kelly.edge ?? 0.05) * 100).toFixed(1)}%, Win prob ${((kelly.win_prob ?? 0.55) * 100).toFixed(0)}%`,
      });
    }

    // Add position limit status
    if (risk) {
      const posUsed = risk.position_limit_used ?? 0;
      votes.push({
        agent: "Position Limit",
        role: "risk",
        recommendation: posUsed > 0.8 ? "REDUCE" : posUsed > 0.5 ? "CAUTION" : "OK",
        confidence: 1 - posUsed,
        reason: `${(posUsed * 100).toFixed(0)}% of limit used`,
      });
    }

    return votes;
  };

  const agentVotes = buildAgentVotes();

  // Convert trades to TradeHistory format
  const formattedTrades = trades.map(t => ({
    id: t.id,
    symbol: t.symbol,
    side: t.side,
    entryPrice: t.entry_price ?? 0,
    exitPrice: t.exit_price,
    pnl: t.pnl,
    pnlPct: t.pnl_pct,
    outcome: t.outcome as "win" | "loss" | "breakeven" | "open",
    openedAt: t.timestamp ?? new Date().toISOString(),
    reasoning: t.reasoning,
  }));

  return (
    <div className="h-screen flex flex-col bg-bb-black overflow-hidden">
      {/* Header - Connection status and branding */}
      <Header isConnected={isConnected} />

      {/* Tab Bar */}
      <div className="bg-bb-panel border-b border-bb-border px-3 md:px-4 py-1.5 flex items-center gap-1">
        <button
          onClick={() => setActiveTab("brain")}
          className={`px-3 py-1.5 text-[10px] font-bold tracking-wider uppercase transition-colors ${
            activeTab === "brain"
              ? "bg-bb-orange text-bb-black"
              : "text-bb-dim hover:text-bb-amber"
          }`}
        >
          [1] Dual Brain
        </button>
        <button
          onClick={() => setActiveTab("trading")}
          className={`px-3 py-1.5 text-[10px] font-bold tracking-wider uppercase transition-colors ${
            activeTab === "trading"
              ? "bg-bb-orange text-bb-black"
              : "text-bb-dim hover:text-bb-amber"
          }`}
        >
          [2] Trading
        </button>
        <div className="flex-1" />
        <div className="hidden md:block">
          <StatsBar
            equity={portfolio?.equity ?? 100}
            pnl={portfolio?.pnl ?? 0}
            pnlPct={portfolio?.pnl_pct ?? 0}
            winRate={portfolio?.win_rate ?? 0}
            totalTrades={portfolio?.total_trades ?? 0}
            regime={regime?.current ?? "loading"}
            volatility={regime?.volatility ?? 0}
            kellySizePct={kelly?.recommended_size_pct ?? 10}
            drawdown={portfolio?.drawdown ?? 0}
            maxDrawdown={portfolio?.max_drawdown ?? 0.2}
            compact={true}
          />
        </div>
      </div>

      {/* Main Content */}
      {activeTab === "brain" ? (
        /* DUAL BRAIN VIEW - Full width, responsive */
        <div className="flex-1 flex flex-col gap-px bg-bb-border p-px overflow-hidden md:overflow-hidden overflow-y-auto">
          {/* TOP - Dual Brain Architecture (full width, primary area) */}
          <div className="min-h-[400px] md:min-h-0 md:flex-[7] shrink-0 md:shrink">
            <DualBrainArchitecture />
          </div>

          {/* BOTTOM - Supporting Panels (3-col on desktop, stacked on mobile) */}
          <div className="min-h-[300px] md:min-h-0 md:flex-[3] grid grid-cols-1 md:grid-cols-3 gap-px">
            <div className="min-h-[200px] md:min-h-0">
              <AgentDebate
                bullConviction={nunchi?.score ?? 0.65}
                bearConviction={1 - (nunchi?.score ?? 0.65)}
                finalDecision={nunchi?.direction === "Bullish" ? "BUY" : nunchi?.direction === "Bearish" ? "SELL" : "HOLD"}
                agentVotes={agentVotes.length > 0 ? agentVotes : undefined}
                isLive={isConnected}
              />
            </div>
            <div className="min-h-[200px] md:min-h-0">
              <CircuitBreaker
                triggered={false}
                riskLevel={(risk?.daily_loss_limit_used ?? 0) > 0.5 ? "elevated" : "normal"}
                currentDrawdown={portfolio?.drawdown ?? 0}
                maxDrawdownLimit={portfolio?.max_drawdown ?? 0.2}
                consecutiveLosses={0}
                maxConsecutiveLosses={3}
                positionLimitUsed={risk?.position_limit_used ?? 0}
                canTrade={true}
              />
            </div>
            <div className="min-h-[200px] md:min-h-0">
              <MemoryInsights
                totalLearnings={portfolio?.total_trades ?? 0}
                totalReflections={Math.round((portfolio?.win_rate ?? 0) / 100 * (portfolio?.total_trades ?? 0))}
              />
            </div>
          </div>
        </div>
      ) : (
        /* TRADING VIEW - responsive grid */
        <div className="flex-1 flex flex-col md:grid md:grid-cols-12 gap-px bg-bb-border p-px overflow-y-auto md:overflow-hidden">
          {/* LEFT COLUMN - Charts */}
          <div className="min-h-[250px] md:min-h-0 md:col-span-4 flex flex-col gap-px">
            <div className="min-h-[200px] md:min-h-0 md:flex-1">
              <PriceChart
                priceHistory={l?.data_ingestion.price_history ?? {}}
                availableSymbols={l?.data_ingestion.symbols.map((s) => s.symbol) ?? []}
              />
            </div>
            <div className="min-h-[200px] md:min-h-0 md:flex-1">
              <PortfolioChart portfolio={portfolio ?? null} />
            </div>
          </div>

          {/* CENTER COLUMN - Decision & Execution */}
          <div className="min-h-[300px] md:min-h-0 md:col-span-5 flex flex-col gap-px">
            <div className="min-h-[200px] md:min-h-0 md:h-[35%]">
              <AgentDebate
                bullConviction={nunchi?.score ?? 0.65}
                bearConviction={1 - (nunchi?.score ?? 0.65)}
                finalDecision={nunchi?.direction === "Bullish" ? "BUY" : nunchi?.direction === "Bearish" ? "SELL" : "HOLD"}
                agentVotes={agentVotes.length > 0 ? agentVotes : undefined}
                isLive={isConnected}
              />
            </div>
            <div className="min-h-[200px] md:min-h-0 md:h-[30%]">
              <OpenPositions
                positions={(l?.decision_risk.positions ?? []).map((p: { symbol: string; side: string; entry_price: number; current_price: number; size_pct: number; pnl: number; pnl_pct: number; opened_at: string }) => ({
                  symbol: p.symbol,
                  side: p.side as "long" | "short",
                  entry_price: p.entry_price,
                  current_price: p.current_price,
                  size_pct: p.size_pct,
                  pnl: p.pnl,
                  pnl_pct: p.pnl_pct,
                  opened_at: p.opened_at,
                }))}
                isLive={isConnected}
              />
            </div>
            <div className="min-h-[200px] md:min-h-0 md:flex-1">
              <TradeHistory trades={formattedTrades} />
            </div>
          </div>

          {/* RIGHT COLUMN - Risk & Actions */}
          <div className="min-h-[200px] md:min-h-0 md:col-span-3 flex flex-col gap-px">
            <div className="min-h-[200px] md:min-h-0 md:h-[50%]">
              <CircuitBreaker
                triggered={false}
                riskLevel={(risk?.daily_loss_limit_used ?? 0) > 0.5 ? "elevated" : "normal"}
                currentDrawdown={portfolio?.drawdown ?? 0}
                maxDrawdownLimit={portfolio?.max_drawdown ?? 0.2}
                consecutiveLosses={0}
                maxConsecutiveLosses={3}
                positionLimitUsed={risk?.position_limit_used ?? 0}
                canTrade={true}
              />
            </div>
            <div className="min-h-[200px] md:min-h-0 md:flex-1">
              <MemoryInsights
                totalLearnings={portfolio?.total_trades ?? 0}
                totalReflections={Math.round((portfolio?.win_rate ?? 0) / 100 * (portfolio?.total_trades ?? 0))}
              />
            </div>
          </div>
        </div>
      )}

      {/* Toast notifications */}
      {toast && (
        <div
          className="fixed bottom-3 right-3 px-4 py-2 text-[10px] font-bold tracking-wide uppercase slide-in z-50 shadow-lg max-w-[300px]"
          style={{
            background: toast.type === "success" ? "#00DD55" : toast.type === "error" ? "#FF2222" : "#FFAA00",
            color: "#000",
          }}
        >
          {toast.message}
        </div>
      )}
    </div>
  );
}
