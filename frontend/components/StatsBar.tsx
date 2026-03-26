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
    if (lower.includes("bull") || lower.includes("trend") || lower.includes("up")) return "text-bb-green bg-bb-green/10 border-bb-green/30";
    if (lower.includes("bear") || lower.includes("crash") || lower.includes("down")) return "text-bb-red bg-bb-red/10 border-bb-red/30";
    if (lower.includes("volatile") || lower.includes("chop") || lower.includes("high")) return "text-bb-amber bg-bb-amber/10 border-bb-amber/30";
    return "text-bb-cyan bg-bb-cyan/10 border-bb-cyan/30";
  };

  return (
    <div className="bg-bb-surface h-11 border-b border-bb-border flex items-stretch shrink-0">
      {/* Equity + P&L */}
      <div className="flex items-center gap-4 px-4 border-r border-bb-border min-w-[200px]">
        <div>
          <div className="text-[7px] text-bb-dim tracking-widest uppercase">EQUITY</div>
          <div className="text-bb-bright font-bold text-[15px] stat-value leading-none">
            ${equity.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
          </div>
        </div>
        <div className={`px-2 py-1 border ${isUp ? "bg-bb-green/10 border-bb-green/30" : "bg-bb-red/10 border-bb-red/30"}`}>
          <div className={`text-[11px] font-bold stat-value ${isUp ? "text-bb-green" : "text-bb-red"}`}>
            {isUp ? "+" : ""}{pnl.toFixed(2)}
          </div>
          <div className={`text-[8px] stat-value ${isUp ? "text-bb-green/70" : "text-bb-red/70"}`}>
            {isUp ? "+" : ""}{pnlPct.toFixed(2)}%
          </div>
        </div>
      </div>

      {/* Win Rate */}
      <div className="flex items-center px-4 border-r border-bb-border">
        <div>
          <div className="text-[7px] text-bb-dim tracking-widest uppercase">WIN RATE</div>
          <div className="flex items-baseline gap-2">
            <span className={`font-bold text-[15px] stat-value leading-none ${winRate >= 50 ? "text-bb-green" : "text-bb-red"}`}>
              {winRate.toFixed(0)}%
            </span>
            <span className="text-[9px] text-bb-dim">{totalTrades} trades</span>
          </div>
        </div>
      </div>

      {/* Regime */}
      <div className="flex items-center px-4 border-r border-bb-border">
        <div>
          <div className="text-[7px] text-bb-dim tracking-widest uppercase">REGIME</div>
          <div className="flex items-center gap-2">
            <span className={`px-2 py-0.5 text-[10px] font-bold uppercase border ${getRegimeColor(regime)}`}>
              {regime}
            </span>
            <span className="text-[9px] text-bb-dim">σ {volatility.toFixed(2)}</span>
          </div>
        </div>
      </div>

      {/* Kelly Size */}
      <div className="flex items-center px-4 border-r border-bb-border">
        <div>
          <div className="text-[7px] text-bb-dim tracking-widest uppercase">KELLY SIZE</div>
          <div className="flex items-baseline gap-1">
            <span className="text-bb-cyan font-bold text-[15px] stat-value leading-none">
              {kellySizePct.toFixed(0)}%
            </span>
            <span className="text-[8px] text-bb-dim">½K</span>
          </div>
        </div>
      </div>

      {/* Drawdown Meter */}
      <div className="flex items-center px-4 flex-1">
        <div className="w-full max-w-[300px]">
          <div className="flex items-center justify-between mb-1">
            <span className="text-[7px] text-bb-dim tracking-widest uppercase">DRAWDOWN</span>
            <span className={`text-[10px] font-medium stat-value ${drawdownPct > 15 ? "text-bb-red" : drawdownPct > 10 ? "text-bb-amber" : "text-bb-green"}`}>
              {drawdownPct.toFixed(1)}% / {maxDdPct.toFixed(0)}%
            </span>
          </div>
          <div className="h-2 bg-bb-border overflow-hidden relative">
            {/* Threshold markers */}
            <div className="absolute top-0 bottom-0 left-[50%] w-px bg-bb-amber/50" />
            <div className="absolute top-0 bottom-0 left-[75%] w-px bg-bb-red/50" />
            {/* Fill bar */}
            <div
              className={`h-full transition-all duration-500 ${
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
