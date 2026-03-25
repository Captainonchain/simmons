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
    <header className="bg-bb-panel h-8 px-3 flex items-center justify-between text-[10px] select-none border-b border-bb-border shrink-0">
      {/* Left: Logo + Name */}
      <div className="flex items-center gap-2">
        <div className="w-5 h-5 bg-bb-orange flex items-center justify-center">
          <span className="text-bb-black font-bold text-[11px]">S</span>
        </div>
        <span className="text-bb-orange font-bold tracking-wider">SIMMONS</span>
        <span className="text-bb-dim">│</span>
        <span className="text-bb-amber text-[9px]">AUTONOMOUS AI TRADING</span>
      </div>

      {/* Center: Status indicators */}
      <div className="flex items-center gap-4 text-[9px]">
        <div className="flex items-center gap-1.5">
          <span className="text-bb-dim">MCP</span>
          <span className={isConnected ? "text-bb-green" : "text-bb-red"}>
            {isConnected ? "CONNECTED" : "DISCONNECTED"}
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          <span className="text-bb-dim">WS</span>
          <span className={`w-1.5 h-1.5 rounded-full ${isConnected ? "bg-bb-green pulse" : "bg-bb-red"}`} />
        </div>
      </div>

      {/* Right: Date/Time */}
      <div className="flex items-center gap-3">
        <span className="text-bb-dim">{date}</span>
        <span className="text-bb-bright font-medium stat-value">{time}</span>
        <div className="flex items-center gap-1">
          {isConnected ? (
            <span className="text-bb-green text-[9px] font-bold">● LIVE</span>
          ) : (
            <span className="text-bb-red text-[9px] font-bold blink">● OFFLINE</span>
          )}
        </div>
      </div>
    </header>
  );
});
