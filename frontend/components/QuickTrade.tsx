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
      // Simulate API call
      await new Promise((r) => setTimeout(r, 500));
      setLoading(null);
      onToast(`${side.toUpperCase()} ${symbol} SUBMITTED`, "success");
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
        <span className="text-[8px] text-bb-dim">{symbol.replace("-USDT", "")}</span>
      </div>

      <div className="flex-1 flex flex-col justify-center p-2">
        <div className="grid grid-cols-2 gap-1 mb-2">
          <button
            onClick={() => handleTrade("long")}
            disabled={loading === "long"}
            className="py-3 text-[11px] font-bold text-bb-black bg-bb-green hover:brightness-110 disabled:opacity-50 transition-all relative group"
          >
            {loading === "long" ? (
              <span className="animate-pulse">...</span>
            ) : (
              <>
                LONG
                <span className="absolute bottom-0.5 right-1 text-[7px] opacity-50 group-hover:opacity-100">L</span>
              </>
            )}
          </button>
          <button
            onClick={() => handleTrade("short")}
            disabled={loading === "short"}
            className="py-3 text-[11px] font-bold text-bb-bright bg-bb-red hover:brightness-110 disabled:opacity-50 transition-all relative group"
          >
            {loading === "short" ? (
              <span className="animate-pulse">...</span>
            ) : (
              <>
                SHORT
                <span className="absolute bottom-0.5 right-1 text-[7px] opacity-50 group-hover:opacity-100">S</span>
              </>
            )}
          </button>
        </div>
        <button
          onClick={() => onToast("TRADE SKIPPED", "info")}
          className="py-1.5 text-[9px] font-medium text-bb-dim bg-bb-raised border border-bb-border hover:bg-bb-border-light transition-all"
        >
          SKIP
        </button>
      </div>
    </div>
  );
});
