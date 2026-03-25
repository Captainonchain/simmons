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

export default function Dashboard() {
  const { data, isConnected } = useWebSocket();
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" | "info" } | null>(null);

  const handleToast = useCallback((message: string, type: "success" | "error" | "info") => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 2000);
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

  const l = data?.layers;
  const portfolio = l?.decision_risk.portfolio;
  const regime = l?.ai_intelligence.regime;
  const kelly = l?.decision_risk.kelly_sizing;
  const risk = l?.decision_risk.risk_metrics;

  return (
    <div className="h-screen flex flex-col bg-bb-black overflow-hidden">
      {/* Header */}
      <Header isConnected={isConnected} />

      {/* Stats Bar */}
      <StatsBar
        equity={portfolio?.equity ?? 1000}
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

      {/* Main Grid - 12 column layout */}
      <div className="flex-1 min-h-0 p-0.5 grid grid-cols-12 grid-rows-[1fr_1fr] gap-0.5">

        {/* Row 1: Chart | Agent Debate | Circuit + Trade */}
        <div className="col-span-4 row-span-1">
          <PriceChart
            priceHistory={l?.data_ingestion.price_history ?? {}}
            availableSymbols={l?.data_ingestion.symbols.map((s) => s.symbol) ?? []}
          />
        </div>

        <div className="col-span-5 row-span-1">
          <AgentDebate />
        </div>

        <div className="col-span-3 row-span-1 grid grid-rows-2 gap-0.5">
          <CircuitBreaker
            triggered={false}
            riskLevel="normal"
            currentDrawdown={portfolio?.drawdown ?? 0}
            maxDrawdownLimit={portfolio?.max_drawdown ?? 0.2}
            consecutiveLosses={0}
            maxConsecutiveLosses={3}
            positionLimitUsed={risk?.position_limit_used ?? 0}
            canTrade={true}
          />
          <QuickTrade onToast={handleToast} />
        </div>

        {/* Row 2: Portfolio | Trade History | Memory */}
        <div className="col-span-4 row-span-1">
          <PortfolioChart portfolio={portfolio ?? null} />
        </div>

        <div className="col-span-5 row-span-1">
          <TradeHistory />
        </div>

        <div className="col-span-3 row-span-1">
          <MemoryInsights />
        </div>
      </div>

      {/* Toast notifications */}
      {toast && (
        <div
          className="fixed bottom-2 right-2 px-3 py-1.5 text-[10px] font-medium slide-in z-50"
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
