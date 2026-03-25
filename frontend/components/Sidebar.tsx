"use client";

import { memo } from "react";
import type { DataIngestionLayer } from "@/lib/types";

interface SidebarProps {
  dataIngestion?: DataIngestionLayer;
  open: boolean;
  onClose: () => void;
}

const layers = [
  { title: "DATA", color: "text-bb-blue", getComponents: (d?: DataIngestionLayer) => [
    { name: "OKX CEX", online: d?.okx_status.connected ?? false },
    { name: "X LAYER", online: d?.xlayer_status.connected ?? false },
    { name: "NUNCHI", online: d?.nunchi_status.connected ?? false },
    { name: "NEWS", online: d?.news_status.connected ?? false },
  ]},
  { title: "ALPHA", color: "text-bb-magenta", getComponents: () => [
    { name: "STRATEGY", online: true }, { name: "RESEARCH", online: true }, { name: "FORECAST", online: true },
  ]},
  { title: "RISK", color: "text-bb-green", getComponents: () => [
    { name: "REBALANCE", online: true }, { name: "CEDEFI ARB", online: false }, { name: "GOVERNOR", online: true },
  ]},
  { title: "EXEC", color: "text-bb-amber", getComponents: () => [
    { name: "ROUTER", online: true }, { name: "COD3X", online: false }, { name: "MEV SHIELD", online: true },
  ]},
  { title: "INFRA", color: "text-bb-cyan", getComponents: () => [
    { name: "ZK-EVM", online: false }, { name: "BRIDGE", online: false }, { name: "DEX/AMM", online: false }, { name: "COD3X", online: false },
  ]},
];

export const Sidebar = memo(function Sidebar({ dataIngestion, open, onClose }: SidebarProps) {
  return (
    <>
      {/* Mobile overlay */}
      {open && <div className="fixed inset-0 bg-black/60 z-40 lg:hidden" onClick={onClose} />}

      <aside className={`
        fixed lg:static inset-y-0 left-0 z-50
        w-[180px] shrink-0 bg-bb-panel border-r border-bb-border overflow-y-auto text-[10px]
        transition-transform duration-200 ease-out
        ${open ? "translate-x-0" : "-translate-x-full"} lg:translate-x-0
      `}>
        <div className="px-2 pt-2 pb-1 text-bb-orange font-bold tracking-widest border-b border-bb-border flex items-center justify-between">
          <span>SYSTEMS</span>
          <button onClick={onClose} className="lg:hidden text-bb-dim px-1 py-0.5 active:opacity-70">✕</button>
        </div>
        {layers.map((layer) => (
          <div key={layer.title} className="border-b border-bb-border">
            <div className={`px-2 py-1 ${layer.color} font-bold bg-bb-surface`}>{layer.title}</div>
            {layer.getComponents(layer.title === "DATA" ? dataIngestion : undefined).map((comp) => (
              <div key={comp.name} className="px-2 py-0.5 flex items-center justify-between hover:bg-bb-surface cursor-pointer">
                <span className="text-bb-white">{comp.name}</span>
                <span className={comp.online ? "text-bb-green" : "text-bb-dim"}>{comp.online ? "ON" : "OFF"}</span>
              </div>
            ))}
          </div>
        ))}
        <div className="px-2 py-2">
          <div className="flex items-center gap-1">
            <span className="text-bb-green blink">●</span>
            <span className="text-bb-dim">FEEDBACK ACTIVE</span>
          </div>
        </div>
      </aside>
    </>
  );
});
