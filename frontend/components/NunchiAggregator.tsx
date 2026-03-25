"use client";

import { memo } from "react";
import type { NunchiScoreData } from "@/lib/types";

export const NunchiAggregator = memo(function NunchiAggregator({ nunchi }: { nunchi: NunchiScoreData | null }) {
  const score = nunchi?.score ?? 0.65;
  const dir = nunchi?.direction ?? "Bullish";
  const m = nunchi?.components?.momentum ?? 0.7;
  const mr = nunchi?.components?.mean_reversion ?? 0.5;
  const r = nunchi?.components?.regime ?? 0.8;

  const barStyle = (v: number) => ({ width: `${v * 100}%` });

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col overflow-hidden">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <span className="text-bb-amber font-bold text-[10px]">NUNCHI</span>
        <span className="text-bb-cyan text-[10px]">{dir.toUpperCase()}</span>
      </div>
      <div className="px-2 py-2 flex-1 min-h-0 overflow-y-auto">
        <div className="text-center mb-3">
          <span className="text-[32px] font-bold text-bb-orange leading-none">{score.toFixed(2)}</span>
        </div>
        {[
          { label: "MOMENTUM", value: m, color: "bg-bb-green" },
          { label: "MEAN REV", value: mr, color: "bg-bb-blue" },
          { label: "REGIME", value: r, color: "bg-bb-cyan" },
        ].map((g) => (
          <div key={g.label} className="mb-1.5">
            <div className="flex justify-between text-[9px] mb-0.5">
              <span className="text-bb-dim">{g.label}</span>
              <span className="text-bb-white">{(g.value * 100).toFixed(0)}%</span>
            </div>
            <div className="h-1 bg-bb-raised">
              <div className={`h-full ${g.color}`} style={barStyle(g.value)} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
});
