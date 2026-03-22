"use client";

import { memo } from "react";
import { StatCard } from "./StatCard";
import type { DashboardUpdate } from "@/lib/types";

export const StatsRow = memo(function StatsRow({ data }: { data: DashboardUpdate | null }) {
  const p = data?.layers.decision_risk.portfolio;
  const n = data?.layers.ai_intelligence.nunchi_score;
  const r = data?.layers.ai_intelligence.regime;
  const k = data?.layers.decision_risk.kelly_sizing;
  const g = data?.layers.execution.gas;
  const pnl = p?.pnl ?? 0;

  return (
    <div className="grid grid-cols-3 md:grid-cols-6 gap-px h-full">
      <StatCard label="EQUITY" value={`$${(p?.equity ?? 100).toFixed(2)}`} change={`${pnl >= 0 ? "+" : ""}${pnl.toFixed(2)} (${(p?.pnl_pct ?? 0).toFixed(1)}%)`} changeUp={pnl >= 0} />
      <StatCard label="WIN RATE" value={`${(p?.win_rate ?? 0).toFixed(0)}%`} change={`${p?.total_trades ?? 0} TRADES`} />
      <StatCard label="NUNCHI" value={(n?.score ?? 0.65).toFixed(2)} change={n?.direction?.toUpperCase() ?? "BULLISH"} />
      <StatCard label="REGIME" value={r?.current?.toUpperCase() ?? "LOADING"} change={`VOL ${r ? r.volatility.toFixed(2) : "--"}`} />
      <StatCard label="KELLY" value={`${(k?.recommended_size_pct ?? 10).toFixed(0)}%`} change="HALF-KELLY" />
      <StatCard label="GAS" value={`${(g?.current_gwei ?? 25).toFixed(0)}`} change="GWEI" />
    </div>
  );
});
