"use client";

import { memo } from "react";
import type { InfrastructureLayer } from "@/lib/types";

const items = [
  { key: "zk", name: "ZK-EVM" },
  { key: "bridge", name: "BRIDGE" },
  { key: "dex", name: "DEX/AMM" },
  { key: "cod3x", name: "COD3X" },
];

export const XLayerInfra = memo(function XLayerInfra({ infra }: { infra: InfrastructureLayer | null }) {
  const s: Record<string, boolean> = {
    zk: infra?.xlayer.connected ?? false, bridge: false,
    dex: (infra?.dex_pools.length ?? 0) > 0, cod3x: infra?.cod3x.connected ?? false,
  };

  return (
    <div className="bg-bb-surface border border-bb-border h-full">
      <div className="px-2 py-1 border-b border-bb-border">
        <span className="text-bb-amber font-bold text-[10px]">X LAYER INFRA</span>
      </div>
      <div className="grid grid-cols-2 sm:grid-cols-4">
        {items.map((item) => (
          <div key={item.key} className="px-2 py-2 text-center border-r border-bb-border last:border-r-0 even:border-r-0 sm:even:border-r sm:last:border-r-0">
            <div className="text-[10px] font-bold text-bb-white mb-1">{item.name}</div>
            <div className={`text-[9px] font-bold ${s[item.key] ? "text-bb-green" : "text-bb-dim"}`}>
              {s[item.key] ? "● ON" : "○ OFF"}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
});
