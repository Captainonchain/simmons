"use client";

import { useCallback, useMemo, useState } from "react";
import { useWebSocket } from "@/hooks/useWebSocket";
import { Header } from "@/components/Header";
import { Sidebar } from "@/components/Sidebar";
import { StatsRow } from "@/components/StatsRow";
import { PriceChart } from "@/components/PriceChart";
import { MarketsTable } from "@/components/MarketsTable";
import { SignalsList } from "@/components/SignalsList";
import { TABrain } from "@/components/TABrain";
import { FundBrain } from "@/components/FundBrain";
import { ConsensusLayer } from "@/components/ConsensusLayer";
import { Orchestrator } from "@/components/Orchestrator";
import { ExecutionPanel } from "@/components/ExecutionPanel";
import { RiskMetrics } from "@/components/RiskMetrics";
import { KellyCriterion } from "@/components/KellyCriterion";
import { PortfolioGrowth } from "@/components/PortfolioGrowth";
import { FeedbackLoop } from "@/components/FeedbackLoop";
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

  const panels: Record<string, React.ReactNode> = useMemo(() => ({
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
    "ta-brain": <TABrain taBrain={null} />,
    "fund-brain": <FundBrain fundBrain={null} />,
    consensus: <ConsensusLayer consensus={null} radarScore={172} />,
    orchestrator: <Orchestrator orchestrator={null} onToast={handleToast} />,
    execution: <ExecutionPanel execMode={null} execution={l?.execution ?? null} />,
    signals: <SignalsList signals={l?.ai_intelligence.strategy_signals ?? []} />,
    portfolio: <PortfolioGrowth portfolio={l?.decision_risk.portfolio ?? null} />,
    risk: <RiskMetrics risk={l?.decision_risk.risk_metrics ?? null} portfolio={l?.decision_risk.portfolio ?? null} />,
    kelly: <KellyCriterion kelly={l?.decision_risk.kelly_sizing ?? null} equity={l?.decision_risk.portfolio?.equity ?? 100} />,
    feedback: <FeedbackLoop feedback={l?.feedback ?? null} />,
  }), [data, l, handleToast]);

  return (
    <div className="h-screen flex flex-col bg-bb-black overflow-hidden">
      <Header
        isConnected={isConnected}
        onToggleSidebar={() => setSidebarOpen(!sidebarOpen)}
      />

      {/* Edit mode toggle bar */}
      <div className="bg-bb-panel border-b border-bb-border h-6 px-2 sm:px-3 flex items-center justify-between text-[9px] shrink-0">
        <div className="flex items-center gap-2 sm:gap-3 min-w-0">
          <span className="text-bb-dim hidden sm:inline">LAYOUT</span>
          <button
            onClick={() => setEditMode(!editMode)}
            className={`px-2 py-0.5 font-bold transition-colors shrink-0 ${
              editMode
                ? "bg-bb-orange text-bb-black"
                : "bg-bb-raised text-bb-dim hover:text-bb-white border border-bb-border"
            }`}
          >
            {editMode ? "● EDITING" : "CUSTOMIZE"}
          </button>
          {editMode && (
            <span className="text-bb-amber hidden sm:inline truncate">DRAG HEADERS TO MOVE · CORNERS TO RESIZE</span>
          )}
        </div>
        <div className="flex items-center gap-2 shrink-0">
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
          className="fixed bottom-3 left-3 right-3 sm:left-auto sm:right-3 sm:max-w-xs bg-bb-surface border border-bb-border px-3 py-2 z-50 animate-slide-up text-[10px]"
          style={{ borderLeft: `2px solid ${toast.type === "success" ? "#00CC66" : toast.type === "error" ? "#FF3333" : "#FF6600"}` }}
        >
          <span className="text-bb-white">{toast.message}</span>
        </div>
      )}
    </div>
  );
}
