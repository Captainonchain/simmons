"use client";

import { memo } from "react";
import type { SymbolData } from "@/lib/types";

interface MarketsTableProps {
  symbols: SymbolData[];
  regime: string;
}

export const MarketsTable = memo(function MarketsTable({ symbols, regime }: MarketsTableProps) {
  return (
    <div className="bg-bb-surface border border-bb-border flex flex-col h-full">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <span className="text-bb-amber font-bold text-[10px]">MARKETS</span>
        <span className="text-bb-dim text-[9px]">{symbols.length} SYM</span>
      </div>
      <div className="overflow-y-auto flex-1 min-h-0">
        <table className="w-full text-[10px]">
          <thead>
            <tr className="text-bb-dim bg-bb-raised sticky top-0">
              <th className="text-left px-2 py-1 font-medium">SYMBOL</th>
              <th className="text-right px-2 py-1 font-medium">PRICE</th>
              <th className="text-right px-2 py-1 font-medium">SPREAD</th>
              <th className="text-right px-2 py-1 font-medium">REGIME</th>
            </tr>
          </thead>
          <tbody>
            {symbols.length > 0 ? symbols.map((s) => (
              <tr key={s.symbol} className="border-t border-bb-border hover:bg-bb-raised cursor-pointer">
                <td className="px-2 py-0.5 text-bb-bright font-bold">{s.symbol}</td>
                <td className="px-2 py-0.5 text-right text-bb-green">${s.price}</td>
                <td className="px-2 py-0.5 text-right text-bb-white">{s.spread_bps}</td>
                <td className="px-2 py-0.5 text-right text-bb-cyan">{regime.toUpperCase()}</td>
              </tr>
            )) : (
              <tr><td colSpan={4} className="px-2 py-4 text-center text-bb-dim">LOADING...</td></tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
});
