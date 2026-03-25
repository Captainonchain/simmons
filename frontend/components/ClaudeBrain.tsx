"use client";

import { memo, useState, useCallback } from "react";
import { submitDecision } from "@/lib/api";

interface ClaudeBrainProps {
  onToast: (message: string, type: "success" | "error" | "info") => void;
}

export const ClaudeBrain = memo(function ClaudeBrain({ onToast }: ClaudeBrainProps) {
  const [loading, setLoading] = useState<string | null>(null);

  const handleTrade = useCallback(async (side: "long" | "short") => {
    setLoading(side);
    const result = await submitDecision({ action: "trade", symbol: "BTC-USDT", side, size_pct: 0.1, reasoning: `Quick ${side} from terminal` });
    setLoading(null);
    if (result.ok) onToast(`${side.toUpperCase()} SUBMITTED`, "success");
    else onToast(`FAILED: ${result.error}`, "error");
  }, [onToast]);

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <span className="text-bb-amber font-bold text-[10px]">CLAUDE BRAIN</span>
        <span className="text-bb-magenta text-[9px]">AI</span>
      </div>
      <div className="px-2 py-3 text-center">
        <div className="text-bb-dim text-[10px] mb-1">AWAITING DECISION</div>
        <div className="text-bb-muted text-[9px]">ANALYZING SIGNALS...</div>
      </div>
      <div className="grid grid-cols-3 border-t border-bb-border mt-auto">
        <button
          onClick={() => handleTrade("long")}
          disabled={loading === "long"}
          className="py-2 text-[10px] font-bold text-bb-black bg-bb-green hover:brightness-110 disabled:opacity-50 transition-all"
        >
          {loading === "long" ? "..." : "LONG"}
        </button>
        <button
          onClick={() => handleTrade("short")}
          disabled={loading === "short"}
          className="py-2 text-[10px] font-bold text-bb-bright bg-bb-red hover:brightness-110 disabled:opacity-50 border-l border-r border-bb-border transition-all"
        >
          {loading === "short" ? "..." : "SHORT"}
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
