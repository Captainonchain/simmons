"use client";

import { memo } from "react";

interface StatsBarProps {
  equity: number;
  pnl: number;
  pnlPct: number;
  winRate: number;
  totalTrades: number;
  regime: string;
  volatility: number;
  kellySizePct: number;
  drawdown: number;
  maxDrawdown: number;
}

export const StatsBar = memo(function StatsBar({
  equity,
  pnl,
  pnlPct,
  winRate,
  totalTrades,
  regime,
  volatility,
  kellySizePct,
  drawdown,
  maxDrawdown,
}: StatsBarProps) {
  const isUp = pnl >= 0;
  const drawdownPct = drawdown * 100;
  const maxDdPct = maxDrawdown * 100;
  const drawdownRatio = maxDrawdown > 0 ? drawdown / maxDrawdown : 0;

  const getRegimeColor = (r: string) => {
    const lower = r.toLowerCase();
    if (lower.includes("bull") || lower.includes("trend")) return "text-bb-green";
    if (lower.includes("bear") || lower.includes("crash")) return "text-bb-red";
    if (lower.includes("volatile") || lower.includes("chop")) return "text-bb-amber";
    return "text-bb-cyan";
  };

  return (
    <div className="bg-bb-surface h-10 border-b border-bb-border flex items-stretch shrink-0">
      {/* Equity */}
      <div className="flex items-center gap-3 px-4 border-r border-bb-border">
        <div>
          <div className="text-[8px] text-bb-dim tracking-wider">EQUITY</div>
          <div className="text-bb-bright font-bold text-sm stat-value leading-none">
            ${equity.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
          </div>
        </div>
        <div className={`text-[11px] font-medium stat-value ${isUp ? "text-bb-green" : "text-bb-red"}`}>
          {isUp ? "▲" : "▼"} {isUp ? "+" : ""}{pnl.toFixed(2)} ({pnlPct.toFixed(2)}%)
        </div>
      </div>

      {/* Win Rate */}
      <div className="flex items-center px-4 border-r border-bb-border">
        <div>
          <div className="text-[8px] text-bb-dim tracking-wider">WIN RATE</div>
          <div className="flex items-baseline gap-1.5">
            <span className={`font-bold text-sm stat-value leading-none ${winRate >= 50 ? "text-bb-green" : "text-bb-red"}`}>
              {winRate.toFixed(0)}%
            </span>
            <span className="text-[9px] text-bb-dim">{totalTrades} trades</span>
          </div>
        </div>
      </div>

      {/* Regime */}
      <div className="flex items-center px-4 border-r border-bb-border">
        <div>
          <div className="text-[8px] text-bb-dim tracking-wider">REGIME</div>
          <div className="flex items-baseline gap-2">
            <span className={`font-bold text-sm leading-none uppercase ${getRegimeColor(regime)}`}>
              {regime}
            </span>
            <span className="text-[9px] text-bb-dim">VOL {volatility.toFixed(2)}</span>
          </div>
        </div>
      </div>

      {/* Kelly Size */}
      <div className="flex items-center px-4 border-r border-bb-border">
        <div>
          <div className="text-[8px] text-bb-dim tracking-wider">KELLY SIZE</div>
          <div className="flex items-baseline gap-1.5">
            <span className="text-bb-cyan font-bold text-sm stat-value leading-none">
              {kellySizePct.toFixed(0)}%
            </span>
            <span className="text-[9px] text-bb-dim">½K</span>
          </div>
        </div>
      </div>

      {/* Drawdown meter */}
      <div className="flex items-center px-4 flex-1">
        <div className="w-full">
          <div className="flex items-center justify-between mb-0.5">
            <span className="text-[8px] text-bb-dim tracking-wider">DRAWDOWN</span>
            <span className={`text-[10px] stat-value ${drawdownPct > 15 ? "text-bb-red" : drawdownPct > 10 ? "text-bb-amber" : "text-bb-dim"}`}>
              {drawdownPct.toFixed(1)}% / {maxDdPct.toFixed(0)}%
            </span>
          </div>
          <div className="h-1.5 bg-bb-border rounded-none overflow-hidden">
            <div
              className={`h-full transition-all duration-300 ${
                drawdownPct > 15 ? "bg-bb-red" : drawdownPct > 10 ? "bg-bb-amber" : "bg-bb-green"
              }`}
              style={{ width: `${Math.min(100, drawdownRatio * 100)}%` }}
            />
          </div>
        </div>
      </div>
    </div>
  );
});
