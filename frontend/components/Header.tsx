"use client";

import { memo, useEffect, useState } from "react";

interface HeaderProps {
  isConnected: boolean;
  onToggleSidebar?: () => void;
}

export const Header = memo(function Header({ isConnected, onToggleSidebar }: HeaderProps) {
  const [time, setTime] = useState("");
  const [date, setDate] = useState("");

  useEffect(() => {
    const update = () => {
      const now = new Date();
      setTime(now.toLocaleTimeString("en-US", { hour12: false }));
      setDate(now.toLocaleDateString("en-US", { month: "short", day: "2-digit", year: "numeric" }).toUpperCase());
    };
    update();
    const id = setInterval(update, 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <header className="bg-bb-panel border-b border-bb-border h-11 px-2 sm:px-3 flex items-center justify-between text-[10px] sm:text-[11px] select-none shrink-0">
      <div className="flex items-center min-w-0">
        {/* Mobile menu toggle */}
        <button onClick={onToggleSidebar} className="lg:hidden text-bb-orange px-2 py-1 -ml-1 mr-1 text-[16px] active:opacity-70">≡</button>
        <div className="flex items-center gap-2 shrink-0 mr-3">
          <img src="/logo1.png" alt="Simmons" className="h-8 w-8 rounded" />
          <span className="text-bb-orange font-bold tracking-wider text-[13px]">SIMMONS</span>
        </div>
        <span className="text-bb-dim hidden sm:inline mr-3">│</span>
        <span className="text-bb-amber hidden sm:inline truncate text-[10px]">AUTONOMOUS AI TRADING ON X LAYER</span>
      </div>
      <div className="flex items-center gap-2 sm:gap-4 shrink-0">
        <span className="text-bb-dim hidden md:inline">{date}</span>
        <span className="text-bb-white">{time}</span>
        <span className="text-bb-dim hidden sm:inline">│</span>
        {isConnected ? (
          <span className="text-bb-green"><span className="blink">●</span> <span className="hidden sm:inline">LIVE</span></span>
        ) : (
          <span className="text-bb-red"><span className="blink">●</span> <span className="hidden sm:inline">DC</span></span>
        )}
      </div>
    </header>
  );
});
