"use client";

import { memo } from "react";
import type { KellySizingData } from "@/lib/types";

export const KellyCriterion = memo(function KellyCriterion({ kelly, equity }: { kelly: KellySizingData | null; equity: number }) {
  const opt = (kelly?.optimal_fraction ?? 0.15) * 100;
  const recPct = kelly?.recommended_size_pct ?? 10;
  const recUsd = equity * recPct / 100;

  return (
    <div className="bg-bb-surface border border-bb-border h-full">
      <div className="px-2 py-1 border-b border-bb-border">
        <span className="text-bb-amber font-bold text-[10px]">KELLY CRITERION</span>
      </div>
      <div className="px-2 py-2 text-center">
        <div className="text-[28px] font-bold text-bb-orange leading-none">{opt.toFixed(0)}%</div>
        <div className="text-[9px] text-bb-dim mt-0.5">OPTIMAL FRACTION</div>
        <div className="mt-3 py-1.5 bg-bb-raised border border-bb-border">
          <div className="text-[9px] text-bb-dim">RECOMMENDED (½ KELLY)</div>
          <div className="text-[16px] font-bold text-bb-green">${recUsd.toFixed(2)}</div>
        </div>
      </div>
    </div>
  );
});
