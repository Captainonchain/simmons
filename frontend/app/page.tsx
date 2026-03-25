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
import { QuickTrade } from "@/components/QuickTrade";

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

  // Keyboard shortcut hints
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "?" && !e.ctrlKey && !e.metaKey) {
        handleToast("L=Long S=Short Esc=Cancel", "info");
      }
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

      {/* Stats Bar - Key metrics at a glance */}
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
      />

      {/* Main Grid - 3 Column Layout */}
      <div className="flex-1 grid grid-cols-12 gap-px bg-bb-border p-px overflow-hidden">
        {/* LEFT COLUMN - Charts (4 cols) */}
        <div className="col-span-4 flex flex-col gap-px">
          {/* Price Chart - Primary market view */}
          <div className="flex-1 min-h-0">
            <PriceChart
              priceHistory={l?.data_ingestion.price_history ?? {}}
              availableSymbols={l?.data_ingestion.symbols.map((s) => s.symbol) ?? []}
            />
          </div>
          {/* Portfolio Chart - Equity curve */}
          <div className="flex-1 min-h-0">
            <PortfolioChart portfolio={portfolio ?? null} />
          </div>
        </div>

        {/* CENTER COLUMN - Decision & Execution (5 cols) */}
        <div className="col-span-5 flex flex-col gap-px">
          {/* Agent Debate - Multi-agent decision making */}
          <div className="h-[55%] min-h-0">
            <AgentDebate
              bullConviction={nunchi?.score ?? 0.65}
              bearConviction={1 - (nunchi?.score ?? 0.65)}
              finalDecision={nunchi?.direction === "Bullish" ? "BUY" : nunchi?.direction === "Bearish" ? "SELL" : "HOLD"}
              agentVotes={signals.length > 0 ? signals.map((s, i) => ({
                agent: s.strategy,
                role: (i < 4 ? "analyst" : i < 6 ? "researcher" : "risk") as "analyst" | "researcher" | "risk",
                recommendation: s.signal === "Long" ? "BUY" : s.signal === "Short" ? "SELL" : "HOLD",
                confidence: s.confidence,
                reason: s.reason,
              })) : undefined}
              isLive={isConnected}
            />
          </div>
          {/* Trade History - Recent trades with details */}
          <div className="flex-1 min-h-0">
            <TradeHistory trades={formattedTrades} />
          </div>
        </div>

        {/* RIGHT COLUMN - Risk & Actions (3 cols) */}
        <div className="col-span-3 flex flex-col gap-px">
          {/* Circuit Breaker - Risk monitoring */}
          <div className="h-[40%] min-h-0">
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
          {/* Quick Trade - Manual execution */}
          <div className="h-[25%] min-h-0">
            <QuickTrade onToast={handleToast} />
          </div>
          {/* Memory Insights - Learning system */}
          <div className="flex-1 min-h-0">
            <MemoryInsights
              totalLearnings={portfolio?.total_trades ?? 0}
              totalReflections={Math.round((portfolio?.win_rate ?? 0) / 100 * (portfolio?.total_trades ?? 0))}
            />
          </div>
        </div>
      </div>

      {/* Toast notifications */}
      {toast && (
        <div
          className="fixed bottom-3 right-3 px-4 py-2 text-[10px] font-bold tracking-wide uppercase slide-in z-50 shadow-lg"
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
