"use client";

import { memo } from "react";
import type { TABrainData } from "@/lib/types";

interface TABrainProps {
  taBrain: TABrainData | null;
}

const DEFAULT_STRATEGIES = {
  mm: [
    { name: "ENGINE", active: true, signal: "hold", confidence: 0.72 },
    { name: "AVELLANEDA", active: true, signal: "buy", confidence: 0.81 },
    { name: "REGIME", active: true, signal: "hold", confidence: 0.65 },
    { name: "GRID", active: false, signal: "skip", confidence: 0 },
    { name: "LIQ", active: true, signal: "sell", confidence: 0.58 },
  ],
  arb: [
    { name: "FUNDING", active: true, signal: "buy", confidence: 0.77 },
    { name: "BASIS", active: false, signal: "skip", confidence: 0 },
  ],
  dir: [
    { name: "MOMENTUM", active: true, signal: "buy", confidence: 0.84 },
    { name: "MEAN_REV", active: true, signal: "hold", confidence: 0.62 },
  ],
  int: [
    { name: "HEDGE", active: true, signal: "hold", confidence: 0.70 },
    { name: "RFE", active: false, signal: "skip", confidence: 0 },
    { name: "CLAUDE", active: true, signal: "buy", confidence: 0.88 },
  ],
};

const signalColor: Record<string, string> = {
  buy: "text-bb-green",
  sell: "text-bb-red",
  hold: "text-bb-dim",
  skip: "text-bb-muted",
};

const pulseTierLabel: Record<number, { label: string; color: string }> = {
  0: { label: "DEAD", color: "text-bb-muted" },
  1: { label: "WEAK", color: "text-bb-dim" },
  2: { label: "LOW", color: "text-bb-dim" },
  3: { label: "MID", color: "text-bb-amber" },
  4: { label: "HIGH", color: "text-bb-amber" },
  5: { label: "STRONG", color: "text-bb-green" },
  6: { label: "PEAK", color: "text-bb-green" },
};

export const TABrain = memo(function TABrain({ taBrain }: TABrainProps) {
  const radar = taBrain?.radar_score ?? 172;
  const pulse = taBrain?.pulse_tier ?? 4;
  const guard = taBrain?.guard ?? { phase: 1, active_stops: 2, triggered: false };
  const apex = taBrain?.apex ?? { concurrent_slots: 4, active_slots: 2, priority_tier: "TIER-1", roe_exit_active: true };
  const strats = taBrain?.strategies ?? DEFAULT_STRATEGIES;

  const radarPct = Math.min((radar / 200) * 100, 100);
  const radarColor = radar > 170 ? "bg-bb-green" : radar > 100 ? "bg-bb-amber" : "bg-bb-red";
  const pt = pulseTierLabel[pulse] ?? pulseTierLabel[0];

  const catColors: Record<string, string> = { mm: "text-bb-cyan", arb: "text-bb-magenta", dir: "text-bb-green", int: "text-bb-amber" };

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col overflow-hidden">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-bb-orange font-bold text-[10px]">TA BRAIN</span>
          <span className="text-bb-dim text-[9px]">NUNCHI 14 STRATS</span>
        </div>
        <span className="text-bb-cyan text-[9px]">{apex.active_slots}/{apex.concurrent_slots} SLOTS</span>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto">
        {/* RADAR / PULSE / GUARD row */}
        <div className="grid grid-cols-3 border-b border-bb-border">
          {/* RADAR */}
          <div className="px-2 py-1.5 border-r border-bb-border">
            <div className="text-[9px] text-bb-dim mb-0.5">RADAR</div>
            <div className="text-[20px] font-bold text-bb-bright leading-none">{radar}</div>
            <div className="h-1 bg-bb-raised mt-1">
              <div className={`h-full ${radarColor} transition-all`} style={{ width: `${radarPct}%` }} />
            </div>
            <div className="text-[8px] text-bb-dim mt-0.5">0 — 200</div>
          </div>

          {/* PULSE */}
          <div className="px-2 py-1.5 border-r border-bb-border">
            <div className="text-[9px] text-bb-dim mb-0.5">PULSE</div>
            <div className="flex items-baseline gap-1">
              <span className="text-[20px] font-bold text-bb-bright leading-none">{pulse}</span>
              <span className={`text-[9px] font-bold ${pt.color}`}>{pt.label}</span>
            </div>
            <div className="flex gap-px mt-1">
              {[1, 2, 3, 4, 5, 6].map((t) => (
                <div
                  key={t}
                  className={`h-1.5 flex-1 ${t <= pulse ? (t <= 2 ? "bg-bb-red" : t <= 4 ? "bg-bb-amber" : "bg-bb-green") : "bg-bb-raised"}`}
                />
              ))}
            </div>
          </div>

          {/* GUARD */}
          <div className="px-2 py-1.5">
            <div className="text-[9px] text-bb-dim mb-0.5">GUARD</div>
            <div className="flex items-baseline gap-1">
              <span className="text-[14px] font-bold text-bb-bright leading-none">P{guard.phase}</span>
              <span className={`text-[9px] font-bold ${guard.triggered ? "text-bb-red blink" : "text-bb-green"}`}>
                {guard.triggered ? "TRIG" : "SAFE"}
              </span>
            </div>
            <div className="text-[9px] text-bb-dim mt-0.5">{guard.active_stops} STOPS</div>
          </div>
        </div>

        {/* APEX info bar */}
        <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between text-[9px]">
          <span className="text-bb-dim">APEX</span>
          <div className="flex items-center gap-2">
            <span className="text-bb-cyan font-bold">{apex.priority_tier}</span>
            <span className={apex.roe_exit_active ? "text-bb-green" : "text-bb-dim"}>ROE {apex.roe_exit_active ? "ON" : "OFF"}</span>
          </div>
        </div>

        {/* 14 Strategies grouped */}
        {(Object.keys(strats) as Array<keyof typeof strats>).map((cat) => (
          <div key={cat} className="border-b border-bb-border last:border-b-0">
            <div className={`px-2 py-0.5 bg-bb-raised text-[9px] font-bold ${catColors[cat]} flex items-center justify-between`}>
              <span>{cat.toUpperCase()}</span>
              <span className="text-bb-dim">{strats[cat].length}</span>
            </div>
            {strats[cat].map((s) => (
              <div key={s.name} className="px-2 py-0.5 flex items-center justify-between text-[10px] hover:bg-bb-raised">
                <div className="flex items-center gap-1.5">
                  <span className={s.active ? "text-bb-green" : "text-bb-muted"}>●</span>
                  <span className="text-bb-white">{s.name}</span>
                </div>
                <div className="flex items-center gap-2">
                  {s.active && <span className="text-bb-dim text-[9px]">{(s.confidence * 100).toFixed(0)}%</span>}
                  <span className={`font-bold text-[9px] w-[28px] text-right ${signalColor[s.signal] ?? "text-bb-dim"}`}>
                    {s.signal.toUpperCase()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
});
