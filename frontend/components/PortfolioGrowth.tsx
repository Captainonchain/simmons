"use client";

import { memo, useMemo } from "react";
import { Liveline } from "liveline";
import type { PortfolioData } from "@/lib/types";

interface PortfolioGrowthProps {
  portfolio: PortfolioData | null;
}

// Generate simulated historical equity curve from current state
function generateEquityCurve(equity: number, pnlPct: number): { time: number; value: number }[] {
  const now = Date.now();
  const points = 120;
  const interval = 60_000; // 1 min per point = 2h window
  const startEquity = equity / (1 + pnlPct / 100);
  const data: { time: number; value: number }[] = [];

  for (let i = 0; i < points; i++) {
    const t = now - (points - i) * interval;
    const progress = i / points;
    // Smooth curve from start to current with some noise
    const noise = (Math.sin(i * 0.7) * 0.3 + Math.sin(i * 1.3) * 0.2 + Math.sin(i * 2.1) * 0.1) * startEquity * 0.002;
    const base = startEquity + (equity - startEquity) * progress;
    data.push({ time: t, value: base + noise });
  }

  return data;
}

export const PortfolioGrowth = memo(function PortfolioGrowth({ portfolio }: PortfolioGrowthProps) {
  const equity = portfolio?.equity ?? 100;
  const pnlPct = portfolio?.pnl_pct ?? 0;
  const pnl = portfolio?.pnl ?? 0;
  const isUp = pnl >= 0;

  const data = useMemo(() => generateEquityCurve(equity, pnlPct), [equity, pnlPct]);

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between min-w-0">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-bb-amber font-bold text-[10px] shrink-0">PORTFOLIO</span>
          <span className="text-bb-dim text-[9px] hidden sm:inline">EQUITY CURVE</span>
        </div>
        <div className="flex items-center gap-1.5 sm:gap-3 text-[10px] shrink-0">
          <span className="text-bb-bright font-bold">${equity.toFixed(2)}</span>
          <span className={isUp ? "text-bb-green" : "text-bb-red"}>
            {isUp ? "▲" : "▼"} {pnlPct.toFixed(2)}%
          </span>
        </div>
      </div>
      <div className="flex-1 min-h-0">
        <Liveline
          data={data}
          value={equity}
          color={isUp ? "#00CC66" : "#FF3333"}
          momentum={true}
          showValue={false}
          exaggerate={true}
          referenceLine={{ value: data[0]?.value ?? equity, color: "#444", label: "START" } as never}
        />
      </div>
    </div>
  );
});
