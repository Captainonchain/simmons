"use client";

import { memo } from "react";
import type { MevShieldData } from "@/lib/types";

export const MevShield = memo(function MevShield({ mev }: { mev: MevShieldData | null }) {
  return (
    <div className="bg-bb-surface border border-bb-border h-full">
      <div className="px-2 py-1 border-b border-bb-border">
        <span className="text-bb-amber font-bold text-[10px]">MEV SHIELD</span>
      </div>
      <div className="px-2 py-1.5 text-[10px] space-y-1">
        <div className="flex justify-between">
          <span className="text-bb-dim">STATUS</span>
          <span className={mev?.enabled ? "text-bb-green font-bold" : "text-bb-red"}>
            {mev?.enabled ? "PROTECTED" : "DISABLED"}
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-bb-dim">POOL</span>
          <span className="text-bb-white">{(mev?.private_pool ?? "FLASHBOTS").toUpperCase()}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-bb-dim">MEV SAVED</span>
          <span className="text-bb-green">${(mev?.mev_saved_usd ?? 0).toFixed(2)}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-bb-dim">RISK</span>
          <span className="text-bb-green">{(mev?.current_risk ?? "LOW").toUpperCase()}</span>
        </div>
      </div>
    </div>
  );
});
