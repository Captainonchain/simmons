"use client";

import { memo } from "react";

interface CircuitBreakerProps {
  triggered?: boolean;
  reason?: string | null;
  riskLevel?: "normal" | "elevated" | "critical";
  currentDrawdown?: number;
  maxDrawdownLimit?: number;
  consecutiveLosses?: number;
  maxConsecutiveLosses?: number;
  positionLimitUsed?: number;
  canTrade?: boolean;
}

export const CircuitBreaker = memo(function CircuitBreaker({
  triggered = false,
  reason = null,
  riskLevel = "normal",
  currentDrawdown = 0,
  maxDrawdownLimit = 0.2,
  consecutiveLosses = 0,
  maxConsecutiveLosses = 3,
  positionLimitUsed = 0,
  canTrade = true,
}: CircuitBreakerProps) {
  const drawdownPct = currentDrawdown * 100;
  const limitPct = maxDrawdownLimit * 100;
  const posLimitPct = positionLimitUsed * 100;

  const getRiskStyle = () => {
    switch (riskLevel) {
      case "critical":
        return { color: "text-bb-red", bg: "bg-bb-red/15", border: "border-bb-red/40", label: "CRITICAL" };
      case "elevated":
        return { color: "text-bb-amber", bg: "bg-bb-amber/15", border: "border-bb-amber/40", label: "ELEVATED" };
      default:
        return { color: "text-bb-green", bg: "bg-bb-green/15", border: "border-bb-green/40", label: "NORMAL" };
    }
  };

  const style = getRiskStyle();

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <div className="flex items-center gap-2">
          <span className="grid-cell-title">CIRCUIT BREAKER</span>
        </div>
        <div className="flex items-center gap-1.5">
          <span
            className={`w-2 h-2 rounded-full ${
              triggered ? "bg-bb-red blink" : canTrade ? "bg-bb-green" : "bg-bb-amber"
            }`}
          />
          <span className={`text-[10px] font-bold ${triggered ? "text-bb-red" : canTrade ? "text-bb-green" : "text-bb-amber"}`}>
            {triggered ? "HALTED" : canTrade ? "ACTIVE" : "LIMITED"}
          </span>
        </div>
      </div>

      <div className="grid-cell-body space-y-2.5">
        {/* Risk Level Badge */}
        <div className={`p-2.5 border ${style.bg} ${style.border} flex items-center justify-between`}>
          <span className="text-[9px] text-bb-dim uppercase tracking-wider">Risk Level</span>
          <span className={`text-[11px] font-bold uppercase ${style.color}`}>
            {style.label}
          </span>
        </div>

        {/* Triggered Alert */}
        {triggered && reason && (
          <div className="p-2 bg-bb-red/15 border border-bb-red/50 slide-in">
            <div className="text-bb-red text-[9px] font-bold uppercase mb-1 flex items-center gap-1">
              <span className="blink">⚠</span> TRADING HALTED
            </div>
            <div className="text-bb-white text-[10px]">{reason}</div>
          </div>
        )}

        {/* Metrics */}
        <div className="space-y-2.5">
          {/* Drawdown */}
          <div className="bg-bb-raised p-2.5 border border-bb-border">
            <div className="flex items-center justify-between text-[9px] mb-1.5">
              <span className="text-bb-dim uppercase tracking-wider">Drawdown</span>
              <span className={`font-bold stat-value ${drawdownPct > 15 ? "text-bb-red" : drawdownPct > 10 ? "text-bb-amber" : "text-bb-green"}`}>
                {drawdownPct.toFixed(1)}%
                <span className="text-bb-dim font-normal"> / {limitPct.toFixed(0)}%</span>
              </span>
            </div>
            <div className="h-[4px] bg-bb-border overflow-hidden">
              <div
                className={`h-full transition-all duration-500 ${
                  drawdownPct > 15 ? "bg-bb-red" : drawdownPct > 10 ? "bg-bb-amber" : "bg-bb-green"
                }`}
                style={{ width: `${Math.min(100, (drawdownPct / limitPct) * 100)}%` }}
              />
            </div>
          </div>

          {/* Consecutive Losses */}
          <div className="bg-bb-raised p-2.5 border border-bb-border">
            <div className="flex items-center justify-between text-[9px] mb-1.5">
              <span className="text-bb-dim uppercase tracking-wider">Consecutive Losses</span>
              <span className={`font-bold stat-value ${consecutiveLosses >= 2 ? "text-bb-red" : "text-bb-dim"}`}>
                {consecutiveLosses} / {maxConsecutiveLosses}
              </span>
            </div>
            <div className="flex gap-1">
              {Array.from({ length: maxConsecutiveLosses }).map((_, i) => (
                <div
                  key={i}
                  className={`flex-1 h-[6px] transition-colors ${
                    i < consecutiveLosses ? "bg-bb-red" : "bg-bb-border"
                  }`}
                />
              ))}
            </div>
          </div>

          {/* Position Limit */}
          <div className="bg-bb-raised p-2.5 border border-bb-border">
            <div className="flex items-center justify-between text-[9px] mb-1.5">
              <span className="text-bb-dim uppercase tracking-wider">Position Limit</span>
              <span className={`font-bold stat-value ${posLimitPct > 80 ? "text-bb-amber" : "text-bb-cyan"}`}>
                {posLimitPct.toFixed(0)}%
              </span>
            </div>
            <div className="h-[4px] bg-bb-border overflow-hidden">
              <div
                className={`h-full transition-all duration-500 ${posLimitPct > 80 ? "bg-bb-amber" : "bg-bb-cyan"}`}
                style={{ width: `${Math.min(100, posLimitPct)}%` }}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});
