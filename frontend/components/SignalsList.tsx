"use client";

import { memo } from "react";
import type { StrategySignalData } from "@/lib/types";

const signalColor: Record<string, string> = {
  buy: "text-bb-green", strongbuy: "text-bb-green",
  sell: "text-bb-red", strongsell: "text-bb-red",
  hold: "text-bb-dim",
};

export const SignalsList = memo(function SignalsList({ signals }: { signals: StrategySignalData[] }) {
  return (
    <div className="bg-bb-surface border border-bb-border flex flex-col h-full">
      <div className="px-2 py-1 border-b border-bb-border">
        <span className="text-bb-amber font-bold text-[10px]">SIGNALS</span>
      </div>
      <div className="overflow-y-auto flex-1 min-h-0">
        {signals.length > 0 ? signals.map((s, i) => (
          <div key={`${s.symbol}-${i}`} className="px-2 py-1 border-b border-bb-border hover:bg-bb-raised text-[10px]">
            <div className="flex items-center justify-between">
              <span className="text-bb-white font-bold">{s.symbol}</span>
              <span className={`font-bold ${signalColor[s.signal.toLowerCase()] ?? "text-bb-dim"}`}>
                {s.signal.toUpperCase()}
              </span>
            </div>
            <div className="text-bb-dim text-[9px] truncate">{s.strategy} — {s.reason}</div>
          </div>
        )) : (
          <div className="px-2 py-6 text-center text-bb-dim text-[10px]">WAITING FOR SIGNALS...</div>
        )}
      </div>
    </div>
  );
});
