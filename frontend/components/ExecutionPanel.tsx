"use client";

import { memo } from "react";
import type { ExecutionModeData, ExecutionLayer, GasData } from "@/lib/types";

interface ExecutionPanelProps {
  execMode: ExecutionModeData | null;
  execution: ExecutionLayer | null;
}

export const ExecutionPanel = memo(function ExecutionPanel({ execMode, execution }: ExecutionPanelProps) {
  const mode = execMode?.mode ?? "paper";
  const paper = execMode?.paper ?? { active: true, engine: "Simmons Rust" };
  const liveDex = execMode?.live_dex ?? { active: false, chains: ["Solana", "Base", "ETH"], router: "OnchainOS" };
  const livePerps = execMode?.live_perps ?? { active: false, venue: "Hyperliquid" };
  const gas: GasData = execution?.gas ?? { current_gwei: 25, recommended_gwei: 20, priority: "MEDIUM", estimated_cost_usd: 0.42, should_wait: false };
  const router = execution?.router ?? { active_venues: ["OKX", "OnchainOS"], best_venue: "OKX", split_enabled: true, avg_slippage_bps: 1.2 };
  const recentExecs = execution?.recent_executions ?? [];

  const modeColors: Record<string, string> = {
    paper: "text-bb-amber",
    live_dex: "text-bb-green",
    live_perps: "text-bb-cyan",
  };

  const modeLabels: Record<string, string> = {
    paper: "PAPER",
    live_dex: "LIVE DEX",
    live_perps: "LIVE PERPS",
  };

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col overflow-hidden">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <span className="text-bb-green font-bold text-[10px]">EXECUTION</span>
        <span className={`text-[9px] font-bold ${modeColors[mode]}`}>
          ● {modeLabels[mode] ?? mode.toUpperCase()}
        </span>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto">
        {/* Mode status grid */}
        <div className="grid grid-cols-3 border-b border-bb-border">
          {/* Paper */}
          <div className={`px-2 py-1.5 text-center border-r border-bb-border ${mode === "paper" ? "bg-bb-raised" : ""}`}>
            <div className="text-[9px] text-bb-dim mb-0.5">PAPER</div>
            <div className={`text-[11px] font-bold ${paper.active ? "text-bb-amber" : "text-bb-muted"}`}>
              {paper.active ? "● ON" : "○ OFF"}
            </div>
            <div className="text-[8px] text-bb-dim mt-0.5">{paper.engine}</div>
          </div>

          {/* Live DEX */}
          <div className={`px-2 py-1.5 text-center border-r border-bb-border ${mode === "live_dex" ? "bg-bb-raised" : ""}`}>
            <div className="text-[9px] text-bb-dim mb-0.5">LIVE DEX</div>
            <div className={`text-[11px] font-bold ${liveDex.active ? "text-bb-green" : "text-bb-muted"}`}>
              {liveDex.active ? "● ON" : "○ OFF"}
            </div>
            <div className="text-[8px] text-bb-dim mt-0.5">{liveDex.router}</div>
          </div>

          {/* Live Perps */}
          <div className={`px-2 py-1.5 text-center ${mode === "live_perps" ? "bg-bb-raised" : ""}`}>
            <div className="text-[9px] text-bb-dim mb-0.5">PERPS</div>
            <div className={`text-[11px] font-bold ${livePerps.active ? "text-bb-cyan" : "text-bb-muted"}`}>
              {livePerps.active ? "● ON" : "○ OFF"}
            </div>
            <div className="text-[8px] text-bb-dim mt-0.5">{livePerps.venue}</div>
          </div>
        </div>

        {/* Chains */}
        <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between text-[9px]">
          <span className="text-bb-dim">CHAINS</span>
          <div className="flex gap-1.5">
            {liveDex.chains.map((c) => (
              <span key={c} className="text-bb-cyan font-bold">{c.toUpperCase()}</span>
            ))}
          </div>
        </div>

        {/* Routing info */}
        <div className="border-b border-bb-border">
          <div className="px-2 py-0.5 bg-bb-raised text-[9px] text-bb-dim font-bold">ROUTING</div>
          <div className="px-2 py-0.5 flex items-center justify-between text-[10px]">
            <span className="text-bb-dim">BEST VENUE</span>
            <span className="text-bb-bright font-bold">{router.best_venue}</span>
          </div>
          <div className="px-2 py-0.5 flex items-center justify-between text-[10px]">
            <span className="text-bb-dim">SLIPPAGE</span>
            <span className="text-bb-white">{router.avg_slippage_bps.toFixed(1)} BPS</span>
          </div>
          <div className="px-2 py-0.5 flex items-center justify-between text-[10px]">
            <span className="text-bb-dim">SPLIT</span>
            <span className={router.split_enabled ? "text-bb-green font-bold" : "text-bb-dim"}>{router.split_enabled ? "ON" : "OFF"}</span>
          </div>
        </div>

        {/* Gas */}
        <div className="border-b border-bb-border">
          <div className="px-2 py-0.5 bg-bb-raised text-[9px] text-bb-dim font-bold">GAS</div>
          <div className="px-2 py-0.5 flex items-center justify-between text-[10px]">
            <span className="text-bb-dim">CURRENT</span>
            <span className="text-bb-bright font-bold">{gas.current_gwei.toFixed(0)} GWEI</span>
          </div>
          <div className="px-2 py-0.5 flex items-center justify-between text-[10px]">
            <span className="text-bb-dim">EST COST</span>
            <span className="text-bb-white">${gas.estimated_cost_usd.toFixed(2)}</span>
          </div>
          <div className="px-2 py-0.5 flex items-center justify-between text-[10px]">
            <span className="text-bb-dim">WAIT</span>
            <span className={gas.should_wait ? "text-bb-amber font-bold" : "text-bb-green font-bold"}>
              {gas.should_wait ? "YES" : "NO"}
            </span>
          </div>
        </div>

        {/* Recent executions */}
        {recentExecs.length > 0 && (
          <div>
            <div className="px-2 py-0.5 bg-bb-raised text-[9px] text-bb-dim font-bold">RECENT</div>
            {recentExecs.slice(0, 5).map((e) => (
              <div key={e.id} className="px-2 py-0.5 flex items-center justify-between text-[9px]">
                <div className="flex items-center gap-1">
                  <span className={e.side === "buy" ? "text-bb-green" : "text-bb-red"}>
                    {e.side === "buy" ? "▲" : "▼"}
                  </span>
                  <span className="text-bb-white">{e.symbol}</span>
                </div>
                <span className="text-bb-dim">{e.slippage_bps.toFixed(1)} BPS</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
});
