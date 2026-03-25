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
        return { color: "text-bb-red", bg: "bg-bb-red/10", border: "border-bb-red/30" };
      case "elevated":
        return { color: "text-bb-amber", bg: "bg-bb-amber/10", border: "border-bb-amber/30" };
      default:
        return { color: "text-bb-green", bg: "bg-bb-green/10", border: "border-bb-green/30" };
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
          <span className={`text-[9px] font-bold ${triggered ? "text-bb-red" : canTrade ? "text-bb-green" : "text-bb-amber"}`}>
            {triggered ? "HALTED" : canTrade ? "ACTIVE" : "LIMITED"}
          </span>
        </div>
      </div>

      <div className="grid-cell-body space-y-2">
        {/* Risk Level Badge */}
        <div className={`p-2 border ${style.bg} ${style.border}`}>
          <div className="flex items-center justify-between">
            <span className="text-[8px] text-bb-dim">RISK LEVEL</span>
            <span className={`text-[10px] font-bold uppercase ${style.color}`}>
              {riskLevel}
            </span>
          </div>
        </div>

        {/* Triggered Reason */}
        {triggered && reason && (
          <div className="p-2 bg-bb-red/10 border border-bb-red/50">
            <div className="text-bb-red text-[8px] font-bold uppercase mb-0.5">HALTED</div>
            <div className="text-bb-white text-[9px]">{reason}</div>
          </div>
        )}

        {/* Metrics */}
        <div className="space-y-1.5">
          {/* Drawdown */}
          <div>
            <div className="flex items-center justify-between text-[8px] mb-0.5">
              <span className="text-bb-dim">DRAWDOWN</span>
              <span className={drawdownPct > 15 ? "text-bb-red" : drawdownPct > 10 ? "text-bb-amber" : "text-bb-dim"}>
                {drawdownPct.toFixed(1)}% / {limitPct.toFixed(0)}%
              </span>
            </div>
            <div className="h-1.5 bg-bb-border">
              <div
                className={`h-full transition-all ${
                  drawdownPct > 15 ? "bg-bb-red" : drawdownPct > 10 ? "bg-bb-amber" : "bg-bb-green"
                }`}
                style={{ width: `${Math.min(100, (drawdownPct / limitPct) * 100)}%` }}
              />
            </div>
          </div>

          {/* Consecutive Losses */}
          <div>
            <div className="flex items-center justify-between text-[8px] mb-0.5">
              <span className="text-bb-dim">CONSECUTIVE LOSSES</span>
              <span className={consecutiveLosses >= 2 ? "text-bb-red" : "text-bb-dim"}>
                {consecutiveLosses} / {maxConsecutiveLosses}
              </span>
            </div>
            <div className="flex gap-0.5">
              {Array.from({ length: maxConsecutiveLosses }).map((_, i) => (
                <div
                  key={i}
                  className={`flex-1 h-1.5 ${i < consecutiveLosses ? "bg-bb-red" : "bg-bb-border"}`}
                />
              ))}
            </div>
          </div>

          {/* Position Limit */}
          <div>
            <div className="flex items-center justify-between text-[8px] mb-0.5">
              <span className="text-bb-dim">POSITION LIMIT</span>
              <span className={posLimitPct > 80 ? "text-bb-amber" : "text-bb-dim"}>
                {posLimitPct.toFixed(0)}%
              </span>
            </div>
            <div className="h-1.5 bg-bb-border">
              <div
                className={`h-full transition-all ${posLimitPct > 80 ? "bg-bb-amber" : "bg-bb-cyan"}`}
                style={{ width: `${Math.min(100, posLimitPct)}%` }}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});
