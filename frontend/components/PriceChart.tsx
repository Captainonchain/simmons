"use client";

import { memo, useState } from "react";
import { ResponsiveContainer, AreaChart, Area, YAxis, XAxis, Tooltip, ReferenceLine } from "recharts";
import type { PricePoint } from "@/lib/types";

interface PriceChartProps {
  priceHistory: Record<string, PricePoint[]>;
  availableSymbols: string[];
}

export const PriceChart = memo(function PriceChart({ priceHistory, availableSymbols }: PriceChartProps) {
  const [sym, setSym] = useState("BTC-USDT");
  const history = priceHistory[sym] ?? [];
  const chartData = history.map((p, i) => ({ idx: i, price: p.price, time: p.time }));
  const symbols = availableSymbols.length > 0 ? availableSymbols : ["BTC-USDT", "ETH-USDT", "SOL-USDT"];

  const currentPrice = chartData.length > 0 ? chartData[chartData.length - 1]?.price : 0;
  const startPrice = chartData.length > 0 ? chartData[0]?.price : currentPrice;
  const change = startPrice > 0 ? ((currentPrice - startPrice) / startPrice) * 100 : 0;
  const isUp = change >= 0;

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <div className="flex items-center gap-2">
          <span className="grid-cell-title">PRICE</span>
          <select
            value={sym}
            onChange={(e) => setSym(e.target.value)}
            className="bg-bb-black border border-bb-border text-bb-white text-[9px] px-1.5 py-0.5 focus:outline-none focus:border-bb-orange cursor-pointer"
          >
            {symbols.map((s) => (
              <option key={s} value={s}>{s.replace("-USDT", "")}</option>
            ))}
          </select>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-bb-bright font-bold text-[12px] stat-value">
            ${currentPrice.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
          </span>
          <span className={`text-[10px] stat-value ${isUp ? "text-bb-green" : "text-bb-red"}`}>
            {isUp ? "+" : ""}{change.toFixed(2)}%
          </span>
        </div>
      </div>
      <div className="flex-1 min-h-0 p-1">
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={chartData} margin={{ top: 4, right: 4, bottom: 0, left: 0 }}>
              <defs>
                <linearGradient id="priceGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={isUp ? "#00DD55" : "#FF2222"} stopOpacity={0.2} />
                  <stop offset="100%" stopColor={isUp ? "#00DD55" : "#FF2222"} stopOpacity={0} />
                </linearGradient>
              </defs>
              <YAxis
                domain={["auto", "auto"]}
                orientation="right"
                tick={{ fill: "#555", fontSize: 8, fontFamily: "IBM Plex Mono" }}
                axisLine={false}
                tickLine={false}
                width={50}
                tickFormatter={(v) => `$${v.toLocaleString()}`}
              />
              <XAxis hide dataKey="idx" />
              <Tooltip
                contentStyle={{
                  background: "#0a0a0a",
                  border: "1px solid #1a1a1a",
                  fontSize: 9,
                  fontFamily: "IBM Plex Mono",
                  color: "#bbb",
                  padding: "4px 8px",
                }}
                formatter={(v: number) => [`$${v.toLocaleString(undefined, { minimumFractionDigits: 2 })}`, ""]}
                labelFormatter={() => ""}
              />
              <ReferenceLine y={startPrice} stroke="#333" strokeDasharray="2 2" />
              <Area
                type="monotone"
                dataKey="price"
                stroke={isUp ? "#00DD55" : "#FF2222"}
                strokeWidth={1.5}
                fill="url(#priceGrad)"
                dot={false}
                isAnimationActive={false}
              />
            </AreaChart>
          </ResponsiveContainer>
        ) : (
          <div className="h-full flex items-center justify-center text-bb-dim text-[10px]">
            AWAITING DATA...
          </div>
        )}
      </div>
    </div>
  );
});
