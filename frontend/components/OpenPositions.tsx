"use client";

import { memo } from "react";

interface Position {
  symbol: string;
  side: "long" | "short";
  entry_price: number;
  current_price: number;
  size_pct: number;
  pnl: number;
  pnl_pct: number;
  opened_at: string;
}

interface OpenPositionsProps {
  positions: Position[];
  isLive?: boolean;
}

export const OpenPositions = memo(function OpenPositions({
  positions = [],
  isLive = true,
}: OpenPositionsProps) {
  const formatTime = (ts: string) => {
    const date = new Date(ts);
    return date.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit" });
  };

  const formatPrice = (price: number) => {
    if (price >= 1000) return price.toLocaleString(undefined, { maximumFractionDigits: 0 });
    if (price >= 1) return price.toFixed(2);
    return price.toFixed(4);
  };

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <div className="flex items-center gap-2">
          <span className="grid-cell-title">OPEN POSITIONS</span>
          {isLive && <span className="w-1.5 h-1.5 bg-bb-green rounded-full pulse" />}
        </div>
        <span className="text-[10px] text-bb-dim">{positions.length} active</span>
      </div>

      <div className="flex-1 overflow-auto">
        {positions.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <div className="text-bb-dim text-[10px] uppercase tracking-wider mb-1">No Open Positions</div>
              <div className="text-bb-muted text-[10px]">Waiting for signals...</div>
            </div>
          </div>
        ) : (
          <div className="divide-y divide-bb-border">
            {positions.map((pos, idx) => {
              const isLong = pos.side === "long";
              const isProfit = pos.pnl >= 0;

              return (
                <div key={idx} className="p-3 hover:bg-bb-raised/50 transition-colors">
                  {/* Symbol + Side */}
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className={`text-[9px] px-1.5 py-0.5 font-bold border ${
                        isLong
                          ? "text-bb-green bg-bb-green/10 border-bb-green/30"
                          : "text-bb-red bg-bb-red/10 border-bb-red/30"
                      }`}>
                        {isLong ? "LONG" : "SHORT"}
                      </span>
                      <span className="text-bb-bright text-[11px] font-semibold">{pos.symbol}</span>
                    </div>
                    <span className="text-[9px] text-bb-dim">{formatTime(pos.opened_at)}</span>
                  </div>

                  {/* Prices */}
                  <div className="grid grid-cols-3 gap-2 mb-2">
                    <div>
                      <div className="text-[8px] text-bb-dim uppercase">Entry</div>
                      <div className="text-[10px] text-bb-white font-medium">${formatPrice(pos.entry_price)}</div>
                    </div>
                    <div>
                      <div className="text-[8px] text-bb-dim uppercase">Current</div>
                      <div className="text-[10px] text-bb-cyan font-medium">${formatPrice(pos.current_price)}</div>
                    </div>
                    <div>
                      <div className="text-[8px] text-bb-dim uppercase">Size</div>
                      <div className="text-[10px] text-bb-white font-medium">{(pos.size_pct * 100).toFixed(0)}%</div>
                    </div>
                  </div>

                  {/* P&L Bar */}
                  <div className="flex items-center gap-2">
                    <div className="flex-1 h-[4px] bg-bb-border overflow-hidden relative">
                      {/* Center line */}
                      <div className="absolute top-0 bottom-0 left-1/2 w-px bg-bb-dim/50" />
                      {/* P&L bar */}
                      <div
                        className={`absolute top-0 bottom-0 transition-all duration-300 ${
                          isProfit ? "bg-bb-green" : "bg-bb-red"
                        }`}
                        style={{
                          left: isProfit ? "50%" : `${50 + Math.max(-50, pos.pnl_pct * 5)}%`,
                          width: `${Math.min(50, Math.abs(pos.pnl_pct) * 5)}%`,
                        }}
                      />
                    </div>
                    <span className={`text-[11px] font-bold stat-value min-w-[55px] text-right ${
                      isProfit ? "text-bb-green" : "text-bb-red"
                    }`}>
                      {isProfit ? "+" : ""}{pos.pnl_pct.toFixed(2)}%
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
});
