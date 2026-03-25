"use client";

import { memo, useState, useCallback } from "react";
import { submitDecision } from "@/lib/api";
import type { OrchestratorData } from "@/lib/types";

interface OrchestratorProps {
  orchestrator: OrchestratorData | null;
  onToast: (message: string, type: "success" | "error" | "info") => void;
}

const strategyColors: Record<string, string> = {
  MM: "text-bb-cyan",
  ARB: "text-bb-magenta",
  Directional: "text-bb-green",
};

export const Orchestrator = memo(function Orchestrator({ orchestrator, onToast }: OrchestratorProps) {
  const [loading, setLoading] = useState<string | null>(null);
  const debate = orchestrator?.debate ?? {
    bull_thesis: "Momentum confirmed across multiple timeframes. RADAR 172 with positive funding rates and rising OI. Whale accumulation signals align with technical breakout pattern.",
    bear_thesis: "Elevated funding suggests crowded long. Historical mean reversion due after 3 consecutive up days. DXY showing early reversal signs.",
    risk_assessment: "Position size within Kelly bounds. VAR 95% acceptable. Guard Phase 1 active with 2 stops. Correlation risk low across portfolio.",
    verdict: "PROCEED — majority bull with risk-managed sizing",
  };
  const selectedStrategy = orchestrator?.selected_strategy ?? "Directional";
  const guardSynced = orchestrator?.guard_synced ?? true;
  const action = orchestrator?.action ?? "long";

  const handleTrade = useCallback(async (side: "long" | "short") => {
    setLoading(side);
    const result = await submitDecision({
      action: "trade",
      symbol: "BTC-USDT",
      side,
      size_pct: 0.1,
      reasoning: `Orchestrator ${side} via ${selectedStrategy}`,
    });
    setLoading(null);
    if (result.ok) onToast(`${side.toUpperCase()} EXECUTED`, "success");
    else onToast(`FAILED: ${result.error}`, "error");
  }, [onToast, selectedStrategy]);

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col overflow-hidden">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-bb-amber font-bold text-[10px]">ORCHESTRATOR</span>
          <span className="text-bb-dim text-[9px]">/simmons-dual</span>
        </div>
        <div className="flex items-center gap-1.5">
          <span className={`text-[9px] font-bold ${strategyColors[selectedStrategy] ?? "text-bb-white"}`}>
            {selectedStrategy.toUpperCase()}
          </span>
          <span className={`text-[9px] ${guardSynced ? "text-bb-green" : "text-bb-red"}`}>
            GUARD {guardSynced ? "✓" : "✗"}
          </span>
        </div>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto">
        {/* Multi-agent debate */}
        <div className="border-b border-bb-border">
          <div className="px-2 py-0.5 bg-bb-raised text-[9px] text-bb-dim font-bold">MULTI-AGENT DEBATE</div>

          {/* Bull */}
          <div className="px-2 py-1 border-b border-bb-border">
            <div className="flex items-center gap-1 text-[9px] mb-0.5">
              <span className="text-bb-green font-bold">BULL</span>
            </div>
            <div className="text-[9px] text-bb-white leading-snug">{debate.bull_thesis}</div>
          </div>

          {/* Bear */}
          <div className="px-2 py-1 border-b border-bb-border">
            <div className="flex items-center gap-1 text-[9px] mb-0.5">
              <span className="text-bb-red font-bold">BEAR</span>
            </div>
            <div className="text-[9px] text-bb-white leading-snug">{debate.bear_thesis}</div>
          </div>

          {/* Risk */}
          <div className="px-2 py-1 border-b border-bb-border">
            <div className="flex items-center gap-1 text-[9px] mb-0.5">
              <span className="text-bb-amber font-bold">RISK</span>
            </div>
            <div className="text-[9px] text-bb-white leading-snug">{debate.risk_assessment}</div>
          </div>

          {/* Verdict */}
          <div className="px-2 py-1 bg-bb-raised">
            <div className="text-[9px] text-bb-dim mb-0.5">VERDICT</div>
            <div className="text-[10px] text-bb-bright font-bold">{debate.verdict}</div>
          </div>
        </div>

        {/* Strategy selection */}
        <div className="px-2 py-1.5 border-b border-bb-border">
          <div className="text-[9px] text-bb-dim mb-1">STRATEGY SELECT</div>
          <div className="flex gap-1">
            {["MM", "ARB", "Directional"].map((s) => (
              <div
                key={s}
                className={`flex-1 text-center py-1 text-[9px] font-bold border ${
                  s === selectedStrategy
                    ? "border-bb-orange bg-bb-orange/10 text-bb-orange"
                    : "border-bb-border text-bb-dim"
                }`}
              >
                {s.toUpperCase()}
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Action buttons */}
      <div className="grid grid-cols-3 border-t border-bb-border mt-auto">
        <button
          onClick={() => handleTrade("long")}
          disabled={loading === "long"}
          className={`py-2 text-[10px] font-bold transition-all disabled:opacity-50 ${
            action === "long"
              ? "bg-bb-green text-bb-black"
              : "bg-bb-raised text-bb-green hover:bg-bb-green/20"
          }`}
        >
          {loading === "long" ? "..." : "▲ LONG"}
        </button>
        <button
          onClick={() => handleTrade("short")}
          disabled={loading === "short"}
          className={`py-2 text-[10px] font-bold border-l border-r border-bb-border transition-all disabled:opacity-50 ${
            action === "short"
              ? "bg-bb-red text-bb-bright"
              : "bg-bb-raised text-bb-red hover:bg-bb-red/20"
          }`}
        >
          {loading === "short" ? "..." : "▼ SHORT"}
        </button>
        <button
          onClick={() => onToast("SKIPPED", "info")}
          className="py-2 text-[10px] font-bold text-bb-dim bg-bb-raised hover:bg-bb-border-light transition-all"
        >
          SKIP
        </button>
      </div>
    </div>
  );
});
