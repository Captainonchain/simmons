"use client";

import { memo } from "react";
import type { RiskMetricsData, PortfolioData } from "@/lib/types";

function barColor(v: number) {
  return v < 0.5 ? "bg-bb-green" : v < 0.8 ? "bg-bb-amber" : "bg-bb-red";
}

export const RiskMetrics = memo(function RiskMetrics({ risk, portfolio }: { risk: RiskMetricsData | null; portfolio: PortfolioData | null }) {
  const metrics = [
    { label: "VAR 95%", value: `$${(risk?.var_95 ?? 250).toFixed(0)}`, pct: 25, r: 0.25 },
    { label: "DRAWDOWN", value: `${(portfolio?.drawdown ?? 0).toFixed(1)}%`, pct: (portfolio?.drawdown ?? 0) * 5, r: (portfolio?.drawdown ?? 0) / 20 },
    { label: "POS USED", value: `${((risk?.position_limit_used ?? 0) * 100).toFixed(0)}%`, pct: (risk?.position_limit_used ?? 0) * 100, r: risk?.position_limit_used ?? 0 },
    { label: "DAILY LOSS", value: `${((risk?.daily_loss_limit_used ?? 0) * 100).toFixed(0)}%`, pct: (risk?.daily_loss_limit_used ?? 0) * 100, r: risk?.daily_loss_limit_used ?? 0 },
  ];

  return (
    <div className="bg-bb-surface border border-bb-border h-full">
      <div className="px-2 py-1 border-b border-bb-border">
        <span className="text-bb-amber font-bold text-[10px]">RISK</span>
      </div>
      <div className="px-2 py-1.5 space-y-1.5">
        {metrics.map((m) => (
          <div key={m.label}>
            <div className="flex justify-between text-[9px]">
              <span className="text-bb-dim">{m.label}</span>
              <span className="text-bb-bright font-bold">{m.value}</span>
            </div>
            <div className="h-1 bg-bb-raised mt-0.5">
              <div className={`h-full ${barColor(m.r)} transition-all`} style={{ width: `${Math.min(m.pct, 100)}%` }} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
});
