"use client";

import { memo, useMemo, useState, useEffect } from "react";
import { ResponsiveContainer, AreaChart, Area, YAxis, ReferenceLine, Tooltip } from "recharts";
import type { PortfolioData } from "@/lib/types";

interface PortfolioChartProps {
  portfolio: PortfolioData | null;
}

function generateEquityCurve(equity: number, pnlPct: number): { time: number; value: number }[] {
  const now = Date.now();
  const points = 100;
  const interval = 60000; // 1 min
  const startEquity = equity / (1 + pnlPct / 100);
  const data: { time: number; value: number }[] = [];

  for (let i = 0; i < points; i++) {
    const t = now - (points - i) * interval;
    const progress = i / points;
    const noise =
      (Math.sin(i * 0.5) * 0.3 + Math.sin(i * 1.2) * 0.2 + Math.sin(i * 2.5) * 0.15) *
      startEquity *
      0.003;
    const base = startEquity + (equity - startEquity) * progress;
    data.push({ time: t, value: base + noise });
  }

  return data;
}

export const PortfolioChart = memo(function PortfolioChart({ portfolio }: PortfolioChartProps) {
  const equity = portfolio?.equity ?? 1000;
  const pnlPct = portfolio?.pnl_pct ?? 0;
  const pnl = portfolio?.pnl ?? 0;
  const capital = portfolio?.capital ?? 1000;
  const sharpe = portfolio?.sharpe_ratio ?? 0;
  const winRate = portfolio?.win_rate ?? 0;
  const isUp = pnl >= 0;

  const data = useMemo(() => generateEquityCurve(equity, pnlPct), [equity, pnlPct]);
  const [animatedData, setAnimatedData] = useState<typeof data>([]);

  // Animate in
  useEffect(() => {
    const timer = setTimeout(() => setAnimatedData(data), 50);
    return () => clearTimeout(timer);
  }, [data]);

  const startValue = data[0]?.value ?? capital;

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <div className="flex items-center gap-2">
          <span className="grid-cell-title">PORTFOLIO</span>
          <span className="text-[8px] text-bb-dim">EQUITY CURVE</span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-bb-bright font-bold text-[12px] stat-value">
            ${equity.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
          </span>
          <span className={`text-[10px] stat-value ${isUp ? "text-bb-green" : "text-bb-red"}`}>
            {isUp ? "▲" : "▼"} {isUp ? "+" : ""}{pnl.toFixed(2)} ({pnlPct.toFixed(2)}%)
          </span>
        </div>
      </div>

      {/* Stats row */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-bb-border bg-bb-panel text-[9px] shrink-0">
        <div className="flex items-center gap-4">
          <div>
            <span className="text-bb-dim">Capital: </span>
            <span className="text-bb-white stat-value">${capital.toLocaleString()}</span>
          </div>
          <div>
            <span className="text-bb-dim">Sharpe: </span>
            <span className={sharpe >= 1 ? "text-bb-green" : sharpe >= 0 ? "text-bb-amber" : "text-bb-red"}>
              {sharpe.toFixed(2)}
            </span>
          </div>
          <div>
            <span className="text-bb-dim">Win Rate: </span>
            <span className={winRate >= 50 ? "text-bb-green" : "text-bb-red"}>
              {winRate.toFixed(0)}%
            </span>
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          <span className={`w-2 h-2 rounded-sm ${isUp ? "bg-bb-green" : "bg-bb-red"}`} />
          <span className={`text-[8px] font-medium ${isUp ? "text-bb-green" : "text-bb-red"}`}>
            {isUp ? "PROFIT" : "LOSS"}
          </span>
        </div>
      </div>

      <div className="flex-1 min-h-0 p-1">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={animatedData} margin={{ top: 4, right: 4, bottom: 0, left: 0 }}>
            <defs>
              <linearGradient id="portfolioGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={isUp ? "#00DD55" : "#FF2222"} stopOpacity={0.25} />
                <stop offset="100%" stopColor={isUp ? "#00DD55" : "#FF2222"} stopOpacity={0} />
              </linearGradient>
            </defs>
            <YAxis
              domain={["dataMin - 10", "dataMax + 10"]}
              orientation="right"
              tick={{ fill: "#555", fontSize: 8, fontFamily: "IBM Plex Mono" }}
              axisLine={false}
              tickLine={false}
              width={45}
              tickFormatter={(v) => `$${v.toFixed(0)}`}
            />
            <Tooltip
              contentStyle={{
                background: "#0a0a0a",
                border: "1px solid #1a1a1a",
                fontSize: 9,
                fontFamily: "IBM Plex Mono",
                color: "#bbb",
                padding: "4px 8px",
              }}
              formatter={(v: number) => [`$${v.toFixed(2)}`, "Equity"]}
              labelFormatter={() => ""}
            />
            <ReferenceLine y={startValue} stroke="#333" strokeDasharray="3 3" />
            <Area
              type="monotone"
              dataKey="value"
              stroke={isUp ? "#00DD55" : "#FF2222"}
              strokeWidth={1.5}
              fill="url(#portfolioGrad)"
              dot={false}
              isAnimationActive={true}
              animationDuration={800}
              animationEasing="ease-out"
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
});
