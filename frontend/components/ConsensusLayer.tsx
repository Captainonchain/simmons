"use client";

import { memo } from "react";
import type { ConsensusData } from "@/lib/types";

interface ConsensusLayerProps {
  consensus: ConsensusData | null;
  radarScore: number;
}

export const ConsensusLayer = memo(function ConsensusLayer({ consensus, radarScore }: ConsensusLayerProps) {
  const taWeight = consensus?.ta_weight ?? 0.6;
  const fundWeight = consensus?.fund_weight ?? 0.4;
  const merged = consensus?.merged_score ?? 0.68;
  const radarMet = radarScore > 170;
  const fundPositive = consensus?.fund_positive ?? true;
  const entryApproved = consensus?.entry_approved ?? (radarMet && fundPositive);
  const conflict = consensus?.conflict ?? false;
  const reflect = consensus?.reflect_adjustments ?? { momentum: 0.02, mean_rev: -0.01 };

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <span className="grid-cell-title">CONSENSUS</span>
        <span className={`text-[9px] font-bold ${entryApproved ? "text-bb-green" : conflict ? "text-bb-amber blink" : "text-bb-red"}`}>
          {entryApproved ? "● ENTRY OK" : conflict ? "● CONFLICT" : "● NO ENTRY"}
        </span>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto">
        {/* Merged Score */}
        <div className="px-2 py-2 border-b border-bb-border text-center">
          <div className="text-[28px] font-bold text-bb-bright leading-none">{(merged * 100).toFixed(0)}</div>
          <div className="text-[9px] text-bb-dim mt-0.5">MERGED SCORE</div>
        </div>

        {/* Weight bars */}
        <div className="px-2 py-1.5 border-b border-bb-border">
          <div className="flex items-center justify-between text-[9px] mb-1">
            <span className="text-bb-orange font-bold">TA BRAIN</span>
            <span className="text-bb-bright">{(taWeight * 100).toFixed(0)}%</span>
          </div>
          <div className="h-1.5 bg-bb-raised mb-1.5">
            <div className="h-full bg-bb-orange transition-all" style={{ width: `${taWeight * 100}%` }} />
          </div>

          <div className="flex items-center justify-between text-[9px] mb-1">
            <span className="text-bb-magenta font-bold">FUND BRAIN</span>
            <span className="text-bb-bright">{(fundWeight * 100).toFixed(0)}%</span>
          </div>
          <div className="h-1.5 bg-bb-raised">
            <div className="h-full bg-bb-magenta transition-all" style={{ width: `${fundWeight * 100}%` }} />
          </div>
        </div>

        {/* Entry conditions */}
        <div className="border-b border-bb-border">
          <div className="px-2 py-0.5 bg-bb-raised text-[9px] text-bb-dim font-bold">ENTRY CONDITIONS</div>
          <div className="px-2 py-0.5 flex items-center justify-between text-[10px]">
            <span className="text-bb-white">RADAR &gt; 170</span>
            <span className={radarMet ? "text-bb-green font-bold" : "text-bb-red font-bold"}>
              {radarScore} {radarMet ? "✓" : "✗"}
            </span>
          </div>
          <div className="px-2 py-0.5 flex items-center justify-between text-[10px]">
            <span className="text-bb-white">FUND POSITIVE</span>
            <span className={fundPositive ? "text-bb-green font-bold" : "text-bb-red font-bold"}>
              {fundPositive ? "YES ✓" : "NO ✗"}
            </span>
          </div>
          {conflict && (
            <div className="px-2 py-0.5 flex items-center justify-between text-[10px]">
              <span className="text-bb-white">CONFLICT</span>
              <span className="text-bb-amber font-bold blink">→ DEBATE</span>
            </div>
          )}
        </div>

        {/* REFLECT adjustments */}
        <div>
          <div className="px-2 py-0.5 bg-bb-raised text-[9px] text-bb-dim font-bold">REFLECT ADAPTIVE</div>
          {Object.entries(reflect).map(([key, val]) => (
            <div key={key} className="px-2 py-0.5 flex items-center justify-between text-[9px]">
              <span className="text-bb-dim">{key.toUpperCase()}</span>
              <span className={val > 0 ? "text-bb-green" : val < 0 ? "text-bb-red" : "text-bb-dim"}>
                {val > 0 ? "+" : ""}{(val * 100).toFixed(1)}%
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
});
