"use client";

import { useCallback, useState } from "react";
import { useWebSocket } from "@/hooks/useWebSocket";
import { Header } from "@/components/Header";
import { Sidebar } from "@/components/Sidebar";
import { StatsRow } from "@/components/StatsRow";
import { PriceChart } from "@/components/PriceChart";
import { MarketsTable } from "@/components/MarketsTable";
import { SignalsList } from "@/components/SignalsList";
import { NunchiAggregator } from "@/components/NunchiAggregator";
import { ClaudeBrain } from "@/components/ClaudeBrain";
import { RiskMetrics } from "@/components/RiskMetrics";
import { KellyCriterion } from "@/components/KellyCriterion";
import { MevShield } from "@/components/MevShield";
import { XLayerInfra } from "@/components/XLayerInfra";
import { FeedbackLoop } from "@/components/FeedbackLoop";
import { Headlines } from "@/components/Headlines";
import { PortfolioGrowth } from "@/components/PortfolioGrowth";
import { DashboardGrid } from "@/components/DashboardGrid";

export default function Dashboard() {
  const { data, isConnected } = useWebSocket();
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" | "info" } | null>(null);

  const handleToast = useCallback((message: string, type: "success" | "error" | "info") => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 2500);
  }, []);

  const l = data?.layers;

  const panels: Record<string, React.ReactNode> = {
    stats: <StatsRow data={data} />,
    chart: (
      <PriceChart
        priceHistory={l?.data_ingestion.price_history ?? {}}
        availableSymbols={l?.data_ingestion.symbols.map((s) => s.symbol) ?? []}
      />
    ),
    markets: (
      <MarketsTable
        symbols={l?.data_ingestion.symbols ?? []}
        regime={l?.ai_intelligence.regime.current ?? "Loading"}
      />
    ),
    signals: <SignalsList signals={l?.ai_intelligence.strategy_signals ?? []} />,
    portfolio: <PortfolioGrowth portfolio={l?.decision_risk.portfolio ?? null} />,
    brain: <ClaudeBrain onToast={handleToast} />,
    nunchi: <NunchiAggregator nunchi={l?.ai_intelligence.nunchi_score ?? null} />,
    risk: <RiskMetrics risk={l?.decision_risk.risk_metrics ?? null} portfolio={l?.decision_risk.portfolio ?? null} />,
    kelly: <KellyCriterion kelly={l?.decision_risk.kelly_sizing ?? null} equity={l?.decision_risk.portfolio.equity ?? 100} />,
    mev: <MevShield mev={l?.execution.mev_shield ?? null} />,
    infra: <XLayerInfra infra={l?.infrastructure ?? null} />,
    feedback: <FeedbackLoop feedback={l?.feedback ?? null} />,
    headlines: <Headlines />,
  };

  return (
    <div className="h-screen flex flex-col bg-bb-black overflow-hidden">
      <Header
        isConnected={isConnected}
        onToggleSidebar={() => setSidebarOpen(!sidebarOpen)}
      />

      {/* Edit mode toggle bar */}
      <div className="bg-bb-panel border-b border-bb-border h-6 px-3 flex items-center justify-between text-[9px] shrink-0">
        <div className="flex items-center gap-3">
          <span className="text-bb-dim">LAYOUT</span>
          <button
            onClick={() => setEditMode(!editMode)}
            className={`px-2 py-0.5 font-bold transition-colors ${
              editMode
                ? "bg-bb-orange text-bb-black"
                : "bg-bb-raised text-bb-dim hover:text-bb-white border border-bb-border"
            }`}
          >
            {editMode ? "● EDITING" : "CUSTOMIZE"}
          </button>
          {editMode && (
            <span className="text-bb-amber">DRAG HEADERS TO MOVE · CORNERS TO RESIZE</span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {editMode && (
            <button
              onClick={() => setEditMode(false)}
              className="px-2 py-0.5 bg-bb-green text-bb-black font-bold"
            >
              DONE
            </button>
          )}
        </div>
      </div>

      <div className="flex flex-1 min-h-0">
        <Sidebar dataIngestion={l?.data_ingestion} open={sidebarOpen} onClose={() => setSidebarOpen(false)} />

        <main className="flex-1 overflow-y-auto">
          <DashboardGrid editMode={editMode}>
            {panels}
          </DashboardGrid>
        </main>
      </div>

      {/* Toast */}
      {toast && (
        <div
          className="fixed bottom-3 right-3 bg-bb-surface border border-bb-border px-3 py-2 z-50 animate-slide-up text-[10px]"
          style={{ borderLeft: `2px solid ${toast.type === "success" ? "#00CC66" : toast.type === "error" ? "#FF3333" : "#FF6600"}` }}
        >
          <span className="text-bb-white">{toast.message}</span>
        </div>
      )}
    </div>
  );
}
