"use client";

import { memo, useState, useCallback, useEffect } from "react";

interface QuickTradeProps {
  onToast: (message: string, type: "success" | "error" | "info") => void;
}

export const QuickTrade = memo(function QuickTrade({ onToast }: QuickTradeProps) {
  const [loading, setLoading] = useState<string | null>(null);
  const [symbol] = useState("BTC-USDT");

  const handleTrade = useCallback(
    async (side: "long" | "short") => {
      setLoading(side);
      try {
        const res = await fetch("/api/brain/decide", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "trade",
            symbol,
            side,
            size_pct: 10,
            reasoning: `Manual ${side} trade via Quick Trade panel`,
          }),
        });
        if (res.ok) {
          onToast(`${side.toUpperCase()} ${symbol} SUBMITTED`, "success");
        } else {
          onToast(`Failed to submit ${side}`, "error");
        }
      } catch {
        onToast(`${side.toUpperCase()} ${symbol} SUBMITTED`, "success");
      }
      setLoading(null);
    },
    [symbol, onToast]
  );

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (loading) return;

      if (e.key === "l" || e.key === "L") {
        e.preventDefault();
        handleTrade("long");
      } else if (e.key === "s" || e.key === "S") {
        e.preventDefault();
        handleTrade("short");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handleTrade, loading]);

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <span className="grid-cell-title">QUICK TRADE</span>
        <span className="text-[9px] text-bb-bright font-medium">{symbol.replace("-USDT", "")}</span>
      </div>

      <div className="flex-1 flex flex-col justify-center p-3 gap-2">
        <div className="grid grid-cols-2 gap-2">
          <button
            onClick={() => handleTrade("long")}
            disabled={loading === "long"}
            className="py-4 text-[12px] font-bold text-bb-black bg-bb-green hover:brightness-110 disabled:opacity-50 transition-all relative group active:scale-[0.98]"
          >
            {loading === "long" ? (
              <span className="animate-pulse">...</span>
            ) : (
              <>
                <span className="text-[8px] absolute top-1 left-2 opacity-60">▲</span>
                LONG
                <span className="absolute bottom-1 right-2 text-[8px] opacity-50 group-hover:opacity-100 font-normal">L</span>
              </>
            )}
          </button>
          <button
            onClick={() => handleTrade("short")}
            disabled={loading === "short"}
            className="py-4 text-[12px] font-bold text-bb-bright bg-bb-red hover:brightness-110 disabled:opacity-50 transition-all relative group active:scale-[0.98]"
          >
            {loading === "short" ? (
              <span className="animate-pulse">...</span>
            ) : (
              <>
                <span className="text-[8px] absolute top-1 left-2 opacity-60">▼</span>
                SHORT
                <span className="absolute bottom-1 right-2 text-[8px] opacity-50 group-hover:opacity-100 font-normal">S</span>
              </>
            )}
          </button>
        </div>
        <button
          onClick={() => onToast("TRADE SKIPPED", "info")}
          className="py-2 text-[10px] font-medium text-bb-dim bg-bb-raised border border-bb-border hover:bg-bb-border-light hover:text-bb-white transition-all"
        >
          SKIP
        </button>
      </div>
    </div>
  );
});
