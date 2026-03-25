"use client";

import { memo, useEffect, useState } from "react";

interface HeaderProps {
  isConnected: boolean;
}

export const Header = memo(function Header({ isConnected }: HeaderProps) {
  const [time, setTime] = useState("");
  const [date, setDate] = useState("");

  useEffect(() => {
    const update = () => {
      const now = new Date();
      setTime(now.toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" }));
      setDate(now.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "2-digit" }).toUpperCase());
    };
    update();
    const id = setInterval(update, 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <header className="bg-bb-panel h-9 px-4 flex items-center justify-between text-[10px] select-none border-b border-bb-border shrink-0">
      {/* Left: Logo + Name */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 bg-bb-orange flex items-center justify-center">
            <span className="text-bb-black font-bold text-[12px]">S</span>
          </div>
          <div className="flex flex-col">
            <span className="text-bb-orange font-bold tracking-widest text-[11px] leading-none">SIMMONS</span>
            <span className="text-bb-dim text-[7px] tracking-wider leading-none mt-0.5">DUAL BRAIN v3.0</span>
          </div>
        </div>
      </div>

      {/* Center: Connection Status */}
      <div className="flex items-center gap-6 text-[9px]">
        <div className="flex items-center gap-2">
          <span className="text-bb-dim">ENGINE</span>
          <div className="flex items-center gap-1.5">
            <span className={`w-2 h-2 rounded-full ${isConnected ? "bg-bb-green pulse" : "bg-bb-red"}`} />
            <span className={isConnected ? "text-bb-green font-medium" : "text-bb-red"}>
              {isConnected ? "ONLINE" : "OFFLINE"}
            </span>
          </div>
        </div>
        <div className="h-3 w-px bg-bb-border" />
        <div className="flex items-center gap-2">
          <span className="text-bb-dim">WS</span>
          <span className={isConnected ? "text-bb-green" : "text-bb-red"}>
            {isConnected ? "CONNECTED" : "DISCONNECTED"}
          </span>
        </div>
      </div>

      {/* Right: Date/Time + Live Status */}
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-3 text-[9px]">
          <span className="text-bb-dim">{date}</span>
          <span className="text-bb-bright font-semibold stat-value tracking-wider">{time}</span>
        </div>
        <div className={`px-2 py-0.5 text-[8px] font-bold tracking-wider ${
          isConnected
            ? "bg-bb-green/20 text-bb-green border border-bb-green/30"
            : "bg-bb-red/20 text-bb-red border border-bb-red/30 blink"
        }`}>
          {isConnected ? "● LIVE" : "● OFFLINE"}
        </div>
      </div>
    </header>
  );
});
