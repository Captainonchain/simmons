"use client";

import { memo, useMemo, useState } from "react";
import { ResponsiveContainer, AreaChart, Area, YAxis, XAxis, Tooltip, CartesianGrid } from "recharts";
import type { PricePoint } from "@/lib/types";

interface PriceChartProps {
  priceHistory: Record<string, PricePoint[]>;
  availableSymbols: string[];
}

export const PriceChart = memo(function PriceChart({ priceHistory, availableSymbols }: PriceChartProps) {
  const [sym, setSym] = useState("BTC-USDT");
  const history = priceHistory[sym] ?? [];
  const chartData = useMemo(() => history.map((p, i) => ({ idx: i, price: p.price })), [history]);
  const symbols = availableSymbols.length > 0 ? availableSymbols : ["BTC-USDT", "ETH-USDT"];

  return (
    <div className="bg-bb-surface border border-bb-border flex flex-col h-full">
      <div className="flex items-center justify-between px-2 py-1 border-b border-bb-border">
        <div className="flex items-center gap-2">
          <span className="text-bb-amber font-bold text-[10px]">PRICE</span>
          <select
            value={sym}
            onChange={(e) => setSym(e.target.value)}
            className="bg-bb-black border border-bb-border text-bb-white text-[10px] px-1 py-0.5 focus:outline-none focus:border-bb-orange"
          >
            {symbols.map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
        </div>
        {chartData.length > 0 && (
          <span className="text-bb-green text-[11px] font-bold">
            ${chartData[chartData.length - 1]?.price.toFixed(2)}
          </span>
        )}
      </div>
      <div className="flex-1 min-h-0">
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={chartData} margin={{ top: 8, right: 8, bottom: 0, left: 0 }}>
              <defs>
                <linearGradient id="bbGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#FF6600" stopOpacity={0.15} />
                  <stop offset="100%" stopColor="#FF6600" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="1 4" stroke="#222" vertical={false} />
              <YAxis domain={["auto", "auto"]} orientation="right" tick={{ fill: "#666", fontSize: 9, fontFamily: "JetBrains Mono" }} axisLine={false} tickLine={false} width={45} tickFormatter={(v: number) => v >= 1000 ? `${(v/1000).toFixed(1)}k` : v.toFixed(0)} />
              <Tooltip
                contentStyle={{ background: "#111", border: "1px solid #333", fontSize: 10, fontFamily: "JetBrains Mono", color: "#ccc" }}
                formatter={(v: number) => [`$${v.toFixed(2)}`, ""]}
                labelFormatter={() => ""}
              />
              <Area type="monotone" dataKey="price" stroke="#FF6600" strokeWidth={1.5} fill="url(#bbGrad)" dot={false} isAnimationActive={false} />
            </AreaChart>
          </ResponsiveContainer>
        ) : (
          <div className="h-full flex items-center justify-center text-bb-dim text-[10px]">NO DATA</div>
        )}
      </div>
    </div>
  );
});
