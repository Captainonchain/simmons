"use client";

import { memo } from "react";

interface StatCardProps {
  label: string;
  value: string;
  change?: string;
  changeUp?: boolean;
}

export const StatCard = memo(function StatCard({ label, value, change, changeUp }: StatCardProps) {
  return (
    <div className="bg-bb-surface border border-bb-border px-2 py-1.5">
      <div className="text-[9px] text-bb-dim uppercase tracking-wider">{label}</div>
      <div className="text-[16px] font-bold text-bb-bright leading-tight">{value}</div>
      {change && (
        <div className={`text-[10px] ${changeUp === true ? "text-bb-green" : changeUp === false ? "text-bb-red" : "text-bb-dim"}`}>
          {change}
        </div>
      )}
    </div>
  );
});
